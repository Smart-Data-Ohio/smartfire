use crate::helpers as h;

use jiff::Timestamp;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AgentStep {
    pub name: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Poll {
    pub id: i64,
    pub room_id: i64,
    pub anonymous: bool,
    pub multiple: bool,
    pub closed: bool,
    pub closes_at: Option<Timestamp>,
    pub options: Vec<PollOption>,
    pub votes: Vec<PollVote>,
    #[serde(default)]
    pub vote_error: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PollOption {
    pub id: i64,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PollVote {
    pub option_id: i64,
    pub user_id: i64,
    pub user_name: Option<String>,
}

// Ruby sprintf rounds decimal ties to even. Scaling a large float before rounding
// loses fractional seconds, so round its shortest decimal representation instead.
pub fn step_seconds(ms: i64) -> String {
    let value = ms as f64 / 1000.0;
    // At this magnitude the float grid is 1/16s or coarser. Decimal half-ties
    // are exactly representable; shortest strings can choose an adjacent tenth.
    if value >= (1_u64 << 48) as f64 {
        return format!("{value:.1}s");
    }
    let seconds = value.to_string();
    let (whole, fraction) = seconds.split_once('.').unwrap_or((&seconds, ""));
    let tenth = fraction.bytes().next().map_or(0, |digit| u64::from(digit - b'0'));
    let round_up = fraction.as_bytes().get(1).is_some_and(|&digit| {
        digit > b'5' || (digit == b'5' && (tenth % 2 == 1 || fraction.bytes().skip(2).any(|digit| digit != b'0')))
    });
    // Non-negative i64 milliseconds yield finite seconds below 1e16, fitting u64 tenths.
    let tenths = whole.parse::<u64>().expect("finite step seconds") * 10 + tenth + u64::from(round_up);
    format!("{}.{}s", tenths / 10, tenths % 10)
}
pub fn votes_label(count: usize) -> String {
    format!("{count} vote{}", if count == 1 { "" } else { "s" })
}
impl AgentStep {
    pub fn status_label(&self) -> String {
        let status = self.status.replace('_', " ");
        let mut chars = status.chars();
        chars.next().map_or(String::new(), |first| {
            first.to_uppercase().to_string() + &chars.as_str().to_lowercase()
        })
    }
    pub fn duration(&self) -> String {
        self.duration_ms.map_or(String::new(), |ms| {
            if ms < 1000 {
                format!("{ms}ms")
            } else {
                step_seconds(ms)
            }
        })
    }
    pub fn input(&self) -> Option<&str> {
        self.input_summary
            .as_deref()
            .filter(|value| !h::is_blank(value))
    }
    pub fn output(&self) -> Option<&str> {
        self.output_summary
            .as_deref()
            .filter(|value| !h::is_blank(value))
    }
}

impl Poll {
    pub fn option_votes(&self, option_id: &i64) -> Vec<&PollVote> {
        self.votes
            .iter()
            .filter(|vote| vote.option_id == *option_id)
            .collect()
    }
    pub fn percent(&self, option_id: &i64) -> usize {
        if self.votes.is_empty() {
            0
        } else {
            (self.option_votes(option_id).len() as f64 * 100.0 / self.votes.len() as f64).round()
                as usize
        }
    }
    pub fn voter_ids(&self, option_id: &i64) -> String {
        if self.anonymous {
            String::new()
        } else {
            self.option_votes(option_id)
                .iter()
                .map(|vote| vote.user_id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    }
    pub fn voter_names(&self, option_id: &i64) -> String {
        let mut names: Vec<_> = self
            .option_votes(option_id)
            .iter()
            .filter_map(|vote| vote.user_name.as_deref())
            .collect();
        names.sort();
        match names.as_slice() {
            [] => String::new(),
            [one] => one.to_string(),
            [one, two] => format!("{one} and {two}"),
            many => format!(
                "{}, and {}",
                many[..many.len() - 1].join(", "),
                many.last().unwrap()
            ),
        }
    }
    pub fn count_label(&self, option_id: &i64) -> String {
        votes_label(self.option_votes(option_id).len())
    }
    pub fn total_label(&self) -> String {
        votes_label(self.votes.len())
    }
    pub fn results_path(&self) -> String {
        campfire_routes::room_poll(self.room_id, self.id)
    }
}

//! The agent-step and poll children of the shared message tree. Domain owners supply facts;
//! these views contain no viewer capabilities, selected ballot or session token.
use crate::{ViewContext, helpers as h};
use askama::Template;
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
                format!("{:.1}s", ms as f64 / 1000.0)
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

#[derive(Template)]
#[template(path = "agent_steps/_steps.html")]
struct Steps<'a> {
    steps: &'a [AgentStep],
}
pub fn steps(steps: &[AgentStep]) -> h::Html {
    h::raw(Steps { steps }.render().expect("agent_steps/steps renders"))
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
    pub fn form_open(&self, class: &str) -> h::Html {
        h::form_with(campfire_routes::vote_room_poll(self.room_id, self.id))
            .class(class)
            .authenticity_token(false)
            .open()
    }
    pub fn close_time(&self, ctx: &ViewContext) -> h::Html {
        self.closes_at.map_or_else(h::empty, |time| {
            crate::time::local_datetime_tag(
                &ctx.time_zone,
                time,
                "datetime",
                h::attrs().class("poll__close-time"),
                "",
            )
        })
    }
    pub fn input(&self, option_id: &i64) -> h::Html {
        let options = h::attrs()
            .id(format!("poll_{}_option_{option_id}", self.id))
            .class("poll__input")
            .data("poll_target", "input");
        if self.multiple {
            h::legacy_tag(
                "input",
                h::attrs()
                    .type_("checkbox")
                    .name("option_ids[]")
                    .id("option_ids_")
                    .value(option_id.to_string())
                    .merge(options),
            )
        } else {
            h::radio_button_tag("option_ids[]", &option_id.to_string(), false, options)
        }
    }
    pub fn submit(&self, retract: bool) -> h::Html {
        let value = if retract { "Retract vote" } else { "Vote" };
        let attrs = h::attrs()
            .type_("submit")
            .name("commit")
            .value(value)
            .class(if retract {
                "btn btn--plain btn--small"
            } else {
                "btn btn--primary btn--small"
            })
            .data("poll_target", if retract { "retract" } else { "submit" });
        h::legacy_tag(
            "input",
            if retract {
                attrs.data("disable_with", value).hidden()
            } else {
                attrs.data("disable_with", value)
            },
        )
    }
}
fn votes_label(count: usize) -> String {
    format!("{count} vote{}", if count == 1 { "" } else { "s" })
}

#[derive(Template)]
#[template(path = "polls/_poll.html")]
struct PollPartial<'a> {
    ctx: &'a ViewContext<'a>,
    poll: &'a Poll,
}
pub fn poll(ctx: &ViewContext, poll: &Poll) -> h::Html {
    h::raw(PollPartial { ctx, poll }.render().expect("polls/poll renders"))
}

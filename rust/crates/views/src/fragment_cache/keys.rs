//! Keys from Github::PullRequestsHelper and users/sidebars/show. Owners supply preloaded data.
use crate::time::Zone;
use jiff::Timestamp;
use sha2::{Digest, Sha256};

/// Rails PR #148 (`fix/cached-fragment-csrf`).
pub const PRESENTATION_CACHE_VERSION: i64 = 3;

/// The subset of ActiveSupport::Cache.expand_cache_key used by our view fragments.
#[derive(Clone, Debug)]
pub enum Key {
    Null,
    Bool(bool),
    Integer(i64),
    Text(String),
    Time(Timestamp),
    Array(Vec<Key>),
}

pub fn expand(key: &Key, zone: &Zone) -> String {
    match key {
        Key::Null => String::new(),
        Key::Bool(value) => value.to_string(),
        Key::Integer(value) => value.to_string(),
        Key::Text(value) => value.clone(),
        // Time#to_param joins Time#to_a, including DST and the zone's abbreviation.
        Key::Time(value) => format!(
            "{}/{}/{}",
            zone.format(*value, "%-S/%-M/%-H/%-d/%-m/%Y/%w/%-j"),
            zone.tz().to_offset_info(*value).dst().is_dst(),
            zone.tz().to_offset_info(*value).abbreviation()
        ),
        Key::Array(values) => values
            .iter()
            .map(|value| expand(value, zone))
            .collect::<Vec<_>>()
            .join("/"),
    }
}

pub fn fragment(template: &str, digest: &str, key: &Key, zone: &Zone) -> String {
    format!("views/{template}:{digest}/{}", expand(key, zone))
}

#[derive(Clone, Debug, Default)]
pub struct MessageKey {
    pub record: String,
    pub cards: Vec<Timestamp>,
    pub embeds: Vec<(i64, Option<Timestamp>)>,
    pub has_pull_requests: bool,
    pub pr_threads_stamp: Option<Timestamp>,
    pub pins: Vec<Timestamp>,
    pub thread_messages_count: Option<i64>,
    pub poll: Option<Timestamp>,
    pub system_note: bool,
    pub streaming: bool,
    pub agent_steps: Vec<Timestamp>,
    /// `max(updated_at, edited_at)` for each source, and the creator/room names.
    pub quotes: Vec<(Timestamp, String, String)>,
}

fn stamp(value: Option<Timestamp>) -> Key {
    value.map(Key::Time).unwrap_or(Key::Null)
}

/// `message_with_pr_cards_cache_key`, including the optional embeds and PR-thread slots.
pub fn message_with_pr_cards(data: &MessageKey) -> Key {
    let mut values = vec![
        Key::Text(data.record.clone()),
        stamp(data.cards.iter().copied().max()),
    ];
    if !data.embeds.is_empty() {
        values.push(Key::Array(
            data.embeds
                .iter()
                .map(|(id, time)| Key::Array(vec![Key::Integer(*id), stamp(*time)]))
                .collect(),
        ));
    }
    if data.has_pull_requests {
        values.push(stamp(data.pr_threads_stamp));
    }
    values.extend([
        stamp(data.pins.iter().copied().max()),
        data.thread_messages_count
            .map(Key::Integer)
            .unwrap_or(Key::Null),
        stamp(data.poll),
        Key::Bool(data.system_note),
        Key::Bool(data.streaming),
        stamp(data.agent_steps.iter().copied().max()),
        stamp(data.quotes.iter().map(|(time, _, _)| *time).max()),
        if data.quotes.is_empty() {
            Key::Null
        } else {
            Key::Text(quote_names_digest(
                &data
                    .quotes
                    .iter()
                    .map(|(_, creator, room)| (creator.clone(), room.clone()))
                    .collect::<Vec<_>>(),
            ))
        },
        Key::Integer(PRESENTATION_CACHE_VERSION),
    ]);
    Key::Array(values)
}

pub fn sidebar_membership(
    record: &str,
    participant_ids: Option<&[i64]>,
    administrator: bool,
) -> Key {
    Key::Array(vec![
        Key::Text(record.into()),
        participant_ids
            .map(|ids| Key::Array(ids.iter().copied().map(Key::Integer).collect()))
            .unwrap_or(Key::Null),
        Key::Bool(administrator),
    ])
}

/// Ruby Array#inspect, used by message_quote_names_digest instead of JSON encoding.
pub fn quote_names_inspect(names: &[(String, String)]) -> String {
    fn inspect(value: &str) -> String {
        let mut output = String::from("\"");
        let mut chars = value.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '"' => output.push_str("\\\""),
                '\\' => output.push_str("\\\\"),
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                '\x07' => output.push_str("\\a"),
                '\x08' => output.push_str("\\b"),
                '\x0b' => output.push_str("\\v"),
                '\x0c' => output.push_str("\\f"),
                '\x1b' => output.push_str("\\e"),
                '#' if chars
                    .peek()
                    .is_some_and(|next| matches!(next, '{' | '$' | '@')) =>
                {
                    output.push_str("\\#")
                }
                ch if ch.is_control() => output.push_str(&format!("\\u{:04X}", ch as u32)),
                ch => output.push(ch),
            }
        }
        output.push('"');
        output
    }
    let mut names = names.to_vec();
    names.sort();
    format!(
        "[{}]",
        names
            .iter()
            .map(|(creator, room)| format!("[{}, {}]", inspect(creator), inspect(room)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn quote_names_digest(names: &[(String, String)]) -> String {
    format!("{:x}", Sha256::digest(quote_names_inspect(names)))
}

/// Direct rooms have nil names in Ruby's quote-name digest. This additive adapter keeps the
/// existing named-room API and the MessageKey fields stable for other view owners.
pub fn quote_names_digest_nullable(names: &[(String, Option<String>)]) -> Result<String, &'static str> {
    let mut names = names.to_vec();
    names.sort();
    if names.windows(2).any(|pair| pair[0].0 == pair[1].0 && pair[0].1.is_none() != pair[1].1.is_none()) {
        return Err("comparison of Array with Array failed");
    }
    let pairs = names.into_iter().map(|(user, room)| {
        let one = quote_names_inspect(&[(user, room.clone().unwrap_or_default())]);
        let pair = one.strip_prefix('[').unwrap().strip_suffix(']').unwrap();
        if room.is_some() { pair.to_owned() }
        else { pair.strip_suffix("\"\"]").unwrap().to_owned() + "nil]" }
    }).collect::<Vec<_>>();
    Ok(format!("{:x}", Sha256::digest(format!("[{}]", pairs.join(", ")))))
}

//! Quiet, transactional page writes (`app/models/slack/message_writer.rb`).
use super::runner::{add, array, integer, present, string};
use super::{markdown, users};
use campfire_db::models::slack_import::{IssueLevel, SlackImport};
use campfire_db::{
    ChannelThread, Membership, Message, NewChannelThread, NewMessage, Result, Room,
    ThreadMembership, Timestamp, Tx, User,
};
use campfire_richtext::markdown::{Icon, IconResolver};
use indexmap::{IndexMap, IndexSet};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::HashSet;

const SKIPPED: &[&str] = &[
    "channel_join",
    "channel_leave",
    "channel_topic",
    "channel_purpose",
    "channel_name",
    "channel_archive",
    "channel_unarchive",
    "group_join",
    "group_leave",
    "group_topic",
    "group_purpose",
    "group_name",
    "group_archive",
    "group_unarchive",
    "pinned_item",
    "bot_add",
    "bot_remove",
    "tombstone",
    "message_deleted",
    "huddle_thread",
];
#[derive(Clone, Copy, Default)]
pub struct Bounds {
    pub oldest: Option<f64>,
    pub latest: Option<f64>,
}
impl Bounds {
    pub fn from_value(v: &Value) -> Self {
        Self {
            oldest: v["oldest"].as_f64(),
            latest: v["latest"].as_f64(),
        }
    }
    fn contains(self, ts: &str) -> Result<bool> {
        let value = ts.parse::<f64>().map_err(|_| {
            campfire_db::Error::Other(format!("invalid value for convert(): {ts:?}"))
        })?;
        Ok(self.oldest.is_none_or(|v| value >= v) && self.latest.is_none_or(|v| value <= v))
    }
}
pub fn slack_time(ts: &str) -> Result<Timestamp> {
    let (negative, ts) = ts.strip_prefix('-').map_or((false, ts), |s| (true, s));
    let ts = ts.strip_prefix('+').unwrap_or(ts);
    let (whole, fraction) = ts.split_once('.').unwrap_or((ts, ""));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(campfire_db::Error::Other(format!(
            "invalid value for convert(): {ts:?}"
        )));
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    let mut digits: String = fraction.chars().take(6).collect();
    while digits.len() < 6 {
        digits.push('0');
    }
    let micro = whole
        .checked_mul(1_000_000)
        .and_then(|v| v.checked_add(digits.parse::<i64>().ok()?))
        .ok_or_else(|| campfire_db::Error::Other("Slack timestamp out of range".into()))?;
    let micro = if negative {
        -micro
            - i64::from(
                fraction
                    .get(6..)
                    .is_some_and(|s| s.bytes().any(|c| c != b'0')),
            )
    } else {
        micro
    };
    Ok(Timestamp::from_microsecond(micro))
}
fn unwrap(original: &Value) -> Value {
    if original["subtype"] == "message_changed" && original["message"].is_object() {
        let mut message = original["message"].clone();
        if present(&message["ts"]).is_none() {
            message["ts"] = original["ts"].clone();
        }
        message
    } else {
        original.clone()
    }
}
fn counts() -> Value {
    json!({"messages":0,"replies":0,"threads":0,"reactions":0,"pins":0,"files_linked":0,"skipped":0})
}
fn issue(tx: &mut Tx<'_>, run: &SlackImport, key: &str, message: &str) -> Result<()> {
    SlackImport::record_issue(tx, run.id, IssueLevel::Warning, Some(key), message)?;
    Ok(())
}
fn enrich(
    tx: &Tx<'_>,
    run: &SlackImport,
    messages: &[Value],
    users: &mut IndexMap<String, User>,
) -> Result<()> {
    static MENTIONS: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<@([A-Z0-9]+)(?:\|[^>]+)?>").unwrap());
    let mut ids = IndexSet::new();
    for original in messages {
        let message = unwrap(original);
        let mut parts = vec![string(&message["text"])];
        for attachment in array(&message["attachments"]) {
            for key in ["pretext", "text", "fallback"] {
                parts.push(string(&attachment[key]));
            }
        }
        for capture in MENTIONS.captures_iter(&parts.join("\n")) {
            if !users.contains_key(&capture[1]) {
                ids.insert(capture[1].to_owned());
            }
        }
    }
    let ids: Vec<Value> = ids.into_iter().map(Value::String).collect();
    users.extend(users::users_for(tx.conn(), run, &ids)?);
    Ok(())
}
struct Planned {
    source: Value,
    ts: String,
    key: String,
    author: i64,
    converted: markdown::Converted,
    created: Timestamp,
    edited: Option<Timestamp>,
}
struct PageScope<'a> {
    conversation: &'a str,
    bounds: Bounds,
    known: &'a HashSet<String>,
    history: bool,
}
fn prepare(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    original: &Value,
    users: &mut IndexMap<String, User>,
    counts: &mut Value,
    scope: PageScope<'_>,
) -> Result<Option<Planned>> {
    let PageScope {
        conversation,
        bounds,
        known,
        history,
    } = scope;
    let source = unwrap(original);
    let ts = string(&source["ts"]);
    let key = format!("{conversation}:{ts}");
    let subtype = string(&source["subtype"]);
    if ts.trim().is_empty()
        || known.contains(&key)
        || SKIPPED.contains(&subtype.as_str())
        || (history && subtype == "thread_broadcast")
        || !bounds.contains(&ts)?
    {
        add(&mut counts["skipped"], 1);
        return Ok(None);
    }
    let author = if let Some(key) = present(&source["user"]) {
        if !users.contains_key(&key) {
            users.insert(key.clone(), users::ensure_author(tx, run, &key)?);
        }
        users[&key].id
    } else {
        users::bot_user(tx, run, &source)?.id
    };
    let names = users
        .iter()
        .map(|(id, user)| (id.clone(), user.name.clone()))
        .collect();
    let converted = markdown::try_convert(&source, &names)?;
    if converted.markdown.trim().is_empty() {
        add(&mut counts["skipped"], 1);
        return Ok(None);
    }
    let created = slack_time(&ts)?;
    let edited = present(&source["edited"]["ts"])
        .or_else(|| present(&source["edited_ts"]))
        .map(|s| slack_time(&s))
        .transpose()?;
    Ok(Some(Planned {
        source,
        ts,
        key,
        author,
        converted,
        created,
        edited,
    }))
}
fn known(
    tx: &Tx<'_>,
    run: &SlackImport,
    conversation: &str,
    messages: &[Value],
) -> Result<HashSet<String>> {
    let mut keys = HashSet::new();
    for message in messages {
        let key = format!("{conversation}:{}", string(&message["ts"]));
        if users::mapped_id(tx.conn(), run.slack_workspace_id, "message", &key)?.is_some() {
            keys.insert(key);
        }
    }
    Ok(keys)
}
fn create(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    room: &Room,
    planned: &Planned,
    thread: Option<i64>,
    reply: Option<i64>,
) -> Result<Message> {
    let message = Message::create_imported(
        tx,
        NewMessage {
            room_id: room.id,
            creator_id: planned.author,
            markdown_source: Some(planned.converted.markdown.clone()),
            thread_id: thread,
            reply_to_message_id: reply,
            ..Default::default()
        },
        planned.created,
        planned.edited,
    )?;
    users::record(
        tx,
        run,
        "message",
        &planned.key,
        "Message",
        message.id,
        true,
    )?;
    if planned.converted.truncated {
        issue(
            tx,
            run,
            &planned.key,
            "Message exceeded 50000 characters and was truncated",
        )?;
    }
    Ok(message)
}
fn leaves(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    room: &Room,
    conversation: &str,
    written: (&Message, &Planned),
    users: &mut IndexMap<String, User>,
    counts: &mut Value,
) -> Result<()> {
    let (message, planned) = written;
    let mut truncated = Vec::new();
    for reaction in array(&planned.source["reactions"]) {
        let name = string(&reaction["name"]);
        let base = if let Some((base, tone)) = name.rsplit_once("::skin-tone-")
            && !tone.is_empty()
            && tone.bytes().all(|c| c.is_ascii_digit())
        {
            base
        } else {
            &name
        };
        let candidate = format!(":{base}:");
        let icon = crate::rich_text::icons(tx.conn())
            .map_err(campfire_db::Error::Other)?
            .find(base);
        let (content, valid) = match icon {
            Some(Icon::Emoji(character)) => (character, true),
            Some(Icon::Brand { name, .. } | Icon::Custom { name, .. }) => {
                (format!(":{name}:"), true)
            }
            None => (
                candidate.clone(),
                !base.is_empty()
                    && base
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
                    && candidate.chars().count() <= 16,
            ),
        };
        let reactors = array(&reaction["users"]);
        if integer(&reaction["count"]) > reactors.len() as i64 && !truncated.contains(&name) {
            truncated.push(name.clone());
        }
        for reactor in reactors {
            let reactor = string(reactor);
            let key = format!(
                "{conversation}:{}:{base}:{reactor}",
                string(&planned.source["ts"])
            );
            if !valid {
                issue(
                    tx,
                    run,
                    &key,
                    &format!(
                        "Skipped reaction :{name}: on {conversation}:{}: unknown emoji",
                        string(&planned.source["ts"])
                    ),
                )?;
                continue;
            }
            if users::mapped_id(tx.conn(), run.slack_workspace_id, "reaction", &key)?.is_some() {
                continue;
            }
            if !users.contains_key(&reactor) {
                users.insert(reactor.clone(), users::ensure_author(tx, run, &reactor)?);
            }
            let id=tx.conn().query_row("INSERT INTO boosts (message_id,booster_id,content,created_at,updated_at) VALUES (?,?,?,?,?) RETURNING id",params![message.id,users[&reactor].id,content,message.created_at,tx.now()],|r|r.get(0))?;
            users::record(tx, run, "reaction", &key, "Boost", id, true)?;
            add(&mut counts["reactions"], 1);
        }
    }
    if !truncated.is_empty() {
        issue(
            tx,
            run,
            &planned.key,
            &format!(
                "Slack truncated the reaction list on {conversation}:{} ({}); imported the listed users only",
                string(&planned.source["ts"]),
                truncated.join(", ")
            ),
        )?;
    }
    if array(&planned.source["pinned_to"]).contains(&json!(conversation))
        && users::mapped_id(tx.conn(), run.slack_workspace_id, "pin", &planned.key)?.is_none()
    {
        let count: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM message_pins WHERE room_id=?",
            [room.id],
            |r| r.get(0),
        )?;
        if count >= 50 {
            issue(
                tx,
                run,
                &planned.key,
                &format!(
                    "Room {} already has 50 pins; extras skipped",
                    room.name.as_deref().unwrap_or("")
                ),
            )?;
        } else {
            let id=tx.conn().query_row("INSERT INTO message_pins (message_id,room_id,pinner_id,created_at,updated_at) VALUES (?,?,?,?,?) RETURNING id",params![message.id,room.id,run.user_id,message.created_at,tx.now()],|r|r.get(0))?;
            users::record(tx, run, "pin", &planned.key, "MessagePin", id, true)?;
            add(&mut counts["pins"], 1);
        }
    }
    Ok(())
}
pub struct History {
    pub counts: Value,
    pub parents: Vec<Value>,
    pub samples: Vec<Value>,
}
pub fn history(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    room: &Room,
    conversation: &str,
    messages: &[Value],
    bounds: Bounds,
    users: &mut IndexMap<String, User>,
) -> Result<History> {
    let mut counts = counts();
    let mut parents = Vec::new();
    enrich(tx, run, messages, users)?;
    let known = known(tx, run, conversation, messages)?;
    for original in messages {
        let message = unwrap(original);
        let ts = string(&message["ts"]);
        let key = format!("{conversation}:{ts}");
        if !ts.is_empty()
            && integer(&message["reply_count"]) > 0
            && bounds.contains(&ts)?
            && !SKIPPED.contains(&string(&message["subtype"]).as_str())
            && message["subtype"] != "thread_broadcast"
            && known.contains(&key)
            && let Some(id) = users::mapped_id(tx.conn(), run.slack_workspace_id, "message", &key)?
            && Message::find_by_id(tx.conn(), id)?.is_some()
        {
            parents.push(json!({"ts":ts,"message_id":id}));
        }
    }
    let mut planned = Vec::new();
    for message in messages {
        if let Some(p) = prepare(
            tx,
            run,
            message,
            users,
            &mut counts,
            PageScope {
                conversation,
                bounds,
                known: &known,
                history: true,
            },
        )? {
            planned.push(p);
        }
    }
    for p in planned {
        let message = create(tx, run, room, &p, None, None)?;
        add(&mut counts["messages"], 1);
        add(&mut counts["files_linked"], p.converted.files_linked as i64);
        leaves(
            tx,
            run,
            room,
            conversation,
            (&message, &p),
            users,
            &mut counts,
        )?;
        if integer(&p.source["reply_count"]) > 0 && bounds.contains(&p.ts)? {
            parents.push(json!({"ts":p.ts,"message_id":message.id}));
        }
    }
    Ok(History {
        counts,
        parents,
        samples: Vec::new(),
    })
}
fn deleted_thread(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    key: &str,
    state: &mut Value,
    message: &str,
) -> Result<Option<i64>> {
    if state["deleted_reported"] != true {
        state["deleted_reported"] = json!(true);
        issue(tx, run, key, message)?;
    }
    Ok(None)
}
fn ensure_thread(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    room: &Room,
    conversation: &str,
    parent_ts: &str,
    parent_id: i64,
    state: &mut Value,
) -> Result<Option<i64>> {
    let key = format!("{conversation}:{parent_ts}");
    let Some(parent) = Message::find_by_id(tx.conn(), parent_id)? else {
        return deleted_thread(
            tx,
            run,
            &key,
            state,
            &format!(
                "Parent message {key} was deleted after import; skipped its thread's new replies"
            ),
        );
    };
    let id = state["thread_id"].as_i64().or(users::mapped_id(
        tx.conn(),
        run.slack_workspace_id,
        "thread",
        &key,
    )?);
    if let Some(id) = id {
        state["thread_id"] = json!(id);
        if ChannelThread::find_by_id(tx.conn(), id)?.is_some() {
            return Ok(Some(id));
        }
        return deleted_thread(
            tx,
            run,
            &key,
            state,
            &format!("Thread {key} was deleted after import; skipped its new replies"),
        );
    }
    let thread = ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: room.id,
            creator_id: run.user_id,
            parent_message_id: Some(parent_id),
            last_activity_at: Some(parent.created_at),
            ..Default::default()
        },
    )?;
    users::record(tx, run, "thread", &key, "ChannelThread", thread.id, true)?;
    state["thread_id"] = json!(thread.id);
    state["created_now"] = json!(true);
    Ok(Some(thread.id))
}
pub struct Replies<'a> {
    pub conversation: &'a str,
    pub parent_ts: &'a str,
    pub parent_id: i64,
    pub messages: &'a [Value],
    pub bounds: Bounds,
    pub state: &'a mut Value,
}
pub fn replies(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    room: &Room,
    page: Replies<'_>,
    users: &mut IndexMap<String, User>,
) -> Result<Value> {
    let mut counts = counts();
    enrich(tx, run, page.messages, users)?;
    let known = known(tx, run, page.conversation, page.messages)?;
    let mut planned = Vec::new();
    for message in page.messages {
        if message["ts"] == page.parent_ts {
            continue;
        }
        if let Some(p) = prepare(
            tx,
            run,
            message,
            users,
            &mut counts,
            PageScope {
                conversation: page.conversation,
                bounds: page.bounds,
                known: &known,
                history: false,
            },
        )? {
            planned.push(p);
        }
    }
    let thread = if !room.direct() && !planned.is_empty() {
        let id = ensure_thread(
            tx,
            run,
            room,
            page.conversation,
            page.parent_ts,
            page.parent_id,
            page.state,
        )?;
        if id.is_none() {
            add(&mut counts["skipped"], planned.len() as i64);
            return Ok(counts);
        }
        id
    } else {
        None
    };
    for p in planned {
        let message = create(
            tx,
            run,
            room,
            &p,
            thread,
            room.direct().then_some(page.parent_id),
        )?;
        if let Some(thread) = thread {
            if page.state["created_now"] == true {
                add(&mut counts["threads"], 1);
                page.state["created_now"] = json!(false);
            }
            let key = format!(
                "{}:{}:follow:{}",
                page.conversation, page.parent_ts, p.author
            );
            if users::mapped_id(tx.conn(), run.slack_workspace_id, "thread_membership", &key)?
                .is_none()
                && ThreadMembership::find_by_thread_and_user(tx.conn(), thread, p.author)?.is_none()
                && Membership::find_by_room_and_user(tx.conn(), room.id, p.author)?.is_some()
            {
                let follow = ThreadMembership::join(tx, thread, p.author)?;
                users::record(
                    tx,
                    run,
                    "thread_membership",
                    &key,
                    "ThreadMembership",
                    follow.id,
                    true,
                )?;
            }
        }
        add(&mut counts["replies"], 1);
        add(&mut counts["files_linked"], p.converted.files_linked as i64);
        leaves(
            tx,
            run,
            room,
            page.conversation,
            (&message, &p),
            users,
            &mut counts,
        )?;
    }
    Ok(counts)
}
pub fn finish_thread(
    tx: &Tx<'_>,
    run: &SlackImport,
    conversation: &str,
    parent_ts: &str,
) -> Result<()> {
    let key = format!("{conversation}:{parent_ts}");
    if let Some(id) = users::mapped_id(tx.conn(), run.slack_workspace_id, "thread", &key)?
        && let Some(thread) = ChannelThread::find_by_id(tx.conn(), id)?
    {
        ChannelThread::refresh_messages_count(tx, id)?;
        tx.conn().execute("UPDATE channel_threads SET last_activity_at=COALESCE((SELECT MAX(created_at) FROM messages WHERE thread_id=?),(SELECT created_at FROM messages WHERE id=?),created_at) WHERE id=?",params![id,thread.parent_message_id,id])?;
    }
    Ok(())
}
pub fn dry_history(
    tx: &Tx<'_>,
    run: &SlackImport,
    conversation_name: &str,
    messages: &[Value],
    bounds: Bounds,
    remaining: usize,
) -> Result<History> {
    let mut counts = counts();
    let mut samples = Vec::new();
    let mut users = IndexMap::new();
    enrich(tx, run, messages, &mut users)?;
    let names = users
        .iter()
        .map(|(id, user)| (id.clone(), user.name.clone()))
        .collect();
    for original in messages {
        let message = unwrap(original);
        let ts = string(&message["ts"]);
        let subtype = string(&message["subtype"]);
        if ts.trim().is_empty()
            || SKIPPED.contains(&subtype.as_str())
            || subtype == "thread_broadcast"
            || !bounds.contains(&ts)?
        {
            add(&mut counts["skipped"], 1);
            continue;
        }
        let converted = markdown::try_convert(&message, &names)?;
        if converted.markdown.trim().is_empty() {
            add(&mut counts["skipped"], 1);
            continue;
        }
        add(&mut counts["messages"], 1);
        add(&mut counts["files_linked"], converted.files_linked as i64);
        if integer(&message["reply_count"]) > 0 {
            add(&mut counts["threads"], 1);
            add(&mut counts["replies"], integer(&message["reply_count"]));
        }
        if samples.len() < remaining {
            samples.push(json!({"conversation":conversation_name,"slack_text":truncate(&string(&message["text"]),200),"markdown":truncate(&converted.markdown,500)}));
        }
    }
    Ok(History {
        counts,
        parents: Vec::new(),
        samples,
    })
}
fn truncate(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        s.into()
    } else {
        format!("{}...", s.chars().take(limit - 3).collect::<String>())
    }
}

#[cfg(test)]
mod tests;

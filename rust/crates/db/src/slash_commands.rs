//! Non-agent chat commands from app/services/slash_commands at fec615be.
//! Call in the request's write transaction; the HTTP membership boundary belongs to WS8b.
use crate::broadcasts::{self, Broadcast, Partial, Streamable, TurboAction, TurboStream};
use crate::{
    ChannelThread, Error, Errors, Event, Membership, Message, NewMessage, Result, Room, SavedItem,
    Timestamp, Tx,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
pub mod time_parser;
mod user_settings;
use time_parser::{WEEKDAYS, end_of_day, present, re, strip, zone};

pub const SHRUG: &str = "¯\\_(ツ)_/¯";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub arg_hint: String,
    pub takes_arguments: bool,
    pub root: bool,
    pub thread: bool,
}
pub fn registry() -> Vec<Command> {
    [
        ("huddle", "Start a call in this room", "", false),
        (
            "event",
            "Open the event form prefilled",
            "<title> <when>",
            false,
        ),
        ("poll", "Open the poll builder", "", false),
        (
            "remind",
            "Post and remind yourself about it later",
            "<when> <text>",
            true,
        ),
        ("status", "Set your custom status", "<emoji> <text>", true),
        (
            "dnd",
            "Toggle Do Not Disturb, optionally for a while",
            "[duration|off]",
            true,
        ),
        (
            "ooo",
            "Set out of office with an optional note",
            "<when> [note]|off",
            true,
        ),
        ("shrug", "Post with a shrug", "[text]", true),
        ("me", "Post an action line", "<action>", true),
        ("play", "Play a chat sound", "<sound>", true),
    ]
    .into_iter()
    .map(|(name, description, arg_hint, takes_arguments)| Command {
        name: name.into(),
        description: description.into(),
        arg_hint: arg_hint.into(),
        takes_arguments,
        root: true,
        thread: name != "poll",
    })
    .collect()
}
pub fn lookup(name: &str) -> Option<Command> {
    registry()
        .into_iter()
        .find(|c| c.name == name.to_lowercase())
}
pub fn available(thread: bool) -> Vec<Command> {
    registry()
        .into_iter()
        .filter(|c| !thread || c.thread)
        .collect()
}
fn command_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        re(r"(?s)\A/(?P<name>[a-zA-Z][a-zA-Z0-9_-]*)(?:[ \t\r\n\x0b\x0c]+(?P<args>.*))?\z")
    })
}
pub fn command_text(text: &str) -> bool {
    command_pattern().is_match(strip(text))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResult {
    pub kind: String,
    pub message: Option<String>,
    pub url: Option<String>,
    pub notice: Option<String>,
    #[serde(skip)]
    pub message_id: Option<i64>,
    #[serde(skip)]
    pub room_id: Option<i64>,
}
impl CommandResult {
    fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            message: None,
            url: None,
            notice: None,
            message_id: None,
            room_id: None,
        }
    }
    fn error(message: impl Into<String>) -> Self {
        let mut r = Self::new("error");
        r.message = Some(message.into());
        r
    }
    fn ephemeral(message: impl Into<String>) -> Self {
        let mut r = Self::new("ephemeral");
        r.message = Some(message.into());
        r
    }
    fn posted(message: &Message) -> Self {
        let mut r = Self::new("posted");
        r.message_id = Some(message.id);
        r
    }
    /// Same payload shape as Registry::Result, including its posted/launch IDs.
    pub fn payload(&self) -> Value {
        if let Some(id) = self.message_id {
            json!({"message_id":id})
        } else if let Some(id) = self.room_id {
            json!({"room_id":id})
        } else {
            json!({})
        }
    }
}
#[derive(Debug, Clone)]
pub struct Context {
    pub user_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub huddles_configured: bool,
}
pub fn dispatch(tx: &mut Tx<'_>, context: &Context, text: &str) -> Result<CommandResult> {
    let Some(c) = command_pattern().captures(strip(text)) else {
        return Ok(CommandResult::error("Type / to see available commands."));
    };
    let name = c["name"].to_ascii_lowercase();
    let args = strip(c.name("args").map(|m| m.as_str()).unwrap_or(""));
    let Some(command) = lookup(&name) else {
        let mut names = available(context.thread_id.is_some())
            .iter()
            .map(|c| format!("/{}", c.name))
            .collect::<Vec<_>>();
        // Agent dispatch is WS11. Include its ordered registrations in Rails' error list.
        let mut stmt = tx
            .conn()
            .prepare("SELECT name FROM agent_slash_commands WHERE room_id=? ORDER BY name")?;
        names.extend(
            stmt.query_map([context.room_id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .map(|n| format!("/{n}")),
        );
        return Ok(CommandResult::error(format!(
            "Unknown command “/{name}”. Available: {}.",
            names.join(", ")
        )));
    };
    if context.thread_id.is_some() && !command.thread {
        return Ok(CommandResult::error(format!(
            "“/{name}” is only available in the channel, not in threads."
        )));
    }
    // Errors from model writes leave the transaction failed; convert validation results
    // only after the command's savepoint has rolled back its rows and deferred effects.
    match tx.savepoint(|tx| handle(tx, context, &name, args)) {
        Ok(result) => Ok(result),
        Err(Error::RecordInvalid(errors)) if name != "dnd" => {
            Ok(CommandResult::error(sentence(errors.full_messages())))
        }
        Err(Error::Other(message)) if message == crate::models::channel_thread::LOCKED_MESSAGE => {
            Ok(CommandResult::error("This thread is locked."))
        }
        Err(error) => Err(error),
    }
}
fn sentence(messages: Vec<String>) -> String {
    match messages.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        _ => format!(
            "{}, and {}",
            messages[..messages.len() - 1].join(", "),
            messages.last().unwrap()
        ),
    }
}
fn user_zone(tx: &Tx<'_>, id: i64) -> Result<String> {
    Ok(tx
        .conn()
        .query_row("SELECT time_zone FROM users WHERE id=?", [id], |r| {
            r.get::<_, Option<String>>(0)
        })?
        .filter(|s| present(s).is_some())
        .unwrap_or_else(|| "UTC".into()))
}
fn long(time: Timestamp, zone_name: &str) -> String {
    time.jiff()
        .to_zoned(zone(zone_name))
        .strftime("%B %d, %Y %H:%M")
        .to_string()
}
fn date_long(time: Timestamp, zone_name: &str) -> String {
    time.jiff()
        .to_zoned(zone(zone_name))
        .strftime("%B %d, %Y")
        .to_string()
}
fn past(time: Timestamp, zone_name: &str) -> CommandResult {
    CommandResult::error(format!("“{}” is in the past.", long(time, zone_name)))
}
fn handle(tx: &mut Tx<'_>, c: &Context, name: &str, args: &str) -> Result<CommandResult> {
    let zone_name = user_zone(tx, c.user_id)?;
    match name {
        "poll" => Ok(CommandResult::new("open_poll")),
        "huddle" => Ok(huddle_stub(c)),
        "event" => Ok(event_stub(c, args, &zone_name, tx.now())),
        "play" => {
            let m = post(tx, c, strip(&format!("/play {args}")), false)?;
            Ok(CommandResult::posted(&m))
        }
        "shrug" => {
            let body = if present(args).is_none() {
                SHRUG.to_string()
            } else {
                format!("{args} {SHRUG}")
            };
            let m = post(tx, c, &body, false)?;
            Ok(CommandResult::posted(&m))
        }
        "me" => {
            if present(args).is_none() {
                return Ok(CommandResult::error(
                    "Usage: /me <action> — for example “/me is reviewing the deploy”.",
                ));
            }
            let m = post(tx, c, args, true)?;
            Ok(CommandResult::posted(&m))
        }
        "remind" => {
            if present(args).is_none() {
                return Ok(CommandResult::error(
                    "Usage: /remind <when> <text> — for example “/remind in 20 minutes review the deploy”.",
                ));
            }
            let Some((time, Some(text))) =
                time_parser::split_leading_time(args, &zone_name, tx.now())
            else {
                return Ok(CommandResult::error(
                    "Usage: /remind <when> <text> — for example “/remind tomorrow 9am file expenses”.",
                ));
            };
            if time <= tx.now() {
                return Ok(past(time, &zone_name));
            }
            let message = post(tx, c, &text, false)?;
            SavedItem::save_for(tx, c.user_id, message.id, Some(time))?;
            let mut result = CommandResult::posted(&message);
            result.notice = Some(format!("Reminder set for {}.", long(time, &zone_name)));
            Ok(result)
        }
        "status" => {
            let mut pieces = args.splitn(2, |ch: char| {
                matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
            });
            let emoji = pieces.next().unwrap_or("");
            let text = pieces
                .next()
                .map(|s| {
                    s.trim_start_matches(|ch: char| {
                        matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
                    })
                })
                .unwrap_or("");
            if present(emoji).is_none() || present(text).is_none() {
                return Ok(CommandResult::error(
                    "Usage: /status <emoji> <text> — for example “/status 🚂 On a train”.",
                ));
            }
            let expiry = end_of_day(
                tx.now().jiff().to_zoned(zone(&zone_name)).date(),
                &zone(&zone_name),
            )
            .ok_or_else(|| Error::Other("date out of range".into()))?;
            user_settings::update(
                tx,
                c.user_id,
                json!({"custom_status_emoji":emoji,"custom_status_text":text,"custom_status_expires_at":expiry.to_db()}),
            )?;
            Ok(CommandResult::ephemeral(format!(
                "Status set to “{emoji} {text}”."
            )))
        }
        "dnd" => {
            let Some((action, time)) = dnd_action(args, &zone_name, tx.now()) else {
                return Ok(CommandResult::error(
                    "Usage: /dnd [30m|2h|until 5pm|off] — bare /dnd toggles.",
                ));
            };
            let active = if action == "toggle" {
                let (enabled, until): (bool, Option<Timestamp>) = tx.conn().query_row(
                    "SELECT dnd_enabled,dnd_until FROM users WHERE id=?",
                    [c.user_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                !(enabled && until.is_none_or(|t| t > tx.now()))
            } else {
                action == "on"
            };
            user_settings::update(
                tx,
                c.user_id,
                json!({"dnd_enabled":active,"dnd_until":time.map(|t|t.to_db())}),
            )?;
            Ok(CommandResult::ephemeral(if !active {
                "Do Not Disturb is off.".into()
            } else if let Some(time) = time {
                format!("Do Not Disturb is on until {}.", long(time, &zone_name))
            } else {
                "Do Not Disturb is on.".into()
            }))
        }
        "ooo" => {
            if args.eq_ignore_ascii_case("off") {
                user_settings::update(tx, c.user_id, json!({"ooo_until":null,"ooo_note":null}))?;
                let until = calendar_ooo_until(tx, c.user_id)?;
                claim_ooo(tx, c.user_id, until.is_some())?;
                broadcast_ooo(tx, c.user_id)?;
                return Ok(CommandResult::ephemeral(if let Some(until) = until {
                    format!(
                        "Manual out of office is off. Your calendar still shows you out until {}.",
                        date_long(until, &zone_name)
                    )
                } else {
                    "Out of office is off.".into()
                }));
            }
            let Some((time, note)) = ooo_time_and_note(args, &zone_name, tx.now()) else {
                return Ok(CommandResult::error(
                    "Usage: /ooo <when> [note] — for example “/ooo tomorrow Back soon”, “/ooo friday”, “/ooo 2026-10-05”, or “/ooo 3d”. Bare days and dates run to the end of the day; “/ooo friday 5pm” keeps the time. “/ooo off” clears it.",
                ));
            };
            if time <= tx.now() {
                return Ok(past(time, &zone_name));
            }
            user_settings::update(
                tx,
                c.user_id,
                json!({"ooo_until":time.to_db(),"ooo_note":note}),
            )?;
            claim_ooo(tx, c.user_id, true)?;
            broadcast_ooo(tx, c.user_id)?;
            let effective = calendar_ooo_until(tx, c.user_id)?
                .map(|t| t.max(time))
                .unwrap_or(time);
            let mut message = format!("Out of office until {}.", date_long(effective, &zone_name));
            if let Some(note) = note {
                message.push_str(&format!(" Note: “{note}”."));
            }
            Ok(CommandResult::ephemeral(message))
        }
        _ => unreachable!("registry has a handler for every entry"),
    }
}
/// WS13 owns launch execution. Rails returns a launch result, and creates no huddle here.
pub fn huddle_stub(c: &Context) -> CommandResult {
    if c.huddles_configured {
        let mut r = CommandResult::new("start_huddle");
        r.room_id = Some(c.room_id);
        r
    } else {
        CommandResult::error("Huddles are not configured in this workspace.")
    }
}
/// WS14 owns the event form/save. This adapter preserves Rails' deferred, prefilled URL.
pub fn event_stub(c: &Context, args: &str, zone_name: &str, now: Timestamp) -> CommandResult {
    let mut r = CommandResult::new("open_url");
    let path = format!("/rooms/{}/events/new", c.room_id);
    if present(args).is_none() {
        r.url = Some(path);
        return r;
    }
    let (title, time) = time_parser::split_trailing_time(args, zone_name, now);
    let Some(title) = title.filter(|t| present(t).is_some()) else {
        return CommandResult::error(
            "Usage: /event <title> <when> — for example “/event Launch party friday 5pm”.",
        );
    };
    if let Some(time) = time
        && time <= now
    {
        return past(time, zone_name);
    }
    // Rails' nested query keys are sorted (starts_at, time_zone, title).
    let mut pairs = Vec::new();
    if let Some(t) = time {
        pairs.push((
            "event[starts_at]",
            t.jiff().strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
        ));
    }
    pairs.push(("event[time_zone]", zone_name.into()));
    pairs.push(("event[title]", title));
    let query = pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", form_encode(k), form_encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    r.url = Some(format!("{path}?{query}"));
    r
}
fn form_encode(text: &str) -> String {
    let mut s = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                s.push(b as char)
            }
            b' ' => s.push('+'),
            _ => s.push_str(&format!("%{b:02X}")),
        }
    }
    s
}
fn post(tx: &mut Tx<'_>, c: &Context, text: &str, action: bool) -> Result<Message> {
    let attrs = NewMessage {
        room_id: c.room_id,
        creator_id: c.user_id,
        markdown_source: Some(text.into()),
        action,
        ..Default::default()
    };
    let message = if let Some(thread) = c.thread_id {
        ChannelThread::find(tx.conn(), thread)?.post_message(tx, c.user_id, attrs)?
    } else {
        Message::create(tx, attrs)?
    };
    // Slash messages have no attachment assignment: both process_attachment calls are no-ops.
    let room = Room::find(tx.conn(), message.room_id)?;
    let target = if let Some(thread) = message.thread_id {
        broadcasts::dom_id("channel_thread", thread, Some("messages"))
    } else {
        broadcasts::room_dom_id(&room, Some("messages"))
    };
    tx.emit_after_commit(Event::Broadcast(Broadcast::append(
        broadcasts::conversation_messages(tx.conn(), &message)?,
        target,
        Partial::Message {
            message_id: message.id,
        },
    )));
    if message.thread_id.is_none() {
        let mentioned = message
            .mentionees(tx.conn(), tx.rich_text())?
            .iter()
            .map(|u| u.id)
            .collect::<Vec<_>>();
        for member in Membership::for_room(tx.conn(), room.id)? {
            if member.involvement != Some(crate::Involvement::Muted)
                || mentioned.contains(&member.user_id)
            {
                tx.emit_after_commit(Event::Broadcast(Broadcast::Cable {
                    stream: format!("user_{}_unreads", member.user_id),
                    payload: json!({"roomId":room.id}),
                }));
            }
        }
        let recipients = if room.direct() {
            room.users(tx.conn())?
        } else {
            message.mentionees(tx.conn(), tx.rich_text())?
        };
        for bot in recipients {
            if bot.id != message.creator_id
                && bot.is_active()
                && bot.is_bot()
                && !tx.conn().query_row(
                    "SELECT EXISTS(SELECT 1 FROM agents WHERE user_id=?)",
                    [bot.id],
                    |r| r.get::<_, bool>(0),
                )?
            {
                bot.deliver_webhook_later(tx, message.id)?;
            }
        }
    }
    Ok(message)
}
fn elapsed(now: Timestamp, n: i64, unit: &str) -> Option<Timestamp> {
    if n <= 0 {
        return None;
    }
    let seconds = n.checked_mul(match unit.bytes().next()? {
        b'm' => 60,
        b'h' => 3600,
        b'd' => 86400,
        _ => 604800,
    })?;
    Some(Timestamp::from_jiff(
        now.jiff()
            .checked_add(jiff::SignedDuration::from_secs(seconds))
            .ok()?,
    ))
}
fn dnd_action(args: &str, zone: &str, now: Timestamp) -> Option<(&'static str, Option<Timestamp>)> {
    if present(args).is_none() {
        return Some(("toggle", None));
    }
    if args.eq_ignore_ascii_case("off") {
        return Some(("off", None));
    }
    if args.eq_ignore_ascii_case("on") {
        return Some(("on", None));
    }
    if let Some(c) =
        re(r"(?i)\A(?P<n>[0-9]+)\s*(?P<unit>m(?:ins?)?|minutes?|h(?:rs?)?|hours?|d(?:ays?)?)\z")
            .captures(args)
    {
        return Some((
            "on",
            Some(elapsed(
                now,
                c["n"].parse().ok()?,
                &c["unit"].to_ascii_lowercase(),
            )?),
        ));
    }
    let text = re(r"(?i)\Auntil\s+").replace(args, "");
    let time = time_parser::parse(&text, zone, now)?;
    (time > now).then_some(("on", Some(time)))
}
fn ooo_time_and_note(
    args: &str,
    zone_name: &str,
    now: Timestamp,
) -> Option<(Timestamp, Option<String>)> {
    if let Some(c)=re(r"(?is)\A(?P<n>[0-9]+)\s*(?P<unit>w(?:eeks?)?|m(?:ins?)?|minutes?|h(?:rs?)?|hours?|d(?:ays?)?)\b(?P<rest>.*)\z").captures(args){return Some((elapsed(now,c["n"].parse().ok()?,&c["unit"].to_ascii_lowercase())?,present(strip(&c["rest"]))));}
    ooo_bare_day(args, zone_name, now)
        .or_else(|| time_parser::split_leading_time(args, zone_name, now))
}
fn ooo_bare_day(
    args: &str,
    zone_name: &str,
    now: Timestamp,
) -> Option<(Timestamp, Option<String>)> {
    let zone = zone(zone_name);
    let date = now.jiff().to_zoned(zone.clone()).date();
    let day_pattern = format!(
        r"(?is)\A(?P<token>today|tomorrow|(?:next\s+)?(?:{}))(?P<rest>\s+.*|\z)",
        WEEKDAYS.join("|")
    );
    let matches = [
        re(&day_pattern),
        re(r"(?s)\A(?P<date>[0-9]{4}-[0-9]{2}-[0-9]{2})(?P<rest>\s+.*|\z)"),
        re(
            r"(?is)\A(?P<month>jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+(?P<day>[0-9]{1,2})(?:st|nd|rd|th)?(?P<rest>\s+.*|\z)",
        ),
    ];
    for pattern in &matches {
        if let Some(c) = pattern.captures(args) {
            let rest = strip(&c["rest"]);
            if re(r"(?i)\A(?:at\s+)?[0-9]{1,2}(?::[0-9]{2})?").is_match(rest) {
                continue;
            }
            let target = if let Some(token) = c.name("token") {
                let token = token
                    .as_str()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_lowercase();
                let delta = match token.as_str() {
                    "today" => 0,
                    "tomorrow" => 1,
                    _ => {
                        let next = token.starts_with("next ");
                        let day = token.strip_prefix("next ").unwrap_or(&token);
                        let target = WEEKDAYS.iter().position(|d| *d == day)? as i8;
                        let delta = (target - date.weekday().to_sunday_zero_offset()).rem_euclid(7);
                        i64::from(if delta == 0 { 7 } else { delta }) + if next { 7 } else { 0 }
                    }
                };
                date.checked_add(jiff::Span::new().days(delta)).ok()?
            } else if let Some(d) = c.name("date") {
                let pieces = d.as_str().split('-').collect::<Vec<_>>();
                time_parser::normalized_date(
                    pieces[0].parse().ok()?,
                    pieces[1].parse().ok()?,
                    pieces[2].parse().ok()?,
                )?
            } else {
                let month = time_parser::month(&c["month"])?;
                let day = c["day"].parse::<i8>().ok()?;
                let target = jiff::civil::Date::new(date.year(), month, day).ok()?;
                if end_of_day(target, &zone)? <= now {
                    jiff::civil::Date::new(date.year() + 1, month, day).ok()?
                } else {
                    target
                }
            };
            return Some((end_of_day(target, &zone)?, present(rest)));
        }
    }
    None
}
fn calendar_ooo_until(tx: &Tx<'_>, user: i64) -> Result<Option<Timestamp>> {
    let enabled: bool = tx.conn().query_row(
        "SELECT ooo_calendar_enabled FROM users WHERE id=?",
        [user],
        |r| r.get(0),
    )?;
    if !enabled {
        return Ok(None);
    }
    let intervals: Option<String> = tx
        .conn()
        .query_row(
            "SELECT ooo_intervals FROM calendar_meeting_caches WHERE user_id=?",
            [user],
            |r| r.get(0),
        )
        .optional()?;
    let pairs: Value = intervals
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    Ok(pairs
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let p = p.as_array()?;
            let start = p.first()?.as_str()?.parse::<jiff::Timestamp>().ok()?;
            let end = p.get(1)?.as_str()?.parse::<jiff::Timestamp>().ok()?;
            let now = tx.now().jiff();
            (start <= now && now < end).then(|| Timestamp::from_jiff(end))
        })
        .max())
}
fn claim_ooo(tx: &Tx<'_>, user: i64, active: bool) -> Result<()> {
    if active {
        tx.conn().execute("UPDATE users SET ooo_broadcast=1,updated_at=? WHERE id=? AND (ooo_broadcast IS NULL OR ooo_broadcast!=1)",params![tx.now(),user])?;
    } else {
        tx.conn().execute("UPDATE users SET ooo_broadcast=0,ooo_until=NULL,ooo_note=NULL,updated_at=? WHERE id=? AND (ooo_broadcast IS NULL OR ooo_broadcast!=0) AND (ooo_until IS NULL OR ooo_until<=?)",params![tx.now(),user,tx.now()])?;
    }
    Ok(())
}
fn broadcast_ooo(tx: &mut Tx<'_>, user: i64) -> Result<()> {
    for (name, target, partial) in [
        (
            "status",
            format!("status_badge_user_{user}"),
            Partial::UserStatus { user_id: user },
        ),
        (
            "ooo_notice",
            format!("ooo_notice_user_{user}"),
            Partial::OooNotice { user_id: user },
        ),
    ] {
        tx.emit_after_commit(Event::Broadcast(Broadcast::Turbo(TurboStream {
            streamables: vec![Streamable::User(user), Streamable::Name(name)],
            action: TurboAction::Update,
            target,
            partial: Some(partial),
            maintain_scroll: false,
        })));
    }
    Ok(())
}

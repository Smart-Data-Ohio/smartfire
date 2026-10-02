//! Rails activity presentation over the owner's permission-filtered ActivityItem APIs.
use campfire_db::{ActivityItem, Connection, Result, User};
use campfire_views::activity::Item;
use rusqlite::OptionalExtension;
use std::{cell::RefCell, collections::HashMap};

/// Request-local association preload, after the owner's accessibility query.
pub struct Sources {
    saved: HashMap<i64, i64>,
    messages: HashMap<i64, campfire_db::Message>,
    rooms: HashMap<i64, campfire_db::Room>,
    work_events: HashMap<i64, campfire_db::WorkThreadEvent>,
    threads: HashMap<i64, campfire_db::ChannelThread>,
    actors: HashMap<i64, User>,
    room_names: RefCell<HashMap<i64, String>>,
}
impl Sources {
    pub fn load(conn: &Connection, rows: &[ActivityItem]) -> Result<Self> {
        let saved_ids: Vec<_> = rows
            .iter()
            .filter(|row| row.source_type == "SavedItem")
            .map(|row| row.source_id)
            .collect();
        let saved: HashMap<_, _> =
            campfire_db::models::saved_item::SavedItem::for_ids(conn, &saved_ids)?
                .into_iter()
                .map(|saved| (saved.id, saved.message_id))
                .collect();
        let ids: Vec<_> = rows
            .iter()
            .filter(|row| row.source_type == "Message")
            .map(|row| row.source_id)
            .chain(saved.values().copied())
            .collect();
        let messages: HashMap<_, _> = campfire_db::Message::for_ids(conn, &ids)?
            .into_iter()
            .map(|message| (message.id, message))
            .collect();
        let work_ids = rows
            .iter()
            .filter(|row| row.source_type == "WorkThreadEvent")
            .map(|row| row.source_id)
            .collect::<Vec<_>>();
        let work_events = campfire_db::WorkThreadEvent::for_ids(conn, &work_ids)?
            .into_iter()
            .map(|event| (event.id, event))
            .collect::<HashMap<_, _>>();
        let thread_ids = work_events
            .values()
            .map(|event| event.channel_thread_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let threads = campfire_db::ChannelThread::for_ids(conn, &thread_ids)?
            .into_iter()
            .map(|thread| (thread.id, thread))
            .collect::<HashMap<_, _>>();
        let actor_ids = work_events
            .values()
            .filter_map(|event| event.actor_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let actors = if actor_ids.is_empty() {
            HashMap::new()
        } else {
            User::where_ids(conn, &actor_ids)?.into_iter().map(|user| (user.id,user)).collect()
        };
        let mut room_ids: Vec<_> = messages
            .values()
            .map(|message| message.room_id)
            .chain(threads.values().map(|thread| thread.room_id))
            .collect();
        room_ids.sort_unstable();
        room_ids.dedup();
        let rooms = campfire_db::Room::for_ids(conn, &room_ids)?
            .into_iter()
            .map(|room| (room.id, room))
            .collect();
        Ok(Self {
            saved,
            messages,
            rooms,
            work_events,
            threads,
            actors,
            room_names: RefCell::default(),
        })
    }
    fn work_event(&self, row: &ActivityItem) -> Result<&campfire_db::WorkThreadEvent> {
        self.work_events
            .get(&row.source_id)
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows.into())
    }
    fn thread(&self, id: i64) -> Result<&campfire_db::ChannelThread> {
        self.threads
            .get(&id)
            .ok_or(campfire_db::Error::RecordNotFound("ChannelThread"))
    }
    fn room_name(&self, conn: &Connection, id: i64, viewer: &User) -> Result<String> {
        self.room(id)?;
        if self.room_names.borrow().is_empty() {
            self.room_names
                .replace(campfire_db::Room::display_names_for(
                    conn,
                    &self.rooms.values().cloned().collect::<Vec<_>>(),
                    Some(viewer),
                )?);
        }
        Ok(self.room_names.borrow()[&id].clone())
    }
    fn room(&self, id: i64) -> Result<&campfire_db::Room> {
        self.rooms
            .get(&id)
            .ok_or(campfire_db::Error::RecordNotFound("Room"))
    }
    fn message(&self, row: &ActivityItem) -> Option<&campfire_db::Message> {
        let id = if row.source_type == "SavedItem" {
            *self.saved.get(&row.source_id)?
        } else {
            row.source_id
        };
        self.messages.get(&id)
    }
}

/// `ActivityItemsHelper#activity_item_source_path`, independent of the JSON source whitelist.
pub fn destination(conn: &Connection, row: &ActivityItem) -> Result<String> {
    let messages = Sources::load(conn, std::slice::from_ref(row))?;
    source_path(conn, row, &messages)
}
fn source_path(conn: &Connection, row: &ActivityItem, messages: &Sources) -> Result<String> {
    let fallback = || "/activity".to_owned();
    Ok(match row.source_type.as_str() {
        "Message" | "SavedItem" => messages
            .message(row)
            .map(|message| {
                if let Some(thread) = message.thread_id {
                    format!(
                        "/rooms/{}?thread={thread}&message_id={}",
                        message.room_id, message.id
                    )
                } else {
                    campfire_routes::room_at_message(message.room_id, message.id)
                }
            })
            .unwrap_or_else(fallback),
        "WorkThreadEvent" => messages.work_events.get(&row.source_id)
            .and_then(|event| messages.threads.get(&event.channel_thread_id))
            .map(|thread| format!("/rooms/{}?thread={}",thread.room_id,thread.id))
            .unwrap_or_else(fallback),
        "BoardSlaNudge" => {
            // FLAGGED WS12 BoardSlaNudge facts reader.
            let thread_id: Option<i64> = conn
                .query_row(
                    "SELECT channel_thread_id FROM board_sla_nudges WHERE id=?",
                    [row.source_id],
                    |r| r.get(0),
                )
                .optional()?;
            thread_id
                .map(|id| campfire_db::ChannelThread::find_by_id(conn, id))
                .transpose()?
                .flatten()
                .map(|thread| format!("/rooms/{}?thread={}", thread.room_id, thread.id))
                .unwrap_or_else(fallback)
        }
        "HuddleGrant" => {
            campfire_db::models::huddle_grant::HuddleGrant::find_by_id(conn, row.source_id)?
                .map(|grant| format!("/rooms/{}", grant.room_id))
                .unwrap_or_else(fallback)
        }
        "Event" => {
            let event =
                campfire_db::models::calendar_event::CalendarEvent::find(conn, row.source_id)?;
            format!("/rooms/{}/events/{}", event.room_id, event.id)
        }
        "AgentApproval" => campfire_db::AgentApproval::find(conn, row.source_id)?
            .map(|approval| format!("/agents/{}/approvals", approval.agent_id))
            .unwrap_or_else(fallback),
        "AgentBudgetNotice" => {
            // FLAGGED WS11 AgentBudgetNotice reader; no mutation is implemented here.
            let agent_id: Option<i64> = conn
                .query_row(
                    "SELECT agent_id FROM agent_budget_notices WHERE id=?",
                    [row.source_id],
                    |r| r.get(0),
                )
                .optional()?;
            agent_id
                .map(|id| campfire_db::Agent::find(conn, id))
                .transpose()?
                .flatten()
                .map(|agent| format!("/account/bots/{}/edit", agent.user_id))
                .unwrap_or_else(fallback)
        }
        "ScheduledMessage" => "/scheduled_messages".into(),
        "TwoFactorCredential" => "/users/me/profile".into(),
        "Session" => "/users/me/sessions".into(),
        _ => fallback(),
    })
}
pub fn item(
    conn: &Connection,
    app: &crate::app::AppState,
    item: &ActivityItem,
    viewer: &User,
    messages: &Sources,
) -> Result<Item> {
    let mut result = Item {
        id: item.id,
        state: item.state().into(),
        event_label: event_label(&item.event_type).into(),
        created_at: None,
        approval: None,
        title: "Unavailable source".into(),
        author: None,
        body: "This source is no longer available.".into(),
    };
    match item.source_type.as_str() {
        "AgentApproval" => {
            if let Some(approval) = campfire_db::AgentApproval::find(conn, item.source_id)? {
                result.created_at = Some(approval.created_at.jiff());
                result.approval = Some(super::agents::history::approval(
                    conn,
                    &app.secrets,
                    &approval,
                    viewer,
                    app.db.env().now(),
                )?);
            }
        }
        "Message" | "SavedItem" => {
            if let Some(message) = messages.message(item) {
                result.created_at = Some(message.created_at.jiff());
                result.title = messages.room_name(conn,message.room_id,viewer)?;
                if let Some(id) = message.thread_id {
                    result.title +=
                        &format!(" · {}", campfire_db::ChannelThread::find(conn, id)?.name);
                }
                result.author =
                    campfire_db::User::find_by_id(conn, message.creator_id)?.map(|u| u.name);
                result.body = message.plain_text_body(conn, &*app.db.env().rich_text)?;
                if item.event_type == "message_reminder" {
                    result.body = format!(
                        "You asked to be reminded about this message: {}",
                        result.body
                    );
                }
            }
        }
        "AgentBudgetNotice" => {
            // FLAGGED WS11 read seam: AgentBudgetNotice has no exported reader.
            // Writes and fan-out stay in owner agent_posting::check_budget.
            let (agent_id, cap, created): (i64, String, campfire_db::Timestamp) = conn.query_row(
                "SELECT agent_id,cap,created_at FROM agent_budget_notices WHERE id=?",
                [item.source_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let agent = campfire_db::Agent::find(conn, agent_id)?.ok_or_else(|| {
                campfire_db::Error::Other("Budget notice agent is missing".into())
            })?;
            let user = campfire_db::User::find(conn, agent.user_id)?;
            let (label, _) = budget_details(&agent, &cap);
            result.created_at = Some(created.jiff());
            result.title = format!("{} · daily {label} budget", user.name);
            result.body = budget_body(&agent, &user.name, &cap);
            result.author = Some(user.name);
        }
        "Event" => {
            let event =
                campfire_db::models::calendar_event::CalendarEvent::find(conn, item.source_id)?;
            result.created_at = Some(event.created_at.jiff());
            result.title = format!(
                "{} · {}",
                super::accounts::room_display_name(
                    conn,
                    &campfire_db::Room::find(conn, event.room_id)?,
                    viewer
                )?,
                event.title
            );
            result.author =
                campfire_db::User::find_by_id(conn, event.organizer_id)?.map(|u| u.name);
            result.body = event_body(conn, &event, &item.event_type)?;
        }
        "HuddleGrant" => {
            let grant =
                campfire_db::models::huddle_grant::HuddleGrant::find_by_id(conn, item.source_id)?
                    .ok_or(campfire_db::Error::RecordNotFound("HuddleGrant"))?;
            result.created_at = Some(grant.created_at.jiff());
            result.title = super::accounts::room_display_name(
                conn,
                &campfire_db::Room::find(conn, grant.room_id)?,
                viewer,
            )?;
            result.author = campfire_db::User::find_by_id(conn, grant.user_id)?.map(|u| u.name);
            let caller = result.author.as_deref().unwrap_or("Someone");
            result.body = if item.event_type == "huddle_missed" {
                format!("You missed a huddle from {caller}")
            } else {
                format!("{caller} started a huddle")
            };
        }
        "WorkThreadEvent" => {
            let event = messages.work_event(item)?;
            let thread = messages.thread(event.channel_thread_id)?;
            result.created_at = Some(event.created_at.jiff());
            result.title = format!(
                "{} · {}",
                messages.room_name(conn,thread.room_id,viewer)?,
                thread.name
            );
            result.author = Some(
                event.actor_id
                    .and_then(|id| messages.actors.get(&id))
                    .map(|user| user.name.clone())
                    .unwrap_or_else(|| "Work thread".into()),
            );
            result.body = work_event_body(event);
        }
        "BoardSlaNudge" => {
            // FLAGGED WS12 BoardSlaNudge facts reader; no writer is implemented here.
            let (thread,status,stage,entered,created) = conn.query_row("SELECT channel_thread_id,work_status,stage,status_entered_at,created_at FROM board_sla_nudges WHERE id=?",[item.source_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,campfire_db::Timestamp>(3)?,r.get::<_,campfire_db::Timestamp>(4)?)))?;
            let thread = campfire_db::ChannelThread::find(conn, thread)?;
            result.created_at = Some(created.jiff());
            result.title = format!(
                "{} · {}",
                super::accounts::room_display_name(
                    conn,
                    &campfire_db::Room::find(conn, thread.room_id)?,
                    viewer
                )?,
                thread.name
            );
            let minutes = (app.db.env().now().jiff().as_second() - entered.jiff().as_second())
                .div_euclid(60)
                .max(0);
            let age = if minutes >= 60 {
                format!("{:.1} hours", (minutes as f64 / 6.0).round() / 10.0)
            } else {
                format!("{minutes} minutes")
            };
            let status = humanize(Some(&status));
            result.body = format!(
                "{}sitting in {status} for {age}",
                if stage == "escalation" {
                    "Escalated: "
                } else {
                    ""
                }
            );
            if stage != "escalation" {
                result.body.replace_range(..1, "S");
            }
        }
        "ScheduledMessage" => {
            let scheduled = campfire_db::models::scheduled_message::ScheduledMessage::find(
                conn,
                item.source_id,
            )?;
            result.created_at = Some(scheduled.created_at.jiff());
            result.title = super::accounts::room_display_name(
                conn,
                &campfire_db::Room::find(conn, scheduled.room_id)?,
                viewer,
            )?;
            result.author = campfire_db::User::find_by_id(conn, scheduled.user_id)?.map(|u| u.name);
            result.body = if let Some(reason) = scheduled
                .drop_reason
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
            {
                format!(
                    "Your scheduled message was not sent ({reason}): {}",
                    scheduled.markdown_source
                )
            } else {
                format!(
                    "You no longer have access to this room, so your scheduled message was not sent: {}",
                    scheduled.markdown_source
                )
            };
        }
        "TwoFactorCredential" => {
            let credential = campfire_db::TwoFactorCredential::find(conn, item.source_id)?;
            result.created_at = Some(credential.created_at.jiff());
            result.title = "Two-step sign-in".into();
            result.body = "Several wrong sign-in codes were entered for your account.".into();
        }
        "Session" => {
            let session = campfire_db::Session::find(conn, item.source_id)?;
            result.created_at = Some(session.created_at.jiff());
            result.title = "Account security".into();
            result.body = session_body(item.created_at, &session);
        }
        other => {
            return Err(campfire_db::Error::Other(format!(
                "Unknown activity source: {other}"
            )));
        }
    }
    if result.body.chars().count() > 500 {
        result.body = result.body.chars().take(497).collect::<String>() + "...";
    }
    Ok(result)
}
fn event_label(kind: &str) -> &str {
    match kind {
        "mention" => "Mention",
        "reply" => "Reply",
        "keyword_alert" => "Keyword alert",
        "thread_activity" => "Followed thread",
        "work_assignment" => "Work assignment",
        "work_update" => "Work update",
        "work_sla" => "SLA breach",
        "huddle_started" => "Incoming huddle",
        "huddle_missed" => "Missed huddle",
        "event_invitation" => "Event invitation",
        "event_update" => "Event update",
        "event_cancelled" => "Event cancelled",
        "event_reminder" => "Event reminder",
        "pr_review_request" => "Review requested",
        "agent_approval_request" => "Approval request",
        "agent_budget_exceeded" => "Budget exceeded",
        "message_reminder" => "Reminder",
        "scheduled_message_dropped" => "Scheduled message not sent",
        "two_factor_lockout" => "Sign-in lockout",
        "new_sign_in" => "New sign-in",
        _ => kind,
    }
}

#[derive(serde::Serialize)]
pub struct Payload {
    id: i64,
    event_type: String,
    state: &'static str,
    read_at: Option<String>,
    handled_at: Option<String>,
    created_at: String,
    pub source: Option<Source>,
}
#[derive(serde::Serialize)]
pub struct Source {
    #[serde(rename = "type")]
    kind: String,
    id: i64,
    room_id: Option<i64>,
    thread_id: Option<i64>,
    creator_id: Option<i64>,
    body: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}
pub fn payload(
    conn: &Connection,
    app: &crate::app::AppState,
    row: &ActivityItem,
) -> Result<Payload> {
    let messages = Sources::load(conn, std::slice::from_ref(row))?;
    payload_with_sources(conn, app, row, &messages)
}
pub fn payload_with_sources(
    conn: &Connection,
    app: &crate::app::AppState,
    row: &ActivityItem,
    messages: &Sources,
) -> Result<Payload> {
    let mut source = Source {
        kind: row.source_type.clone(),
        id: row.source_id,
        room_id: None,
        thread_id: None,
        creator_id: None,
        body: String::new(),
        path: "/activity".into(),
        status: None,
    };
    let source = match row.source_type.as_str() {
        "Message" | "SavedItem" => {
            let message = messages
                .message(row)
                .ok_or(campfire_db::Error::RecordNotFound("Message"))?;
            let room = messages.room(message.room_id)?;
            source.room_id = Some(message.room_id);
            source.thread_id = message.thread_id;
            source.creator_id = Some(message.creator_id);
            source.path = if let Some(thread) = message.thread_id {
                format!(
                    "/rooms/{}?thread={thread}&message_id={}",
                    room.id, message.id
                )
            } else {
                campfire_routes::room_at_message(room.id, message.id)
            };
            source.body = message.plain_text_body(conn, &*app.db.env().rich_text)?;
            // Message JSON is always plain text; SavedItem uses the reminder helper.
            if row.source_type == "SavedItem" && row.event_type == "message_reminder" {
                source.body = format!(
                    "You asked to be reminded about this message: {}",
                    source.body
                );
            }
            Some(source)
        }
        "HuddleGrant" => {
            let grant =
                campfire_db::models::huddle_grant::HuddleGrant::find_by_id(conn, row.source_id)?
                    .ok_or(campfire_db::Error::RecordNotFound("HuddleGrant"))?;
            let caller = campfire_db::User::find_by_id(conn, grant.user_id)?
                .map(|user| user.name)
                .unwrap_or_else(|| "Someone".into());
            source.body = if row.event_type == "huddle_missed" {
                format!("You missed a huddle from {caller}")
            } else {
                format!("{caller} started a huddle")
            };
            source.room_id = Some(grant.room_id);
            source.creator_id = Some(grant.user_id);
            source.path = format!("/rooms/{}", grant.room_id);
            Some(source)
        }
        "WorkThreadEvent" => {
            let event = messages.work_event(row)?;
            source.body = work_event_body(event);
            let thread = messages.thread(event.channel_thread_id)?;
            source.room_id = Some(thread.room_id);
            source.thread_id = Some(thread.id);
            source.creator_id = event.actor_id;
            source.path = format!("/rooms/{}?thread={}", thread.room_id, thread.id);
            Some(source)
        }
        "Event" => {
            let event =
                campfire_db::models::calendar_event::CalendarEvent::find(conn, row.source_id)?;
            source.body = event_body(conn, &event, &row.event_type)?;
            source.room_id = Some(event.room_id);
            source.creator_id = Some(event.organizer_id);
            source.path = format!("/rooms/{}/events/{}", event.room_id, event.id);
            Some(source)
        }
        "AgentApproval" => {
            let approval = campfire_db::AgentApproval::find(conn, row.source_id)?
                .ok_or(campfire_db::Error::RecordNotFound("AgentApproval"))?;
            let agent = campfire_db::Agent::find(conn, approval.agent_id)?
                .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            source.room_id = approval.room_id;
            source.creator_id = Some(agent.user_id);
            source.body = approval.summary.clone();
            source.path = format!("/agents/{}/approvals", agent.id);
            source.status = Some(approval.effective_status(app.db.env().now()).into());
            Some(source)
        }
        "AgentBudgetNotice" => {
            // FLAGGED WS11 AgentBudgetNotice facts reader (owner has not exported a model).
            let (id, cap): (i64, String) = conn.query_row(
                "SELECT agent_id,cap FROM agent_budget_notices WHERE id=?",
                [row.source_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let agent = campfire_db::Agent::find(conn, id)?
                .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            let user = campfire_db::User::find(conn, agent.user_id)?;
            source.body = budget_body(&agent, &user.name, &cap);
            source.creator_id = Some(agent.user_id);
            source.path = format!("/account/bots/{}/edit", agent.user_id);
            source.status = Some(cap);
            Some(source)
        }
        "Session" => {
            let session = campfire_db::Session::find(conn, row.source_id)?;
            source.body = session_body(row.created_at, &session);
            source.path = "/users/me/sessions".into();
            Some(source)
        }
        // Rails intentionally has no JSON source branch for these three models.
        "BoardSlaNudge" | "ScheduledMessage" | "TwoFactorCredential" => None,
        _ => None,
    };
    let source = source.map(|mut source| {
        source.body = truncate(source.body);
        source
    });
    let stamp = |t: campfire_db::Timestamp| t.jiff().strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    Ok(Payload {
        id: row.id,
        event_type: row.event_type.clone(),
        state: row.state(),
        read_at: row.read_at.map(stamp),
        handled_at: row.handled_at.map(stamp),
        created_at: stamp(row.created_at),
        source,
    })
}
fn truncate(text: String) -> String {
    if text.chars().count() > 500 {
        text.chars().take(497).collect::<String>() + "..."
    } else {
        text
    }
}
fn work_event_body(event: &campfire_db::WorkThreadEvent) -> String {
    work_body(
        event.from_status.clone(),
        event.to_status.clone(),
        event.from_owner_id,
        event.to_owner_id,
        event.from_owner_name.clone(),
        event.to_owner_name.clone(),
    )
}

fn humanize(value: Option<&str>) -> String {
    let value = value
        .filter(|v| !campfire_richtext::ruby::is_blank(v))
        .unwrap_or("None")
        .replace('_', " ");
    let mut chars = value.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().to_string() + &rails_compat::unicode::downcase(chars.as_str()))
        .unwrap_or_default()
}

fn event_body(
    conn: &Connection,
    event: &campfire_db::models::calendar_event::CalendarEvent,
    kind: &str,
) -> Result<String> {
    let start = campfire_views::time::Zone::for_user(Some(&event.time_zone))
        .format(event.starts_at.jiff(), "%B %-d, %Y at %-I:%M %p %Z");
    Ok(match kind {
        "event_invitation" => {
            if let (Some(rule), Some(until)) = (
                event
                    .recurrence_rule
                    .as_deref()
                    .filter(|r| !campfire_richtext::ruby::is_blank(r)),
                event.recurrence_until,
            ) {
                format!(
                    "You are invited: {start} (repeats {} until {}).",
                    campfire_db::models::calendar_event::recurrence::phrase(rule),
                    until.strftime("%B %-d, %Y")
                )
            } else {
                format!("You are invited: {start}.")
            }
        }
        "event_update" => format!("The time changed: {start}."),
        "event_cancelled" => "This event was cancelled.".into(),
        "event_reminder" => {
            if let Some(id) = event.venue_room_id {
                format!(
                    "Starts in 15 minutes: {} in {}.",
                    event.title,
                    campfire_db::Room::find(conn, id)?.name.unwrap_or_default()
                )
            } else {
                format!("Starts in 15 minutes: {}.", event.title)
            }
        }
        _ => format!("Event updated: {start}."),
    })
}
fn work_body(
    from: Option<String>,
    to: Option<String>,
    from_id: Option<i64>,
    to_id: Option<i64>,
    from_name: Option<String>,
    to_name: Option<String>,
) -> String {
    let mut changes = Vec::new();
    if from != to {
        changes.push(format!(
            "Status: {} → {}",
            humanize(from.as_deref()),
            humanize(to.as_deref())
        ));
    }
    if from_id != to_id {
        let name = |value: Option<String>| {
            value
                .filter(|v| !campfire_richtext::ruby::is_blank(v))
                .unwrap_or_else(|| "unassigned".into())
        };
        changes.push(format!("Owner: {} → {}", name(from_name), name(to_name)));
    }
    if changes.is_empty() {
        "Work thread updated".into()
    } else {
        campfire_views::helpers::to_sentence(&changes, " and ")
    }
}
fn session_body(created_at: campfire_db::Timestamp, session: &campfire_db::Session) -> String {
    let at = created_at
        .jiff()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%B %-d, %Y at %-I:%M %p %Z");
    format!(
        "New sign-in to your account from {}, {at}. Wasn't you? Review your sessions.",
        crate::authentication::device_description(session)
    )
}
fn budget_details<'a>(agent: &campfire_db::Agent, cap: &'a str) -> (&'a str, Option<i64>) {
    match cap {
        "messages" => ("messages", agent.daily_message_cap),
        "board_posts" => ("board posts", agent.daily_board_post_cap),
        "external_actions" => ("external actions", agent.daily_external_action_cap),
        _ => (cap, None),
    }
}
fn budget_body(agent: &campfire_db::Agent, name: &str, cap: &str) -> String {
    let (label, limit) = budget_details(agent, cap);
    format!(
        "{name} hit its daily {label} budget ({}/day).",
        limit.map(|v| v.to_string()).unwrap_or_default()
    )
}

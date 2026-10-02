//! Rails activity presentation over the owner's permission-filtered ActivityItem APIs.
use campfire_db::{ActivityItem, Connection, Result, User};
use campfire_views::activity::Item;
use rusqlite::OptionalExtension;
use std::collections::HashMap;

/// Request-local association preload, after the owner's accessibility query.
pub struct MessageSources {
    saved: HashMap<i64, i64>,
    messages: HashMap<i64, campfire_db::Message>,
}
impl MessageSources {
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
        let messages = campfire_db::Message::for_ids(conn, &ids)?
            .into_iter()
            .map(|message| (message.id, message))
            .collect();
        Ok(Self { saved, messages })
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
    let messages = MessageSources::load(conn, std::slice::from_ref(row))?;
    source_path(conn, row, &messages)
}
fn source_path(conn: &Connection, row: &ActivityItem, messages: &MessageSources) -> Result<String> {
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
        "WorkThreadEvent" | "BoardSlaNudge" => {
            // FLAGGED WS12 facts readers, shared with the presentation below.
            let table = if row.source_type == "WorkThreadEvent" {
                "work_thread_events"
            } else {
                "board_sla_nudges"
            };
            let thread_id: Option<i64> = conn
                .query_row(
                    &format!("SELECT channel_thread_id FROM {table} WHERE id=?"),
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
    messages: &MessageSources,
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
                let room = campfire_db::Room::find(conn, message.room_id)?;
                result.created_at = Some(message.created_at.jiff());
                result.title = super::accounts::room_display_name(conn, &room, viewer)?;
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
            let (label, limit) = match cap.as_str() {
                "messages" => ("messages", agent.daily_message_cap),
                "board_posts" => ("board posts", agent.daily_board_post_cap),
                "external_actions" => ("external actions", agent.daily_external_action_cap),
                _ => (cap.as_str(), None),
            };
            result.created_at = Some(created.jiff());
            result.title = format!("{} · daily {label} budget", user.name);
            result.body = format!(
                "{} hit its daily {label} budget ({}/day).",
                user.name,
                limit.map(|v| v.to_string()).unwrap_or_default()
            );
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
            let start = campfire_views::time::Zone::for_user(Some(&event.time_zone))
                .format(event.starts_at.jiff(), "%B %-d, %Y at %-I:%M %p %Z");
            result.body = match item.event_type.as_str() {
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
            };
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
            // FLAGGED WS12 WorkThreadEvent facts reader; domain access stays in ActivityItem.
            let (thread,actor,from,to,from_id,to_id,from_name,to_name,created) = conn.query_row(
                "SELECT channel_thread_id,actor_id,from_status,to_status,from_owner_id,to_owner_id,from_owner_name,to_owner_name,created_at FROM work_thread_events WHERE id=?",[item.source_id],
                |r| Ok((r.get::<_,i64>(0)?,r.get::<_,Option<i64>>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<i64>>(4)?,r.get::<_,Option<i64>>(5)?,r.get::<_,Option<String>>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,campfire_db::Timestamp>(8)?)))?;
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
            result.author = Some(
                actor
                    .map(|id| campfire_db::User::find_by_id(conn, id))
                    .transpose()?
                    .flatten()
                    .map(|u| u.name)
                    .unwrap_or_else(|| "Work thread".into()),
            );
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
            result.body = if changes.is_empty() {
                "Work thread updated".into()
            } else {
                campfire_views::helpers::to_sentence(&changes, " and ")
            };
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
            let at = item
                .created_at
                .jiff()
                .to_zoned(jiff::tz::TimeZone::UTC)
                .strftime("%B %-d, %Y at %-I:%M %p %Z");
            result.body = format!(
                "New sign-in to your account from {}, {at}. Wasn't you? Review your sessions.",
                crate::authentication::device_description(&session)
            );
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
    viewer: &User,
) -> Result<Payload> {
    let messages = MessageSources::load(conn, std::slice::from_ref(row))?;
    payload_with_sources(conn, app, row, viewer, &messages)
}
pub fn payload_with_sources(
    conn: &Connection,
    app: &crate::app::AppState,
    row: &ActivityItem,
    viewer: &User,
    messages: &MessageSources,
) -> Result<Payload> {
    let view = item(conn, app, row, viewer, messages)?;
    let mut source = Source {
        kind: row.source_type.clone(),
        id: row.source_id,
        room_id: None,
        thread_id: None,
        creator_id: None,
        body: view.body,
        path: "/activity".into(),
        status: None,
    };
    let source = match row.source_type.as_str() {
        "Message" | "SavedItem" => {
            let message = messages
                .message(row)
                .ok_or(campfire_db::Error::RecordNotFound("Message"))?;
            source.room_id = Some(message.room_id);
            source.thread_id = message.thread_id;
            source.creator_id = Some(message.creator_id);
            source.path = if let Some(thread) = message.thread_id {
                format!(
                    "/rooms/{}?thread={thread}&message_id={}",
                    message.room_id, message.id
                )
            } else {
                campfire_routes::room_at_message(message.room_id, message.id)
            };
            // Message JSON uses plain text directly; SavedItem uses the reminder helper.
            if row.source_type == "Message" {
                source.body = truncate(message.plain_text_body(conn, &*app.db.env().rich_text)?);
            }
            Some(source)
        }
        "HuddleGrant" => {
            let grant =
                campfire_db::models::huddle_grant::HuddleGrant::find_by_id(conn, row.source_id)?
                    .ok_or(campfire_db::Error::RecordNotFound("HuddleGrant"))?;
            source.room_id = Some(grant.room_id);
            source.creator_id = Some(grant.user_id);
            source.path = format!("/rooms/{}", grant.room_id);
            Some(source)
        }
        "WorkThreadEvent" => {
            // FLAGGED WS12 WorkThreadEvent facts reader; access and mutations use ActivityItem.
            let (thread, actor): (i64, Option<i64>) = conn.query_row(
                "SELECT channel_thread_id,actor_id FROM work_thread_events WHERE id=?",
                [row.source_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let thread = campfire_db::ChannelThread::find(conn, thread)?;
            source.room_id = Some(thread.room_id);
            source.thread_id = Some(thread.id);
            source.creator_id = actor;
            source.path = format!("/rooms/{}?thread={}", thread.room_id, thread.id);
            Some(source)
        }
        "Event" => {
            let event =
                campfire_db::models::calendar_event::CalendarEvent::find(conn, row.source_id)?;
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
            source.body = truncate(approval.summary.clone());
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
            source.creator_id = Some(agent.user_id);
            source.path = format!("/account/bots/{}/edit", agent.user_id);
            source.status = Some(cap);
            Some(source)
        }
        "Session" => {
            source.path = "/users/me/sessions".into();
            Some(source)
        }
        // Rails intentionally has no JSON source branch for these three models.
        "BoardSlaNudge" | "ScheduledMessage" | "TwoFactorCredential" => None,
        _ => None,
    };
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

//! FLAGGED WS12 read seam: exact pinned ActivityItem.accessible_to SQL. No writes.
//! Replace the query with WS12's owner reader when available; approval expiry uses WS11.
use campfire_db::{ActivityItem, Connection, Result, User};
use campfire_views::activity::Item;
pub fn accessible(conn: &Connection, user: &User) -> Result<Vec<ActivityItem>> {
    if !user.is_active() || user.is_bot() {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(&format!(
        "{} ORDER BY activity_items.updated_at DESC, activity_items.id DESC",
        include_str!("activity_access.sql")
    ))?;
    let ids = statement
        .query_map([user.id], |r| r.get::<_, i64>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ids.into_iter()
        .map(|id| ActivityItem::find(conn, id))
        .collect()
}
pub fn state(item: &ActivityItem) -> &'static str {
    if item.handled_at.is_some() {
        "handled"
    } else if item.read_at.is_some() {
        "read"
    } else {
        "unread"
    }
}
pub fn matches_type(item: &ActivityItem, kind: &str) -> bool {
    let types: &[&str] = match kind {
        "mentions" => &["mention", "reply", "keyword_alert"],
        "threads" => &[
            "thread_activity",
            "work_update",
            "work_assignment",
            "work_sla",
        ],
        "events" => &[
            "event_invitation",
            "event_update",
            "event_cancelled",
            "event_reminder",
        ],
        "agents" => &["agent_approval_request", "agent_budget_exceeded"],
        "github" => &["pr_review_request"],
        "huddles" => &["huddle_started", "huddle_missed"],
        "reminders" => &["message_reminder"],
        "security" => &["new_sign_in", "two_factor_lockout"],
        _ => return true,
    };
    types.contains(&item.event_type.as_str())
}
pub fn item(
    conn: &Connection,
    app: &crate::app::AppState,
    item: &ActivityItem,
    viewer: &User,
) -> Result<Item> {
    let mut result = Item {
        id: item.id,
        state: state(item).into(),
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
            let message_id = if item.source_type == "SavedItem" {
                conn.query_row(
                    "SELECT message_id FROM saved_items WHERE id=?",
                    [item.source_id],
                    |r| r.get::<_, i64>(0),
                )?
            } else {
                item.source_id
            };
            if let Some(message) = campfire_db::Message::find_by_id(conn, message_id)? {
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
        "Event" => {
            type EventFacts = (
                campfire_db::Timestamp,
                i64,
                String,
                i64,
                campfire_db::Timestamp,
                Option<String>,
                Option<String>,
                Option<campfire_db::Timestamp>,
            );
            let (created,room_id,title,organizer,starts,zone,recurrence,until):EventFacts=conn.query_row(
                "SELECT created_at,room_id,title,organizer_id,starts_at,time_zone,recurrence_rule,recurrence_until FROM events WHERE id=?",
                [item.source_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
            result.created_at = Some(created.jiff());
            result.title = format!(
                "{} · {title}",
                super::accounts::room_display_name(
                    conn,
                    &campfire_db::Room::find(conn, room_id)?,
                    viewer
                )?
            );
            result.author = campfire_db::User::find_by_id(conn, organizer)?.map(|u| u.name);
            let start = campfire_views::time::Zone::for_user(zone.as_deref())
                .format(starts.jiff(), "%B %-d, %Y at %-I:%M %p %Z");
            result.body = match item.event_type.as_str() {
                "event_invitation" => {
                    if recurrence.is_some() && until.is_some() {
                        return Err(campfire_db::Error::Other(
                            "WS14e recurring event phrase reader not yet exported".into(),
                        ));
                    } else {
                        format!("You are invited: {start}.")
                    }
                }
                "event_update" => format!("The time changed: {start}."),
                "event_cancelled" => "This event was cancelled.".into(),
                _ => format!("Event updated: {start}."),
            };
        }
        // Other source presentation remains an explicit continuation, rather than
        // silently pretending a missing source has been deleted.
        other => {
            return Err(campfire_db::Error::Other(format!(
                "WS12 activity source presentation not yet ported: {other}"
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

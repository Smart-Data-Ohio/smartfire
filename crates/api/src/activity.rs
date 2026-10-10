//! The S3 activity inbox on `/api/v1` (`campfire_api_types::activity` documents each endpoint):
//! listing, the badge count, the state actions and `open`. Each reuses `activity_items`' lookup
//! (`ActivityItem::find_accessible`, `query_accessible`) and the model's state writes, whose
//! `ActivityChannel` frames the classic inbox hears and whose `activity.item` twins the other tabs
//! do.

use campfire_api_types as api;
use campfire_app::app::{AppCtx, AppState};
use campfire_db::models::activity_item::ActivityQuery;
use campfire_db::{ActivityItem, Connection, Timestamp, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_runtime::concerns;
use campfire_runtime::presenters::activity as presenter;
use campfire_runtime::context::db_error;

use crate::dto;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, validation};

endpoint!(
    /// `GET /api/v1/activity`
    index => index_activity
);
endpoint!(
    /// `GET /api/v1/activity/unread_count`
    unread_count => show_unread_count
);
endpoint!(
    /// `PATCH /api/v1/activity/:id`
    update => update_item
);
endpoint!(
    /// `POST /api/v1/activity/:id/open`
    open => open_item
);

/// `activity_items#index`' page size.
const PAGE: usize = 100;

/// The inbox rows as the wire carries them, their sources read in one batch
/// (`activity::Sources::load_spa`). A row holding a value the contract doesn't know (a new event
/// type, say), or whose source (or a row it leads to) has gone missing, is left out and logged
/// rather than failing the whole list.
pub(crate) fn items(
    conn: &Connection,
    app: &AppState,
    viewer: &User,
    rows: &[ActivityItem],
) -> campfire_db::Result<Vec<api::ActivityItem>> {
    let sources = presenter::Sources::load_spa(conn, rows)?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let (view, refs) = match presenter::spa_row(conn, app, row, viewer, &sources) {
            Ok(presented) => presented,
            Err(
                error @ (campfire_db::Error::RecordNotFound(_)
                | campfire_db::Error::Sqlite(rusqlite::Error::QueryReturnedNoRows)),
            ) => {
                tracing::warn!(%error, item_id = row.id, "activity: row with a missing source left out of the list");
                continue;
            }
            Err(error) => return Err(error),
        };
        match wire_row(row, view, refs) {
            Ok(item) => items.push(item),
            Err(error) => {
                tracing::warn!(%error, item_id = row.id, "activity: row left out of the list");
            }
        }
    }
    Ok(items)
}

fn wire_row(
    row: &ActivityItem,
    view: campfire_presentation::activity::Item,
    refs: presenter::SourceRefs,
) -> campfire_db::Result<api::ActivityItem> {
    let occurred_at = view
        .created_at
        .map(Timestamp::from_jiff)
        .unwrap_or(row.created_at);
    Ok(api::ActivityItem {
        id: row.id,
        event_type: wire(&row.event_type)?,
        state: wire(row.state())?,
        read_at: row.read_at.map(dto::time),
        handled_at: row.handled_at.map(dto::time),
        created_at: dto::time(row.created_at),
        updated_at: dto::time(row.updated_at),
        source: api::ActivitySource {
            source_type: wire(&snake_case(&row.source_type))?,
            source_id: row.source_id,
            room_id: refs.room_id,
            thread_id: refs.thread_id,
            message_id: refs.message_id,
            event_id: refs.event_id,
            creator_id: refs.creator_id,
            title: refs.approval_title.unwrap_or(view.title),
            body: refs.approval_body.unwrap_or(view.body),
            occurred_at: dto::time(occurred_at),
            approval_status: refs.approval_status.as_deref().map(wire).transpose()?,
            budget_cap: refs.budget_cap.as_deref().map(wire).transpose()?,
            path: refs.path,
        },
    })
}

/// A stored word as its wire enum (the enums' serde names are the stored values).
fn wire<T: serde::de::DeserializeOwned>(value: &str) -> campfire_db::Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|_| {
        campfire_db::Error::Other(format!(
            "an activity value the contract doesn't know: {value}"
        ))
    })
}

/// `activity_items.source_type`'s class name (`WorkThreadEvent`) as its wire form
/// (`work_thread_event`).
fn snake_case(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 4);
    for (index, letter) in class.chars().enumerate() {
        if letter.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(letter.to_ascii_lowercase());
        } else {
            out.push(letter);
        }
    }
    out
}

/// The item as `activity.item` and the state actions carry it, with the owner's unread count;
/// `Ok(None)` when they can't see it (any more).
pub(crate) fn changed(
    conn: &Connection,
    app: &AppState,
    viewer: &User,
    item_id: i64,
) -> campfire_db::Result<Option<api::ActivityItemChanged>> {
    let Some(row) = ActivityItem::find_accessible(conn, viewer, item_id)? else {
        return Ok(None);
    };
    let Some(item) = items(conn, app, viewer, std::slice::from_ref(&row))?.pop() else {
        return Ok(None);
    };
    let unread = ActivityItem::unread_snapshot(conn, viewer.id, app.db.env().now())?;
    Ok(Some(api::ActivityItemChanged {
        item,
        unread_count: unread.count,
        unread_revision: unread.revision,
        evaluated_at: unread.evaluated_at.to_evaluation_time(),
    }))
}

/// `activity_items#no_store`: the inbox is never cached.
fn no_store(c: &mut Ctx) {
    c.no_store();
    c.set_header("cache-control", "no-store");
    c.set_header("pragma", "no-cache");
}

async fn index_activity(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    no_store(c);
    let viewer = concerns::require_current_user(c)?.clone();
    let status = c
        .param_str("status")
        .filter(|status| ["unread", "read", "handled"].contains(status))
        .unwrap_or("unread")
        .to_owned();
    let tab = c
        .param_str("type")
        .filter(|tab| {
            campfire_presentation::activity::TYPES
                .iter()
                .any(|(key, _)| key == tab)
        })
        .unwrap_or("all")
        .to_owned();
    let after = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match crate::cursor::decode(raw) {
            Some(key) => Some(key),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    // Listing settles overdue huddle invitations and agent approvals first, as the page does.
    let viewer_id = viewer.id;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::huddle_invitations::resolve_overdue(tx, Some(viewer_id))?;
            campfire_db::AgentApproval::resolve_overdue(tx, Some(viewer_id))
        })
        .await
        .map_err(db_error)?;
    let (app, now) = (c.app().clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| {
            let mut rows = ActivityItem::query_accessible_after(
                conn,
                &viewer,
                ActivityQuery {
                    state: Some(&status),
                    type_filter: Some(&tab),
                    before: None,
                    limit: Some(PAGE + 1),
                },
                after,
            )?;
            let more = rows.len() > PAGE;
            rows.truncate(PAGE);
            let next_cursor = rows
                .last()
                .filter(|_| more)
                .map(|row| crate::cursor::encode(row.updated_at, row.id));
            let items = items(conn, &app, &viewer, &rows)?;
            let creators = items
                .iter()
                .filter_map(|item| item.source.creator_id)
                .collect::<std::collections::BTreeSet<_>>();
            let unread = ActivityItem::unread_snapshot(conn, viewer.id, now)?;
            Ok(api::ActivityList {
                users: dto::users(conn, &app.secrets, creators, now)?,
                unread_count: unread.count,
                unread_revision: unread.revision,
                evaluated_at: unread.evaluated_at.to_evaluation_time(),
                items,
                next_cursor,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn show_unread_count(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    no_store(c);
    let viewer = concerns::require_current_user(c)?.clone();
    let now = now(c);
    let unread = c
        .app()
        .db
        .read(move |conn| ActivityItem::unread_snapshot(conn, viewer.id, now))
        .await
        .map_err(db_error)?;
    c.json(
        StatusCode::OK,
        &api::ActivityUnreadCount {
            unread_count: unread.count,
            unread_revision: unread.revision,
            evaluated_at: unread.evaluated_at.to_evaluation_time(),
        },
    )
}

/// `activity_items#find`: the viewer's accessible item `:id`, or a 404.
async fn find(c: &mut Ctx) -> Result<(User, ActivityItem)> {
    before_actions(c).await?;
    no_store(c);
    let viewer = concerns::require_current_user(c)?.clone();
    let id = c
        .param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    let reader = viewer.clone();
    let item = c
        .app()
        .db
        .read(move |conn| ActivityItem::find_accessible(conn, &reader, id))
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    Ok((viewer, item))
}

async fn update_item(c: &mut Ctx) -> Result {
    let (viewer, item) = find(c).await?;
    let api::UpdateActivityItem { action } = body(c).await?;
    let saved = c
        .app()
        .db
        .write(move |tx| match action {
            api::ActivityAction::Read => item.mark_read(tx),
            api::ActivityAction::Unread => item.mark_unread(tx),
            api::ActivityAction::Handled => item.mark_handled(tx),
            api::ActivityAction::Unhandled => item.mark_unhandled(tx),
        })
        .await
        .map_err(db_error)?;
    respond(c, viewer, saved.id).await
}

/// `activity_items#open`: marks it read (never handled) before the client navigates to it.
async fn open_item(c: &mut Ctx) -> Result {
    let (viewer, item) = find(c).await?;
    let saved = c
        .app()
        .db
        .write(move |tx| item.mark_read(tx))
        .await
        .map_err(db_error)?;
    respond(c, viewer, saved.id).await
}

async fn respond(c: &mut Ctx, viewer: User, item_id: i64) -> Result {
    let app = c.app().clone();
    let changed = c
        .app()
        .db
        .read(move |conn| changed(conn, &app, &viewer, item_id))
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    c.json(StatusCode::OK, &changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stored_value_has_its_wire_variant() {
        for event_type in campfire_db::models::activity_item::EVENT_TYPES {
            assert!(
                wire::<api::ActivityEventType>(event_type).is_ok(),
                "{event_type}"
            );
        }
        for state in ["unread", "read", "handled"] {
            assert!(wire::<api::ActivityState>(state).is_ok(), "{state}");
        }
        // `ActivityItem::source_type`'s classes (access.sql's joins).
        for class in [
            "Message",
            "SavedItem",
            "WorkThreadEvent",
            "BoardSlaNudge",
            "HuddleGrant",
            "Event",
            "AgentApproval",
            "AgentBudgetNotice",
            "ScheduledMessage",
            "TwoFactorCredential",
            "Session",
        ] {
            assert!(
                wire::<api::ActivitySourceType>(&snake_case(class)).is_ok(),
                "{class}"
            );
        }
        for status in campfire_db::models::agent_approval::STATUSES {
            assert!(wire::<api::AgentApprovalStatus>(status).is_ok(), "{status}");
        }
        // `agent_posting::Cap`'s names, which `agent_budget_notices.cap` stores.
        for cap in ["messages", "board_posts", "external_actions"] {
            assert!(wire::<api::AgentBudgetCap>(cap).is_ok(), "{cap}");
        }
    }

    #[test]
    fn a_row_the_contract_cannot_carry_is_an_error_not_a_panic() {
        assert!(wire::<api::ActivityEventType>("carrier_pigeon").is_err());
    }

    #[test]
    fn source_classes_become_their_wire_names() {
        for (class, wire_name) in [
            ("Message", "message"),
            ("WorkThreadEvent", "work_thread_event"),
            ("TwoFactorCredential", "two_factor_credential"),
        ] {
            assert_eq!(snake_case(class), wire_name);
            assert!(
                wire::<api::ActivitySourceType>(wire_name).is_ok(),
                "{wire_name}"
            );
        }
    }
}

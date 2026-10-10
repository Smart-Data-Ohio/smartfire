//! app/controllers/activity_items_controller.rb; state and accessibility belong to WS12.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters::{activity, view_context},
};
use askama::Template;
use campfire_db::{ActivityItem, models::activity_item::ActivityQuery};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, format};
use campfire_views::{activity::Inbox};
fn filters(c: &Ctx) -> (String, String) {
    let status = c
        .param_str("status")
        .filter(|s| ["unread", "read", "handled"].contains(s))
        .unwrap_or("unread")
        .into();
    let kind = c
        .param_str("type")
        .filter(|s| {
            campfire_views::activity::TYPES
                .iter()
                .any(|(key, _)| key == s)
        })
        .unwrap_or("all")
        .into();
    (status, kind)
}
pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let viewer_id = viewer.id;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::huddle_invitations::resolve_overdue(tx, Some(viewer_id))?;
            campfire_db::AgentApproval::resolve_overdue(tx, Some(viewer_id))
        })
        .await
        .map_err(Error::internal)?;
    let (filter, kind) = filters(c);
    let before = c
        .param_str("before")
        .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|s| s.parse::<i64>().ok());
    let app = c.app().clone();
    let f = filter.clone();
    let k = kind.clone();
    let raw_before = c.param_str("before").map(str::to_owned);
    let json =
        c.respond_to(&[&format::HTML, &format::JSON])? == &format::JSON;
    let (items, payloads, unread, next) = c
        .app()
        .db
        .read(move |conn| {
            let rows = ActivityItem::query_accessible(
                conn,
                &viewer,
                ActivityQuery {
                    state: Some(&f),
                    type_filter: Some(&k),
                    before: raw_before.as_deref(),
                    limit: Some(100),
                },
            )?;
            let unread = ActivityItem::unread_count(conn, &viewer, app.db.env().now())? as usize;
            let next = (rows.len() == 100).then(|| rows.last().unwrap().id);
            let sources = if json {
                activity::Sources::load_json(conn, &rows)?
            } else {
                activity::Sources::load(conn, &rows)?
            };
            let payloads = if json {
                rows.iter()
                    .map(|i| activity::payload_with_sources(conn, &app, i, &sources))
                    .collect::<campfire_db::Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let items = if json {
                Vec::new()
            } else {
                rows.iter()
                    .map(|i| activity::item(conn, &app, i, &viewer, &sources))
                    .collect::<campfire_db::Result<Vec<_>>>()?
            };
            Ok((items, payloads, unread, next))
        })
        .await
        .map_err(Error::internal)?;
    no_store(c);
    if json {
        #[derive(serde::Serialize)]
        struct Index<'a> {
            activity_items: Vec<activity::Payload>,
            filter: &'a str,
            type_filter: &'a str,
            unread_count: usize,
            next_cursor: Option<i64>,
        }
        return c.json(
            StatusCode::OK,
            &Index {
                activity_items: payloads,
                filter: &filter,
                type_filter: &kind,
                unread_count: unread,
                next_cursor: next,
            },
        );
    }
    let now = c.now();
    c.respond_to(&[&format::HTML])?;
    let mut response =
        view_context::page_or_frame(
            c,
            StatusCode::OK,
            |ctx| {
                Inbox {
                    ctx,
                    items: &items,
                    filter: &filter,
                    type_filter: &kind,
                    before,
                    next_cursor: next,
                    unread_count: unread,
                    now,
                }
                .render()
            },
            |ctx| {
                let page = Inbox {
                    ctx,
                    items: &items,
                    filter: &filter,
                    type_filter: &kind,
                    before,
                    next_cursor: next,
                    unread_count: unread,
                    now,
                };
                campfire_views::layouts::frame(ctx, page.as_head(), page.as_content())
            },
        )
        .await?;
    response
        .headers
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers
        .insert("pragma", "no-cache".parse().unwrap());
    Ok(response)
}
pub async fn unread_count(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let now = c.app().db.env().now();
    let count = c
        .app()
        .db
        .read(move |conn| ActivityItem::unread_count(conn, &viewer, now))
        .await
        .map_err(Error::internal)?;
    let mut response = c.json(StatusCode::OK, &serde_json::json!({"unread_count":count}))?;
    response
        .headers
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers
        .insert("pragma", "no-cache".parse().unwrap());
    Ok(response)
}

fn no_store(c: &mut Ctx) {
    c.no_store();
    c.set_header("cache-control", "no-store");
    c.set_header("pragma", "no-cache");
}

pub async fn read(c: &mut Ctx) -> Result {
    change(c, false).await
}
pub async fn handled(c: &mut Ctx) -> Result {
    change(c, true).await
}

async fn find(c: &mut Ctx) -> Result<ActivityItem> {
    concerns::before_actions(c, Before::default()).await?;
    no_store(c);
    let viewer = concerns::require_current_user(c)?.clone();
    let id = c
        .param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    let item = c
        .app()
        .db
        .read(move |conn| ActivityItem::find_accessible(conn, &viewer, id))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    Ok(item)
}

async fn change(c: &mut Ctx, handled: bool) -> Result {
    let item = find(c).await?;
    // A compound Rails parameter has a nonempty to_s, so it cannot select the default state.
    let state = c
        .param("state")
        .map(|v| {
            v.to_s()
                .unwrap_or_else(|| "invalid compound parameter".into())
        })
        .unwrap_or_default();
    let valid = if handled {
        ["", "handled", "unhandled"].contains(&state.as_str())
    } else {
        ["", "read", "unread"].contains(&state.as_str())
    };
    if !valid {
        let message = if handled {
            "State must be handled or unhandled"
        } else {
            "State must be read or unread"
        };
        return state_response(c, None, Some(message)).await;
    }
    let saved = c
        .app()
        .db
        .write(move |tx| match (handled, state.as_str()) {
            (true, "unhandled") => item.mark_unhandled(tx),
            (true, _) => item.mark_handled(tx),
            (false, "unread") => item.mark_unread(tx),
            (false, _) => item.mark_read(tx),
        })
        .await
        .map_err(Error::internal)?;
    state_response(c, Some(saved), None).await
}

async fn state_response(
    c: &mut Ctx,
    item: Option<ActivityItem>,
    error: Option<&str>,
) -> Result {
    let chosen = c.respond_to(&[&format::HTML, &format::JSON])?;
    if chosen == &format::JSON {
        if let Some(error) = error {
            return c.json(
                StatusCode::UNPROCESSABLE_ENTITY,
                &serde_json::json!({"error":error}),
            );
        }
        let app = c.app().clone();
        let item = item.expect("successful mutation supplies a row");
        let payload = c
            .app()
            .db
            .read(move |conn| activity::payload(conn, &app, &item))
            .await
            .map_err(Error::internal)?;
        return c.json(StatusCode::OK, &payload);
    }
    let code = StatusCode::FOUND;
    if let Some(error) = error {
        c.flash().set_alert(error);
    }
    let (state, kind) = filters(c);
    let response = c.redirect_to_with(
        &campfire_views::activity::path(&state, &kind, None, false),
        Redirect {
            status: Some(code),
            ..Default::default()
        },
    )?;
    Ok(response)
}

pub async fn open(c: &mut Ctx) -> Result {
    let item = find(c).await?;
    let item = c
        .app()
        .db
        .write(move |tx| item.mark_read(tx))
        .await
        .map_err(Error::internal)?;
    let app = c.app().clone();
    let chosen = c.respond_to(&[&format::HTML, &format::JSON])?;
    if chosen == &format::JSON {
        let payload = c
            .app()
            .db
            .read(move |conn| activity::payload(conn, &app, &item))
            .await
            .map_err(Error::internal)?;
        c.json(StatusCode::OK, &payload)
    } else {
        let destination = c
            .app()
            .db
            .read(move |conn| activity::destination(conn, &item))
            .await
            .map_err(Error::internal)?;
        let response = c.redirect_to_with(
            &destination,
            Redirect {
                status: Some(StatusCode::SEE_OTHER),
                ..Default::default()
            },
        )?;
        Ok(response)
    }
}

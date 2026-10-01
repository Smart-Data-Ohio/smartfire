//! Message rendering at the WS7 publisher boundary. Domain events never contain HTML.
use askama::Template;
use campfire_db::{Message, Room};
use campfire_kit::{Ctx, Result};
use campfire_views::messages as views;
use crate::app::{App, AppCtx};
use crate::channels::broadcasts::{Stream, message_dom_id};
use crate::controllers::presenters::{Presenter, page::{self, db_error}};

pub async fn broadcast_edit(c: &Ctx, room: &Room, message: &Message, drive_given: bool) -> Result<()> {
    broadcast_edit_in(c, room, message, drive_given, false).await
}

pub async fn broadcast_thread_edit(c: &Ctx, room: &Room, message: &Message, drive_given: bool) -> Result<()> {
    broadcast_edit_in(c, room, message, drive_given, true).await
}

async fn broadcast_edit_in(c: &Ctx, room: &Room, message: &Message, drive_given: bool, thread_scoped: bool) -> Result<()> {
    let (app, room, message, base) = (c.app().clone(), room.clone(), message.clone(), page::renderer_base_url(c));
    let refreshes = c.app().db.read(move |conn| {
        let presenter = Presenter::new(conn, &app, None);
        let view = presenter.message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        let parts = page::render_detached_at(&app, account.as_ref(), &base, |ctx| {
            let mut parts = vec![
                ("presentation", views::PresentationPartial { ctx, message: &view }.render()?),
                ("meta", views::MetaPartial { ctx, message: &view }.render()?),
                ("github_pr_cards", views::cards(&view, "github_pr_cards", "github-pr-cards", 0, &view.components.github_cards).0),
                ("twitter_cards", views::cards(&view, "twitter_cards", "x-post-cards", 2, &view.components.twitter_cards).0),
                ("message_link_cards", views::cards(&view, "message_link_cards", "message-link-cards", 0, &view.components.message_link_cards).0),
                ("fizzy_cards", views::cards(&view, "fizzy_cards", "fizzy-cards", 0, &view.components.fizzy_cards).0),
                ("linkedin_cards", views::cards(&view, "linkedin_cards", "linkedin-post-cards", 2, &view.components.linkedin_cards).0),
                ("link_embed_cards", views::cards(&view, "link_embed_cards", "link-embed-cards", 2, &view.components.link_embed_cards).0),
            ];
            if drive_given { parts.push(("drive_attachments", views::DriveAttachmentsPartial { ctx, message: &view }.render()?)); }
            Ok::<_, askama::Error>(parts)
        }).map_err(|error| campfire_db::Error::Other(error.to_string()))?;
        for (part, html) in parts {
            if thread_scoped {
                app.broadcasts.turbo(&Stream::conversation(&room, &message), campfire_cable::turbo::Action::Replace,
                    &message_dom_id(&message, Some(part)), Some(&html), true);
            } else { app.broadcasts.message_part_replace(&room, &message, part, &html); }
        }
        Ok(presenter.take_github_refreshes())
    }).await.map_err(db_error)?;
    crate::integrations::github::pull_requests::refresh_after_render(&c.app().db, refreshes).await;
    Ok(())
}

pub async fn broadcast_tombstones(c: &Ctx, ids: Vec<i64>) -> Result<()> {
    let (app, base) = (c.app().clone(), page::renderer_base_url(c));
    c.app().db.read(move |conn| {
        let account = campfire_db::Account::first(conn)?;
        let presenter = Presenter::new(conn, &app, None);
        for id in ids {
            let message = Message::find(conn, id)?;
            let room = Room::find(conn, message.room_id)?;
            let view = presenter.message(&message)?;
            let html = page::render_detached_at(&app, account.as_ref(), &base, |ctx| views::uncached_message(ctx, &view));
            app.broadcasts.turbo(&Stream::conversation(&room, &message), campfire_cable::turbo::Action::Replace,
                &message_dom_id(&message, None), Some(&html), true);
        }
        Ok(())
    }).await.map_err(db_error)
}

pub async fn broadcast_thread_refresh(app: &App, room_id: i64, thread_id: i64) -> Result<()> {
    let runtime = app.clone();
    app.db.read(move |conn| {
        for membership in campfire_db::Membership::for_room(conn, room_id)? {
            runtime.broadcasts.channel(&format!("user_{}_unread_threads", membership.user_id),
                &serde_json::json!({"threadId": thread_id, "roomId": room_id, "refreshOnly": true}));
        }
        Ok(())
    }).await.map_err(db_error)
}

/// WS8a's after-commit message descriptions, rendered synchronously on a reader connection so
/// publisher ordering is preserved. Other domain owners add their own partial handler here.
pub fn domain_partial(app: &App, partial: &campfire_db::broadcasts::Partial) -> campfire_db::Result<Option<String>> {
    use campfire_db::broadcasts::Partial;
    let (id, count) = match partial {
        Partial::Message { message_id } | Partial::MessageReplace { message_id } => (*message_id, None),
        Partial::ThreadIndicator { message_id, reply_count } => (*message_id, Some(*reply_count)),
        _ => return Ok(None),
    };
    app.db.read_blocking(|conn| {
        let message = Message::find(conn, id)?;
        let mut view = Presenter::new(conn, app, None).message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        page::render_detached(app, account.as_ref(), |ctx| {
            if let Some(count) = count {
                view.details.reply_count = count.try_into().map_err(|_| campfire_db::Error::Other("negative reply count".into()))?;
                views::ThreadIndicatorPartial { ctx, message: &view }.render().map(Some).map_err(|error| campfire_db::Error::Other(error.to_string()))
            } else { Ok(Some(views::uncached_message(ctx, &view))) }
        })
    })
}

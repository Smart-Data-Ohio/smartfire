//! HTML adapter only; shared X cards use persisted, session-independent facts.
use crate::{
    app::App,
    channels::broadcasts::{Stream, message_dom_id},
};
use campfire_db::{Connection, Message, Room};
use campfire_views::helpers::{AvatarIcon, IconSource};

pub fn cards(
    presenter: &super::Presenter<'_>,
    message: &Message,
) -> campfire_db::Result<Vec<campfire_views::twitter::Card>> {
    let logo_url = match presenter.resolve_avatar_icon("x") {
        Some(AvatarIcon::Image { url, .. }) => Some(url),
        _ => None,
    };
    Ok(presenter
        .twitter_posts(message)?
        .into_iter()
        .map(|post| {
            presenter.request_twitter_fetch(&post);
            campfire_views::twitter::Card {
                view_url: post.view_url(),
                display_name: post.display_name(),
                display_handle: post.display_handle(),
                profile_url: post.profile_url(),
                post_id: post.post_id,
                author_avatar_url: post.author_avatar_url,
                text: post.text,
                posted_at: post.posted_at.map(|t| t.jiff()),
                replies: post.replies,
                reposts: post.reposts,
                likes: post.likes,
                media: post.media,
                quote: post.quote,
                fetched_at: post.fetched_at.map(|t| t.jiff()),
                fetch_error: post.fetch_error,
                logo_url: logo_url.clone(),
            }
        })
        .collect())
}
pub fn container(app: &App, conn: &Connection, message: &Message) -> campfire_db::Result<String> {
    let view = super::Presenter::new(conn, app, None).message(message)?;
    Ok(super::page::render_detached(app, None, |ctx| {
        campfire_views::twitter::cards(ctx, &view).0
    }))
}
pub fn broadcast_updates(app: &App, post_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        let mut query = conn.prepare("SELECT DISTINCT message_id FROM twitter_post_references WHERE twitter_post_id=? ORDER BY message_id")?;
        let ids = query.query_map([post_id], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for id in ids {
            let message = Message::find(conn, id)?;
            let room = Room::find(conn, message.room_id)?;
            let html = container(&app2, conn, &message)?;
            app2.broadcasts.turbo(&Stream::conversation(&room, &message), campfire_cable::turbo::Action::Replace,
                &message_dom_id(&message, Some("twitter_cards")), Some(&html), true);
        }
        Ok(())
    })?;
    Ok(())
}
#[cfg(test)]
mod tests;

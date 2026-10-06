//! HTML adapter only; shared X cards use persisted, session-independent facts.
use crate::{
    app::App,
    cable::broadcasts::{Stream, message_dom_id},
};
use campfire_db::{Account, Message, Room};
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
            card_from_post(post, logo_url.clone())
        })
        .collect())
}
fn card_from_post(
    post: crate::integrations::twitter::post::Post,
    logo_url: Option<String>,
) -> campfire_views::twitter::Card {
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
        logo_url,
    }
}
#[cfg(test)]
pub fn container(
    app: &App,
    conn: &campfire_db::Connection,
    message: &Message,
) -> campfire_db::Result<String> {
    let view = super::Presenter::new(conn, app, None).message(message)?;
    Ok(super::page::render_detached(app, None, |ctx| {
        campfire_views::twitter::cards(ctx, &view).0
    }))
}
pub fn broadcast_updates(app: &App, post_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        use crate::integrations::{
            message_batches::{self, Reference},
            twitter::post::Post,
        };
        let account = Account::first(conn)?;
        let logo_url = match super::resolve_avatar_icon(conn, "x") {
            Some(AvatarIcon::Image { url, .. }) => Some(url),
            _ => None,
        };
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::TwitterPost(post_id), after)?;
            let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
            let room_ids: Vec<_> = messages
                .iter()
                .map(|m| m.room_id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let rooms: std::collections::HashMap<_, _> = Room::for_ids(conn, &room_ids)?
                .into_iter()
                .map(|r| (r.id, r))
                .collect();
            let mut posts = Post::for_messages(conn, &ids)?;
            for message in &messages {
                let room = rooms
                    .get(&message.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                let mut ordered = posts.remove(&message.id).unwrap_or_default();
                Post::order_cards(&mut ordered);
                let cards: Vec<_> = ordered
                    .into_iter()
                    .map(|p| card_from_post(p, logo_url.clone()))
                    .collect();
                let html = super::page::render_detached(&app2, account.as_ref(), |ctx| {
                    campfire_views::twitter::cards_for_client_id(
                        ctx,
                        &message.client_message_id,
                        &[],
                        &cards,
                    )
                    .0
                });
                app2.broadcasts.turbo(
                    &Stream::conversation(room, message),
                    campfire_cable::turbo::Action::Replace,
                    &message_dom_id(message, Some("twitter_cards")),
                    Some(&html),
                    true,
                );
            }
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
        }
        Ok(())
    })?;
    Ok(())
}
#[cfg(test)]
mod tests;

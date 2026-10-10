//! Shared X-card facts from persisted records, including their fetch intents.
use campfire_app::app::App;
use campfire_db::Message;
use campfire_presentation::helpers::{AvatarIcon, IconSource};

pub fn cards(
    presenter: &super::Presenter<'_>,
    message: &Message,
) -> campfire_db::Result<Vec<campfire_presentation::twitter::Card>> {
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
pub fn card_from_post(
    post: crate::integrations::twitter::post::Post,
    logo_url: Option<String>,
) -> campfire_presentation::twitter::Card {
    campfire_presentation::twitter::Card {
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

pub fn broadcast_updates(app: &App, post_id: i64) -> anyhow::Result<()> {
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::TwitterPost(post_id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE { break; }
            after = messages.last().map(|message| message.id);
        }
        Ok(())
    })?;
    Ok(())
}

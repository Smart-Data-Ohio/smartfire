//! The HTML adapter for LinkEmbed broadcasts and message child components.
use crate::integrations::link_embed::Reference;
use crate::{
    app::App,
    channels::broadcasts::{Stream, message_dom_id},
};
use campfire_db::{Connection, Message, Room};
use campfire_views::messages::{self, MessageComponents};

pub fn components(presenter: &super::Presenter<'_>, message: &Message) -> campfire_db::Result<MessageComponents> {
    let mut components = MessageComponents { twitter_posts: super::twitter_cards::cards(presenter, message)?, fizzy_cards: super::fizzy_cards::frames(presenter.conn, message)?, ..Default::default() };
    if !message.embeds_suppressed {
        for reference in Reference::for_message(presenter.conn, message)? {
            presenter.request_link_fetch(&reference.embed);
            if reference.embed.linkedin() {
                let embed = &reference.embed;
                let card = campfire_views::linkedin_cards::Card {
                    embed: campfire_views::link_embeds::Card {
                        url: reference.display_url().into(),
                        title: embed.title.clone(),
                        description: embed.description.clone(),
                        image_url: embed.image_url.clone(),
                        ..Default::default()
                    },
                    player_url: crate::integrations::linkedin::embed_url_for(reference.display_url()),
                };
                components.linkedin_cards.push(format!("\n    {}\n  ", card.render()));
            } else if reference.embed.usable() {
                let embed = &reference.embed;
                let card = campfire_views::link_embeds::Card {
                    url: reference.display_url().into(),
                    title: embed.title.clone(),
                    description: embed.description.clone(),
                    site_name: embed.site_name.clone(),
                    image_url: embed.image_url.clone(),
                };
                components.link_embed_cards.push(format!("\n    {}\n  ", card.render()));
            }
        }
    }
    Ok(components)
}

/// Claims and durable job rows share this write. Rechecking TTL/claim handles concurrent views.
pub async fn enqueue_render_fetches(app: &App, ids: Vec<i64>, twitter_ids: Vec<i64>) -> campfire_db::Result<()> {
    if ids.is_empty() && twitter_ids.is_empty() {
        return Ok(());
    }
    app.db
        .write(move |tx| {
            for id in ids {
                match crate::integrations::link_embed::Embed::find(tx.conn(), id) {
                    Ok(embed) => {
                        crate::integrations::link_embed::store::request_fetch(tx, &embed)?;
                    }
                    Err(campfire_db::Error::RecordNotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            for id in twitter_ids {
                match crate::integrations::twitter::post::Post::find(tx.conn(), id) {
                    Ok(post) if post.fetch_pending() => { post.request_fetch(tx)?; }
                    Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })
        .await
}

/// WS8b's actions-menu seam: the HTTP endpoint also independently enforces this policy.
#[allow(
    dead_code,
    reason = "WS8b consumes the actions-menu policy; endpoint authorization is already wired"
)]
pub fn can_offer_suppression(conn: &Connection, message: &Message, viewer: i64) -> campfire_db::Result<bool> {
    if message.creator_id != viewer || message.system_note || message.embeds_suppressed {
        return Ok(false);
    }
    if let Some(thread) = message.thread_id
        && conn.query_row("SELECT locked_at IS NOT NULL FROM channel_threads WHERE id=?", [thread], |r| {
            r.get::<_, bool>(0)
        })?
    {
        return Ok(false);
    }
    Ok(Reference::for_message(conn, message)?
        .iter()
        .any(|reference| reference.embed.usable() || reference.embed.linkedin()))
}

pub fn container(app: &App, conn: &Connection, message: &Message, linkedin: bool) -> campfire_db::Result<String> {
    let view = super::Presenter::new(conn, app, None).message(message)?;
    let (part, class, cards) = if linkedin {
        ("linkedin_cards", "linkedin-post-cards", &view.components.linkedin_cards)
    } else {
        ("link_embed_cards", "link-embed-cards", &view.components.link_embed_cards)
    };
    Ok(messages::cards(&view, part, class, 2, cards).0)
}

pub fn broadcast_updates(app: &App, embed_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        let embed = crate::integrations::link_embed::Embed::find(conn, embed_id)?;
        let mut query = conn.prepare("SELECT DISTINCT message_id FROM link_embed_references WHERE link_embed_id=? ORDER BY message_id")?;
        let ids = query
            .query_map([embed_id], |row| row.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for id in ids {
            let message = Message::find(conn, id)?;
            broadcast_message(&app2, conn, &message, embed.linkedin())?;
        }
        Ok(())
    })?;
    Ok(())
}

pub fn broadcast_message(app: &App, conn: &Connection, message: &Message, linkedin: bool) -> campfire_db::Result<()> {
    let room = Room::find(conn, message.room_id)?;
    let part = if linkedin { "linkedin_cards" } else { "link_embed_cards" };
    let html = container(app, conn, message, linkedin)?;
    app.broadcasts.turbo(
        &Stream::conversation(&room, message),
        campfire_cable::turbo::Action::Replace,
        &message_dom_id(message, Some(part)),
        Some(&html),
        true,
    );
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod linkedin_tests;

//! The HTML adapter for LinkEmbed broadcasts and message child components.
use crate::integrations::link_embed::Reference;
use crate::{
    app::App,
    cable::broadcasts::{Stream, message_dom_id},
};
use campfire_db::{Connection, Message, Room};
use campfire_views::messages::{self, MessageComponents};

pub fn components(
    presenter: &super::Presenter<'_>,
    message: &Message,
) -> campfire_db::Result<MessageComponents> {
    let mut components = MessageComponents {
        twitter_posts: super::twitter_cards::cards(presenter, message)?,
        fizzy_cards: super::fizzy_cards::frames_from_cards(
            message,
            &presenter.fizzy_cards(message)?,
        ),
        ..Default::default()
    };
    if !message.embeds_suppressed {
        let references = presenter.link_references(message)?;
        for reference in &references {
            presenter.request_link_fetch(&reference.embed);
        }
        let cards = reference_components(&references, None);
        components.linkedin_cards = cards.linkedin_cards;
        components.link_embed_cards = cards.link_embed_cards;
    }
    Ok(components)
}

fn reference_components(references: &[Reference], linkedin: Option<bool>) -> MessageComponents {
    let mut components = MessageComponents::default();
    for reference in references
        .iter()
        .filter(|r| linkedin.is_none_or(|kind| r.embed.linkedin() == kind))
    {
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
            components
                .linkedin_cards
                .push(format!("\n    {}\n  ", card.render()));
        } else if reference.embed.usable() {
            let embed = &reference.embed;
            let card = campfire_views::link_embeds::Card {
                url: reference.display_url().into(),
                title: embed.title.clone(),
                description: embed.description.clone(),
                site_name: embed.site_name.clone(),
                image_url: embed.image_url.clone(),
            };
            components
                .link_embed_cards
                .push(format!("\n    {}\n  ", card.render()));
        }
    }
    components
}

/// Claims and durable job rows share this write. Rechecking TTL/claim handles concurrent views.
pub async fn enqueue_render_fetches(app: &App, ids: Vec<i64>, twitter_ids: Vec<i64>) -> campfire_db::Result<()> {
    if ids.is_empty() && twitter_ids.is_empty() {
        return Ok(());
    }
    let result = app.db
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
        .await;
    // Collection fragments can be stored before this writer rejects a fetch job.
    // Do not let a retry reuse that incomplete render and lose its pending requests.
    if result.is_err() { app.fragment_cache.clear(); }
    result
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
    Ok(container_view(&view, linkedin))
}
fn container_view(view: &messages::MessageView, linkedin: bool) -> String {
    let (part, class, cards) = if linkedin {
        ("linkedin_cards", "linkedin-post-cards", &view.components.linkedin_cards)
    } else {
        ("link_embed_cards", "link-embed-cards", &view.components.link_embed_cards)
    };
    messages::cards(view, part, class, 2, cards).0
}

pub fn broadcast_updates(app: &App, embed_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        let embed = crate::integrations::link_embed::Embed::find(conn, embed_id)?;
        use crate::integrations::message_batches::{self, Reference as Source};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Source::LinkEmbed(embed_id), after)?;
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
            let visible: Vec<_> = messages
                .iter()
                .filter(|m| !m.embeds_suppressed)
                .map(|m| m.id)
                .collect();
            let references = Reference::for_messages(conn, &visible)?;
            for message in &messages {
                let room = rooms
                    .get(&message.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                let components = reference_components(
                    references
                        .get(&message.id)
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                    Some(embed.linkedin()),
                );
                let (part, class, cards) = if embed.linkedin() {
                    (
                        "linkedin_cards",
                        "linkedin-post-cards",
                        &components.linkedin_cards,
                    )
                } else {
                    (
                        "link_embed_cards",
                        "link-embed-cards",
                        &components.link_embed_cards,
                    )
                };
                let html = messages::cards_for_client_id(
                    &message.client_message_id,
                    part,
                    class,
                    2,
                    cards,
                )
                .0;
                app2.broadcasts.turbo(
                    &Stream::conversation(room, message),
                    campfire_cable::turbo::Action::Replace,
                    &message_dom_id(message, Some(part)),
                    Some(&html),
                    true,
                );
            }
            app2.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
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

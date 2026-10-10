//! The HTML adapter for LinkEmbed broadcasts and message child components.
use crate::integrations::link_embed::Reference;
use crate::{
    app::App,
};
use campfire_db::{Connection, Message};
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
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::LinkEmbed(embed_id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE { break; }
            after = messages.last().map(|message| message.id);
        }
        Ok(())
    })?;
    Ok(())
}

pub fn broadcast_message(app: &App, conn: &Connection, message: &Message, _linkedin: bool) -> campfire_db::Result<()> {
    app.broadcasts.sync_message_cards(conn, std::slice::from_ref(message));
    Ok(())
}

use campfire_views::rendering::*;
use crate::controllers::presenters::{Rendering};
pub use campfire_runtime::presenters::link_embeds::*;

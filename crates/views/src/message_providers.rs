//! Shared public provider cards. Private provider payloads stay at their per-viewer endpoints.
pub mod events;

use crate::{ViewContext, helpers as h};
use askama::Template;
pub trait GithubCardRendering {
    fn avatar_tag(&self, ctx: &ViewContext) -> h::Html;
    fn updated(&self, ctx: &ViewContext) -> h::Html;
    fn link(&self) -> h::Html;
    fn discuss(&self, room: i64, message: i64) -> h::Html;
}
impl GithubCardRendering for GithubCard {
    fn avatar_tag(&self, ctx: &ViewContext) -> h::Html {
        h::image_tag(
            ctx,
            &self.avatar,
            h::attrs()
                .attr("alt", "")
                .size("16x16")
                .class("github-pr-card__avatar")
                .attr("loading", "lazy"),
        )
    }
    fn updated(&self, ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            self.updated_at.expect("updated timestamp"),
            "time",
            h::attrs(),
            "",
        )
    }
    fn link(&self) -> h::Html {
        h::link_to_text(
            "View on GitHub",
            &self.url,
            h::attrs()
                .class("github-pr-card__link")
                .target("_blank")
                .attr("rel", "noopener noreferrer"),
        )
    }
    fn discuss(&self, room: i64, message: i64) -> h::Html {
        match self.thread_id {
            Some(id) => h::link_to_text(
                "Discuss",
                &format!("/rooms/{room}/threads/{id}"),
                h::attrs()
                    .class("github-pr-card__discuss")
                    .data("turbo_frame", "_top"),
            ),
            None => h::button_to_form_params(
                &format!("/rooms/{room}/github/pull_request_threads"),
                h::attrs()
                    .class("github-pr-card__discuss")
                    .attr("form_class", "github-pr-card__discuss-form")
                    .attr("authenticity_token", false),
                h::attrs().data("turbo_frame", "_top"),
                "Discuss",
                &[
                    ("message_id", &message.to_string()),
                    ("pull_request_id", &self.id.to_string()),
                ],
            ),
        }
    }
}

#[derive(Template)]
#[template(path = "message_providers/_github.html")]
pub struct GithubPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub card: &'a GithubCard,
    pub room_id: i64,
    pub message_id: i64,
    pub root_message: bool,
}
pub trait EmbedCardRendering {
    fn title_link(&self) -> h::Html;
    fn view_link(&self, chip: bool) -> h::Html;
    fn image_link(&self, ctx: &ViewContext, kind: &str) -> h::Html;
}
impl EmbedCardRendering for EmbedCard {
    fn title_link(&self) -> h::Html {
        h::link_to_text(
            &self.title,
            &self.url,
            h::attrs()
                .target("_blank")
                .attr("rel", "nofollow noopener noreferrer"),
        )
    }
    fn view_link(&self, chip: bool) -> h::Html {
        let mut a = h::attrs();
        if !chip {
            a = a.class("linkedin-post-card__link");
        }
        h::link_to_text(
            "View post on LinkedIn",
            &self.url,
            a.target("_blank")
                .attr("rel", "nofollow noopener noreferrer"),
        )
    }
    fn image_link(&self, ctx: &ViewContext, kind: &str) -> h::Html {
        let img = h::image_tag(
            ctx,
            &self.image,
            h::attrs()
                .attr("alt", "")
                .attr("loading", "lazy")
                .class(format!("{kind}__image")),
        );
        h::link_to(
            &self.url,
            h::attrs()
                .target("_blank")
                .attr("rel", "nofollow noopener noreferrer")
                .class(format!("{kind}__image-link"))
                .attr("tabindex", "-1")
                .aria("hidden", "true"),
            &format!("\n        {}\n", img.0),
        )
    }
}

#[derive(Template)]
#[template(path = "message_providers/_embed.html")]
pub struct EmbedPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub card: &'a EmbedCard,
}
#[derive(Template)]
#[template(path = "message_providers/_linkedin.html")]
pub struct LinkedinPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub card: &'a EmbedCard,
}

/// WS8bm2 root composition seam. Context is used only by pure partial helpers.
pub fn github_cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
    if let Some(html) = &message.components.github_cards_html { return h::raw(html.clone()); }
    let values = message.components.provider_github.as_ref().map(|entries| {
        entries
            .iter()
            .map(|entry| match entry {
                GithubEntry::Private(frame) => frame.clone(),
                GithubEntry::Public(card) => format!(
                    "\n    {}\n",
                    GithubPartial {
                        ctx,
                        card,
                        room_id: message.room_id,
                        message_id: message.id,
                        root_message: message.details.thread_id.is_none()
                    }
                    .render()
                    .expect("public PR renders")
                ),
            })
            .collect::<Vec<_>>()
    });
    crate::messages::cards(
        message,
        "github_pr_cards",
        "github-pr-cards",
        0,
        values
            .as_deref()
            .unwrap_or(&message.components.github_cards),
    )
}
pub fn embed_cards(
    ctx: &ViewContext,
    message: &crate::messages::MessageView,
    linkedin: bool,
) -> h::Html {
    let values = message.components.provider_embeds.as_ref().map(|entries| {
        entries
            .iter()
            .filter(|entry| entry.linkedin == linkedin && (linkedin || entry.card.usable()))
            .map(|entry| {
                let body = if linkedin {
                    LinkedinPartial {
                        ctx,
                        card: &entry.card,
                    }
                    .render()
                } else {
                    EmbedPartial {
                        ctx,
                        card: &entry.card,
                    }
                    .render()
                }
                .expect("embed renders");
                format!("\n    {body}\n  ")
            })
            .collect::<Vec<_>>()
    });
    let (prefix, class, fallback) = if linkedin {
        (
            "linkedin_cards",
            "linkedin-post-cards",
            &message.components.linkedin_cards,
        )
    } else {
        (
            "link_embed_cards",
            "link-embed-cards",
            &message.components.link_embed_cards,
        )
    };
    crate::messages::cards(
        message,
        prefix,
        class,
        2,
        values.as_deref().unwrap_or(fallback),
    )
}
pub use campfire_presentation::message_providers::*;

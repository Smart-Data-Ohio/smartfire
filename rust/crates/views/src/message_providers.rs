//! Shared public provider cards. Private provider payloads stay at their per-viewer endpoints.
use crate::{ViewContext, helpers as h};
use askama::Template;
use jiff::Timestamp;

fn present(value: &str) -> bool {
    !h::is_blank(value)
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct GithubCard {
    pub id: i64,
    pub full_name: String,
    pub display_name: String,
    pub number: i64,
    pub state: String,
    pub title: String,
    pub author: String,
    pub avatar: String,
    pub base: String,
    pub head: String,
    pub review: String,
    pub checks: String,
    pub updated_at: Option<Timestamp>,
    pub url: String,
    pub error: String,
    pub thread_id: Option<i64>,
}
impl GithubCard {
    fn has(&self, value: &str) -> bool {
        present(value)
    }
    fn state_label(&self) -> &'static str {
        match self.state.as_str() {
            "merged" => "Merged",
            "closed" => "Closed",
            "draft" => "Draft",
            _ => "Open",
        }
    }
    fn review_label(&self) -> Option<&'static str> {
        match self.review.as_str() {
            "approved" => Some("Approved"),
            "changes_requested" => Some("Changes requested"),
            "review_required" => Some("Review required"),
            _ => None,
        }
    }
    fn checks_label(&self) -> &'static str {
        match self.checks.as_str() {
            "passing" => "Checks passing",
            "pending" => "Checks pending",
            "failing" => "Checks failing",
            _ => "No checks",
        }
    }
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

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct EmbedCard {
    pub url: String,
    pub title: String,
    pub description: String,
    pub site: String,
    pub image: String,
    pub player_url: Option<String>,
}
impl EmbedCard {
    fn has(&self, value: &str) -> bool {
        present(value)
    }
    pub fn usable(&self) -> bool {
        present(&self.title) || present(&self.description)
    }
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

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub enum GithubEntry {
    Public(Box<GithubCard>),
    Private(String),
}
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct EmbedEntry {
    pub linkedin: bool,
    pub card: EmbedCard,
}

/// WS8bm2 root composition seam. Context is used only by pure partial helpers.
pub fn github_cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
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

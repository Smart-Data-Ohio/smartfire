//! Plain X card facts. Rendering uses the request's assets and time zone.
pub use campfire_presentation::twitter::cards::*;

use crate::{ViewContext, helpers as h};
use askama::Template;
use serde_json::Value;
pub trait CardRendering {
    fn render(&self, ctx: &ViewContext) -> String;
    fn avatar(&self, ctx: &ViewContext) -> Option<h::Html>;
    fn link(&self, label: &str, url: &str, class: &str) -> h::Html;
    fn view_link(&self) -> h::Html;
    fn name_link(&self) -> h::Html;
    fn handle_link(&self, handle: &str) -> h::Html;
    fn logo(&self, ctx: &ViewContext) -> h::Html;
    fn media_link(&self, item: &Value) -> h::Html;
    fn media_image(&self, ctx: &ViewContext, item: &Value) -> h::Html;
    fn quote_author(&self, quote: &Value) -> h::Html;
    fn quote_header(&self, quote: &Value) -> h::Html;
    fn quote_text(quote: &Value) -> h::Html;
    fn time(&self, ctx: &ViewContext, at: &jiff::Timestamp) -> h::Html;
}
impl CardRendering for Card {
    fn render(&self, ctx: &ViewContext) -> String {
        format!(
            "{}\n",
            CardPartial { ctx, card: self }
                .render()
                .expect("X card renders")
        )
    }
    fn avatar(&self, ctx: &ViewContext) -> Option<h::Html> {
        present(self.author_avatar_url.as_deref()).map(|url| {
            h::image_tag(
                ctx,
                url,
                h::attrs()
                    .alt("")
                    .attr("loading", "lazy")
                    .size("40x40")
                    .class("x-post-card__avatar"),
            )
        })
    }
    fn link(&self, label: &str, url: &str, class: &str) -> h::Html {
        h::link_to_text(
            label,
            url,
            h::attrs()
                .class(class)
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer"),
        )
    }
    fn view_link(&self) -> h::Html {
        self.link("View on X", &self.view_url, "x-post-card__link")
    }
    fn name_link(&self) -> h::Html {
        self.link(
            &self.display_name,
            self.profile_url.as_deref().unwrap_or(&self.view_url),
            "x-post-card__name",
        )
    }
    fn handle_link(&self, handle: &str) -> h::Html {
        self.link(
            &format!("@{handle}"),
            self.profile_url.as_deref().unwrap_or(&self.view_url),
            "x-post-card__handle",
        )
    }
    fn logo(&self, ctx: &ViewContext) -> h::Html {
        self.logo_url
            .as_deref()
            .map(|url| {
                h::image_tag(
                    ctx,
                    url,
                    h::attrs()
                        .class("icon icon--brand x-post-card__logo")
                        .alt("")
                        .attr("aria-hidden", "true"),
                )
            })
            .unwrap_or_else(h::empty)
    }
    fn media_link(&self, item: &Value) -> h::Html {
        let (class, label) = if Self::photo(item) {
            ("x-post-card__media-link", "View post on X")
        } else {
            (
                "x-post-card__media-link x-post-card__media-link--video",
                "View video on X",
            )
        };
        // Rails' block link has its closing tag emitted after the captured inner whitespace.
        let tag = h::link_to(
            &self.view_url,
            h::attrs()
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
                .class(class)
                .attr("aria-label", label),
            "",
        )
        .0;
        h::raw(tag.strip_suffix("</a>").unwrap())
    }
    fn media_image(&self, ctx: &ViewContext, item: &Value) -> h::Html {
        let (url, alt, class) = if Self::photo(item) {
            (&item["url"], text(&item["alt"]), "x-post-card__photo")
        } else {
            (&item["thumbnail_url"], String::new(), "x-post-card__poster")
        };
        let mut attrs = h::attrs().alt(alt).attr("loading", "lazy").class(class);
        for name in ["width", "height"] {
            if !item[name].is_null() {
                attrs = attrs.attr(name, text(&item[name]));
            }
        }
        h::image_tag(ctx, text(url), attrs)
    }
    fn quote_author(&self, quote: &Value) -> h::Html {
        let mut author = String::new();
        if let Some(name) = present(quote["author_name"].as_str()) {
            author.push_str(&format!(
                "            <strong>{}</strong>\n",
                h::escape(name)
            ));
        }
        if let Some(handle) = present(quote["author_handle"].as_str()) {
            author.push_str(&format!(
                "            <span class=\"x-post-card__quote-handle\">@{}</span>\n",
                h::escape(handle)
            ));
        }
        h::raw(author)
    }
    fn quote_header(&self, quote: &Value) -> h::Html {
        let author = self.quote_author(quote);
        // Rails links only stored quote URLs that start with http:// or https://.
        let url = present(quote["url"].as_str())
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"));
        if let Some(url) = url {
            h::link_to(
                url,
                h::attrs()
                    .attr("target", "_blank")
                    .attr("rel", "noopener noreferrer")
                    .class("x-post-card__quote-link"),
                &author.0,
            )
        } else {
            author
        }
    }
    fn quote_text(quote: &Value) -> h::Html {
        super::formatter::format(&text(&quote["text"]))
    }
    fn time(&self, ctx: &ViewContext, at: &jiff::Timestamp) -> h::Html {
        crate::time::local_datetime_tag(&ctx.time_zone, *at, "time", h::attrs(), "")
    }
}

#[derive(Template)]
#[template(path = "twitter/posts/_card.html")]
struct CardPartial<'a> {
    ctx: &'a ViewContext<'a>,
    card: &'a Card,
}

pub fn cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
    cards_for_client_id(
        ctx,
        &message.client_message_id,
        &message.components.twitter_cards,
        &message.components.twitter_posts,
    )
}
/// Card-only callbacks reuse the container without loading a message's body,
/// cache stamps, avatars, polls, boosts or unrelated provider associations.
pub fn cards_for_client_id(
    ctx: &ViewContext,
    client_id: &str,
    legacy: &[String],
    posts: &[Card],
) -> h::Html {
    let mut bodies = legacy.to_vec();
    bodies.extend(
        posts
            .iter()
            .map(|card| format!("\n    {}\n  ", card.render(ctx))),
    );
    crate::messages::cards_for_client_id(client_id, "twitter_cards", "x-post-cards", 2, &bodies)
}
#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;

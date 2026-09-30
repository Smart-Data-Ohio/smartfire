//! Plain X card facts. Rendering uses the request's assets and time zone.
use crate::{ViewContext, helpers as h};
use askama::Template;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Card {
    pub post_id: String,
    pub view_url: String,
    pub display_name: String,
    pub display_handle: Option<String>,
    pub profile_url: Option<String>,
    pub author_avatar_url: Option<String>,
    pub text: Option<String>,
    pub posted_at: Option<jiff::Timestamp>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub likes: Option<i64>,
    #[serde(deserialize_with = "media_or_empty")]
    pub media: Vec<Value>,
    pub quote: Option<Value>,
    pub fetched_at: Option<jiff::Timestamp>,
    pub fetch_error: Option<String>,
    pub logo_url: Option<String>,
}
fn media_or_empty<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Value>, D::Error> {
    Ok(Option::<Vec<Value>>::deserialize(deserializer)?.unwrap_or_default())
}
fn present(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !h::is_blank(s))
}
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}
impl Card {
    pub fn render(&self, ctx: &ViewContext) -> String {
        format!(
            "{}\n",
            CardPartial { ctx, card: self }
                .render()
                .expect("X card renders")
        )
    }
    fn error(&self) -> bool {
        present(self.fetch_error.as_deref()).is_some()
    }
    fn handle(&self) -> Option<&str> {
        present(self.display_handle.as_deref())
    }
    fn body(&self) -> Option<&str> {
        present(self.text.as_deref())
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
    fn media(&self) -> &[Value] {
        &self.media[..self.media.len().min(4)]
    }
    fn photo(item: &Value) -> bool {
        item["type"] == "photo"
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
    fn quote(&self) -> Option<&Value> {
        self.quote.as_ref().filter(|q| match q {
            Value::Null | Value::Bool(false) => false,
            Value::String(s) => !h::is_blank(s),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            _ => true,
        })
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
        if let Some(url) = present(quote["url"].as_str()) {
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

/// Rails number_to_human: round to three significant digits before choosing K/M/B.
pub fn compact_count(number: &i64) -> String {
    let number = *number;
    let magnitude = i128::from(number).abs();
    let digits = magnitude.to_string().len() as u32;
    let quantum = 10_i128.pow(digits.saturating_sub(3));
    let rounded = (magnitude + quantum / 2) / quantum * quantum;
    let power = if rounded >= 1_000_000_000 {
        9
    } else if rounded >= 1_000_000 {
        6
    } else if rounded >= 1_000 {
        3
    } else {
        0
    };
    let divisor = 10_i128.pow(power);
    let mut value = (rounded / divisor).to_string();
    let remainder = rounded % divisor;
    if remainder != 0 {
        value.push('.');
        value
            .push_str(format!("{remainder:0width$}", width = power as usize).trim_end_matches('0'));
    }
    format!(
        "{}{value}{}",
        if number < 0 { "-" } else { "" },
        match power {
            3 => "K",
            6 => "M",
            9 => "B",
            _ => "",
        }
    )
}
#[derive(Template)]
#[template(path = "twitter/posts/_card.html")]
struct CardPartial<'a> {
    ctx: &'a ViewContext<'a>,
    card: &'a Card,
}

pub fn cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
    let mut bodies = message.components.twitter_cards.clone();
    bodies.extend(
        message
            .components
            .twitter_posts
            .iter()
            .map(|card| format!("\n    {}\n  ", card.render(ctx))),
    );
    crate::messages::cards(message, "twitter_cards", "x-post-cards", 2, &bodies)
}
#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;

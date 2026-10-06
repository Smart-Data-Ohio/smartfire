//! `ContentFilters::TextMessagePresentationFilters` (reference/app/helpers/content_filters/*.rb):
//! RemoveSoloUnfurledLinkText, StyleUnfurledTwitterAvatars, SanitizeTags, SanitizeAttributes.

use crate::Error;
use crate::attachables::{OPENGRAPH_EMBED_CONTENT_TYPE, RenderContext};
use crate::content::{ATTACHMENT_TAG, Content};
use crate::ruby::{is_blank, strip};
use crate::sanitizer::{self, SafeList, sanitize_tags_allowed_tags};
use crate::uri::{self, UriError};

pub fn apply(content: Content, ctx: &RenderContext) -> Result<Content, Error> {
    let content = remove_solo_unfurled_link_text(content, ctx)?;
    let content = style_unfurled_twitter_avatars(content)?;
    let content = sanitize_tags(content);
    sanitize_attributes(content)
}

// --- RemoveSoloUnfurledLinkText -----------------------------------------------------------------

/// A message that is nothing but a link to what it unfurls shows just the unfurl.
pub fn remove_solo_unfurled_link_text(content: Content, ctx: &RenderContext) -> Result<Content, Error> {
    let unfurled_links: Vec<_> = content
        .dom
        .descendants(content.root)
        .into_iter()
        .filter(|&n| {
            content.dom.local_name(n) == Some(ATTACHMENT_TAG) && content.dom.attr(n, "content-type") == Some(OPENGRAPH_EMBED_CONTENT_TYPE)
        })
        .collect();
    let solo_unfurled_url = if unfurled_links.len() == 1 { content.dom.attr(unfurled_links[0], "href").map(str::to_owned) } else { None };
    let plain_text = content.to_plain_text(ctx)?;
    let applicable = normalize_tweet_url(solo_unfurled_url.as_deref())? == normalize_tweet_url(Some(&plain_text))?;
    if !applicable {
        return Ok(content);
    }

    let Content { mut dom, root } = content;
    // Our fork removes only text siblings, preserving the original attachment once.
    for div in dom.descendants(root).into_iter().filter(|&n| dom.local_name(n) == Some("div")).collect::<Vec<_>>() {
        if dom.ancestors(div).into_iter().any(|n| dom.local_name(n) == Some(ATTACHMENT_TAG)) {
            continue;
        }
        for child in dom.children(div).to_vec() {
            if dom.local_name(child) != Some(ATTACHMENT_TAG) {
                dom.detach(child);
            }
        }
    }
    Ok(Content { dom, root })
}

const TWITTER_DOMAINS: &[&str] = &["x.com", "twitter.com"];

fn normalize_tweet_url(url: Option<&str>) -> Result<Option<String>, Error> {
    let Some(url) = url else { return Ok(None) };
    let is_twitter_url = !is_blank(url) && TWITTER_DOMAINS.iter().any(|d| strip(url).contains(d));
    if !is_twitter_url {
        return Ok(Some(url.to_string()));
    }
    match uri::parse(url) {
        Err(UriError::InvalidUri) => Ok(Some(url.to_string())),
        Err(UriError::InvalidComponent) => Err(Error::Raised("URI::InvalidComponentError")),
        Ok(mut parsed) => {
            if parsed.host.as_deref().map(str::to_lowercase).as_deref() == Some("x.com") {
                parsed.host = Some("twitter.com".to_string());
            }
            parsed.query = None;
            Ok(Some(parsed.to_s()))
        }
    }
}

/// `ContentFilters::StyleUnfurledTwitterAvatars`: the first div, when any stored
/// opengraph attachment URL contains the Twitter avatar prefix. No div raises in Ruby.
pub fn style_unfurled_twitter_avatars(content: Content) -> Result<Content, Error> {
    let Content { mut dom, root } = content;
    let has_avatar = dom.descendants(root).into_iter().any(|n| {
        dom.local_name(n) == Some(ATTACHMENT_TAG)
            && dom.attr(n, "content-type") == Some(OPENGRAPH_EMBED_CONTENT_TYPE)
            && dom.attr(n, "url").is_some_and(|url| url.contains("https://pbs.twimg.com/profile_images"))
    });
    if has_avatar {
        let div = dom
            .descendants(root)
            .into_iter()
            .find(|&n| dom.local_name(n) == Some("div"))
            .ok_or(Error::Raised("NoMethodError: []= for nil"))?;
        dom.set_attr(div, "class", "cf-twitter-avatar");
    }
    Ok(Content { dom, root })
}

// --- SanitizeTags ------------------------------------------------------------------------------

/// Removes every element outside the allowlist, together with its contents.
pub fn sanitize_tags(content: Content) -> Content {
    let Content { mut dom, root } = content;
    let allowed = sanitize_tags_allowed_tags();
    let disallowed: Vec<_> =
        dom.descendants(root).into_iter().filter(|&n| dom.local_name(n).is_some_and(|name| !allowed.contains(&name))).collect();
    for node in disallowed {
        dom.detach(node);
    }
    Content { dom, root }
}

// --- SanitizeAttributes ------------------------------------------------------------------------

/// Scrubs attributes with Rails' safe-list sanitizer over SanitizeTags' own tags.
pub fn sanitize_attributes(content: Content) -> Result<Content, Error> {
    let html = sanitizer::sanitize(&content.to_html(), &SafeList::content_filter()).map_err(Error::Parse)?;
    Content::wrap(&html)
}

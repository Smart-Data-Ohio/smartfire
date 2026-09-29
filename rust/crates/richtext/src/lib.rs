//! Campfire's Action Text content pipeline: what a stored message body looks like on screen, in
//! the editor, and as plain text.
//!
//! The Rails app is the oracle (see reference/app/helpers/messages_helper.rb,
//! reference/app/helpers/content_filters/, reference/app/models/message/{markdown,legacy_markdown}.rb and
//! reference/lib/rails_ext/). Every step reproduces the Ruby it mirrors, including where Nokogiri
//! serializes markup and parses it back, because the output is shaped by those round trips.
//!
//! Sanitization is our own allowlist walker over html5ever (`sanitizer`), not ammonia: the layers
//! need Loofah's exact rules (unwrapping rather than dropping HTML elements, dropping foreign
//! elements with their contents, its URI protocol checks, its attribute re-escaping), a
//! content-removing tag filter, and the markup to come back serialized exactly as Nokogiri does
//! so auto_link's regular expressions see the same text. ammonia can express none of those.

pub mod attachables;
pub mod autolink;
pub mod content;
pub mod dom;
pub mod filters;
pub mod legacy_markdown;
pub mod markdown;
pub mod plain_text;
pub mod ruby;
pub mod sanitizer;
pub mod uri;

pub use attachables::{AttachableResolver, GidLookup, MentionUser, RenderContext, SignedLookup};
pub use content::Content;

use content::attachment_nodes;
use ruby::is_blank;
use sanitizer::SafeList;

/// Something Ruby would have raised while rendering. Callers mirror what the Rails code does with
/// the exception (`message_presentation` rescues everything and renders "").
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Markdown source exceeds 50000 characters")]
    SourceTooLong,
    #[error("{0}")]
    Parse(dom::ParseError),
    #[error("raised {0}")]
    Raised(&'static str),
    /// Raised with a message that isn't valid UTF-8, so the rescue's own logging raises too.
    #[error("raised {0} with an unloggable message")]
    Unrenderable(&'static str),
}

/// How `messages/_message.html.erb` shows a text message body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presentation {
    Html(String),
    /// `message_tag` rescued an exception: render `messages/_unrenderable.html.erb` instead.
    Unrenderable,
}

/// The text branch of `MessagesHelper#message_presentation`:
/// `auto_link h(TextMessagePresentationFilters.apply(message.body.body)), ...`.
pub fn message_presentation(body: &str, ctx: &RenderContext) -> Result<String, Error> {
    let content = Content::load(body, ctx)?;
    let filtered = filters::apply(content, ctx)?;
    let rendered = filtered.to_rendered_html_with_layout(ctx)?;
    autolink::auto_link(&rendered, &SafeList::auto_link()).map_err(Error::Parse)
}

/// `message_presentation` with its rescue: an exception renders as an empty string, unless logging
/// it raises again, in which case the whole message is unrenderable.
pub fn present_message(body: &str, ctx: &RenderContext) -> Presentation {
    match message_presentation(body, ctx) {
        Ok(html) => Presentation::Html(html),
        Err(Error::Unrenderable(_)) => Presentation::Unrenderable,
        Err(_) => Presentation::Html(String::new()),
    }
}

/// `message.body.to_plain_text` (Action Text's `RichText#to_plain_text`), which feeds
/// `Message#plain_text_body`: the search index, push notification bodies, the webhook's plain
/// body, emoji detection and `/play` commands.
pub fn to_plain_text(body: &str, ctx: &RenderContext) -> Result<String, Error> {
    Content::load(body, ctx)?.to_plain_text(ctx)
}

/// `Message#editable_markdown_source`: preserve exact source for Markdown records, convert
/// legacy bodies (including forwarded HTML snapshots) to Markdown for the textarea.
pub fn editable_markdown_source(body: &str, source: Option<&str>, ctx: &RenderContext) -> Result<String, Error> {
    match source {
        Some(source) => Ok(source.to_owned()),
        None => legacy_markdown::render(&Content::load(body, ctx)?.to_html(), ctx),
    }
}

/// Compatibility entry point for existing presenter callers. The fork edits legacy bodies as
/// Markdown, so it returns textarea source; new callers should use `editable_markdown_source`.
pub fn editable_value(body: &str, ctx: &RenderContext) -> Result<Option<String>, Error> {
    let source = editable_markdown_source(body, None, ctx)?;
    Ok((!is_blank(&source)).then_some(source))
}

/// `Message::Mentionee#mentioned_users`: users from preloaded or verified attachables, once each.
pub fn mentioned_users(body: &str, ctx: &RenderContext) -> Result<Vec<MentionUser>, Error> {
    let content = Content::load(body, ctx)?;
    let mut users: Vec<MentionUser> = Vec::new();
    for node in attachment_nodes(&content.dom, content.root) {
        if let attachables::Attachable::User(user) = attachables::action_text_attachable_from_node(&content.dom, node, ctx)
            && !users.iter().any(|u| u.id == user.id)
        {
            users.push(user);
        }
    }
    Ok(users)
}

/// `Webhook#without_recipient_mentions`: the plain body with the bot's own "@Name" removed and
/// leading and trailing Unicode whitespace trimmed.
pub fn without_recipient_mentions(plain_text: &str, recipient_name: &str) -> String {
    plain_text.replace(&format!("@{recipient_name}"), "").trim_matches(char::is_whitespace).to_string()
}

/// Text-message branching in `MessagesHelper#message_presentation`. Attachment and sound
/// presentations remain with their controllers. Forwarded Markdown snapshots retain tables/icons.
pub fn text_message_presentation(
    body: &str,
    markdown: bool,
    forwarded_markdown: bool,
    ctx: &RenderContext,
    icons: &dyn markdown::IconResolver,
    asset_host: Option<&str>,
) -> Result<String, Error> {
    if markdown || forwarded_markdown { markdown::presentation(body, ctx, icons, asset_host) } else { message_presentation(body, ctx) }
}

//! Resolving `<action-text-attachment>` nodes to what they attach, and rendering each attachable's
//! partial, as Action Text and the Smartfire fork do.

use base64::Engine;
use regex::Regex;
use std::sync::LazyLock;

use crate::Error;
use crate::dom::{Dom, NodeId};
use crate::ruby::{html_escape, is_blank, presence, truncate};
use crate::uri::{self, UriError};

pub const MENTION_CONTENT_TYPE: &str = "application/vnd.campfire.mention";
pub const OPENGRAPH_EMBED_CONTENT_TYPE: &str = "application/vnd.actiontext.opengraph-embed";
const TWITTER_AVATAR_URL_PREFIX: &str = "https://pbs.twimg.com/profile_images";

/// What the app knows about a user, for rendering `users/_mention.html.erb`. Plain text and
/// mentions only read `id` and `name`; the other fields are only rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionUser {
    pub id: i64,
    pub name: String,
    /// `User#title`: name and bio joined with " – ".
    pub title: String,
    /// `user.attachable_sgid`: a freshly minted SGID for the "attachable" purpose.
    pub attachable_sgid: String,
    /// `user_path(user)`
    pub user_path: String,
    /// `fresh_user_avatar_path(user)`
    pub avatar_path: String,
}

/// A record `GlobalID.find` located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GidLookup {
    User(MentionUser),
    /// Found, but not a `User`: the invalid-signature fallback ignores it.
    OtherModel,
    /// `GlobalID.find` returned nil or raised `ActiveRecord::RecordNotFound`.
    NotFound,
    /// `GlobalID.find` raised anything else (an unknown model constant, say).
    Raises,
}

/// What a signed GlobalID verified for the "attachable" purpose points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignedLookup {
    /// The signature verified and the user exists.
    User(MentionUser),
    /// The signature verified (`SignedGlobalID.parse` succeeds) but the record is gone.
    MissingRecord { model_name: String },
    /// Bad signature, wrong purpose, expired, or not an SGID at all.
    Invalid,
}

/// Application inputs for record resolution and server-rendered paths/avatars. Signature
/// verification and request-scoped preloading belong to the app; this crate stays DB-free.
pub trait AttachableResolver {
    /// `GlobalID::Locator.locate_signed(sgid, for: "attachable")` (and, when that finds nothing,
    /// whether `SignedGlobalID.parse(sgid, for: "attachable")` still verifies). Campfire's only
    /// attachables are users (mentions).
    fn locate_signed(&self, sgid: &str) -> SignedLookup;

    /// `GlobalID.find(gid)` for a `gid://` URI string, with no signature involved.
    fn find_gid(&self, gid: &str) -> GidLookup;

    /// `Message::MentionPreloader.preloaded_user_for`: request-scoped, server-loaded User
    /// lookup by the SGID's unverified GID. This precedes both normal Action Text lookups.
    /// Return None when no matching User is cached, including malformed payloads.
    fn preloaded_user_for_sgid(&self, _sgid: &str) -> Option<MentionUser> {
        None
    }

    /// `Embeds::ImageProxy.signed_path`: signing belongs to the application's verifier.
    /// Fail closed until wired; never send a viewer directly to the remote embed image.
    fn embed_image_path(&self, _url: &str) -> Result<String, Error> {
        Err(Error::Raised("embed_image signer unavailable"))
    }

    /// The fork hides legacy OpenGraph cards once a first-class Twitter post row exists.
    fn twitter_post_exists_for_url(&self, _url: &str) -> bool {
        false
    }

    /// `avatar_image_tag(user, size: 48, aria: { hidden: true })`. Callers with bot icons
    /// supply their server-rendered icon/emoji markup; normal users use the signed avatar URL.
    fn mention_avatar_html(&self, user: &MentionUser) -> String {
        default_mention_avatar(user)
    }
}

/// Everything rendering needs from the request and the app.
pub struct RenderContext<'a> {
    pub resolver: &'a dyn AttachableResolver,
    /// `Current.request_host`
    pub request_host: Option<String>,
}

/// `ActionText::Attachment::OpengraphEmbed` (reference/lib/rails_ext/actiontext_opengraph_embeds.rb)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpengraphEmbed {
    pub href: Option<String>,
    pub url: Option<String>,
    pub filename: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attachable {
    User(MentionUser),
    OpengraphEmbed(OpengraphEmbed),
    /// `ActionText::Attachables::ContentAttachment`
    Content {
        content: String,
    },
    /// `ActionText::Attachables::RemoteImage`
    RemoteImage {
        url: String,
        width: Option<String>,
        height: Option<String>,
    },
    /// Lexxy's `ActionText::Attachables::RemoteVideo`
    RemoteVideo {
        url: String,
        content_type: String,
        width: Option<String>,
        height: Option<String>,
        filename: Option<String>,
    },
    /// `ActionText::Attachables::MissingAttachable`, remembering the model a still-valid SGID named
    Missing {
        signed_model: Option<String>,
    },
}

impl Attachable {
    /// `attachable_content_type`, which only some attachables define.
    pub fn attachable_content_type(&self) -> Result<&str, Error> {
        match self {
            Attachable::User(_) => Ok(MENTION_CONTENT_TYPE),
            Attachable::OpengraphEmbed(_) => Ok(OPENGRAPH_EMBED_CONTENT_TYPE),
            _ => Err(Error::Raised("NoMethodError: attachable_content_type")),
        }
    }
}

/// The attachment node's attributes an attachment reads, plus its resolved attachable.
pub struct Attachment {
    pub attachable: Attachable,
    pub caption: Option<String>,
}

// --- Resolution --------------------------------------------------------------------------------

static OPENGRAPH_CONTENT_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"application/vnd.actiontext.opengraph-embed").unwrap());
static IMAGE_CONTENT_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^image(/.+|$)").unwrap());
static MARSHALED_GID_RE: LazyLock<regex::bytes::Regex> =
    LazyLock::new(|| regex::bytes::Regex::new(r"(?-u)(gid://campfire/[^/]+/[0-9]+)").unwrap());

/// Campfire's `ActionText::Attachment.from_node` (reference/lib/rails_ext/action_text_attachables.rb):
/// an opengraph embed, else a preloaded User, else a User found through a possibly invalid SGID,
/// else Action Text's own lookup.
pub fn attachment_from_node(dom: &Dom, node: NodeId, ctx: &RenderContext) -> Result<Attachment, Error> {
    let attachable = attachable_from_node(dom, node, ctx)?;
    Ok(Attachment { attachable, caption: presence(dom.attr(node, "caption")).map(str::to_string) })
}

fn attachable_from_node(dom: &Dom, node: NodeId, ctx: &RenderContext) -> Result<Attachable, Error> {
    if let Some(embed) = opengraph_embed_from_node(dom, node, ctx)? {
        return Ok(Attachable::OpengraphEmbed(embed));
    }
    if let Some(user) = dom.attr(node, "sgid").and_then(|sgid| ctx.resolver.preloaded_user_for_sgid(sgid)) {
        return Ok(Attachable::User(user));
    }
    if let Some(user) = attachable_from_possibly_expired_sgid(dom.attr(node, "sgid"), ctx)? {
        return Ok(Attachable::User(user));
    }
    Ok(action_text_attachable_from_node(dom, node, ctx))
}

/// `ActionText::Attachable.from_node` from the fork (no Lexxy remote-video fallback).
/// `Content#attachables` (and so `Message#mentionees`) uses this directly, with the request-scoped
/// preload hook but without the uncached invalid-signature fallback.
pub fn action_text_attachable_from_node(dom: &Dom, node: NodeId, ctx: &RenderContext) -> Attachable {
    if let Some(user) = dom.attr(node, "sgid").and_then(|sgid| ctx.resolver.preloaded_user_for_sgid(sgid)) {
        return Attachable::User(user);
    }
    let signed = dom.attr(node, "sgid").map(|sgid| ctx.resolver.locate_signed(sgid)).unwrap_or(SignedLookup::Invalid);
    if let SignedLookup::User(user) = signed {
        return Attachable::User(user);
    }
    let content_type = dom.attr(node, "content-type");
    if let Some(content) = dom.attr(node, "content")
        && content_type.is_some_and(|t| t.contains("html"))
        && !is_blank(content)
    {
        return Attachable::Content { content: content.to_string() };
    }
    if let Some(url) = dom.attr(node, "url")
        && IMAGE_CONTENT_TYPE_RE.is_match(content_type.unwrap_or(""))
    {
        return Attachable::RemoteImage {
            url: url.to_string(),
            width: dom.attr(node, "width").map(str::to_string),
            height: dom.attr(node, "height").map(str::to_string),
        };
    }
    Attachable::Missing {
        signed_model: match signed {
            SignedLookup::MissingRecord { model_name } => Some(model_name),
            _ => None,
        },
    }
}

/// `attachable_from_possibly_expired_sgid`: reads the GlobalID out of an SGID without checking its
/// signature, and only ever returns a User.
fn attachable_from_possibly_expired_sgid(sgid: Option<&str>, ctx: &RenderContext) -> Result<Option<MentionUser>, Error> {
    let Some(sgid) = sgid else { return Ok(None) };
    // `sgid.split("--").first`: Ruby drops trailing empty fields, so "" and "--" have no first
    let Some(message) = sgid.split("--").collect::<Vec<_>>().into_iter().rev().skip_while(|f| f.is_empty()).last() else {
        return Ok(None);
    };
    if is_blank(message) {
        return Ok(None);
    }
    let decoded = decode_base64(message)?;
    // Ruby's JSON parser takes the bytes as UTF-8 without validating what's inside strings
    let json = match crate::ruby::json_parse(&String::from_utf8_lossy(&decoded)) {
        Some(json) => json,
        None if json_parser_error_is_unloggable(&decoded) => return Err(Error::Unrenderable("JSON::ParserError")),
        None => return Err(Error::Raised("JSON::ParserError")),
    };
    let rails = match &json {
        serde_json::Value::Object(map) => map.get("_rails"),
        _ => return Err(Error::Raised("NoMethodError: dig")),
    };
    let rails = match rails {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Object(map)) => Some(map),
        Some(_) => return Err(Error::Raised("TypeError: dig")),
    };
    let truthy =
        |v: Option<&serde_json::Value>| v.filter(|v| !matches!(v, serde_json::Value::Null | serde_json::Value::Bool(false))).cloned();
    let gid: Option<String> = if let Some(data) = truthy(rails.and_then(|r| r.get("data"))) {
        // GlobalID.find of anything but a string finds nothing
        data.as_str().map(str::to_string)
    } else if let Some(message) = truthy(rails.and_then(|r| r.get("message"))) {
        // Rails 7 Marshal-dumped the GID. The signature isn't verified, so the dump can't be
        // safely loaded; the GID is matched out of its bytes instead.
        let serde_json::Value::String(message) = message else {
            return Err(Error::Raised("NoMethodError: unpack1"));
        };
        let bytes = decode_base64(&message)?;
        MARSHALED_GID_RE.find(&bytes).map(|m| String::from_utf8_lossy(m.as_bytes()).into_owned())
    } else {
        None
    };
    let Some(gid) = gid else { return Ok(None) };
    match ctx.resolver.find_gid(&gid) {
        GidLookup::User(user) => Ok(Some(user)),
        GidLookup::OtherModel | GidLookup::NotFound => Ok(None),
        GidLookup::Raises => Err(Error::Raised("GlobalID.find")),
    }
}

/// `Base64.strict_decode64(message) rescue Base64.urlsafe_decode64(message)`
fn decode_base64(message: &str) -> Result<Vec<u8>, Error> {
    let strict = base64::engine::general_purpose::STANDARD;
    if let Ok(bytes) = strict.decode(message) {
        return Ok(bytes);
    }
    let padded = if !message.ends_with('=') && !message.len().is_multiple_of(4) {
        format!("{message}{}", "=".repeat(4 - message.len() % 4))
    } else {
        message.to_string()
    };
    strict.decode(padded.replace('-', "+").replace('_', "/")).map_err(|_| Error::Raised("ArgumentError: invalid base64"))
}

// --- Opengraph embeds --------------------------------------------------------------------------

/// `ActionText::Attachment::OpengraphEmbed.from_node`
pub fn opengraph_embed_from_node(dom: &Dom, node: NodeId, ctx: &RenderContext) -> Result<Option<OpengraphEmbed>, Error> {
    let Some(content_type) = dom.attr(node, "content-type") else {
        return Ok(None);
    };
    if !OPENGRAPH_CONTENT_TYPE_RE.is_match(content_type) {
        return Ok(None);
    }
    let host = ctx.request_host.as_deref().unwrap_or("");
    Ok(Some(OpengraphEmbed {
        href: web_url(dom.attr(node, "href"), host)?,
        url: web_url(dom.attr(node, "url"), host)?,
        filename: dom.attr(node, "filename").map(str::to_string),
        description: dom.attr(node, "caption").map(str::to_string),
    }))
}

/// `web_url`: an absolute http(s) URL on a named host other than this Campfire's.
pub fn web_url(value: Option<&str>, request_host: &str) -> Result<Option<String>, Error> {
    let Some(value) = value.filter(|v| !is_blank(v)) else {
        return Ok(None);
    };
    match uri::parse(value) {
        Err(UriError::InvalidUri) => Ok(None),
        Err(UriError::InvalidComponent) => Err(Error::Raised("URI::InvalidComponentError")),
        Ok(parsed) => {
            if parsed.is_http() && elsewhere(parsed.host.as_deref(), request_host)? {
                Ok(Some(value.to_string()))
            } else {
                Ok(None)
            }
        }
    }
}

fn elsewhere(host: Option<&str>, request_host: &str) -> Result<bool, Error> {
    let Some(host) = host else { return Ok(false) };
    if !named_host(host)? {
        return Ok(false);
    }
    Ok(canonical_host(host) != canonical_host(request_host))
}

fn named_host(host: &str) -> Result<bool, Error> {
    if is_blank(host) || host.contains('%') || !host.contains('.') {
        return Ok(false);
    }
    // `host.split(".").last`: Ruby drops trailing empty labels, and nil.match? raises
    let trimmed = host.trim_end_matches('.');
    if trimmed.is_empty() {
        return Err(Error::Raised("NoMethodError: match?"));
    }
    let label = trimmed.rsplit('.').next().unwrap_or("");
    Ok(label.chars().any(|c| c.is_ascii_alphabetic()) && !label.to_ascii_lowercase().starts_with("0x"))
}

fn canonical_host(host: &str) -> String {
    let lower = host.to_lowercase();
    lower.strip_suffix('.').map(str::to_string).unwrap_or(lower)
}

impl OpengraphEmbed {
    pub fn twitter_avatar(&self) -> bool {
        self.url.as_deref().unwrap_or("").starts_with(TWITTER_AVATAR_URL_PREFIX)
    }
}

// --- Partials ----------------------------------------------------------------------------------

/// `render_action_text_attachment(attachment)`: the attachable's partial, chomped. `render_content`
/// renders a nested content attachment's own content (`ContentAttachment#to_html`).
pub fn render_attachment(
    attachment: &Attachment,
    ctx: &RenderContext,
    render_content: &dyn Fn(&str) -> Result<String, Error>,
) -> Result<String, Error> {
    let html = match &attachment.attachable {
        Attachable::User(user) => render_mention_in_context(user, ctx),
        Attachable::OpengraphEmbed(embed) => render_opengraph_embed(embed, ctx)?,
        // A still-valid User SGID whose row is gone raises in the fork's missing-partial lookup.
        Attachable::Missing { signed_model: Some(model) } if model == "User" => {
            return Err(Error::Raised("NoMethodError: to_missing_attachable_partial_path"));
        }
        Attachable::Missing { .. } => "☒".to_string(),
        Attachable::Content { content } => {
            format!("<figure class=\"attachment attachment--content\">\n  {}\n</figure>\n", render_content(content)?)
        }
        Attachable::RemoteImage { url, width, height } => {
            let mut html = String::from("<figure class=\"attachment attachment--preview\">\n  ");
            html.push_str(&image_tag(url, width.as_deref(), height.as_deref())?);
            html.push('\n');
            if let Some(caption) = &attachment.caption {
                html.push_str(&format!(
                    "    <figcaption class=\"attachment__caption\">\n      {}\n    </figcaption>\n",
                    html_escape(caption)
                ));
            }
            html.push_str("</figure>\n");
            html
        }
        Attachable::RemoteVideo { url, content_type, width, height, .. } => {
            let mut html =
                String::from("<figure class=\"attachment attachment--preview attachment--video\">\n  <video controls=\"controls\"");
            for (name, value) in [("width", width), ("height", height)] {
                if let Some(v) = value {
                    html.push_str(&format!(" {name}=\"{}\"", html_escape(v)));
                }
            }
            html.push_str(&format!(">\n    <source src=\"{}\" type=\"{}\">\n</video>", html_escape(url), html_escape(content_type)));
            if let Some(caption) = &attachment.caption {
                html.push_str(&format!(
                    "    <figcaption class=\"attachment__caption\">\n      {}\n    </figcaption>\n",
                    html_escape(caption)
                ));
            }
            html.push_str("</figure>\n");
            html
        }
    };
    Ok(crate::ruby::chomp(&html).to_string())
}

/// reference/app/views/users/_mention.html.erb, with `avatar_tag` (users/avatars_helper.rb).
fn default_mention_avatar(user: &MentionUser) -> String {
    format!("<img aria-hidden=\"true\" src=\"{}\" width=\"48\" height=\"48\" />", html_escape(&user.avatar_path))
}

pub fn render_mention(user: &MentionUser) -> String {
    render_mention_with_avatar(user, &default_mention_avatar(user))
}

pub fn render_mention_in_context(user: &MentionUser, ctx: &RenderContext) -> String {
    render_mention_with_avatar(user, &ctx.resolver.mention_avatar_html(user))
}

fn render_mention_with_avatar(user: &MentionUser, avatar: &str) -> String {
    format!(
        "<div class=\"mention mention--user-{}\" sgid=\"{}\" data-user-id=\"{}\">\n  <a title=\"{}\" class=\"btn avatar\" data-turbo-frame=\"_top\" data-action=\"click-&gt;profile-card#open\" data-profile-card-url=\"{}/card\" href=\"{}\">{}</a>\n  <button name=\"button\" type=\"button\" class=\"profile-card-name\" data-action=\"click-&gt;profile-card#open\" data-profile-card-url=\"{}/card\">{}</button>\n</div>\n",
        user.id,
        html_escape(&user.attachable_sgid),
        user.id,
        html_escape(&user.title),
        html_escape(&user.user_path),
        html_escape(&user.user_path),
        avatar,
        html_escape(&user.user_path),
        html_escape(&user.name),
    )
}

/// reference/app/views/action_text/attachables/_opengraph_embed.html.erb
pub fn render_opengraph_embed(embed: &OpengraphEmbed, ctx: &RenderContext) -> Result<String, Error> {
    if embed.href.as_deref().is_some_and(|url| ctx.resolver.twitter_post_exists_for_url(url)) {
        return Ok(String::new());
    }
    let title = match (&embed.href, &embed.filename) {
        (Some(href), filename) => {
            let text = match filename {
                Some(f) => html_escape(&truncate(f, 280, "…")),
                None => html_escape(href),
            };
            format!("<a rel=\"noreferrer\" target=\"_blank\" href=\"{}\">{}</a>", html_escape(href), text)
        }
        (None, Some(f)) => html_escape(&truncate(f, 280, "…")),
        (None, None) => String::new(),
    };
    let mut html = format!(
        "<figure class=\"attachment attachment--content attachment--og\">\n  <actiontext-opengraph-embed>\n    <div class=\"og-embed gap\">\n      <div class=\"og-embed__content\">\n        <div class=\"og-embed__title\">\n          {}\n        </div>\n        <div class=\"og-embed__description\">{}</div>\n      </div>\n",
        title,
        html_escape(&truncate(embed.description.as_deref().unwrap_or(""), 560, "…")),
    );
    if let Some(url) = &embed.url {
        html.push_str(&format!(
            "        <div class=\"og-embed__image\">\n          <img src=\"{}\" class=\"image center\" alt=\"\">\n        </div>\n",
            html_escape(&ctx.resolver.embed_image_path(url)?)
        ));
    }
    html.push_str("    </div>\n  </actiontext-opengraph-embed>\n</figure>\n");
    Ok(html)
}

static ASSET_URI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?mi)^[-a-z]+://|^(?:cid|data):|^//").unwrap());

/// `image_tag(url, width:, height:)` for a remote image. Sources that aren't URLs go through the
/// asset pipeline, which raises for anything it doesn't know; a rooted path passes through.
fn image_tag(url: &str, width: Option<&str>, height: Option<&str>) -> Result<String, Error> {
    let src = if is_blank(url) {
        String::new()
    } else if ASSET_URI_RE.is_match(url) || url.starts_with('/') {
        url.to_string()
    } else {
        return Err(Error::Raised("Propshaft::MissingAssetError"));
    };
    let mut html = String::from("<img");
    for (name, value) in [("width", width), ("height", height)] {
        if let Some(v) = value {
            html.push_str(&format!(" {name}=\"{}\"", html_escape(v)));
        }
    }
    html.push_str(&format!(" src=\"{}\" />", html_escape(&src)));
    Ok(html)
}

/// `Attachment#to_plain_text`
pub fn attachment_plain_text(attachment: &Attachment) -> PlainTextRepresentation {
    let caption = attachment.caption.clone();
    match &attachment.attachable {
        Attachable::User(user) => PlainTextRepresentation::Html(format!("@{}", user.name)),
        Attachable::OpengraphEmbed(_) => PlainTextRepresentation::Html(String::new()),
        Attachable::Content { content } => PlainTextRepresentation::Content(content.clone()),
        Attachable::RemoteImage { .. } => PlainTextRepresentation::Html(format!("[{}]", caption.unwrap_or_else(|| "Image".into()))),
        Attachable::RemoteVideo { filename, .. } => {
            PlainTextRepresentation::Html(format!("[{}]", caption.or_else(|| filename.clone()).unwrap_or_else(|| "Video".into())))
        }
        Attachable::Missing { .. } => PlainTextRepresentation::Html(caption.unwrap_or_default()),
    }
}

/// A plain-text representation replaces the attachment node: strings are parsed as markup in the
/// node's parent, while a content attachment's fragment is moved in as is.
pub enum PlainTextRepresentation {
    Html(String),
    Content(String),
}

// json 2.21.2's parser.c: first-key colon, array separators and EOF errors use fixed
// ASCII messages. Other errors quote a bounded byte fragment from the parser cursor.
// Invalid UTF-8 elsewhere in the payload therefore does not make logging fail.
fn json_parser_error_is_unloggable(decoded: &[u8]) -> bool {
    if std::str::from_utf8(decoded).is_ok() {
        return false;
    }
    let mut masked = decoded.to_vec();
    let mut offset = 0;
    while let Err(error) = std::str::from_utf8(&masked[offset..]) {
        offset += error.valid_up_to();
        let end = offset + error.error_len().unwrap_or(masked.len() - offset);
        masked[offset..end].fill(b'?');
        offset = end;
    }
    let mut in_string = false;
    let mut index = 0;
    while index < masked.len() {
        match masked[index] {
            b'\\' if in_string => index += 1,
            b'"' => in_string = !in_string,
            b'/' if !in_string && masked.get(index + 1) == Some(&b'*') => {
                let start = index;
                index += 2;
                while index + 1 < masked.len() && &masked[index..index + 2] != b"*/" {
                    index += 1;
                }
                if index + 1 >= masked.len() {
                    return false;
                }
                index += 1;
                for byte in &mut masked[start..=index] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
            }
            b'/' if !in_string && masked.get(index + 1) == Some(&b'/') => {
                while index < masked.len() && masked[index] != b'\n' {
                    masked[index] = b' ';
                    index += 1;
                }
                if index == masked.len() {
                    return false;
                }
            }
            _ => {}
        }
        index += 1;
    }
    let Err(error) = serde_json::from_slice::<serde_json::Value>(&masked) else {
        return false;
    };
    let reason = error.to_string();
    if error.is_eof() || reason.starts_with("expected `,` or `]`") {
        return false;
    }
    let line_start = masked.iter().enumerate().filter(|(_, b)| **b == b'\n').nth(error.line().saturating_sub(2)).map_or(0, |(i, _)| i + 1);
    let mut cursor = if error.line() == 1 { 0 } else { line_start } + error.column().saturating_sub(1);
    cursor = cursor.min(decoded.len());
    if reason.starts_with("expected `:`") {
        let mut stack = Vec::new();
        let mut quoted = false;
        let mut i = 0;
        while i < cursor {
            match masked[i] {
                b'\\' if quoted => i += 1,
                b'"' => quoted = !quoted,
                b'{' | b'[' if !quoted => stack.push((masked[i], false)),
                b'}' | b']' if !quoted => {
                    stack.pop();
                }
                b',' if !quoted => {
                    if let Some(last) = stack.last_mut() {
                        last.1 = true;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if stack.last() == Some(&(b'{', false)) {
            return false;
        }
    }
    if reason.starts_with("invalid number") {
        while cursor > 0 && b"-+0123456789.eE".contains(&masked[cursor - 1]) {
            cursor -= 1;
        }
    }
    if reason.starts_with("invalid escape") && masked.get(cursor) == Some(&b'\\') {
        cursor += 1;
    }
    if (reason.starts_with("invalid escape") || reason.contains("unicode") || reason.contains("surrogate"))
        && let Some(start) = masked[..cursor].iter().rposition(|byte| *byte == b'\\')
        && masked.get(start + 1) == Some(&b'u')
    {
        cursor = start;
    }
    if reason.starts_with("control character") && error.column() == 0 {
        cursor = cursor.saturating_sub(1);
    }
    let rest = &decoded[cursor..];
    let mut length = rest.iter().take(32).position(|b| b"\0\n \t\r".contains(b)).unwrap_or(rest.len().min(32));
    if length == 0 {
        let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
        return std::str::from_utf8(&rest[..end]).is_err();
    }
    while length > 0 && (0x80..0xc0).contains(&rest[length - 1]) {
        length -= 1;
    }
    if length > 0 && rest[length - 1] >= 0xc0 {
        length -= 1;
    }
    std::str::from_utf8(&rest[..length]).is_err()
}

#[cfg(test)]
mod json_error_tests {
    use super::*;

    #[test]
    fn malformed_json_error_encoding_matches_the_ruby_gem() {
        let cases: serde_json::Value = serde_json::from_str(include_str!("../tests/corpus/json-errors.json")).unwrap();
        let mut failures = Vec::new();
        for case in cases.as_array().unwrap() {
            let decoded = decode_base64(case["payload"].as_str().unwrap()).unwrap();
            let actual = json_parser_error_is_unloggable(&decoded);
            let expected = case["unloggable"].as_bool().unwrap();
            if actual != expected {
                failures.push(format!("{}: {actual} != {expected}: {}", case["name"], case["message"]));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

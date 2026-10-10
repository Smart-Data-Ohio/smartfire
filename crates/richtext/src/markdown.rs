//! `app/models/message/markdown.rb`. Rendering is DB-free: callers supply room members,
//! signed user attachment IDs, and the current icon catalog (including digested asset URLs).
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use comrak::Options;
use comrak::nodes::{AstNode, NodeValue};
use regex::Regex;

use crate::attachables::{self, Attachable, MENTION_CONTENT_TYPE, MentionUser, RenderContext};
use crate::content::{Content, attachment_nodes};
use crate::dom::{Dom, NodeId};
use crate::ruby::{is_blank, strip};
use crate::sanitizer::{self, ATTACHMENT_ATTRIBUTES, SafeList};
use crate::{Error, uri};

pub const SOURCE_LIMIT: usize = 50_000;
pub const MARKDOWN_TAGS: &[&str] = &[
    "a",
    "blockquote",
    "br",
    "code",
    "del",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "input",
    "li",
    "ol",
    "p",
    "pre",
    "span",
    "strong",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "ul",
];
pub const MARKDOWN_ATTRIBUTES: &[&str] =
    &["align", "checked", "class", "data-spoiler", "disabled", "href", "rel", "start", "target", "title", "type"];
pub const ALLOWED_CLASSES: &[&str] = &["contains-task-list", "markdown-body", "spoiler", "task-list-item"];
const BLOCK_TAGS: &[&str] = &["blockquote", "h1", "h2", "h3", "h4", "h5", "h6", "li", "ol", "p", "pre", "table", "tr", "ul"];
static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@\[([^\[\]\r\n]+)\]|<@([1-9][0-9]*)>").unwrap());
static SHORTCODE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":([a-z0-9_]+):").unwrap());
static ICON_ALT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:([a-z0-9_]+):$").unwrap());
static LANGUAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^language-[a-zA-Z0-9_+#.\-]+$").unwrap());
static AVATAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^/users/[^/?#]+/avatar(?:[?#]|$)").unwrap());
static EMOJI_ALIASES: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../data/gemoji-4.1.0.json")).expect("Ruby-generated gemoji alias table"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    Brand { name: String, title: String, url: Option<String> },
    Custom { name: String, title: String, url: String },
    Emoji(String),
}

pub trait IconResolver {
    /// Mirrors `Icons.find`: brand aliases, then workspace icons, then gemoji aliases.
    fn find(&self, name: &str) -> Option<Icon>;
}

/// Brand entries include aliases pointing to the canonical record. URLs come from the asset
/// system; workspace URLs use `/icons/:name`. Unicode aliases are generated from gemoji 4.1.0.
#[derive(Debug, Default)]
pub struct IconCatalog {
    pub brands: HashMap<String, Icon>,
    pub custom: HashMap<String, Icon>,
}
impl IconResolver for IconCatalog {
    fn find(&self, name: &str) -> Option<Icon> {
        let key = name.trim().to_lowercase();
        self.find_normalized(&key)
    }
}
impl IconCatalog {
    /// WS8br model-validation seam: Room has already applied Ruby's ASCII String#strip
    /// and downcase. Resolve that exact storage key without Unicode-trimming it again.
    /// The existing markdown entry point retains its behavior and all owner internals.
    pub fn find_normalized(&self, key: &str) -> Option<Icon> {
        self.brands
            .get(key)
            .or_else(|| self.custom.get(key))
            .cloned()
            .or_else(|| EMOJI_ALIASES.get(key).map(|raw| Icon::Emoji(raw.clone())))
    }
}

pub struct RoomMember {
    pub user: MentionUser,
    pub active: bool,
}

pub trait MentionResolver {
    /// Exactly one active member of the current room with this exact, case-sensitive name.
    fn unique_active_member(&self, name: &str) -> Option<MentionUser>;
    /// An active member of the current room with this stable user id.
    fn active_member(&self, _id: i64) -> Option<MentionUser> {
        None
    }
}
impl MentionResolver for &[RoomMember] {
    fn unique_active_member(&self, name: &str) -> Option<MentionUser> {
        let mut matches = self.iter().filter(|m| m.active && m.user.name == name);
        let first = matches.next()?;
        matches.next().is_none().then(|| first.user.clone())
    }
    fn active_member(&self, id: i64) -> Option<MentionUser> {
        self.iter()
            .find(|m| m.active && m.user.id == id)
            .map(|m| m.user.clone())
    }
}
impl<F: Fn(&str) -> Option<MentionUser>> MentionResolver for F {
    fn unique_active_member(&self, name: &str) -> Option<MentionUser> {
        self(name)
    }
}

pub fn markdown_allowlist() -> SafeList {
    SafeList { tags: MARKDOWN_TAGS.to_vec(), attributes: MARKDOWN_ATTRIBUTES.to_vec() }
}
pub fn presentation_allowlist() -> SafeList {
    let mut tags = MARKDOWN_TAGS.to_vec();
    tags.extend(["action-text-attachment", "div", "figure", "figcaption", "img", "span"]);
    let mut attributes = MARKDOWN_ATTRIBUTES.to_vec();
    attributes.extend(ATTACHMENT_ATTRIBUTES);
    attributes.extend(["alt", "aria-hidden", "data-turbo-frame", "data-user-id", "draggable", "height", "src", "width"]);
    SafeList { tags, attributes }
}

/// The validation boundary in `Message#render_markdown_body` counts Unicode characters, not bytes.
/// The source is never truncated. Rails validates blank source separately, considering attachments.
pub fn render(source: &str, mentions: &dyn MentionResolver, icons: &dyn IconResolver) -> Result<String, Error> {
    if source.chars().count() > SOURCE_LIMIT {
        return Err(Error::SourceTooLong);
    }
    let (protected, tokens, pattern) = protect_mentions(source);
    let mut options = Options::default();
    options.extension.autolink = true;
    options.extension.spoiler = true;
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tagfilter = true;
    options.extension.tasklist = true;
    // Commonmarker Config::OPTIONS defaults (not comrak's defaults), overridden by our Ruby.
    options.parse.default_info_string = Some(String::new());
    options.render.width = 80;
    options.render.escaped_char_spans = true;
    options.render.r#unsafe = false;
    options.render.hardbreaks = false;
    options.render.github_pre_lang = false;
    let protected = end_autolinks_at_spoilers(&protected, &options);
    let html = comrak::markdown_to_html(&protected, &options);
    let safe = sanitizer::sanitize(&html, &markdown_allowlist()).map_err(Error::Parse)?;
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&safe).map_err(Error::Parse)?;
    constrain_generated_markup(&mut dom, root);
    if !tokens.is_empty() {
        restore_mentions(&mut dom, root, &tokens, &pattern, mentions);
    }
    expand_shortcodes(&mut dom, root, icons);
    Ok(dom.to_html(root))
}

/// What ends a bare URL before a `||` (`AUTOLINK_BREAK`): comrak's autolinks stop at `<`, and
/// raw HTML renders as nothing here (`unsafe` is off and the sanitizer drops the comment).
const AUTOLINK_BREAK: &str = "<!-- -->";

/// GFM autolinks run to the next space, so `||https://example.com/x||` makes the closing `||`
/// part of the URL and the spoiler never closes: the URL would show, and unfurl. Each bare URL
/// (or `www.` link) holding a `||` is ended just before that `||`, and the break is kept only if
/// the parser then puts the link inside a spoiler. A URL whose `||` closes no spoiler
/// (`https://example.com/a||b`) renders as before. Positions come from the parser's own source
/// positions, so code spans and link destinations are never touched. Every candidate is tried at
/// once and the ones that make no spoiler are dropped, so a message takes a few parses, not one
/// per URL.
fn end_autolinks_at_spoilers<'s>(source: &'s str, options: &Options) -> Cow<'s, str> {
    if !source.contains("||") || !(source.contains("://") || source.contains("www.")) {
        return source.into();
    }
    // Break offsets in `source` (just before a `||`), each with the start of the URL it ends.
    let mut breaks = BTreeMap::<usize, usize>::new();
    let mut rejected = BTreeSet::<usize>::new();
    for _ in 0..8 {
        let attempt = with_breaks(source, &breaks);
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, &attempt, options);
        let lines = line_starts(&attempt);
        let mut spoilered = BTreeSet::new();
        let mut candidates = Vec::new();
        for node in root.descendants() {
            let Some((at, text)) = autolink_at(node, &attempt, &lines) else {
                continue;
            };
            // A link's text never holds a break: the `<` in it ends the URL.
            let start = source_offset(at, &breaks);
            if node.ancestors().any(|ancestor| matches!(ancestor.data().value, NodeValue::SpoileredText)) {
                spoilered.insert(start);
            }
            if let Some(bar) = text.find("||") {
                candidates.push((start + bar, start));
            }
        }
        let mut changed = false;
        breaks.retain(|_, start| {
            let keep = spoilered.contains(start);
            if !keep {
                rejected.insert(*start);
                changed = true;
            }
            keep
        });
        for (bar, start) in candidates {
            if !rejected.contains(&start) && breaks.insert(bar, start).is_none() {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if breaks.is_empty() { source.into() } else { with_breaks(source, &breaks).into() }
}

/// `source` with `AUTOLINK_BREAK` inserted at each break offset.
fn with_breaks(source: &str, breaks: &BTreeMap<usize, usize>) -> String {
    let mut out = String::with_capacity(source.len() + breaks.len() * AUTOLINK_BREAK.len());
    let mut cursor = 0;
    for &at in breaks.keys() {
        out.push_str(&source[cursor..at]);
        out.push_str(AUTOLINK_BREAK);
        cursor = at;
    }
    out.push_str(&source[cursor..]);
    out
}

/// An offset in `with_breaks(source, breaks)`, outside any break, mapped back to `source`.
fn source_offset(at: usize, breaks: &BTreeMap<usize, usize>) -> usize {
    let before = breaks.keys().enumerate().take_while(|&(index, &position)| position + index * AUTOLINK_BREAK.len() < at).count();
    at - before * AUTOLINK_BREAK.len()
}

/// A link the autolink extension made: its label is its own source text (`https://…`, or
/// `www.…` for `http://www.…`), found at its source position. Returns that byte offset and text.
fn autolink_at<'n>(node: &'n AstNode<'n>, source: &str, lines: &[usize]) -> Option<(usize, String)> {
    let data = node.data();
    let NodeValue::Link(link) = &data.value else {
        return None;
    };
    let child = node.first_child()?;
    if child.next_sibling().is_some() {
        return None;
    }
    let NodeValue::Text(text) = &child.data().value else {
        return None;
    };
    if link.url != text.as_ref() && link.url.strip_prefix("http://") != Some(text.as_ref()) {
        return None;
    }
    let position = data.sourcepos.start;
    let start = lines.get(position.line.checked_sub(1)?)? + position.column.checked_sub(1)?;
    source.get(start..)?.starts_with(text.as_ref()).then(|| (start, text.to_string()))
}

fn line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0).chain(source.match_indices('\n').map(|(at, _)| at + 1)).collect()
}

pub fn mention_token(name: &str) -> Option<String> {
    (!is_blank(name) && !name.contains(['[', ']', '\r', '\n'])).then(|| format!("@[{name}]"))
}

pub fn user_mention_token(id: i64) -> String {
    format!("<@{id}>")
}

fn protect_mentions(source: &str) -> (String, Vec<String>, Regex) {
    // Unpredictable and absent from the input, so a user cannot forge an attachment placeholder.
    let prefix = loop {
        let bytes: [u8; 12] = rand::random();
        let hex = bytes.iter().map(|byte| format!("{byte:02X}")).collect::<String>();
        let prefix = format!("SMARTFIREMENTION{hex}");
        if !source.contains(&prefix) {
            break prefix;
        }
    };
    let mut protected = String::with_capacity(source.len());
    let mut tokens = Vec::new();
    let mut cursor = 0;
    for c in MENTION_RE.captures_iter(source) {
        let m = c.get(0).unwrap();
        if m.start() > 0 && source.as_bytes()[m.start() - 1] == b'\\' {
            continue;
        }
        protected.push_str(&source[cursor..m.start()]);
        protected.push_str(&format!("{prefix}{}TOKEN", tokens.len()));
        tokens.push(m.as_str().to_owned());
        cursor = m.end();
    }
    protected.push_str(&source[cursor..]);
    let pattern = Regex::new(&format!("{prefix}([0-9]+)TOKEN")).unwrap();
    (protected, tokens, pattern)
}

/// A Markdown source with no stored HTML (a scheduled message) as preview text: the Scheduled
/// page and the inbox notice when one isn't sent. With no `||` it is the source as written, as
/// the classic page shows it. Otherwise it is rendered as a message is (`render`, spoilers
/// included) and read back as `plain_text`, so it agrees with the message exactly: a spoiler is
/// the word "spoiler", a link or image around one shows no URL or title, and reference definitions
/// are resolved, never shown. A source that can't be rendered (too long) is all hidden.
pub fn redacted_excerpt(source: &str) -> String {
    if !source.contains("||") {
        return source.to_owned();
    }
    let icons = IconCatalog::default();
    let ctx = RenderContext { resolver: &NoAttachables, request_host: None };
    render(source, &(|_: &str| None), &icons)
        .and_then(|html| plain_text(&html, &ctx, &icons))
        .unwrap_or_else(|_| "spoiler".to_owned())
}

/// No mentions or attachments resolve in an excerpt: names stay as written.
struct NoAttachables;
impl crate::AttachableResolver for NoAttachables {
    fn locate_signed(&self, _: &str) -> crate::SignedLookup {
        crate::SignedLookup::Invalid
    }
    fn find_gid(&self, _: &str) -> crate::GidLookup {
        crate::GidLookup::NotFound
    }
}

/// A spoiler span, as stored HTML may mark one (`sanitizer::is_spoiler_span`).
pub fn is_spoiler(dom: &Dom, node: NodeId) -> bool {
    sanitizer::is_spoiler_span(dom, node)
}

/// Whether `node` hides a spoiler's words: the spoiler itself, or a link whose label holds one
/// (`[||ending||](url "title")`). The link's URL and title would tell what the spoiler hides, so
/// unfurls, cards and references skip both.
pub fn conceals_spoiler(dom: &Dom, node: NodeId) -> bool {
    is_spoiler(dom, node) || (dom.local_name(node) == Some("a") && dom.descendants(node).into_iter().any(|inner| is_spoiler(dom, inner)))
}

fn constrain_generated_markup(dom: &mut Dom, root: NodeId) {
    for node in dom.descendants(root) {
        if let Some(classes) = dom.attr(node, "class") {
            let classes =
                classes.split_whitespace().filter(|c| ALLOWED_CLASSES.contains(c) || LANGUAGE_RE.is_match(c)).collect::<Vec<_>>().join(" ");
            if classes.is_empty() {
                dom.remove_attr(node, "class");
            } else {
                dom.set_attr(node, "class", &classes);
            }
        }
        // Comrak's spoiler extension emits `<span class="spoiler">`. `data-spoiler` is the only
        // new attribute, and the SPA and plain-text previews key off it. Escaped-character spans
        // are not spoilers: unwrap them so they don't appear in stored HTML. Spoilers nest one
        // level only: a spoiler inside another is unwrapped, so `||a ||b|| c||` hides all three.
        if dom.local_name(node) == Some("span") {
            let spoiler = dom.attr(node, "class").is_some_and(|classes| classes.split_whitespace().any(|class| class == "spoiler"));
            let nested = spoiler && dom.ancestors(node).iter().any(|&ancestor| dom.has_attr(ancestor, "data-spoiler"));
            if spoiler && !nested {
                dom.set_attr(node, "data-spoiler", "");
            } else if dom.parent(node).is_some() {
                for child in dom.children(node).to_vec() {
                    dom.insert_before(node, child);
                }
                dom.detach(node);
            }
        }
        if dom.local_name(node) == Some("input") {
            if dom.attr(node, "type") == Some("checkbox") {
                dom.set_attr(node, "disabled", "disabled");
                dom.remove_attr(node, "value");
            } else {
                dom.detach(node);
            }
        }
        if dom.local_name(node) == Some("a") {
            let href = dom.attr(node, "href").map(str::to_owned);
            if let Some(href) = href {
                if is_blank(&href) {
                    dom.remove_attr(node, "href");
                } else if !in_app_href(&href) {
                    dom.set_attr(node, "target", "_blank");
                    dom.set_attr(node, "rel", "nofollow noopener noreferrer");
                }
            }
        }
    }
}

fn skipped(dom: &Dom, node: NodeId, tags: &[&str]) -> bool {
    dom.ancestors(node).iter().any(|&a| dom.local_name(a).is_some_and(|name| tags.contains(&name)))
}
fn restore_mentions(dom: &mut Dom, root: NodeId, tokens: &[String], pattern: &Regex, mentions: &dyn MentionResolver) {
    let users = tokens
        .iter()
        .map(|token| {
            let user = if token.starts_with("@[") {
                mentions.unique_active_member(&token[2..token.len() - 1])
            } else {
                token[2..token.len() - 1]
                    .parse()
                    .ok()
                    .and_then(|id| mentions.active_member(id))
            };
            (token, user)
        })
        .collect::<HashMap<_, _>>();
    for node in dom.descendants(root) {
        for (key, value) in dom.attrs(node) {
            let restored = pattern.replace_all(&value, |c: &regex::Captures| tokens[c[1].parse::<usize>().unwrap()].clone());
            dom.set_attr(node, &key, &restored);
        }
        let Some(text) = dom.text(node).map(str::to_owned) else {
            continue;
        };
        if !pattern.is_match(&text) {
            continue;
        }
        let skip = skipped(dom, node, &["a", "code", "pre"]);
        let mut replacements = Vec::new();
        // Nokogiri coalesces adjacent inserted text nodes. Keep unresolved mentions in the
        // same text run: shortcode expansion must not see its preceding space as a blank tail.
        let mut pending_text = String::new();
        let mut cursor = 0;
        for c in pattern.captures_iter(&text) {
            let m = c.get(0).unwrap();
            if cursor < m.start() {
                pending_text.push_str(&text[cursor..m.start()]);
            }
            let token = &tokens[c[1].parse::<usize>().unwrap()];
            let user = if skip { None } else { users.get(token).and_then(Option::as_ref) };
            match user {
                Some(user) => {
                    if !pending_text.is_empty() {
                        replacements.push(dom.create_text(&pending_text));
                        pending_text.clear();
                    }
                    replacements.push(dom.create_element("action-text-attachment", &[("sgid", &user.attachable_sgid), ("content-type", MENTION_CONTENT_TYPE)]));
                }
                None => pending_text.push_str(token),
            }
            cursor = m.end();
        }
        if !is_blank(&text[cursor..]) {
            pending_text.push_str(&text[cursor..]);
        }
        if !pending_text.is_empty() {
            replacements.push(dom.create_text(&pending_text));
        }
        dom.replace_with_nodes(node, &replacements);
    }
}

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
fn shortcode(text: &str) -> Option<(usize, usize, &str)> {
    for c in SHORTCODE_RE.captures_iter(text) {
        let m = c.get(0).unwrap();
        let left = text[..m.start()].chars().next_back();
        let right = text[m.end()..].chars().next();
        if left.is_none_or(|c| !word(c) && c != ':') && right.is_none_or(|c| !word(c)) {
            return Some((m.start(), m.end(), c.get(1).unwrap().as_str()));
        }
    }
    None
}
fn icon_node(dom: &mut Dom, name: &str, icons: &dyn IconResolver) -> NodeId {
    let icon = icons.find(name);
    let (canonical, title, url, class) = match icon {
        Some(Icon::Emoji(raw)) => return dom.create_text(&raw),
        Some(Icon::Brand { name, title, url: Some(url) }) => (name, title, url, "icon icon--brand"),
        Some(Icon::Custom { name, title, url }) => (name, title, url, "icon icon--custom"),
        _ => return dom.create_text(&format!(":{name}:")),
    };
    dom.create_element(
        "img",
        &[("class", class), ("src", &url), ("alt", &format!(":{canonical}:")), ("title", &title), ("draggable", "false")],
    )
}
fn expand_shortcodes(dom: &mut Dom, root: NodeId, icons: &dyn IconResolver) {
    for node in dom.descendants(root) {
        if skipped(dom, node, &["a", "action-text-attachment", "code", "pre"]) {
            continue;
        }
        let Some(text) = dom.text(node).map(str::to_owned) else {
            continue;
        };
        let mut remaining = text.as_str();
        let mut replacements = Vec::new();
        while let Some((start, end, name)) = shortcode(remaining) {
            if start > 0 {
                replacements.push(dom.create_text(&remaining[..start]));
            }
            replacements.push(icon_node(dom, name, icons));
            remaining = &remaining[end..];
        }
        if !replacements.is_empty() {
            if !is_blank(remaining) {
                replacements.push(dom.create_text(remaining));
            }
            dom.replace_with_nodes(node, &replacements);
        }
    }
}

pub fn in_app_href(href: &str) -> bool {
    href.starts_with('/')
        && !href.starts_with("//")
        && !href.starts_with("/\\")
        && !href.starts_with("/rails/")
        && !href.contains(['\t', '\n', '\r'])
}

/// Asset-host input is the string after evaluating Rails' optional Proc on `/users/x/avatar`.
/// Plain, URL, protocol-relative, trailing slash and `%d` wildcard host forms match Rails.
pub fn avatar_src(src: &str, asset_host: Option<&str>) -> bool {
    let Ok(parsed) = uri::parse(src) else {
        return false;
    };
    if !AVATAR_RE.is_match(parsed.path.as_deref().unwrap_or("")) {
        return false;
    }
    if parsed.scheme.is_none() && parsed.host.is_none() {
        return true;
    }
    if !parsed.is_http() {
        return false;
    }
    let Some(configured) = asset_host.filter(|h| !is_blank(h)) else {
        return false;
    };
    let configured = configured.trim_start_matches("//");
    let configured = if configured.to_ascii_lowercase().starts_with("http://") {
        &configured[7..]
    } else if configured.to_ascii_lowercase().starts_with("https://") {
        &configured[8..]
    } else {
        configured
    };
    let host = configured.split(['/', '?', '#']).next().unwrap_or("");
    let pattern = format!("(?i)^{}$", regex::escape(host).replace("%d", r"\d+"));
    Regex::new(&pattern).is_ok_and(|re| re.is_match(parsed.host.as_deref().unwrap_or("")))
}

pub fn sanitize_presentation(html: &str, icons: &dyn IconResolver, asset_host: Option<&str>) -> Result<String, Error> {
    let safe = sanitizer::sanitize(html, &presentation_allowlist()).map_err(Error::Parse)?;
    if !safe.contains("<img") && !safe.contains("href=\"/") {
        return Ok(safe);
    }
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&safe).map_err(Error::Parse)?;
    for node in dom.descendants(root) {
        if dom.local_name(node) == Some("a") && dom.attr(node, "href").is_some_and(in_app_href) {
            dom.remove_attr(node, "target");
            dom.set_attr(node, "data-turbo-frame", "_top");
            dom.set_attr(node, "data-turbo-prefetch", "false");
        }
        if dom.local_name(node) != Some("img") {
            continue;
        }
        let alt = dom.attr(node, "alt").unwrap_or("").to_owned();
        let icon = ICON_ALT_RE.captures(&alt).and_then(|c| icons.find(&c[1]));
        let url = match icon {
            Some(Icon::Brand { url, .. }) => url,
            Some(Icon::Custom { url, .. }) => Some(url),
            _ => None,
        };
        if let Some(url) = url {
            dom.set_attr(node, "src", &url);
        } else if ICON_ALT_RE.is_match(&alt) {
            let text = dom.create_text(&alt);
            dom.replace_with_nodes(node, &[text]);
        } else if !avatar_src(dom.attr(node, "src").unwrap_or(""), asset_host) {
            dom.detach(node);
        }
    }
    Ok(dom.to_html(root))
}

/// `MessagesHelper#markdown_message_presentation`: only User attachments receive a partial;
/// others keep their canonical empty attachment node, followed by the presentation sanitizer.
pub fn presentation(body: &str, ctx: &RenderContext, icons: &dyn IconResolver, asset_host: Option<&str>) -> Result<String, Error> {
    let Content { mut dom, root } = Content::load(body, ctx)?;
    for node in attachment_nodes(&dom, root) {
        let attachment = attachables::attachment_from_node(&dom, node, ctx)?;
        if let Attachable::User(user) = attachment.attachable {
            dom.set_inner_html(node, &attachables::render_mention_in_context(&user, ctx)).map_err(Error::Parse)?;
        }
    }
    // A mention is a `<div>`. Parsed again by the sanitizer, that start tag closes the open
    // paragraph and the spoiler span, so the name and the words after it would render in the open.
    crate::content::contain_spoiler_blocks(&mut dom, root);
    let html = sanitize_presentation(&dom.to_html(root), icons, asset_host)?;
    Ok(format!("<div class=\"markdown-body\" data-controller=\"drive-link\">{html}</div>"))
}

/// `Message::Markdown.plain_text`: HTML-shaped attachment expansion followed by block/cell rules.
pub fn plain_text(body: &str, ctx: &RenderContext, icons: &dyn IconResolver) -> Result<String, Error> {
    let Content { mut dom, root } = Content::load(body, ctx)?;
    for node in attachment_nodes(&dom, root) {
        let attachment = attachables::attachment_from_node(&dom, node, ctx)?;
        match attachables::attachment_plain_text(&attachment) {
            attachables::PlainTextRepresentation::Html(text) => dom.replace_with_html(node, &text).map_err(Error::Parse)?,
            attachables::PlainTextRepresentation::Content(html) => dom.replace_with_html(node, &html).map_err(Error::Parse)?,
        }
    }
    let text = dom.children(root).iter().map(|&n| plain_node(&dom, n, icons)).collect::<String>();
    let text = text
        .split_inclusive('\n')
        .map(|line| line.trim_end_matches(['\n', '\r', ' ', '\t', '\u{b}', '\u{c}']))
        .collect::<Vec<_>>()
        .join("\n");
    static BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());
    Ok(strip(&BLANK_LINES.replace_all(&text, "\n\n")).to_owned())
}
fn plain_node(dom: &Dom, node: NodeId, icons: &dyn IconResolver) -> String {
    if let Some(text) = dom.text(node) {
        return text.to_owned();
    }
    let name = dom.local_name(node).unwrap_or("");
    if name == "br" {
        return "\n".to_owned();
    }
    // Push, email, activity and sidebar excerpts are this plain text. The hidden words stay in
    // the HTML for the reader who reveals them, and are replaced here so a preview can't leak them.
    if sanitizer::is_spoiler_span(dom, node) {
        return "spoiler".to_owned();
    }
    if name == "img" {
        let alt = dom.attr(node, "alt").unwrap_or("");
        let icon = ICON_ALT_RE.captures(alt).and_then(|c| icons.find(&c[1]));
        let icon_class = dom.attr(node, "class").unwrap_or("").split_whitespace().any(|c| matches!(c, "icon--brand" | "icon--custom"));
        if matches!(icon, Some(Icon::Brand { .. } | Icon::Custom { .. })) || icon_class {
            return alt.to_owned();
        }
    }
    let text = dom.children(node).iter().map(|&n| plain_node(dom, n, icons)).collect::<String>();
    if matches!(name, "td" | "th") {
        format!("{text}\t")
    } else if BLOCK_TAGS.contains(&name) {
        format!("{text}\n\n")
    } else {
        text
    }
}

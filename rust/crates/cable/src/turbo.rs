//! turbo-rails: `Turbo::StreamsChannel`, the `<turbo-stream>` tags its broadcasts carry
//! (`Turbo::Streams::ActionHelper#turbo_stream_action_tag`) and the `broadcast_*_to` helpers
//! (`Turbo::Streams::Broadcasts`).
use std::sync::Arc;

use rails_compat::Secrets;
use serde_json::Value;

use crate::channel::{Channel, ChannelError, ChannelResult, Params, Subscription};
use crate::{Server, naming};

pub const STREAMS_CHANNEL: &str = "Turbo::StreamsChannel";

type Verifier = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
type Guard = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// `Turbo::StreamsChannel`, optionally with a guard prepended the way Campfire prepends
/// `RoomStreamsAreAuthorized` (reference/app/channels/concerns/room_streams_are_authorized.rb).
#[derive(Clone)]
pub struct StreamsChannel {
    verifier: Verifier,
    guard: Option<Guard>,
}

impl StreamsChannel {
    /// Verifies signed stream names with `Turbo.signed_stream_verifier`.
    pub fn new(secrets: Arc<Secrets>) -> Self {
        Self::with_verifier(move |signed| rails_compat::turbo::verified_stream_name(&secrets, signed))
    }

    pub fn with_verifier(verifier: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Self {
        Self {
            verifier: Arc::new(verifier),
            guard: None,
        }
    }

    /// Rejects any subscription whose verified stream name `guarded` returns true for, before
    /// the stock behavior runs. The guard also sees names that failed verification, as `""`
    /// (Ruby's `nil.to_s`).
    pub fn guarded_by(mut self, guarded: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.guard = Some(Arc::new(guarded));
        self
    }

    /// `verified_stream_name_from_params`, for channels (like `RoomMessagesChannel`) that take
    /// signed stream names too.
    pub fn verified_stream_name_from_params(&self, params: &Params) -> ChannelResult<Option<String>> {
        verified_stream_name_from_params(params, |signed| (self.verifier)(signed))
    }
}

/// `params[:signed_stream_name]` verified with `verifier`. A missing or `null` name is simply
/// unverified; any other non-string makes `MessageVerifier#verified` raise.
pub fn verified_stream_name_from_params(params: &Params, verifier: impl Fn(&str) -> Option<String>) -> ChannelResult<Option<String>> {
    match params.get("signed_stream_name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(signed)) => Ok(verifier(signed)),
        Some(other) => Err(ChannelError(format!("undefined method 'valid_encoding?' for {other}"))),
    }
}

#[async_trait::async_trait]
impl<U: Send + Sync + 'static> Channel<U> for StreamsChannel {
    async fn subscribed(&mut self, sub: &mut Subscription<U>) -> ChannelResult {
        let stream_name = self.verified_stream_name_from_params(&sub.params())?;
        if let Some(guard) = &self.guard
            && guard(stream_name.as_deref().unwrap_or(""))
        {
            sub.reject();
            return Ok(());
        }
        match stream_name {
            Some(stream_name) => sub.stream_from(stream_name),
            None => sub.reject(),
        }
        Ok(())
    }
}

/// Turbo Stream actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Append,
    Prepend,
    Replace,
    Update,
    Remove,
    Before,
    After,
    Refresh,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Append => "append",
            Self::Prepend => "prepend",
            Self::Replace => "replace",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Before => "before",
            Self::After => "after",
            Self::Refresh => "refresh",
        }
    }
}

/// Where the action applies. Records are passed as their `dom_id` (`dom_id(@room, :list)`), and
/// `targets` as a CSS selector (records become `#<dom_id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    None,
    Target(&'a str),
    Targets(&'a str),
}

/// `turbo_stream_action_tag(action, target:, targets:, template:, **attributes)`.
///
/// Extra attributes come first, then `action`, then `target`/`targets`, exactly as the tag
/// helper's hash is built. Attribute names are used as given (Rails only dasherizes the tag
/// name, so `maintain_scroll: true` renders `maintain_scroll="true"`), and a `None` value omits
/// the attribute. `template` is already-safe HTML; `remove` and `refresh` never carry one.
pub fn action_tag(action: Action, target: Target<'_>, template: Option<&str>, attributes: &[(&str, Option<&str>)]) -> String {
    let mut tag = String::from("<turbo-stream");
    for (name, value) in attributes {
        if let Some(value) = value {
            push_attribute(&mut tag, name, value);
        }
    }
    push_attribute(&mut tag, "action", action.as_str());
    match target {
        Target::Target(target) => push_attribute(&mut tag, "target", target),
        Target::Targets(targets) => push_attribute(&mut tag, "targets", targets),
        Target::None => {}
    }
    tag.push('>');
    if !matches!(action, Action::Remove | Action::Refresh) {
        tag.push_str("<template>");
        tag.push_str(template.unwrap_or(""));
        tag.push_str("</template>");
    }
    tag.push_str("</turbo-stream>");
    tag
}

/// `turbo_stream_refresh_tag(request_id:)`.
pub fn refresh_tag(request_id: Option<&str>) -> String {
    action_tag(
        Action::Refresh,
        Target::None,
        None,
        &[("request-id", request_id.filter(|id| !id.is_empty()))],
    )
}

fn push_attribute(tag: &mut String, name: &str, value: &str) {
    tag.push(' ');
    tag.push_str(name);
    tag.push_str("=\"");
    tag.push_str(&html_escape(value));
    tag.push('"');
}

/// `ERB::Util.unwrapped_html_escape`.
pub fn html_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    escaped
}

/// Markup that belongs to a browser session and must never be shared by a broadcast.
/// This is a conservative backstop, not a browser-conformance check. Inspect both
/// scripting modes and raw tag attributes; an empty nonce attribute is harmless.
pub fn session_bound(html: &str) -> Option<&'static str> {
    session_bound_tree(html, true)
        .or_else(|| session_bound_tree(html, false))
        .or_else(|| session_bound_source(html))
}

fn session_bound_tree(html: &str, scripting_enabled: bool) -> Option<&'static str> {
    use html5ever::{ParseOpts, QualName, local_name, ns, parse_fragment, tendril::TendrilSink};
    use markup5ever_rcdom::{NodeData, RcDom};

    let mut opts = ParseOpts::default();
    opts.tree_builder.scripting_enabled = scripting_enabled;
    let dom = parse_fragment(
        RcDom::default(),
        opts,
        QualName::new(None, ns!(html), local_name!("body")),
        vec![],
        false,
    )
    .one(html);
    let mut nodes = vec![dom.document.clone()];
    while let Some(node) = nodes.pop() {
        if let NodeData::Element { name, attrs, template_contents, .. } = &node.data {
            let attrs = attrs.borrow();
            let attr = |name: &str| {
                attrs.iter().find(|a| a.name.ns == ns!() && a.name.local.as_ref() == name).map(|a| a.value.as_ref())
            };
            if name.ns == ns!(html) {
                let reason = match (name.local.as_ref(), attr("name")) {
                    ("input", Some("authenticity_token")) => Some("a CSRF token"),
                    ("meta", Some("csrf-token" | "csrf-param")) => Some("a CSRF meta tag"),
                    ("meta", Some("csp-nonce")) if attr("content").is_some_and(|v| !v.is_empty()) => Some("a CSP nonce"),
                    _ => None,
                };
                if reason.is_some() {
                    return reason;
                }
            }
            if attr("nonce").is_some_and(|v| !v.is_empty()) {
                return Some("a CSP nonce");
            }
            // Turbo Stream partials live in template contents, outside the element's children.
            if let Some(contents) = template_contents.borrow().as_ref() {
                nodes.push(contents.clone());
            }
        }
        nodes.extend(node.children.borrow().iter().rev().cloned());
    }
    None
}

fn session_bound_source(html: &str) -> Option<&'static str> {
    session_bound_source_with_work(html, &mut SourceWork::default())
}

#[derive(Default)]
struct SourceWork {
    // Count consumed bytes, lexical-state steps and inspected name values in tests.
    // The counter has no storage or updates in production builds.
    #[cfg(test)]
    bytes: usize,
}

impl SourceWork {
    fn scan(&mut self, bytes: usize) {
        #[cfg(test)]
        {
            self.bytes += bytes;
        }
        #[cfg(not(test))]
        let _ = bytes;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceValue {
    Other,
    Nonce,
    Name(usize),
}

impl SourceValue {
    fn finish(self, html: &str, end: usize, work: &mut SourceWork) -> Option<&'static str> {
        let Self::Name(start) = self else { return None };
        let value = &html[start..end];
        work.scan(value.len());
        let decoded;
        let value = if value.contains('&') {
            decoded = source_attribute_value(value);
            decoded.as_str()
        } else {
            value
        };
        match value {
            "authenticity_token" => Some("a CSRF token"),
            "csrf-token" | "csrf-param" => Some("a CSRF meta tag"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceAttr {
    TagName,
    BeforeName,
    Name { candidates: u8, matched: u8 },
    AfterName(u8),
    BeforeValue(u8),
    Quoted(u8, SourceValue),
    Unquoted(SourceValue),
}

impl SourceAttr {
    fn step(&mut self, html: &str, at: usize, work: &mut SourceWork) -> Option<&'static str> {
        let byte = html.as_bytes()[at];
        match *self {
            Self::TagName if byte.is_ascii_whitespace() || byte == b'/' => *self = Self::BeforeName,
            Self::BeforeName if !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'<' | b'=') => {
                *self = Self::Name { candidates: 3, matched: 0 };
                return self.step(html, at, work);
            }
            Self::Name { candidates, matched } => {
                if byte.is_ascii_whitespace() || matches!(byte, b'=' | b'/' | b'<') {
                    let kind = match (candidates, matched) {
                        (1, 4) => 1, // name
                        (2, 5) => 2, // nonce
                        _ => 0,
                    };
                    *self = match byte {
                        b'=' => Self::BeforeValue(kind),
                        b'/' | b'<' => Self::BeforeName,
                        _ => Self::AfterName(kind),
                    };
                } else {
                    let mut next = 0;
                    for (bit, name) in [(1, b"name".as_slice()), (2, b"nonce".as_slice())] {
                        if candidates & bit != 0 && name.get(matched as usize) == Some(&byte.to_ascii_lowercase()) {
                            next |= bit;
                        }
                    }
                    *self = Self::Name { candidates: next, matched: if next == 0 { 0 } else { matched + 1 } };
                }
            }
            Self::AfterName(kind) if !byte.is_ascii_whitespace() => {
                *self = if byte == b'=' { Self::BeforeValue(kind) } else { Self::BeforeName };
                if byte != b'=' {
                    return self.step(html, at, work);
                }
            }
            Self::BeforeValue(kind) if !byte.is_ascii_whitespace() => {
                let quoted = matches!(byte, b'\'' | b'"');
                let value = match kind {
                    1 => SourceValue::Name(at + usize::from(quoted)),
                    2 => SourceValue::Nonce,
                    _ => SourceValue::Other,
                };
                *self = if quoted { Self::Quoted(byte, value) } else { Self::Unquoted(value) };
                if !quoted && kind == 2 {
                    return Some("a CSP nonce");
                }
            }
            Self::Quoted(quote, value) => {
                if byte == quote {
                    *self = Self::BeforeName;
                    return value.finish(html, at, work);
                }
                if value == SourceValue::Nonce {
                    return Some("a CSP nonce");
                }
                if byte == b'<' {
                    // A literal '<' makes this value unequal to every sensitive name. Keep
                    // its quote state so outer attributes after a nested raw tag still count.
                    *self = Self::Quoted(quote, SourceValue::Other);
                }
            }
            Self::Unquoted(value) => {
                if byte.is_ascii_whitespace() {
                    *self = Self::BeforeName;
                    return value.finish(html, at, work);
                }
                if byte == b'<' {
                    *self = Self::Unquoted(SourceValue::Other);
                }
            }
            _ => {}
        }
        None
    }
}

struct SourceTag {
    start: usize,
    name_start: usize,
    name_end: Option<usize>,
    closing: bool,
    attrs: Vec<SourceAttr>,
    reason: Option<&'static str>,
}

impl SourceTag {
    fn new(start: usize, closing: bool) -> Self {
        Self {
            start,
            name_start: start + if closing { 2 } else { 1 },
            name_end: None,
            closing,
            attrs: vec![SourceAttr::TagName],
            reason: None,
        }
    }

    fn step(&mut self, html: &str, at: usize, work: &mut SourceWork) {
        let byte = html.as_bytes()[at];
        if self.name_end.is_none() && at >= self.name_start && (byte.is_ascii_whitespace() || byte == b'/') {
            self.name_end = Some(at);
        }
        if self.closing || self.reason.is_some() {
            return;
        }
        for attr in &mut self.attrs {
            work.scan(1);
            if let Some(reason) = attr.step(html, at, work) {
                self.reason = Some(reason);
                return;
            }
        }
        if byte == b'<' && html.as_bytes().get(at + 1).is_some_and(u8::is_ascii_alphabetic) {
            self.attrs.push(SourceAttr::TagName);
        }
        // Equivalent lexical states share all future work. Pending name values lose their
        // offsets at each literal '<', so nested tags cannot grow the state set with input.
        self.attrs.sort_unstable();
        self.attrs.dedup();
    }

    fn finish(&mut self, html: &str, at: usize, work: &mut SourceWork) -> Option<&'static str> {
        if self.closing {
            return None;
        }
        if self.reason.is_none() {
            for attr in &self.attrs {
                work.scan(1);
                if let SourceAttr::Quoted(_, value) | SourceAttr::Unquoted(value) = *attr
                    && let Some(reason) = value.finish(html, at, work)
                {
                    self.reason = Some(reason);
                    break;
                }
            }
        }
        self.reason
    }

    fn name<'a>(&self, html: &'a str, end: usize) -> &'a str {
        &html[self.name_start..self.name_end.unwrap_or(end)]
    }
}

enum SourceBody {
    Data,
    Tag(SourceTag),
    Text(&'static str),
}

struct SourceComment {
    selects: usize,
    foreign: usize,
    reason: Option<&'static str>,
}

fn session_bound_source_with_work(html: &str, work: &mut SourceWork) -> Option<&'static str> {
    let bytes = html.as_bytes();
    let mut body = SourceBody::Data;
    let mut comment: Option<SourceComment> = None;
    let mut selects = 0_usize;
    let mut foreign = 0_usize;
    let mut at = 0;
    while at < bytes.len() {
        work.scan(1);
        if comment.is_some() && bytes[at..].starts_with(b"-->") {
            let old = comment.take().unwrap();
            selects = old.selects;
            foreign = old.foreign;
            body = SourceBody::Data;
            at += 3;
            continue;
        }
        if comment.is_some() && bytes[at..].starts_with(b"--!>") {
            // Ambiguous comments remain eligible for refusal. All prospective tags have
            // already been checked while waiting; there is no rewind after the terminator.
            if let Some(reason) = comment.take().unwrap().reason {
                return Some(reason);
            }
        }
        match &mut body {
            SourceBody::Data => {
                if comment.is_none() && bytes[at..].starts_with(b"<!--")
                    && !bytes[at + 4..].starts_with(b">") && !bytes[at + 4..].starts_with(b"->")
                {
                    comment = Some(SourceComment { selects, foreign, reason: None });
                    at += 4;
                    continue;
                }
                if bytes[at] == b'<' {
                    let closing = bytes.get(at + 1) == Some(&b'/');
                    let name_start = at + if closing { 2 } else { 1 };
                    if bytes.get(name_start).is_some_and(u8::is_ascii_alphabetic) {
                        body = SourceBody::Tag(SourceTag::new(at, closing));
                        at = name_start;
                        continue;
                    }
                }
            }
            SourceBody::Text(close) => {
                if bytes[at..].get(..close.len()).is_some_and(|s| s.eq_ignore_ascii_case(close.as_bytes()))
                    && bytes.get(at + close.len()).is_some_and(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'>'))
                {
                    body = SourceBody::Tag(SourceTag::new(at, true));
                    at += 2;
                    continue;
                }
            }
            SourceBody::Tag(tag) => {
                if bytes[at] == b'>' {
                    let reason = tag.finish(html, at, work);
                    if let Some(pending) = &mut comment {
                        pending.reason = pending.reason.or(reason);
                    } else if reason.is_some() {
                        return reason;
                    }
                    let name = tag.name(html, at);
                    let self_closing = html[tag.start..at].trim_end().ends_with('/');
                    body = if tag.closing {
                        if name.eq_ignore_ascii_case("select") {
                            selects = selects.saturating_sub(1);
                        } else if name.eq_ignore_ascii_case("svg") || name.eq_ignore_ascii_case("math") {
                            foreign = foreign.saturating_sub(1);
                        }
                        SourceBody::Data
                    } else {
                        if name.eq_ignore_ascii_case("select") {
                            selects += 1;
                        } else if !self_closing && (name.eq_ignore_ascii_case("svg") || name.eq_ignore_ascii_case("math")) {
                            foreign += 1;
                        }
                        if selects == 0 && foreign == 0 && !self_closing {
                            if name.eq_ignore_ascii_case("textarea") {
                                SourceBody::Text("</textarea")
                            } else if name.eq_ignore_ascii_case("title") {
                                SourceBody::Text("</title")
                            } else if name.eq_ignore_ascii_case("script") {
                                SourceBody::Text("</script")
                            } else {
                                SourceBody::Data
                            }
                        } else {
                            SourceBody::Data
                        }
                    };
                } else {
                    tag.step(html, at, work);
                }
            }
        }
        at += 1;
    }
    // An unfinished tag is never reparsed at EOF. An unfinished comment only exposes
    // completed prospective tags; a clearly terminated comment discards them above.
    comment.and_then(|pending| pending.reason)
}

fn source_attribute_value(value: &str) -> String {
    use html5ever::tendril::StrTendril;
    use html5ever::tokenizer::{BufferQueue, StartTag, TagToken, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts};
    use std::cell::RefCell;

    #[derive(Default)]
    struct Value(RefCell<String>);
    impl TokenSink for Value {
        type Handle = ();

        fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
            if let TagToken(tag) = token
                && tag.kind == StartTag
                && let Some(attr) = tag.attrs.iter().find(|a| a.name.local.as_ref() == "value")
            {
                *self.0.borrow_mut() = attr.value.to_string();
            }
            TokenSinkResult::Continue
        }
    }

    // Use only the attribute lexer for character references, never its element structure.
    let tokenizer = Tokenizer::new(Value::default(), TokenizerOpts::default());
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(&format!("<x value=\"{}\">", value.replace('"', "&quot;"))));
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.0.into_inner()
}

/// `Turbo::StreamsChannel.broadcast_*_to`. Streamables are the stream name parts (GID params
/// and symbols); blank ones are dropped and nothing is sent if none remain, as in
/// `broadcast_stream_to`.
impl<U: Send + Sync + 'static> Server<U> {
    /// Refuses (logs an error and sends nothing) content that carries [`session_bound`] markup.
    pub fn broadcast_stream_to(&self, streamables: &[&str], content: &str) -> usize {
        let streamables: Vec<&str> = streamables.iter().copied().filter(|s| !s.trim().is_empty()).collect();
        if streamables.is_empty() {
            return 0;
        }
        let stream = naming::stream_name_from(&streamables);
        if let Some(what) = session_bound(content) {
            tracing::error!(stream, "refusing to broadcast {what}: broadcasts go to every subscriber");
            return 0;
        }
        self.broadcast(&stream, content)
    }

    pub fn broadcast_action_to(
        &self,
        streamables: &[&str],
        action: Action,
        target: Target<'_>,
        html: Option<&str>,
        attributes: &[(&str, Option<&str>)],
    ) -> usize {
        self.broadcast_stream_to(streamables, &action_tag(action, target, html, attributes))
    }

    pub fn broadcast_append_to(&self, streamables: &[&str], target: &str, html: &str) -> usize {
        self.broadcast_action_to(streamables, Action::Append, Target::Target(target), Some(html), &[])
    }

    pub fn broadcast_prepend_to(&self, streamables: &[&str], target: &str, html: &str) -> usize {
        self.broadcast_action_to(streamables, Action::Prepend, Target::Target(target), Some(html), &[])
    }

    pub fn broadcast_replace_to(&self, streamables: &[&str], target: &str, html: &str) -> usize {
        self.broadcast_action_to(streamables, Action::Replace, Target::Target(target), Some(html), &[])
    }

    pub fn broadcast_update_to(&self, streamables: &[&str], target: &str, html: &str) -> usize {
        self.broadcast_action_to(streamables, Action::Update, Target::Target(target), Some(html), &[])
    }

    pub fn broadcast_remove_to(&self, streamables: &[&str], target: &str) -> usize {
        self.broadcast_action_to(streamables, Action::Remove, Target::Target(target), None, &[])
    }

    pub fn broadcast_refresh_to(&self, streamables: &[&str], request_id: Option<&str>) -> usize {
        self.broadcast_stream_to(streamables, &refresh_tag(request_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_tag() {
        assert_eq!(
            action_tag(
                Action::Append,
                Target::Target("messages_room_1"),
                Some("<div id=\"m\">Hi &amp; bye</div>"),
                &[]
            ),
            r#"<turbo-stream action="append" target="messages_room_1"><template><div id="m">Hi &amp; bye</div></template></turbo-stream>"#
        );
    }

    #[test]
    fn remove_tag_has_no_template() {
        assert_eq!(
            action_tag(Action::Remove, Target::Target("message_1"), None, &[]),
            r#"<turbo-stream action="remove" target="message_1"></turbo-stream>"#
        );
    }

    #[test]
    fn attributes_come_before_action_and_are_not_dasherized() {
        assert_eq!(
            action_tag(
                Action::Replace,
                Target::Target("presentation_message_1"),
                Some("x"),
                &[("maintain_scroll", Some("true"))]
            ),
            r#"<turbo-stream maintain_scroll="true" action="replace" target="presentation_message_1"><template>x</template></turbo-stream>"#
        );
    }

    #[test]
    fn targets_and_escaping() {
        assert_eq!(
            action_tag(Action::Update, Target::Targets("#a > b[data-x='1']"), None, &[]),
            r##"<turbo-stream action="update" targets="#a &gt; b[data-x=&#39;1&#39;]"><template></template></turbo-stream>"##
        );
    }

    #[test]
    fn session_bound_markup() {
        let form =
            r#"<form action="/x" method="post"><input type="hidden" name="authenticity_token" value="abc" autocomplete="off"></form>"#;
        assert_eq!(session_bound(form), Some("a CSRF token"));
        assert_eq!(session_bound(r#"<meta name="csrf-token" content="abc">"#), Some("a CSRF meta tag"));
        assert_eq!(
            session_bound(r#"<meta name="csrf-param" content="authenticity_token">"#),
            Some("a CSRF meta tag")
        );
        assert_eq!(session_bound(r#"<script nonce="r4nd0m">x()</script>"#), Some("a CSP nonce"));
        // What `ApplicationController.render` gives in our Rails app: no token, no nonce.
        assert_eq!(
            session_bound(r#"<form class="button_to" method="post" action="/y"><button type="submit">b</button></form>"#),
            None
        );
        assert_eq!(session_bound(r#"<script nonce="">x()</script>"#), None);
        // Typed by a user, and so escaped.
        assert_eq!(
            session_bound("<p>name=&quot;authenticity_token&quot; nonce=&quot;x&quot;</p>"),
            None
        );
    }

    #[test]
    fn session_bound_checks_attributes_instead_of_text() {
        for html in [
            "<div>nonce=\"example\" name=\"authenticity_token\"</div>",
            "<!-- <input name=authenticity_token value=secret> -->",
            "<textarea><input name=authenticity_token></textarea>",
            "<script>const example = '<input name=authenticity_token>';</script>",
            "<p>&lt;script nonce=\"example\"&gt;</p>",
            "<div title='nonce=\"example\"'>quoted attribute text</div>",
        ] {
            assert_eq!(session_bound(html), None, "{html}");
        }
        for html in [
            "<INPUT VALUE='secret' NAME='authenticity_token'>",
            "<input name=authenticity&#95;token value=secret>",
            "<template><input name=authenticity_token value=secret></template>",
        ] {
            assert_eq!(session_bound(html), Some("a CSRF token"), "{html}");
        }
        for html in [
            "<SCRIPT NONCE='secret'></SCRIPT>",
            "<link nonce=secret>",
            "<meta name=csp-nonce content=secret>",
        ] {
            assert_eq!(session_bound(html), Some("a CSP nonce"), "{html}");
        }
    }

    fn assert_session_bound_fragment(html: &str, reason: &'static str) {
        assert_eq!(session_bound(html), Some(reason), "{html}");
        let broadcast = action_tag(Action::Append, Target::Target("messages"), Some(html), &[]);
        assert_eq!(session_bound(&broadcast), Some(reason), "{broadcast}");
    }

    #[test]
    fn session_bound_noscript_token() {
        assert_session_bound_fragment("<noscript><input name=authenticity_token></noscript>", "a CSRF token");
    }

    #[test]
    fn session_bound_select_csrf_token_meta() {
        assert_session_bound_fragment("<select><meta name=csrf-token content=secret></select>", "a CSRF meta tag");
    }

    #[test]
    fn session_bound_select_csrf_param_meta() {
        assert_session_bound_fragment("<select><meta name=csrf-param content=authenticity_token></select>", "a CSRF meta tag");
    }

    #[test]
    fn session_bound_select_nonce_style() {
        assert_session_bound_fragment("<select><style nonce=secret></style></select>", "a CSP nonce");
    }

    #[test]
    fn session_bound_select_title_is_an_intentional_conservative_refusal() {
        // This is text in current browsers. Our templates never emit it; refusing it is
        // intentional rather than relying on the parser's changing select insertion rules.
        let html = "<select><title><input name=authenticity_token></title></select>";
        assert_session_bound_fragment(html, "a CSRF token");
        assert_eq!(session_bound_source(html), Some("a CSRF token"));
    }

    #[test]
    fn session_bound_raw_backstop_checks_any_tag_attributes() {
        for (html, reason) in [
            ("<div name=authenticity_token>", "a CSRF token"),
            ("<select><span NAME='csrf-token'></span></select>", "a CSRF meta tag"),
            ("<select><span name=csrf-param></span></select>", "a CSRF meta tag"),
            ("<select><style NONCE='secret'></style></select>", "a CSP nonce"),
            ("<select><style nonce='' nonce=secret></style></select>", "a CSP nonce"),
            ("<select><span name=ignored name=csrf-token></span></select>", "a CSRF meta tag"),
            ("<select><span name=csrf&#45;param></span></select>", "a CSRF meta tag"),
            ("<select><style nonce=secret title='>'></style></select>", "a CSP nonce"),
            ("<select><x/name=authenticity_token></select>", "a CSRF token"),
        ] {
            assert_session_bound_fragment(html, reason);
        }
    }

    fn assert_source_scan_linear(label: &str, input: impl Fn(usize) -> String) {
        let mut previous = 0;
        for n in [256, 512, 1024, 2048, 4096] {
            let html = input(n);
            let mut work = SourceWork::default();
            assert_eq!(session_bound_source_with_work(&html, &mut work), None);
            println!("source work {label}: n={n} bytes={} operations={}", html.len(), work.bytes);
            if previous != 0 {
                assert!(work.bytes <= previous * 2 + 64, "{label}: doubling input grew work from {previous} to {}", work.bytes);
            }
            previous = work.bytes;
        }
        let bytes = input(4096).len();
        assert!(previous <= bytes * 16, "{label}: {previous} operations for {bytes} bytes");
    }

    #[test]
    fn session_bound_source_nested_tags_have_linear_work() {
        assert_source_scan_linear("nested tags", |n| "<x ".repeat(n) + ">");
    }

    #[test]
    fn session_bound_source_unclosed_comments_have_linear_work() {
        assert_source_scan_linear("unclosed comments", |n| "<!--".repeat(n));
    }

    #[test]
    fn session_bound_source_preserves_nested_attributes_and_comment_controls() {
        for (html, reason) in [
            ("<select><x title=\"<y a='\" name=authenticity_token>", Some("a CSRF token")),
            ("<select><x title='<y a=\"' nonce=secret>", Some("a CSP nonce")),
            ("<!-- <input name=authenticity_token>", Some("a CSRF token")),
            ("<!-- <input name=authenticity_token> --!>", Some("a CSRF token")),
            ("<!-- <input name=authenticity_token> -->", None),
            ("<!-- <select> --> <title><input name=authenticity_token></title>", None),
            ("<!-- <input name=authenticity_token", None),
            ("<x nonce=secret", None),
        ] {
            assert_eq!(session_bound_source(html), reason, "{html}");
        }
    }

    #[test]
    fn session_bound_svg_self_closing_style_token() {
        for html in [
            "<svg><style/></svg><input name=authenticity_token value=secret>",
            "<svg><style/><foreignObject><input name=authenticity_token value=secret></foreignObject></svg>",
        ] {
            assert_session_bound_fragment(html, "a CSRF token");
        }
    }

    #[test]
    fn session_bound_svg_self_closing_style_nonce() {
        for html in [
            "<svg><style/></svg><script nonce=secret></script>",
            "<svg><style/><script nonce=secret></script></svg>",
        ] {
            assert_session_bound_fragment(html, "a CSP nonce");
        }
    }

    #[test]
    fn session_bound_mathml_self_closing_style_token() {
        for html in [
            "<math><style/></math><input name=authenticity_token value=secret>",
            "<math><style/><mtext><input name=authenticity_token value=secret></mtext></math>",
        ] {
            assert_session_bound_fragment(html, "a CSRF token");
        }
    }

    #[test]
    fn session_bound_mathml_self_closing_style_nonce() {
        for html in [
            "<math><style/></math><script nonce=secret></script>",
            "<math><style/><script nonce=secret></script></math>",
        ] {
            assert_session_bound_fragment(html, "a CSP nonce");
        }
    }

    #[test]
    fn session_bound_preserves_text_controls() {
        for html in [
            "<div>nonce=\"example\" name=\"authenticity_token\" csrf-token csp-nonce</div>",
            "<textarea><input name=authenticity_token><script nonce=example></script></textarea>",
            "<title><input name=authenticity_token><script nonce=example></script></title>",
            "<script>const example = '<input name=authenticity_token><script nonce=example>';</script>",
            "<p>&lt;input name=authenticity_token&gt;&lt;script nonce=example&gt;</p>",
            "<p>&lt;input name=&quot;authenticity_token&quot;&gt; name=&quot;authenticity_token&quot;</p>",
            "<select><span data-name=csrf-token data-nonce=secret></span></select>",
            "<select><style nonce=''></style></select>",
            "<input /> <svg/> <math/> <div title='words nonce=secret name=authenticity_token'></div>",
        ] {
            assert_eq!(session_bound(html), None, "{html}");
            let broadcast = action_tag(Action::Append, Target::Target("messages"), Some(html), &[]);
            assert_eq!(session_bound(&broadcast), None, "{broadcast}");
        }
    }

    #[test]
    fn refresh_tags() {
        assert_eq!(refresh_tag(None), r#"<turbo-stream action="refresh"></turbo-stream>"#);
        assert_eq!(
            refresh_tag(Some("abc")),
            r#"<turbo-stream request-id="abc" action="refresh"></turbo-stream>"#
        );
    }
}

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
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    let mut selects = 0_usize;
    let mut foreign = 0_usize;
    while let Some(offset) = lower[cursor..].find('<') {
        let start = cursor + offset;
        cursor = start + 1;

        // Preserve clear comment/text controls. Ambiguous markup is scanned, not exempted.
        if let Some(comment) = lower[start..].strip_prefix("<!--")
            && !comment.starts_with('>')
            && !comment.starts_with("->")
            && let Some(end) = comment.find("-->")
            && !comment[..end].contains("--!>")
        {
            cursor = start + 4 + end + 3;
            continue;
        }
        let closing = lower[start..].starts_with("</");
        let name_start = start + if closing { 2 } else { 1 };
        if !lower.as_bytes().get(name_start).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let Some(end) = lower[start..].find('>').map(|offset| start + offset) else {
            break;
        };
        let name = lower[name_start..=end]
            .split(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
            .next()
            .unwrap();
        let tag = &html[start..=end];
        cursor = end + 1;
        if closing {
            match name {
                "select" => selects = selects.saturating_sub(1),
                "svg" | "math" => foreign = foreign.saturating_sub(1),
                _ => {}
            }
            continue;
        }

        // Inspect every literal '<' followed by a letter, including one inside another tag.
        // Attribute tokenization decodes entities but never applies tree-builder filtering.
        for (offset, _) in tag.match_indices('<') {
            if tag.as_bytes().get(offset + 1).is_some_and(u8::is_ascii_alphabetic)
                && let Some(reason) = session_bound_source_tag(&tag[offset..])
            {
                return Some(reason);
            }
        }
        let self_closing = tag[..tag.len() - 1].trim_end().ends_with('/');
        match name {
            "select" => selects += 1,
            "svg" | "math" if !self_closing => foreign += 1,
            _ => {}
        }
        // Select's obsolete insertion rules must never hide a source tag. In particular,
        // <select><title><input ...> is intentionally refused, even if a browser treats it as text.
        if selects == 0 && foreign == 0 && !self_closing && matches!(name, "textarea" | "title" | "script") {
            let close = format!("</{name}");
            cursor = lower[cursor..]
                .match_indices(&close)
                .find(|(offset, _)| {
                    lower.as_bytes().get(cursor + offset + close.len())
                        .is_some_and(|c| c.is_ascii_whitespace() || matches!(c, b'/' | b'>'))
                })
                .map_or(html.len(), |(offset, _)| cursor + offset);
        }
    }
    None
}

fn session_bound_source_tag(tag: &str) -> Option<&'static str> {
    // Read source attributes independently: no tree insertion rules, duplicate-attribute
    // filtering or EOF recovery can discard a suspicious value before we inspect it.
    let mut attrs = tag[1..tag.len() - 1].trim_start_matches(|c: char| !c.is_ascii_whitespace() && c != '/');
    while !attrs.is_empty() {
        attrs = attrs.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if attrs.is_empty() {
            break;
        }
        let end = attrs.find(|c: char| c.is_ascii_whitespace() || matches!(c, '=' | '/' | '>' | '<')).unwrap_or(attrs.len());
        let name = &attrs[..end];
        if end == 0 {
            attrs = &attrs[1..];
            continue;
        }
        attrs = attrs[end..].trim_start_matches(|c: char| c.is_ascii_whitespace());
        let Some(value) = attrs.strip_prefix('=') else {
            continue;
        };
        attrs = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
        let value = if attrs.starts_with(['\'', '"']) {
            let quote = attrs.as_bytes()[0] as char;
            attrs = &attrs[1..];
            let end = attrs.find(quote).unwrap_or(attrs.len());
            let value = &attrs[..end];
            attrs = attrs.get(end + 1..).unwrap_or("");
            value
        } else {
            let end = attrs.find(|c: char| c.is_ascii_whitespace() || c == '>').unwrap_or(attrs.len());
            let value = &attrs[..end];
            attrs = &attrs[end..];
            value
        };
        if name.eq_ignore_ascii_case("nonce") && !value.is_empty() {
            return Some("a CSP nonce");
        }
        if name.eq_ignore_ascii_case("name") {
            let value = if value.contains('&') { source_attribute_value(value) } else { value.to_string() };
            match value.as_str() {
                "authenticity_token" => return Some("a CSRF token"),
                "csrf-token" | "csrf-param" => return Some("a CSRF meta tag"),
                _ => {}
            }
        }
    }
    None
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

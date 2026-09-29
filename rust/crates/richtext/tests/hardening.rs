//! Regression tests for places where the port deliberately diverges from the Rails pipeline to
//! close a hole or bound the work a message body can cause (see "Known differences" in README.md).

use std::time::{Duration, Instant};

use campfire_richtext::dom::Dom;
use campfire_richtext::{AttachableResolver, GidLookup, RenderContext, SignedLookup, editable_value, message_presentation};

struct NoRecords;

impl AttachableResolver for NoRecords {
    fn locate_signed(&self, _sgid: &str) -> SignedLookup {
        SignedLookup::Invalid
    }

    fn find_gid(&self, _gid: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

fn ctx() -> RenderContext<'static> {
    RenderContext { resolver: &NoRecords, request_host: Some("once.campfire.test".into()) }
}

fn presentation(body: &str) -> String {
    message_presentation(body, &ctx()).unwrap()
}

/// Every element and attribute name in `html` as a browser would parse it.
fn parsed_markup(html: &str) -> Vec<String> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let mut names = Vec::new();
    for node in dom.descendants(root) {
        if let Some(name) = dom.local_name(node) {
            if name == "div" && dom.attr(node, "class") == Some("trix-content") {
                continue; // the layout's wrapper
            }
            names.push(name.to_string());
            names.extend(dom.attrs(node).into_iter().map(|(attr, _)| format!("{name}[{attr}]")));
        }
    }
    names
}

fn assert_quick(started: Instant, what: &str) {
    // Generous enough for a debug build; release takes a few milliseconds.
    let bound = if cfg!(debug_assertions) { Duration::from_secs(5) } else { Duration::from_secs(1) };
    assert!(started.elapsed() < bound, "{what} took {:?}", started.elapsed());
}

// --- Autolinking inside attribute values ---------------------------------------------------------

#[test]
fn a_url_after_a_greater_than_sign_in_an_attribute_cannot_break_out_of_it() {
    // Rails serializes the title unescaped (`title="x> http://..."`), so auto_link's "inside a
    // tag" check misses, the inserted <a href="..."> closes the attribute, and the <img> after it
    // becomes live markup.
    let body = r#"<p title="x> http://evil.test/ <img src=x onerror=alert(1)>">hi</p>"#;
    let html = presentation(body);
    let markup = parsed_markup(&html);
    assert!(!markup.iter().any(|m| m == "img" || m.contains("onerror")), "{html}");
    assert_eq!(markup, ["p", "p[title]"], "{html}");
}

#[test]
fn an_email_address_after_a_greater_than_sign_in_an_attribute_cannot_break_out_of_it() {
    let body = r#"<p><abbr title="x> me@evil.test <img src=x onerror=alert(1)>">hi</abbr></p>"#;
    let html = presentation(body);
    let markup = parsed_markup(&html);
    assert_eq!(markup, ["p", "abbr", "abbr[title]"], "{html}");
}

#[test]
fn name_attributes_cant_clobber_the_pages_globals() {
    let html = presentation(r#"<p><a name="body" href="/x">x</a><span name="cookie">y</span></p>"#);
    assert_eq!(parsed_markup(&html), ["p", "a", "a[href]", "span"], "{html}");
}

#[test]
fn urls_in_text_are_still_linked() {
    let html = presentation("<p>see http://example.com/a?b=1&amp;c=2 and me@example.com</p>");
    assert!(html.contains(
        "<p>see <a target=\"_blank\" href=\"http://example.com/a?b=1&amp;c=2\">http://example.com/a?b=1&amp;c=2</a> and <a target=\"_blank\" href=\"mailto:me@example.com\">me@example.com</a></p>"
    ), "{html}");
}

// --- Bounded work ----------------------------------------------------------------------------------

#[test]
fn many_bare_domains_autolink_in_linear_time() {
    let body = "<p>www.a.com</p>".repeat(64 * 1024 / 16);
    let started = Instant::now();
    let html = presentation(&body);
    assert_quick(started, "autolinking 64 KB of bare domains");
    assert_eq!(html.matches("<a target=\"_blank\" href=\"http://www.a.com\">").count(), 64 * 1024 / 16);
}

/// Content attachments nested `levels` deep, each saying which level it is.
fn nested_content_attachments(levels: usize, padding: &str) -> String {
    let mut body = String::new();
    for level in (1..=levels).rev() {
        let content = format!("<p>level {level}{padding}</p>{body}").replace('&', "&amp;").replace('"', "&quot;");
        body = format!("<action-text-attachment content-type=\"text/html\" content=\"{content}\"></action-text-attachment>");
    }
    body
}

#[test]
fn content_attachments_render_eight_levels_deep() {
    let html = presentation(&nested_content_attachments(12, ""));
    for level in 1..=12 {
        assert_eq!(html.contains(&format!("level {level}<")), level <= 8, "level {level} in {html}");
    }
}

#[test]
fn deeply_nested_content_attachments_render_quickly() {
    let body = nested_content_attachments(200, &"x".repeat(1000));
    assert!(body.len() > 200_000);
    let started = Instant::now();
    presentation(&body);
    assert_quick(started, "rendering 200 nested content attachments");
}

/// Every SGID names a user who has since been deleted.
struct DeletedUsers;

impl AttachableResolver for DeletedUsers {
    fn locate_signed(&self, _sgid: &str) -> SignedLookup {
        SignedLookup::MissingRecord { model_name: "User".into() }
    }

    fn find_gid(&self, _gid: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

const DELETED_MENTION: &str = r#"<p>Hi <action-text-attachment sgid="eyJfcmFpbHMiOnsiZGF0YSI6ImdpZDovL2NhbXBmaXJlL1VzZXIvNj9leHBpcmVzX2luIiwicHVyIjoiYXR0YWNoYWJsZSJ9fQ==--fc4f83a239475557295b8e2f5ff55482bebc9bfe" content-type="application/vnd.campfire.mention"></action-text-attachment>, welcome</p>"#;

#[test]
fn a_deleted_user_mention_matches_the_forks_empty_presentation() {
    let ctx = RenderContext { resolver: &DeletedUsers, request_host: None };
    assert_eq!(campfire_richtext::present_message(DELETED_MENTION, &ctx), campfire_richtext::Presentation::Html(String::new()));
}

#[test]
fn a_mention_of_a_deleted_user_leaves_the_editor() {
    let ctx = RenderContext { resolver: &DeletedUsers, request_host: None };
    let value = editable_value(DELETED_MENTION, &ctx).unwrap().unwrap();
    assert!(!value.contains("action-text-attachment") && value.contains("welcome"), "{value}");
}

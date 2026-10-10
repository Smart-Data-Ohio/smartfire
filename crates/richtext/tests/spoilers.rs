//! Discord-style `||spoiler||` text: inline only, inert in code, redacted in plain-text previews.
use campfire_richtext::markdown::{self, IconCatalog};
use campfire_richtext::{AttachableResolver, Content, GidLookup, MentionUser, RenderContext, SignedLookup};

struct NoRecords;
impl AttachableResolver for NoRecords {
    fn locate_signed(&self, _: &str) -> SignedLookup {
        SignedLookup::Invalid
    }
    fn find_gid(&self, _: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

fn render(source: &str) -> String {
    markdown::render(source, &(|_: &str| None), &IconCatalog::default()).unwrap()
}

fn preview(source: &str) -> String {
    let ctx = RenderContext {
        resolver: &NoRecords,
        request_host: None,
    };
    markdown::plain_text(&render(source), &ctx, &IconCatalog::default()).unwrap()
}

#[test]
fn a_spoiler_is_an_inline_span() {
    let html = render("see ||secret|| now");
    assert_eq!(
        html,
        "<p>see <span class=\"spoiler\" data-spoiler=\"\">secret</span> now</p>\n"
    );
}

#[test]
fn several_spoilers_in_one_paragraph_each_get_a_span() {
    let html = render("||one|| and ||two||");
    assert_eq!(
        html,
        "<p><span class=\"spoiler\" data-spoiler=\"\">one</span> and <span class=\"spoiler\" data-spoiler=\"\">two</span></p>\n"
    );
}

#[test]
fn an_unclosed_spoiler_stays_text() {
    let html = render("||nope");
    assert!(!html.contains("data-spoiler"), "{html}");
    assert!(html.contains("||nope"), "{html}");
}

#[test]
fn a_spoiler_does_not_span_blocks() {
    let html = render("||top\n\nbottom||");
    assert!(!html.contains("data-spoiler"), "{html}");
    assert!(html.contains("||top"), "{html}");
    assert!(html.contains("bottom||"), "{html}");
}

#[test]
fn spoilers_inside_code_spans_and_fences_are_unchanged() {
    let inline = render("use `||secret||` here");
    assert!(!inline.contains("data-spoiler"), "{inline}");
    assert!(inline.contains("<code>||secret||</code>"), "{inline}");

    let fenced = render("```\n||secret||\n```");
    assert!(!fenced.contains("data-spoiler"), "{fenced}");
    assert!(fenced.contains("||secret||"), "{fenced}");
}

#[test]
fn nested_formatting_stays_inside_the_spoiler() {
    let html = render("||**secret** and *hidden*||");
    assert_eq!(
        html,
        "<p><span class=\"spoiler\" data-spoiler=\"\"><strong>secret</strong> and <em>hidden</em></span></p>\n"
    );
}

#[test]
fn plain_text_previews_replace_spoiler_contents() {
    assert_eq!(preview("see ||secret words|| now"), "see spoiler now");
    assert_eq!(preview("||one|| and ||two||"), "spoiler and spoiler");
    assert_eq!(preview("||**secret**||"), "spoiler");
    assert!(preview("use `||secret||` here").contains("||secret||"));
}

fn david() -> MentionUser {
    MentionUser {
        id: 1,
        name: "David".into(),
        title: "David – Founder".into(),
        attachable_sgid: "eyJfcmFpbHMiOnsiZGF0YSI6ImdpZDovL2NhbXBmaXJlL1VzZXIvMT9leHBpcmVzX2luIiwicHVyIjoiYXR0YWNoYWJsZSJ9fQ==--f7d8e8773314d3310320f3cdd08e5597bb51ca1a".into(),
        user_path: "/users/1".into(),
        avatar_path: "/users/1/avatar?v=1".into(),
    }
}

struct DavidRecords;
impl AttachableResolver for DavidRecords {
    fn locate_signed(&self, sgid: &str) -> SignedLookup {
        if sgid == david().attachable_sgid { SignedLookup::User(david()) } else { SignedLookup::Invalid }
    }
    fn find_gid(&self, _: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

fn david_ctx() -> RenderContext<'static> {
    RenderContext { resolver: &DavidRecords, request_host: None }
}

fn render_david(source: &str) -> String {
    markdown::render(source, &(|name: &str| (name == "David").then(david)), &IconCatalog::default()).unwrap()
}

/// Richtext mentions are `@[Name]`. There is no `<@id>` form on this path.
fn assert_mention_stays_concealed(html: &str) {
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let spoilers: Vec<_> = dom.descendants(root).into_iter().filter(|&node| dom.has_attr(node, "data-spoiler")).collect();
    assert_eq!(spoilers.len(), 1, "{html}");
    let text = dom.text_content(spoilers[0]);
    assert!(text.contains("David") && text.contains("killer"), "{html}");
    for node in dom.descendants(root) {
        if dom.text(node).is_some_and(|value| value.contains("David") || value.contains("killer")) {
            let hidden = dom.ancestors(node).iter().any(|&ancestor| dom.has_attr(ancestor, "data-spoiler"));
            assert!(hidden, "{html}");
        }
    }
    let mention_inside = dom.descendants(root).into_iter().any(|node| {
        dom.attr(node, "class").is_some_and(|classes| classes.split_whitespace().any(|class| class == "mention"))
            && dom.ancestors(node).iter().any(|&ancestor| dom.has_attr(ancestor, "data-spoiler"))
    });
    assert!(mention_inside, "{html}");
}

#[test]
fn a_mention_inside_a_spoiler_stays_concealed() {
    let body = render_david("||@[David] is the killer||");
    let shown = markdown::presentation(&body, &david_ctx(), &IconCatalog::default(), None).unwrap();
    assert_mention_stays_concealed(&shown);

    let timeline = Content::load(&body, &david_ctx()).unwrap().to_rendered_html_with_layout(&david_ctx()).unwrap();
    assert_mention_stays_concealed(&timeline);
}

#[test]
fn plain_text_hides_a_mention_inside_a_spoiler() {
    let text = markdown::plain_text(&render_david("||@[David] is the killer||"), &david_ctx(), &IconCatalog::default()).unwrap();
    assert_eq!(text, "spoiler");
}

#[test]
fn presentation_keeps_the_spoiler_and_drops_anything_else_new() {
    let icons = IconCatalog::default();
    let shown = markdown::sanitize_presentation(&render("||secret||"), &icons, None).unwrap();
    assert!(
        shown.contains("<span class=\"spoiler\" data-spoiler=\"\">secret</span>"),
        "{shown}"
    );

    let hostile = r#"<span class="spoiler" data-spoiler="" onclick="alert(1)" data-evil="1" style="color:red">x</span><script>y</script>"#;
    let safe = markdown::sanitize_presentation(hostile, &icons, None).unwrap();
    assert!(safe.contains("data-spoiler"), "{safe}");
    assert!(!safe.contains("onclick"), "{safe}");
    assert!(!safe.contains("data-evil"), "{safe}");
    assert!(!safe.contains("style"), "{safe}");
    assert!(!safe.contains("script"), "{safe}");
}

#[test]
fn a_spoiler_inside_a_spoiler_joins_the_outer_one() {
    // One level only. Inner markers don't open a second spoiler or reveal the words between them.
    let spoiler = "<span class=\"spoiler\" data-spoiler=\"\">";
    assert_eq!(render("||outer ||SECRET|| tail||"), format!("<p>{spoiler}outer SECRET tail</span></p>\n"));
    assert_eq!(
        render("||a **||b||** c||"),
        format!("<p>{spoiler}a <strong>b</strong> c</span></p>\n")
    );
    assert_eq!(preview("||outer ||SECRET|| tail||"), "spoiler");
}

#[test]
fn a_link_whose_label_holds_a_spoiler_conceals_it() {
    let html = render(r#"[||ending||](https://example.com/alice-dies "Alice dies") and [open](https://example.com/shown)"#);
    assert_eq!(
        html,
        "<p><a href=\"https://example.com/alice-dies\" title=\"Alice dies\" target=\"_blank\" rel=\"nofollow noopener noreferrer\"><span class=\"spoiler\" data-spoiler=\"\">ending</span></a> and <a href=\"https://example.com/shown\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">open</a></p>\n"
    );
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(&html).unwrap();
    let links: Vec<_> = dom.descendants(root).into_iter().filter(|&node| dom.local_name(node) == Some("a")).collect();
    assert!(markdown::conceals_spoiler(&dom, links[0]));
    assert!(!markdown::conceals_spoiler(&dom, links[1]));
}

#[test]
fn redact_spoilers_hides_from_the_first_pair_marker_to_the_last_in_each_block() {
    // The same cases as the SPA's redactMarkdownSpoilers (frontend/src/lib/spoiler-text.test.ts).
    for (source, redacted) in [
        ("see ||secret words|| now", "see spoiler now"),
        ("||@[David] is the killer||", "spoiler"),
        ("||nope", "||nope"),
        ("a ||| b", "a ||| b"),
        ("||top\n\nbottom||", "||top\n\nbottom||"),
        ("plain\n\n||secret|| end", "plain\n\nspoiler end"),
        ("||outer ||SECRET|| tail||", "spoiler"),
        ("\\`||SECRET||\\`", "\\`spoiler\\`"),
        ("`||` ||SECRET|| tail", "`spoiler tail"),
        ("```\n||SECRET||\n```", "```\nspoiler\n```"),
        ("||one|| and ||two||", "spoiler"),
        ("no markers at all", "no markers at all"),
    ] {
        assert_eq!(markdown::redact_spoilers(source), redacted, "{source:?}");
    }
}

#[test]
fn a_forwarded_spoiler_survives_an_edit() {
    // A forward stores the rendered HTML with no Markdown source. Editing it starts from
    // Markdown made from that HTML, and saving renders that Markdown again.
    let forwarded = render("before ||SECRET||");
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    let source = campfire_richtext::editable_markdown_source(&forwarded, None, &ctx).unwrap();
    assert_eq!(source, "before ||SECRET||");

    let shown = markdown::presentation(&forwarded, &ctx, &IconCatalog::default(), None).unwrap();
    let from_shown = campfire_richtext::legacy_markdown::render(&shown, &ctx).unwrap();
    assert_eq!(from_shown, "before ||SECRET||");

    let edited = render(&source.replace("before", "after"));
    assert_eq!(edited, "<p>after <span class=\"spoiler\" data-spoiler=\"\">SECRET</span></p>\n");
}

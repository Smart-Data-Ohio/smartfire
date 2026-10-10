//! Discord-style `||spoiler||` text: inline only, inert in code, redacted in plain-text previews.
use campfire_richtext::markdown::{self, IconCatalog};
use campfire_richtext::{AttachableResolver, GidLookup, RenderContext, SignedLookup};

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

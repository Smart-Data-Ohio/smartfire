//! GFM tables in Markdown messages: stored by `markdown::render`, and kept by the rendered body
//! the SPA shows (`Content::to_rendered_markdown_html_with_layout`).
use campfire_richtext::markdown::{self, IconCatalog};
use campfire_richtext::{AttachableResolver, Content, GidLookup, RenderContext, SignedLookup};

struct NoRecords;
impl AttachableResolver for NoRecords {
    fn locate_signed(&self, _: &str) -> SignedLookup {
        SignedLookup::Invalid
    }
    fn find_gid(&self, _: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

fn stored(source: &str) -> String {
    let icons = IconCatalog::default();
    let html = markdown::render(source, &(|_: &str| None), &icons).unwrap();
    // Saving canonicalizes the rendered body, as the message model does.
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    Content::load(&html, &ctx).unwrap().to_html()
}

/// The body the SPA receives for a Markdown message.
fn shown(source: &str) -> String {
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    Content::load(&stored(source), &ctx).unwrap().to_rendered_markdown_html_with_layout(&ctx).unwrap()
}

/// Whitespace between tags dropped, so the structure reads on one line.
fn compact(html: &str) -> String {
    html.split('\n').map(str::trim).collect()
}

const SIMPLE: &str = "| a | b | c |\n|---|---|---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |";

#[test]
fn a_simple_table_is_stored_and_shown_as_a_table() {
    let table = "<table><thead><tr><th>a</th><th>b</th><th>c</th></tr></thead><tbody>\
                 <tr><td>1</td><td>2</td><td>3</td></tr><tr><td>4</td><td>5</td><td>6</td></tr></tbody></table>";
    assert_eq!(compact(&stored(SIMPLE)), table);
    assert_eq!(compact(&shown(SIMPLE)), format!("<div class=\"trix-content\">{table}</div>"));
}

#[test]
fn the_action_text_rendering_still_drops_tables() {
    // Legacy (Trix) bodies keep Rails' allowlist: the cells' text without the table.
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    let html = Content::load(&stored(SIMPLE), &ctx).unwrap().to_rendered_html_with_layout(&ctx).unwrap();
    assert!(!html.contains("<table") && !html.contains("<td"), "{html}");
    assert!(html.contains('a') && html.contains('6'), "{html}");
}

#[test]
fn column_alignment_survives() {
    let html = compact(&shown("| L | C | R | N |\n| :--- | :---: | ---: | --- |\n| 1 | 2 | 3 | 4 |"));
    assert!(
        html.contains("<tr><th align=\"left\">L</th><th align=\"center\">C</th><th align=\"right\">R</th><th>N</th></tr>"),
        "{html}"
    );
    assert!(
        html.contains("<tr><td align=\"left\">1</td><td align=\"center\">2</td><td align=\"right\">3</td><td>4</td></tr>"),
        "{html}"
    );
}

#[test]
fn cells_keep_inline_code_links_bold_and_emoji() {
    let html = compact(&shown("| Code | Link | Bold |\n|---|---|---|\n| `a \\| b` | [docs](https://example.com) | **yes** :smile: |"));
    assert!(html.contains("<td><code>a | b</code></td>"), "{html}");
    assert!(html.contains("<td><a href=\"https://example.com\">docs</a></td>"), "{html}");
    assert!(html.contains("<td><strong>yes</strong> 😄</td>"), "{html}");
}

#[test]
fn a_table_right_after_a_paragraph_line_is_a_table() {
    let html = compact(&shown("Here is the summary:\n| Name | Status |\n|---|---|\n| Alpha | done |"));
    assert!(html.contains("<p>Here is the summary:</p><table><thead><tr><th>Name</th><th>Status</th></tr></thead>"), "{html}");
}

#[test]
fn a_pasted_table_with_indents_trailing_spaces_and_crlf_is_a_table() {
    for source in [
        "  | Name | Status |  \n  |------|--------|  \n  | Alpha | done |  \n",
        "| Name | Status |\r\n|------|--------|\r\n| Alpha | done |\r\n",
        "\n\n| Name | Status |\n|:-----|:------:|\n| Alpha | done |\n\n",
    ] {
        let html = compact(&shown(source));
        assert!(html.contains("<table><thead><tr><th"), "{source:?}: {html}");
        assert!(html.contains(">Alpha</td>"), "{source:?}: {html}");
    }
}

#[test]
fn the_markdown_body_allowlist_adds_only_table_tags_and_align() {
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    let hostile = "<table onclick=\"x()\" style=\"position:fixed\"><tr><td align=\"right\" onmouseover=\"x()\" \
                   colspan=\"9\">a</td></tr></table><input type=\"checkbox\"><iframe></iframe>";
    let html = compact(&Content::load(hostile, &ctx).unwrap().to_rendered_markdown_html_with_layout(&ctx).unwrap());
    assert_eq!(html, "<div class=\"trix-content\"><table><tbody><tr><td align=\"right\">a</td></tr></tbody></table></div>");
}

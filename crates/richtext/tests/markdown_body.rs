//! Markdown messages' tables, task lists, numbered lists and links: stored by `markdown::render`,
//! and kept by the rendered body the SPA shows (`Content::to_rendered_markdown_html_with_layout`).
use campfire_richtext::markdown::{self, IconCatalog, MARKDOWN_ATTRIBUTES, MARKDOWN_TAGS};
use campfire_richtext::sanitizer::SafeList;
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
    assert!(
        html.contains("<td><a href=\"https://example.com\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">docs</a></td>"),
        "{html}"
    );
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

/// `html`, as the SPA's rendered body shows it when it is a Markdown body.
fn shown_html(html: &str) -> String {
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    compact(&Content::load(html, &ctx).unwrap().to_rendered_markdown_html_with_layout(&ctx).unwrap())
}

#[test]
fn the_markdown_body_allowlist_drops_what_markdown_never_stores() {
    let hostile = "<table onclick=\"x()\" style=\"position:fixed\"><tr><td align=\"right\" onmouseover=\"x()\" \
                   colspan=\"9\">a</td></tr></table><iframe></iframe>";
    assert_eq!(shown_html(hostile), "<div class=\"trix-content\"><table><tbody><tr><td align=\"right\">a</td></tr></tbody></table></div>");
}

#[test]
fn the_markdown_body_allowlist_is_action_text_plus_the_markdown_renderers_own() {
    let list = SafeList::markdown_body();
    let action_text = SafeList::action_text();
    for tag in MARKDOWN_TAGS.iter().chain(&action_text.tags) {
        assert!(list.tags.contains(tag), "{tag}");
    }
    for attribute in MARKDOWN_ATTRIBUTES.iter().chain(&action_text.attributes) {
        assert!(list.attributes.contains(attribute), "{attribute}");
    }
    for tag in &list.tags {
        assert!(MARKDOWN_TAGS.contains(tag) || action_text.tags.contains(tag), "{tag}");
    }
    for attribute in &list.attributes {
        assert!(MARKDOWN_ATTRIBUTES.contains(attribute) || action_text.attributes.contains(attribute), "{attribute}");
    }
}

#[test]
fn the_sent_message_shows_exactly_what_markdown_stored() {
    // The composer preview shows `markdown::render`'s markup; the sent message must not lose any.
    let source = "# Plan\n\n> quoted **bold** _em_ ~~gone~~ ||secret||\n\n3. three\n4. four\n\n- [ ] todo\n- [x] done\n\n\
                  | L | R |\n| :- | -: |\n| [docs](https://example.com) | `x` |\n\n[room](/rooms/1) https://example.org\n\n---\n\n\
                  ```rust\nfn main() {}\n```";
    let stored = stored(source);
    for marker in ["<ol start=\"3\">", "type=\"checkbox\"", "checked", "<table>", "target=\"_blank\"", "data-spoiler", "<del>", "<hr>", "language-rust"] {
        assert!(stored.contains(marker), "{marker}: {stored}");
    }
    assert_eq!(compact(&shown(source)), format!("<div class=\"trix-content\">{}</div>", compact(&stored)));
}

#[test]
fn a_task_list_keeps_its_disabled_checkboxes() {
    assert_eq!(
        compact(&shown("- [ ] todo\n- [x] done")),
        "<div class=\"trix-content\"><ul><li><input type=\"checkbox\" disabled=\"disabled\"> todo</li>\
         <li><input type=\"checkbox\" checked=\"\" disabled=\"disabled\"> done</li></ul></div>"
    );
}

#[test]
fn only_a_disabled_checkbox_input_survives() {
    let html = shown_html(
        "<ul><li><input type=\"checkbox\" name=\"n\" value=\"v\" onclick=\"x()\" autofocus>a</li>\
         <li><input type=\"text\" value=\"v\">b</li><li><input>c</li><li><input type=\"submit\">d</li></ul>",
    );
    assert_eq!(
        html,
        "<div class=\"trix-content\"><ul><li><input type=\"checkbox\" disabled=\"disabled\">a</li><li>b</li><li>c</li><li>d</li></ul></div>"
    );
}

#[test]
fn an_ordered_list_keeps_its_start() {
    assert!(compact(&shown("7. seven\n8. eight")).contains("<ol start=\"7\"><li>seven</li><li>eight</li></ol>"));
    assert_eq!(
        shown_html("<ol start=\"2; x\"><li>a</li></ol><ul start=\"3\"><li>b</li></ul><p start=\"4\" type=\"x\" checked disabled>c</p>"),
        "<div class=\"trix-content\"><ol><li>a</li></ol><ul><li>b</li></ul><p>c</p></div>"
    );
}

#[test]
fn an_off_site_link_opens_in_a_new_tab_with_the_safe_rel() {
    let html = compact(&shown("[docs](https://example.com) and [room](/rooms/1)"));
    assert!(html.contains("<a href=\"https://example.com\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">docs</a>"), "{html}");
    assert!(html.contains("<a href=\"/rooms/1\">room</a>"), "{html}");
}

#[test]
fn a_links_target_and_rel_are_held_to_what_markdown_emits() {
    let html = shown_html(
        "<a href=\"https://a.example\" target=\"_top\" rel=\"opener\">a</a>\
         <a href=\"https://b.example\" rel=\"opener\">b</a>\
         <a href=\"/rooms/1\" target=\"_blank\" rel=\"opener\">c</a>\
         <a target=\"_blank\" rel=\"opener\">d</a>\
         <strong target=\"_blank\" rel=\"opener\">e</strong>",
    );
    assert_eq!(
        html,
        "<div class=\"trix-content\"><a href=\"https://a.example\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">a</a>\
         <a href=\"https://b.example\">b</a><a href=\"/rooms/1\">c</a><a>d</a><strong>e</strong></div>"
    );
}

#[test]
fn only_the_renderers_classes_survive() {
    let html = shown_html(
        "<p class=\"visually-hidden spoiler\">a</p><table class=\"visually-hidden\"><tr><td class=\"sr-only\">x</td></tr></table>\
         <pre><code class=\"language-rust hidden\">y</code></pre><ul class=\"contains-task-list x\"><li class=\"task-list-item\">z</li></ul>\
         <img class=\"icon icon--custom hidden\" src=\"https://example.com/i.png\" alt=\":i:\"><span class=\"icon\">q</span>",
    );
    assert_eq!(
        html,
        "<div class=\"trix-content\"><p>a</p><table><tbody><tr><td>x</td></tr></tbody></table>\
         <pre><code class=\"language-rust\">y</code></pre><ul class=\"contains-task-list\"><li class=\"task-list-item\">z</li></ul>\
         <img class=\"icon icon--custom\" src=\"https://example.com/i.png\" alt=\":i:\"><span>q</span></div>"
    );
}

#[test]
fn styles_and_handlers_never_survive() {
    let html = shown_html(
        "<table style=\"display:none\" onclick=\"x()\"><tr><td style=\"color:red\" onmouseover=\"x()\">a</td></tr></table>\
         <a href=\"https://example.com\" target=\"_blank\" onclick=\"x()\" style=\"x\">b</a>\
         <ul><li><input type=\"checkbox\" onchange=\"x()\" style=\"x\">c</li></ul><ol start=\"2\" onfocus=\"x()\"><li>d</li></ol>",
    );
    assert!(!html.contains(" style=") && !html.contains(" on"), "{html}");
    assert!(html.contains("<td>a</td>") && html.contains("<ol start=\"2\"><li>d</li></ol>"), "{html}");
}

#[test]
fn alignment_is_only_left_center_or_right_on_a_cell() {
    let html = shown_html(
        "<table><tr><th align=\"justify\">a</th><td align=\"RIGHT\">b</td><td align=\"center\">c</td><td align=\"left;x\">d</td></tr></table>\
         <p align=\"right\">e</p>",
    );
    assert_eq!(
        html,
        "<div class=\"trix-content\"><table><tbody><tr><th>a</th><td>b</td><td align=\"center\">c</td><td>d</td></tr></tbody></table>\
         <p>e</p></div>"
    );
}

#[test]
fn checkbox_list_start_and_link_values_are_the_renderers() {
    let html = shown_html(
        "<ul><li><input type=\"checkbox\" checked=\"checked\" disabled=\"no\">a</li></ul><ol start=\"9999999999\"><li>b</li></ol>\
         <a href=\"javascript:alert(1)\" target=\"_blank\" rel=\"opener\">c</a>",
    );
    assert_eq!(
        html,
        "<div class=\"trix-content\"><ul><li><input type=\"checkbox\" checked=\"\" disabled=\"disabled\">a</li></ul><ol><li>b</li></ol><a>c</a></div>"
    );
}

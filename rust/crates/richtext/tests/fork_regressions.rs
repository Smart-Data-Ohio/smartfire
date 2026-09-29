use campfire_richtext::{AttachableResolver, GidLookup, RenderContext, SignedLookup, message_presentation};
struct NoRecords;
impl AttachableResolver for NoRecords {
    fn locate_signed(&self, _: &str) -> SignedLookup {
        SignedLookup::Invalid
    }
    fn find_gid(&self, _: &str) -> GidLookup {
        GidLookup::NotFound
    }
}
#[test]
fn legacy_wrapper_and_allowlist_follow_our_fork() {
    let ctx = RenderContext { resolver: &NoRecords, request_host: Some("once.campfire.test".into()) };
    let html =
        message_presentation("<div>hello<table><tr><td>hidden</td></tr></table><s>hidden</s><u>hidden</u><mark>hidden</mark></div>", &ctx)
            .unwrap();
    assert!(html.starts_with("<div class=\"trix-content\">"), "{html}");
    assert!(!html.contains("hidden"), "{html}");
}

#[test]
fn solo_unfurls_match_raw_relative_and_same_host_hrefs() {
    let ctx = RenderContext { resolver: &NoRecords, request_host: Some("once.campfire.test".into()) };
    for href in ["/rooms/1", "https://once.campfire.test/rooms/1"] {
        let body = format!(
            "<div>{href}</div><action-text-attachment content-type=\"application/vnd.actiontext.opengraph-embed\" href=\"{href}\" filename=\"Room\"></action-text-attachment>"
        );
        let content = campfire_richtext::Content::load(&body, &ctx).unwrap();
        let filtered = campfire_richtext::filters::remove_solo_unfurled_link_text(content, &ctx).unwrap();
        assert!(filtered.dom.text_content(filtered.root).is_empty(), "{href}: {}", filtered.to_html());
        assert_eq!(campfire_richtext::content::attachment_nodes(&filtered.dom, filtered.root).len(), 1);
    }
}

#[test]
fn malformed_json_with_invalid_utf8_can_have_a_loggable_error() {
    let ctx = RenderContext { resolver: &NoRecords, request_host: None };
    let body = "<action-text-attachment sgid=\"eyJfcmFpbHMiOnsiZGF0YSIgIv8ifX0=\"></action-text-attachment>";
    assert_eq!(campfire_richtext::present_message(body, &ctx), campfire_richtext::Presentation::Html(String::new()));
    assert_eq!(
        campfire_richtext::present_message("<action-text-attachment sgid=\"nope\"></action-text-attachment>", &ctx),
        campfire_richtext::Presentation::Unrenderable
    );
}

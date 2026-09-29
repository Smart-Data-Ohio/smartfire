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

macro_rules! sgid_regression {
    ($name:ident, $sgid:literal, $expected:expr) => {
        #[test]
        fn $name() {
            let ctx = RenderContext { resolver: &NoRecords, request_host: None };
            let body = concat!("<action-text-attachment sgid=\"", $sgid, "\"></action-text-attachment>");
            assert_eq!(campfire_richtext::present_message(body, &ctx), $expected);
        }
    };
}
sgid_regression!(reviewer_unknown_token_before_line_comment, "eC8v/3k=", campfire_richtext::Presentation::Unrenderable);
sgid_regression!(reviewer_unknown_token_before_block_comment, "eC8q/3k=", campfire_richtext::Presentation::Unrenderable);
sgid_regression!(reviewer_false_token_before_invalid_utf8, "ZiD/eQ==", campfire_richtext::Presentation::Html(String::new()));
sgid_regression!(
    reviewer_nested_false_token_before_invalid_utf8,
    "WzEsMix7IngiOmYgbHP/LDNd",
    campfire_richtext::Presentation::Html(String::new())
);

sgid_regression!(
    reviewer_keyword_before_unterminated_comment,
    "WzEsMix7IngiOmZhbC8qeCo6c2V9LDOAXQ==",
    campfire_richtext::Presentation::Unrenderable
);
sgid_regression!(
    reviewer_keyword_before_later_invalid_token,
    "eyJhIjpbdHJ1ZSxmYWxzCSxudf9sbCwidGVzdCJdLCJiIjoxMi4zfQ==",
    campfire_richtext::Presentation::Html(String::new())
);

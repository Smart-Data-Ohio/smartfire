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

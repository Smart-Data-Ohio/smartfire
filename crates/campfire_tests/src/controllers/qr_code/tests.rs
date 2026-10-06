use super::*;

#[test]
fn decodes_like_ruby_urlsafe_decode64() {
    assert_eq!(urlsafe_decode64("aHR0cDovL2NhbXBmaXJlLnRlc3Q").unwrap(), b"http://campfire.test");
    assert_eq!(urlsafe_decode64("aHR0cDovL2NhbXBmaXJlLnRlc3Q=").unwrap(), b"http://campfire.test");
    assert_eq!(urlsafe_decode64("-_8").unwrap(), vec![0xfb, 0xff]);
    assert_eq!(urlsafe_decode64(""), Some(vec![]));
    assert_eq!(urlsafe_decode64("a"), None);
    assert_eq!(urlsafe_decode64("ab=c"), None);
    assert_eq!(urlsafe_decode64("aB=="), None);
    assert_eq!(urlsafe_decode64("a*bc"), None);
}

#[tokio::test]
async fn qr_http_matches_rails_bytes_cache_and_capacity_errors() {
    use crate::controllers::presenters::test_support::TestApp;
    let Some(app) = TestApp::boot().await else { return };
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/users_public.json")).unwrap();
    for vector in vectors["qr"].as_array().unwrap() {
        let response = app.anonymous().get(&format!("/qr_code/{}", vector["id"].as_str().unwrap())).await;
        assert_eq!(response.status.as_u16(), vector["status"].as_u64().unwrap() as u16);
        if let Some(body) = vector["body"].as_str() {
            assert_eq!(response.text(), body);
            assert_eq!(response.header("cache-control"), vector["cache_control"].as_str());
            assert_eq!(response.content_type(), Some("image/svg+xml; charset=utf-8"));
        }
    }
    for bad in ["a", "ab=c", "aB==", "a*bc"] {
        assert_eq!(app.anonymous().get(&format!("/qr_code/{bad}")).await.status.as_u16(), 500);
    }
}

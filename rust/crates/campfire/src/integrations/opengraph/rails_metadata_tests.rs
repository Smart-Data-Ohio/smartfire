//! Native equivalents of all eleven pinned Opengraph::MetadataTest cases.
use super::*;
use crate::integrations::{net::Network, test_support::*};
use std::sync::Arc;
async fn metadata(
    body: &str,
    canonical: Option<&str>,
    image: Option<&str>,
    status: u16,
    content_type: &str,
) -> (FakeServer, Metadata, Network) {
    let tags = format!(
        "{}{}{}",
        body,
        canonical
            .map(|u| format!("<meta property='og:url' content='{u}'>"))
            .unwrap_or_default(),
        image
            .map(|u| format!("<meta property='og:image' content='{u}'>"))
            .unwrap_or_default()
    );
    let (server, roots) = FakeServer::start_named_tls_ws15e(
        vec![
            Route::new("GET", "www.example.com", "/", status)
                .header("Content-Type", content_type)
                .body(tags),
            Route::new("HEAD", "example.com", "/image.png", 200)
                .header("Content-Type", "image/png"),
            Route::new("HEAD", "example.com", "/image.svg", 200)
                .header("Content-Type", "image/svg+xml"),
        ],
        vec!["www.example.com".into(), "example.com".into()],
    )
    .await;
    let resolver = Arc::new(FakeResolver::new([
        ("www.example.com", vec!["93.184.216.34"]),
        ("example.com", vec!["93.184.216.34"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let net = Network {
        resolver,
        dialer,
        tls: crate::integrations::net::tls_config(roots),
    };
    let metadata = Metadata::from_url(&net, "https://www.example.com")
        .await
        .unwrap();
    (server, metadata, net)
}
const TAGS: &str =
    "<meta property='og:title' content='Hey!'><meta property='og:description' content='Hello'>";
#[tokio::test]
async fn ws15e_rails_metadata_success() {
    let (_s, mut m, n) = metadata(
        TAGS,
        Some("https://example.com"),
        Some("https://example.com/image.png"),
        200,
        "text/html",
    )
    .await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(
        (
            m.get("url"),
            m.get("title"),
            m.get("description"),
            m.get("image")
        ),
        (
            Some("https://example.com"),
            Some("Hey!"),
            Some("Hello"),
            Some("https://example.com/image.png")
        )
    );
}
#[tokio::test]
async fn ws15e_rails_metadata_missing_tags() {
    let (_s, mut m, n) = metadata("<html><head></head></html>", None, None, 200, "text/html").await;
    assert!(!m.validate(&n).await.unwrap());
    let json: serde_json::Value = serde_json::from_str(&m.to_json()).unwrap();
    assert_eq!(
        json["errors"],
        serde_json::json!({"title":["can't be blank"],"description":["can't be blank"]})
    );
}
#[tokio::test]
async fn ws15e_rails_metadata_missing_canonical() {
    let (_s, mut m, n) = metadata(
        TAGS,
        None,
        Some("https://example.com/image.png"),
        200,
        "text/html",
    )
    .await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(m.get("url"), Some("https://www.example.com"));
}
#[tokio::test]
async fn ws15e_rails_metadata_invalid_canonical() {
    let (_s, mut m, n) = metadata(
        TAGS,
        Some("/foo"),
        Some("https://example.com/image.png"),
        200,
        "text/html",
    )
    .await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(m.get("url"), Some("https://www.example.com"));
}
#[tokio::test]
async fn ws15e_rails_metadata_missing_body() {
    let (_s, mut m, n) = metadata("", None, None, 403, "text/html").await;
    assert!(!m.validate(&n).await.unwrap());
}
#[tokio::test]
async fn ws15e_rails_metadata_non_html() {
    let (_s, mut m, n) = metadata("[blob]", None, None, 200, "image/jpeg").await;
    assert!(!m.validate(&n).await.unwrap());
}
#[tokio::test]
async fn ws15e_rails_metadata_relative_images() {
    for image in ["/image.png", "foo", "https/incorrect", "~/etc/password"] {
        let (s, mut m, n) = metadata(
            TAGS,
            Some("https://example.com"),
            Some(image),
            200,
            "text/html",
        )
        .await;
        assert!(m.validate(&n).await.unwrap());
        assert_eq!(m.get("image"), None);
        assert_eq!(s.received().len(), 1);
    }
}
#[tokio::test]
async fn ws15e_rails_metadata_sanitizes_markup() {
    let (_s,mut m,n)=metadata("<meta property=\"og:title\" content=\"Hey!<script>alert('hi')</script>\"><meta property=\"og:description\" content=\"Hello<script>alert('hi')</script>\">",None,Some("https://example.com/image.png"),200,"text/html").await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(m.get("title"), Some("Hey!alert('hi')"));
    assert_eq!(m.get("description"), Some("Helloalert('hi')"));
}
#[tokio::test]
async fn ws15e_rails_metadata_sanitizes_encoded_tags() {
    let (_s,mut m,n)=metadata("<meta property=\"og:title\" content=\"Hey!&lt;/script&gt;&lt;img src=a onerror=prompt(1)&gt;\"><meta property=\"og:description\" content=\"Hello&lt;/script&gt;&lt;img src=a onerror=prompt(2)&gt;\">",None,Some("https://example.com/image.png"),200,"text/html").await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(m.get("title"), Some("Hey!"));
    assert_eq!(m.get("description"), Some("Hello"));
}
#[tokio::test]
async fn ws15e_rails_metadata_entire_markup_rejected() {
    let (_s,mut m,n)=metadata("<meta property=\"og:title\" content=\"<img src='x' onerror='alert(document.domain)'/>\"><meta property=\"og:description\" content=\"<img src='x' onerror='alert(document.domain)'/>\">",None,Some("https://example.com/image.png"),200,"text/html").await;
    assert!(!m.validate(&n).await.unwrap());
    assert_eq!(m.get("title"), Some(""));
    assert_eq!(m.get("description"), Some(""));
    let json: serde_json::Value = serde_json::from_str(&m.to_json()).unwrap();
    assert_eq!(
        json["errors"],
        serde_json::json!({"title":["can't be blank"],"description":["can't be blank"]})
    );
}
#[tokio::test]
async fn ws15e_rails_metadata_svg_rejected() {
    let (_s, mut m, n) = metadata(
        TAGS,
        Some("https://example.com"),
        Some("https://example.com/image.svg"),
        200,
        "text/html",
    )
    .await;
    assert!(m.validate(&n).await.unwrap());
    assert_eq!(m.get("image"), None);
}
#[tokio::test]
async fn ws15e_metadata_errors_and_assignment_order_match_pinned_rails_bytes() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/ws15e_opengraph_validation.json"
    ))
    .unwrap();
    let network = Network::system();
    for case in vectors["cases"].as_array().unwrap() {
        let attributes = case["attributes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                let key = match key.as_str() {
                    "title" => "title",
                    "url" => "url",
                    "description" => "description",
                    _ => panic!("unknown oracle attribute"),
                };
                (key, value.as_str().map(str::to_owned))
            })
            .collect();
        let mut model = Metadata {
            attributes,
            errors: Default::default(),
            validation_position: None,
        };
        assert_eq!(
            model.validate(&network).await.unwrap(),
            case["valid"].as_bool().unwrap()
        );
        assert_eq!(
            serde_json::json!(model.errors.full_messages()),
            case["full_messages"]
        );
        assert_eq!(model.to_json(), case["json"].as_str().unwrap());
        // Revalidation clears old errors and retains the original instance-variable position.
        assert_eq!(
            model.validate(&network).await.unwrap(),
            case["valid"].as_bool().unwrap()
        );
        assert_eq!(model.to_json(), case["json"].as_str().unwrap());
    }
}

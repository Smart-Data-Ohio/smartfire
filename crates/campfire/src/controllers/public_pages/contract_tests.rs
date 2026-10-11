//! The SPA's About, Privacy and Terms contract against the retained pages it copies.
use crate::controllers::presenters::test_support::{DAVID, Reply, Req, TestApp, seed_clock};
use axum::http::{Method, StatusCode};
use campfire_api_types::{PublicPage, PublicPageName};
use serde_json::Value;
use std::path::Path;

const PAGES: [(&str, PublicPageName); 3] = [
    ("about", PublicPageName::About),
    ("privacy", PublicPageName::Privacy),
    ("terms", PublicPageName::Terms),
];

fn json_get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

/// The retained page's article: what its layout puts in the reading column.
fn retained_article(body: &str) -> &str {
    let (_, rest) = body
        .split_once("<article class=\"public-prose\">")
        .expect("retained article");
    rest.split_once("</article>").expect("article end").0.trim()
}

/// The contract's article with its links back on the retained pages.
fn as_retained(html: &str) -> String {
    html.replace("href=\"/app/privacy\"", "href=\"/privacy\"")
        .replace("href=\"/app/terms\"", "href=\"/terms\"")
        .replace("href=\"/app/session/new\"", "href=\"/session/new\"")
}

fn inline_boot(reply: &Reply) -> Value {
    let re =
        regex::Regex::new(r#"(?s)<script type="application/json" id="boot"[^>]*>(.*?)</script>"#)
            .unwrap();
    serde_json::from_str(&re.captures(&reply.text()).expect("boot script")[1]).unwrap()
}

/// Every page of `app` matches its retained render, for anyone (and a `member`, where the seed has
/// people); answers the three contracts.
async fn assert_matches_retained(app: &TestApp, state: &str, member: bool) -> Vec<PublicPage> {
    let policy = app.booted.app.config.public_policy.clone();
    let mut pages = Vec::new();
    for (slug, name) in PAGES {
        let reply = app
            .anonymous()
            .send(json_get(&format!("/api/v1/public_pages/{slug}")))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{state} {slug}");
        assert!(
            reply
                .header("content-type")
                .is_some_and(|value| value.starts_with("application/json")),
            "{state} {slug}"
        );
        assert_eq!(reply.header("set-cookie"), None, "{state} {slug}");
        let page: PublicPage = serde_json::from_slice(&reply.body).unwrap();
        assert_eq!(page.page, name);
        assert_eq!(page.policy.operator_name, policy.operator_name, "{state} {slug}");
        assert_eq!(page.policy.contact_email, policy.contact_email, "{state} {slug}");
        assert_eq!(page.policy.effective_date, policy.effective_date, "{state} {slug}");

        let retained = app.anonymous().get(&format!("/{slug}")).await;
        assert_eq!(retained.status, StatusCode::OK);
        let body = retained.text();
        assert!(body.contains(&format!("<title>{}</title>", page.title)), "{state} {slug}");
        assert!(
            body.contains(&format!("<meta name=\"description\" content=\"{}\">", page.description)),
            "{state} {slug}"
        );
        assert_eq!(as_retained(&page.html).trim(), retained_article(&body), "{state} {slug}");
        // Every in-text link to a page the SPA has stays in the SPA.
        for retained_link in ["href=\"/privacy\"", "href=\"/terms\"", "href=\"/session/new\""] {
            assert!(!page.html.contains(retained_link), "{state} {slug}: {retained_link}");
        }

        if member {
            let signed_in = app.sign_in(DAVID).await.send(json_get(&format!("/api/v1/public_pages/{slug}"))).await;
            assert_eq!(signed_in.status, StatusCode::OK);
            assert_eq!(signed_in.body, reply.body, "{state} {slug}: same for a member");
        }
        pages.push(page);
    }
    pages
}

#[tokio::test]
async fn public_page_contract_matches_the_retained_render_for_each_policy() {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../../vectors/users_public.json")).unwrap();
    for (index, state) in vectors["pages"].as_array().unwrap().iter().enumerate() {
        let vars: Vec<_> = state["env"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect();
        let app = TestApp::boot_seed_with_env("default", seed_clock(), &vars)
            .await
            .expect("restore the frozen seeds");
        let pages = assert_matches_retained(&app, &format!("state {index}"), true).await;
        // What decides each page: the operator and address, when set, and the effective date.
        let policy = &app.booted.app.config.public_policy;
        for page in &pages {
            assert_eq!(
                page.html.contains("This workspace is operated by <strong>"),
                policy.operator_name.is_some(),
                "state {index}"
            );
            assert_eq!(
                page.html.contains("<a href=\"mailto:"),
                policy.contact_email.is_some(),
                "state {index}"
            );
            if page.page != PublicPageName::About {
                assert_eq!(
                    page.html.contains("Your workspace\n    administrator can tell you who that is."),
                    policy.operator_name.is_none(),
                    "state {index}"
                );
            }
        }
        assert_eq!(
            pages[0].html.contains("<h2>Who runs this workspace</h2>"),
            policy.operator_name.is_some() || policy.contact_email.is_some(),
            "state {index}"
        );
    }
    let app = TestApp::boot_seed_with_env("first_run", seed_clock(), &[])
        .await
        .expect("restore the frozen seeds");
    assert_matches_retained(&app, "first run", false).await;
}

#[tokio::test]
async fn public_page_contract_refuses_other_pages() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for path in [
        "/api/v1/public_pages/contact",
        "/api/v1/public_pages/About",
        "/api/v1/public_pages/about.json",
    ] {
        let reply = app.anonymous().send(json_get(path)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(reply.body.is_empty(), "{path}");
    }
}

#[tokio::test]
async fn spa_public_pages_serve_the_signed_out_shell_or_the_app() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for path in ["/app/about", "/app/privacy", "/app/terms"] {
        let reply = app.anonymous().get(path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
        assert_eq!(reply.header("cache-control"), Some("no-store"), "{path}");
        assert_eq!(inline_boot(&reply)["kind"], "signedOut", "{path}");
        // No public discovery beyond the retained pages: nothing asks robots to skip them there.
        assert!(!reply.text().contains("noindex"), "{path}");

        let mut browser = app.sign_in(DAVID).await;
        let reply = browser.get(path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        let boot = inline_boot(&reply);
        assert_ne!(boot["kind"], "signedOut", "{path}");
        assert!(boot["user"].is_object(), "{path}: the signed-in boot");
    }
}

/// The mock backend's copy of the contract (frontend/mock/s2/public-pages.json), with the policy
/// the auth-page fixtures use.
#[tokio::test]
async fn public_page_mock_fixture_matches_the_contract() {
    let app = TestApp::boot_seed_with_env(
        "default",
        seed_clock(),
        &[
            ("LEGAL_OPERATOR_NAME", "Example <&> Labs"),
            ("LEGAL_CONTACT_EMAIL", "team+auth@example.test"),
            ("LEGAL_EFFECTIVE_DATE", "2026-10-07"),
        ],
    )
    .await
    .expect("restore the frozen seeds");
    let mut pages = serde_json::Map::new();
    for (slug, _) in PAGES {
        let reply = app
            .anonymous()
            .send(json_get(&format!("/api/v1/public_pages/{slug}")))
            .await;
        pages.insert(slug.into(), reply.json());
    }
    let text = serde_json::to_string_pretty(&Value::Object(pages)).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/mock/s2/public-pages.json");
    if std::env::var("UPDATE_AUTH_PAGE_FIXTURES").as_deref() == Ok("1") {
        std::fs::write(&path, &text).unwrap();
    } else {
        assert_eq!(
            std::fs::read_to_string(&path).ok().as_deref(),
            Some(text.as_str()),
            "the public page mock fixture differs. Rerun with UPDATE_AUTH_PAGE_FIXTURES=1 cargo test -j 4 -p campfire --bin campfire -- public_page_mock_fixture"
        );
    }
}

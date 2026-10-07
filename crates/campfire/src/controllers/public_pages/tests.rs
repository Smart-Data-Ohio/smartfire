use crate::controllers::presenters::test_support::{Req, TestApp};
use axum::http::{Method, StatusCode};

/// PublicPagesControllerTest uses nav ancestry and real links, which comments cannot satisfy.
pub(super) fn assert_public_links(body: &str) {
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(body).unwrap();
    let navs = dom.descendants(root).into_iter().filter(|id| {
        dom.name(*id) == "nav" && dom.attr(*id, "aria-label") == Some("About this workspace")
    }).collect::<Vec<_>>();
    assert_eq!(navs.len(), 1, "exact About this workspace nav selector");
    for path in ["/about", "/privacy", "/terms"] {
        let links = dom.descendants(navs[0]).into_iter().filter(|id| {
            dom.name(*id) == "a" && dom.attr(*id, "href") == Some(path)
                && dom.attr(*id, "target") == Some("_blank")
                && dom.attr(*id, "rel") == Some("noopener")
        }).count();
        assert_eq!(links, 1, "exact nav public link selector for {path}");
    }
}

#[tokio::test]
async fn unconfigured_sign_in_links_all_public_pages_in_new_tabs() {
    // PublicPagesControllerTest at d7c7de92; render WS9's sign-in page directly.
    let app = TestApp::boot_frozen().await.expect("seed required");
    let response = app.anonymous().get("/session/new").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(!response.text().contains("Sign in with Google"));
    let body = response.text();
    assert_public_links(&body);
}

#[tokio::test]
async fn public_pages_bypass_authentication_browser_and_private_state() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    assert!(!app.booted.app.config.google_client.configured());
    assert!(!app.booted.app.google.sign_in().config.configured());
    for ua in [
        "",
        "curl/8.0",
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0",
    ] {
        for path in ["/about", "/privacy", "/terms"] {
            let response = app
                .anonymous()
                .send(Req::new(Method::GET, path).header("user-agent", ua))
                .await;
            assert_eq!(response.status, StatusCode::OK, "{path} {ua}");
            assert_eq!(response.header("set-cookie"), None);
            assert_eq!(response.header("x-version"), None);
            assert_eq!(response.header("x-rev"), None);
            for private in [
                "current-user-id",
                "david@",
                "vapid-public-key",
                "brand-icon-names",
                "google-picker",
                "importmap",
                "csrf-token",
                "csrf-param",
                "noindex",
                "action-cable",
                "turbo-prefetch",
                "google-drive-previews",
                "Upgrade to a supported web browser",
            ] {
                assert!(
                    !response.text().contains(private),
                    "{path} contains {private}"
                );
            }
        }
    }
    for path in ["/about", "/privacy", "/terms"] {
        let mut browser = app.david();
        let response = browser.get(path).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.header("set-cookie"), None);
        assert_eq!(response.text(), app.anonymous().get(path).await.text());
        let head = browser.send(Req::new(Method::HEAD, path)).await;
        assert_eq!(head.status, StatusCode::OK);
        assert!(head.body.is_empty());
    }
}

#[tokio::test]
async fn public_pages_are_html_only_and_allow_wildcard_accept() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    for path in ["/about", "/privacy", "/terms"] {
        for accept in ["*/*", "text/html"] {
            let response = app
                .anonymous()
                .send(Req::new(Method::GET, path).header("accept", accept))
                .await;
            assert_eq!(response.status, StatusCode::OK);
            assert!(response.text().contains("<title>Smartfire"));
        }
        for accept in [
            "application/json",
            "text/plain",
            "text/vnd.turbo-stream.html",
        ] {
            let response = app
                .anonymous()
                .send(Req::new(Method::GET, path).header("accept", accept))
                .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND);
            assert!(response.body.is_empty());
        }
        for suffix in ["json", "txt", "xml"] {
            assert_eq!(
                app.anonymous()
                    .get(&format!("{path}.{suffix}"))
                    .await
                    .status,
                StatusCode::NOT_FOUND
            );
        }
    }
}

#[tokio::test]
async fn public_policy_text_links_and_escaped_values_match_rails() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/users_public.json")).unwrap();
    for state in vectors["pages"].as_array().unwrap() {
        let vars: Vec<_> = state["env"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect();
        let Some(app) = TestApp::boot_seed_with_env(
            "default",
            crate::controllers::presenters::test_support::seed_clock(),
            &vars,
        )
        .await
        else {
            return;
        };
        for page in ["about", "privacy", "terms"] {
            let response = app.anonymous().get(&format!("/{page}")).await;
            assert_eq!(response.status, StatusCode::OK);
            let expected = state["bodies"][page].as_str().unwrap();
            let actual = response.text();
            crate::form_contracts::assert_public_page(page, &actual, expected);
            assert_eq!(response.header("set-cookie"), None);
        }
    }
    let Some(app) = TestApp::boot_seed_with_env(
        "first_run",
        crate::controllers::presenters::test_support::seed_clock(),
        &[],
    )
    .await
    else {
        return;
    };
    for page in ["about", "privacy", "terms"] {
        crate::form_contracts::assert_public_page(page,
            &app.anonymous().get(&format!("/{page}")).await.text(),
            vectors["pages"][0]["bodies"][page].as_str().unwrap());
    }
}
#[tokio::test]
async fn public_pages_accept_anonymous_head_without_a_body() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for path in ["/about", "/privacy", "/terms"] {
        let reply = app.anonymous().send(Req::new(Method::HEAD, path)).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: anonymous HEAD");
        assert!(reply.body.is_empty(), "{path}: original empty HEAD body");
    }
}

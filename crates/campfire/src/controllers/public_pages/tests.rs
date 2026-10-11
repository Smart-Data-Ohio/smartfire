use crate::controllers::presenters::test_support::{Reply, Req, TestApp};
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

/// The retained page as its handler drew it before the move: the public shell, the auth
/// stylesheet and script, and the app's `LEGAL_*` settings.
pub(super) fn retained_page(app: &TestApp, page: &str) -> String {
    use campfire_retained::{helpers as h, public_pages};
    let page = match page {
        "about" => public_pages::Page::About,
        "privacy" => public_pages::Page::Privacy,
        "terms" => public_pages::Page::Terms,
        _ => unreachable!("{page}"),
    };
    let policy = &app.booted.app.config.public_policy;
    public_pages::render(
        page,
        &policy.operator_name,
        &policy.contact_email,
        &policy.effective_date,
        h::raw(format!("{}\n{}", h::auth_stylesheet_tag(), h::auth_script_tag())),
    )
    .unwrap()
}

/// `reply` sends `path` (`/about`, ...) for good to the SPA's copy, with `query` kept.
fn assert_moved(reply: &Reply, path: &str, query: &str) {
    assert_eq!(reply.status, StatusCode::MOVED_PERMANENTLY, "{path}{query}");
    assert_eq!(
        reply.location().map(str::to_owned),
        Some(format!("http://campfire.test/app{path}{query}")),
        "{path}{query}"
    );
}

#[tokio::test]
async fn unconfigured_sign_in_links_all_public_pages_in_new_tabs() {
    // PublicPagesControllerTest at d7c7de92; render WS9's sign-in page directly, as
    // /session/new now sends visitors to the SPA's.
    let app = TestApp::boot_frozen().await.expect("seed required");
    let response = app.anonymous().get("/session/new").await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.location(), Some("http://campfire.test/app/session/new"));
    let body = crate::controllers::users::people_tests::render_with(
        &app,
        |ctx| {
            ctx.current_user = None;
        },
        |ctx| {
            use askama::Template;
            let ctx = crate::controllers::users::people_tests::retained(ctx);
            campfire_retained::sessions::New {
                ctx: &ctx,
                email_address: None,
                help_contact: None,
                google_sign_in_domains: app.booted.app.config.sign_in_google_domains.clone(),
            }
            .as_content()
            .render()
            .unwrap()
        },
    );
    assert!(!body.contains("Sign in with Google"));
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
            assert_moved(&response, path, "");
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
        assert_moved(&response, path, "");
        assert_eq!(response.header("set-cookie"), None);
        assert_eq!(response.text(), app.anonymous().get(path).await.text());
        let head = browser.send(Req::new(Method::HEAD, path)).await;
        assert_moved(&head, path, "");
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
            assert_moved(&response, path, "");
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

/// The retained pages' renderer still draws Rails' text, links and escaped values; the URLs
/// themselves now send everyone to the SPA's copies.
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
            assert_moved(&response, &format!("/{page}"), "");
            assert_eq!(response.header("set-cookie"), None);
            let expected = state["bodies"][page].as_str().unwrap();
            crate::form_contracts::assert_public_page(page, &retained_page(&app, page), expected);
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
        assert_moved(&app.anonymous().get(&format!("/{page}")).await, &format!("/{page}"), "");
        crate::form_contracts::assert_public_page(page,
            &retained_page(&app, page),
            vectors["pages"][0]["bodies"][page].as_str().unwrap());
    }
}
#[tokio::test]
async fn public_pages_accept_anonymous_head_without_a_body() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for path in ["/about", "/privacy", "/terms"] {
        let reply = app.anonymous().send(Req::new(Method::HEAD, path)).await;
        assert_moved(&reply, path, "");
        assert!(reply.body.is_empty(), "{path}: original empty HEAD body");
    }
}

/// The retained URLs moved for good to the SPA's pages, which draw them for anyone.
#[tokio::test]
async fn public_pages_move_permanently_to_the_spa_with_their_query() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for path in ["/about", "/privacy", "/terms"] {
        for query in ["", "?ref=footer&lang=en"] {
            let target = format!("{path}{query}");
            for method in [Method::GET, Method::HEAD] {
                let reply = app.anonymous().send(Req::new(method.clone(), &target)).await;
                assert_moved(&reply, path, query);
                if method == Method::HEAD {
                    assert!(reply.body.is_empty(), "{target}");
                }
            }
            let mut browser = app.anonymous();
            let moved = browser.get(&target).await;
            let page = browser.get(moved.location().unwrap()).await;
            assert_eq!(page.status, StatusCode::OK, "{target}: {}", page.text());
            assert!(
                page.text().contains(r#"<script type="application/json" id="boot""#),
                "{target}"
            );
        }
    }
}

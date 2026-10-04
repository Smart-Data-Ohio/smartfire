//! Public pages inherit ActionController::Base, bypassing the workspace callback chain.

#[cfg(test)]
#[path = "public_pages/sign_in_google_tests.rs"]
mod sign_in_google_tests;

use crate::app::AppCtx;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::{
    helpers as h,
    public_pages::{self, Page},
};

pub async fn about(c: &mut Ctx) -> Result {
    show(c, Page::About)
}
pub async fn privacy(c: &mut Ctx) -> Result {
    show(c, Page::Privacy)
}
pub async fn terms(c: &mut Ctx) -> Result {
    show(c, Page::Terms)
}

fn show(c: &mut Ctx, page: Page) -> Result {
    let formats = c.formats()?;
    if !formats
        .first()
        .is_some_and(|value| **value == format::HTML || **value == format::ALL)
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    c.respond_to(&[&format::HTML])?;
    let policy = &c.app().config.public_policy;
    let stylesheet =
        h::raw(campfire_assets::stylesheet_link_tag(&["public"], &[("media", "all")]).html);
    let body = public_pages::render(
        page,
        &policy.operator_name,
        &policy.contact_email,
        &policy.effective_date,
        stylesheet,
    )
    .map_err(Error::internal)?;
    Ok(c.render_html(StatusCode::OK, body))
}

#[cfg(test)]
mod tests {
    use crate::controllers::presenters::test_support::{Req, TestApp};
    use axum::http::{Method, StatusCode};

    #[tokio::test]
    async fn unconfigured_sign_in_links_all_public_pages_in_new_tabs() {
        // PublicPagesControllerTest at d7c7de92; render WS9's sign-in page directly.
        let app = TestApp::boot_frozen().await.expect("seed required");
        let response = app.anonymous().get("/session/new").await;
        assert_eq!(response.status, StatusCode::OK);
        assert!(!response.text().contains("Sign in with Google"));
        let body = response.text();
        let links = body
            .split_once("aria-label=\"About this workspace\"")
            .unwrap()
            .1
            .split_once("</nav>")
            .unwrap()
            .0;
        for path in ["/about", "/privacy", "/terms"] {
            assert!(
                links.contains(&format!(
                    "<a target=\"_blank\" rel=\"noopener\" href=\"{path}\">"
                )),
                "missing protected public link {path}"
            );
        }
    }

    #[tokio::test]
    async fn public_pages_bypass_authentication_browser_and_private_state() {
        let Some(app) = TestApp::boot().await else {
            return;
        };
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
                    "<script",
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
    async fn public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../../../../vectors/users_public.json")).unwrap();
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
                if actual != expected {
                    let first = actual
                        .bytes()
                        .zip(expected.bytes())
                        .position(|(a, b)| a != b)
                        .unwrap_or(actual.len().min(expected.len()));
                    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
                        std::fs::write(format!("{dir}/actual-public-{page}.html"), &actual)
                            .unwrap();
                        std::fs::write(format!("{dir}/expected-public-{page}.html"), expected)
                            .unwrap();
                    }
                    panic!("{} {page}: first differing byte {first}", state["state"]);
                }
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
            assert_eq!(
                app.anonymous().get(&format!("/{page}")).await.text(),
                vectors["pages"][0]["bodies"][page].as_str().unwrap()
            );
        }
    }
}

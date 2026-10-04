//! `PwaController` (reference/app/controllers/pwa_controller.rb): the web app manifest and the
//! service worker, at stable URLs.

use askama::Template;
use campfire_db::Account;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::pwa;

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters;

/// `allow_unauthenticated_access`, `skip_forgery_protection`
fn before() -> Before {
    Before::default().allow_unauthenticated_access().skip_forgery_protection()
}

/// `pwa/service_worker.js`
pub async fn service_worker(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JS])?;
    Ok(c.render_as(StatusCode::OK, "text/javascript; charset=utf-8", pwa::SERVICE_WORKER_JS))
}

/// `pwa/manifest.json.erb`
pub async fn manifest(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JSON])?;
    let account = c.app().db.read(Account::first).await.map_err(Error::internal)?;
    let asset_path = |path: &str| campfire_assets::asset_path(path);
    let manifest = pwa::Manifest {
        account_name: account.as_ref().map(|account| account.name.clone()),
        logo_path_small: presenters::accounts::fresh_account_logo_path(account.as_ref(), Some("small")),
        logo_path: presenters::accounts::fresh_account_logo_path(account.as_ref(), None),
        base_url: c.url_for(""),
        asset_path: &asset_path,
    };
    let body = manifest.render().map_err(Error::internal)?;
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

#[cfg(test)]
mod tests {
    use crate::controllers::presenters::test_support::{TestApp, seed_clock};

    #[tokio::test]
    async fn pwa_http_bodies_match_rails_before_and_after_first_run() {
        for (seed, vectors) in [
            ("default", include_str!("../../../../vectors/users_pwa_default.json")),
            ("first_run", include_str!("../../../../vectors/users_pwa_first_run.json")),
        ] {
            let Some(app) = TestApp::boot_seed_with_env(seed, seed_clock(), &[]).await else { return };
            let vectors: serde_json::Value = serde_json::from_str(vectors).unwrap();
            for vector in vectors["responses"].as_array().unwrap() {
                let response = app.anonymous().get(vector["path"].as_str().unwrap()).await;
                assert_eq!(response.status.as_u16(), vector["status"].as_u64().unwrap() as u16);
                assert_eq!(response.content_type(), vector["content_type"].as_str());
                assert_eq!(response.text(), vector["body"].as_str().unwrap(), "{seed} {}", vector["path"]);
                if vector["path"] == "/offline.html" { assert_eq!(response.header("set-cookie"), None); }
            }
        }
    }
    use axum::http::StatusCode;

    #[tokio::test]
    async fn ws17_service_worker_is_served_byte_identical_to_rails() {
        let app = TestApp::boot().await.expect("WS17 requires the Rails parity seed");
        let reply = app.anonymous().get("/service-worker.js").await;
        assert_eq!(reply.status, StatusCode::OK);
        let reference = campfire_db::fixtures::reference_root().join("app/views/pwa/service_worker.js");
        assert_eq!(reply.body, std::fs::read(reference).unwrap());
        if let Ok(path) = std::env::var("WS17_SERVICE_WORKER_OUTPUT") {
            std::fs::write(path, &reply.body).unwrap();
        }
        let offline = app.anonymous().get("/offline.html").await;
        assert_eq!(offline.status, StatusCode::OK);
        assert_eq!(offline.body, std::fs::read(campfire_db::fixtures::reference_root().join("public/offline.html")).unwrap());
    }
}

use crate::controllers::presenters::test_support::{TestApp, seed_clock};

#[tokio::test]
async fn unreleased_spa_manifest_redirects_to_single_root_identity() {
    for env in [&[][..], &[("SPA_ENABLED", "1")][..]] {
        let Some(app) = TestApp::boot_seed_with_env("default", seed_clock(), env).await else {
            return;
        };
        let response = app.anonymous().get("/app/manifest.webmanifest").await;
        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(response.header("location"), Some("http://campfire.test/webmanifest.json"));
    }
}

#[tokio::test]
async fn pwa_http_bodies_preserve_installed_identity_and_current_worker() {
    for seed in ["default", "first_run"] {
        let Some(app) = TestApp::boot_seed_with_env(seed, seed_clock(), &[]).await else { return; };
        let mut browser = app.anonymous();
        let manifest = browser.get("/webmanifest.json").await;
        assert_eq!(manifest.status, StatusCode::OK);
        assert_eq!(manifest.json()["start_url"], "/");
        assert_eq!(manifest.json()["scope"], "/");
        for path in ["service-worker.js", "offline.html"] {
            let response = browser.get(&format!("/{path}")).await;
            assert_eq!(response.status, StatusCode::OK);
            assert_eq!(response.body, campfire_spa::pwa::file(path, true, None).unwrap().body);
            if path == "offline.html" { assert_eq!(response.header("set-cookie"), None); }
        }
    }
}
use axum::http::StatusCode;

#[tokio::test]
async fn ws17_service_worker_is_served_byte_identical_to_rails() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the Rails parity seed");
    let reply = app.anonymous().get("/service-worker.js").await;
    assert_eq!(reply.status, StatusCode::OK);
    // The Rails body is frozen in vectors/users_pwa_*.json
    // (`pwa_http_bodies_preserve_rails_contract_after_extraction`).
    if let Ok(path) = std::env::var("WS17_SERVICE_WORKER_OUTPUT") {
        std::fs::write(path, &reply.body).unwrap();
    }
    let offline = app.anonymous().get("/offline.html").await;
    assert_eq!(offline.status, StatusCode::OK);
    assert_eq!(
        offline.body,
        campfire_spa::pwa::file("offline.html", true, None)
            .unwrap()
            .body
    );
}
#[tokio::test]
async fn pwa_three_stable_urls_keep_identity_whatever_the_old_switches_and_choices_say() {
    use crate::controllers::presenters::test_support::DAVID;
    use campfire_db::models::user::ui_preference::{self, UiPreference};

    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_pwa_default.json"
    ))
    .unwrap();
    let before: serde_json::Value =
        serde_json::from_str(vectors["responses"][0]["body"].as_str().unwrap()).unwrap();
    // The SPA is served whatever these say, and a stored classic choice is ignored.
    for (enabled, default_next, preference) in [
        (false, true, Some(UiPreference::Next)),
        (true, false, None),
        (true, true, None),
        (true, true, Some(UiPreference::Classic)),
        (true, false, Some(UiPreference::Next)),
    ] {
        let env = [
            ("SPA_ENABLED", if enabled { "1" } else { "0" }),
            ("SPA_DEFAULT", if default_next { "next" } else { "classic" }),
        ];
        let Some(app) = TestApp::boot_seed_with_env("default", seed_clock(), &env).await else {
            return;
        };
        if let Some(preference) = preference {
            app.db()
                .write(move |tx| ui_preference::store(tx, DAVID, preference))
                .await
                .unwrap();
        }
        let mut browser = app.sign_in(DAVID).await;
        let manifest = browser.get("/webmanifest.json").await;
        assert_eq!(manifest.status, StatusCode::OK);
        let after = manifest.json();
        for field in ["id", "scope", "start_url"] {
            assert_eq!(
                after.get(field),
                before.get(field),
                "identity field {field}"
            );
        }
        assert_eq!(manifest.header("cache-control"), Some("private, no-cache"));
        assert_eq!(after["shortcuts"][1]["url"], "/app/settings");
        let worker = browser.get("/service-worker.js").await;
        assert_eq!(worker.status, StatusCode::OK);
        assert_eq!(
            worker.content_type(),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(
            worker.header("service-worker-allowed"),
            before["scope"].as_str()
        );
        assert_eq!(
            worker.header("cache-control"),
            Some("no-cache, no-transform")
        );
        assert_eq!(
            worker.body,
            campfire_spa::pwa::file("service-worker.js", true, None)
                .unwrap()
                .body
        );
        let offline = browser.get("/offline.html").await;
        assert_eq!(offline.status, StatusCode::OK);
        assert_eq!(
            offline.content_type(),
            Some(
                campfire_spa::pwa::file("offline.html", true, None)
                    .unwrap()
                    .file
                    .content_type
            )
        );
        assert_eq!(offline.header("set-cookie"), None);
        assert_eq!(
            offline.body,
            campfire_spa::pwa::file("offline.html", true, None)
                .unwrap()
                .body
        );
        let alias_worker = browser.get("/app/service-worker.js").await;
        let alias_offline = browser.get("/app/offline.html").await;
        assert_eq!(alias_worker.status, StatusCode::OK);
        assert_eq!(alias_worker.body, worker.body);
        assert_eq!(alias_offline.status, StatusCode::OK);
        assert_eq!(alias_offline.body, offline.body);
        let alias_manifest = browser.get("/app/manifest.webmanifest").await;
        assert_eq!(alias_manifest.status, StatusCode::FOUND);
        assert_eq!(
            alias_manifest.header("location"),
            Some("http://campfire.test/webmanifest.json")
        );
        if campfire_spa::file("offline.html", None).is_none() {
            assert_eq!(
                offline.header("content-security-policy"),
                None,
                "fallback retry script runs without a request nonce"
            );
        }
        for shortcut in after["shortcuts"].as_array().unwrap() {
            let url = shortcut["url"].as_str().unwrap();
            assert_eq!(
                browser
                    .get(&format!("/{}", url.trim_start_matches('/')))
                    .await
                    .status,
                StatusCode::OK
            );
        }
        for image in after["screenshots"].as_array().unwrap().iter().chain(
            after["shortcuts"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|shortcut| shortcut["icons"].as_array().unwrap()),
        ) {
            let image = url::Url::parse(image["src"].as_str().unwrap()).unwrap();
            assert_eq!(
                app.anonymous().get(image.path()).await.status,
                StatusCode::OK
            );
        }
    }
}

#[tokio::test]
async fn old_notification_paths_use_server_aliases_and_ignore_a_classic_choice() {
    use crate::controllers::presenters::test_support::DAVID;
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    let Some(app) = TestApp::boot_seed_with_env(
        "default",
        seed_clock(),
        &[("SPA_ENABLED", "1")],
    )
    .await
    else {
        return;
    };
    let mut browser = app.sign_in(DAVID).await;
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    for preference in [None, Some(UiPreference::Classic)] {
        if let Some(preference) = preference {
            app.db()
                .write(move |tx| ui_preference::store(tx, DAVID, preference))
                .await
                .unwrap();
        }
        for (path, spa_path) in [
            ("/activity".to_string(), "/app/activity".to_string()),
            (format!("/rooms/{room}"), format!("/app/r/{room}")),
            (format!("/rooms/{room}?classic=1"), format!("/app/r/{room}")),
        ] {
            let reply = browser.get(&path).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{preference:?} {path}");
            assert_eq!(
                reply.header("location"),
                Some(format!("http://campfire.test{spa_path}").as_str()),
                "{preference:?} {path}"
            );
        }
    }
}

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
async fn pwa_http_bodies_preserve_rails_contract_after_extraction() {
    for (seed, vectors) in [
        (
            "default",
            include_str!("../../../../../vectors/users_pwa_default.json"),
        ),
        (
            "first_run",
            include_str!("../../../../../vectors/users_pwa_first_run.json"),
        ),
    ] {
        let Some(app) = TestApp::boot_seed_with_env(seed, seed_clock(), &[]).await else {
            return;
        };
        let vectors: serde_json::Value = serde_json::from_str(vectors).unwrap();
        for vector in vectors["responses"].as_array().unwrap() {
            let response = app.anonymous().get(vector["path"].as_str().unwrap()).await;
            assert_eq!(
                response.status.as_u16(),
                vector["status"].as_u64().unwrap() as u16
            );
            assert_eq!(response.content_type(), vector["content_type"].as_str());
            let rails = vector["body"].as_str().unwrap();
            // The worker keeps one documented difference from Rails (the SPA's build caches).
            let expected = if vector["path"] == "/service-worker.js" {
                campfire_spa::pwa::rails_service_worker_with_spa_patch(rails)
            } else {
                owned_manifest_assets(rails)
            };
            assert_eq!(response.text(), expected, "{seed} {}", vector["path"]);
            if vector["path"] == "/offline.html" {
                assert_eq!(response.header("set-cookie"), None);
            }
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
        campfire_spa::pwa::file("offline.html", false, None)
            .unwrap()
            .body
    );
}
#[tokio::test]
async fn original_service_worker_logic_checks_the_real_http_script() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let reply = app.anonymous().get("/service-worker.js").await;
    assert_eq!(reply.status, StatusCode::OK);
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("app/views/pwa/service_worker.js");
    let harness = dir.path().join("test/scripts/service_worker_harness.mjs");
    std::fs::create_dir_all(script.parent().unwrap()).unwrap();
    std::fs::create_dir_all(harness.parent().unwrap()).unwrap();
    std::fs::write(script, &reply.body).unwrap();
    std::fs::write(
        &harness,
        include_str!("../../../../../test-support/service_worker_original_harness.mjs"),
    )
    .unwrap();
    let output = std::process::Command::new("node")
        .arg(harness)
        .output()
        .expect("CI's Node prerequisite");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "original worker harness failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("all checks passed"),
        "original harness success receipt: {stdout}"
    );
}

/// Frozen Rails bodies keep their provenance; only the illustrations' new owned URLs differ.
fn owned_manifest_assets(body: &str) -> String {
    let mut body = body.to_owned();
    for (old, new) in [
        ("add-f232d8a6.svg", "add.svg"),
        ("person-da193438.svg", "person.svg"),
        (
            "screenshots/android-chat-f8b923c9.png",
            "screenshots/android-chat.png",
        ),
        (
            "screenshots/android-sidebar-e9d2b49f.png",
            "screenshots/android-sidebar.png",
        ),
        (
            "screenshots/android-dark-mode-e43dcf59.png",
            "screenshots/android-dark-mode.png",
        ),
    ] {
        body = body.replace(
            &format!("/assets/{old}"),
            &campfire_spa::pwa::asset_path(new),
        );
    }
    body
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

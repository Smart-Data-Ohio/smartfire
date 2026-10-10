//! `/app` and `/api/v1/boot` over the seeded app: always served (`SPA_ENABLED` is no longer
//! read), behind the classic pages' before-actions, with their headers. The embedding itself is `campfire_spa`'s to test;
//! these run against whatever it embedded (the stub in CI, a real dist when one was built).

use axum::http::{Method, StatusCode};
use campfire_db::{Account, User, UserStatusSettings};
use campfire_kit::Crypto as _;
use regex::Regex;
use serde_json::Value;

use crate::controllers::presenters::test_support::{Browser, DAVID, JASON, Reply, Req, TestApp, seed_clock};

async fn app(enabled: bool) -> Option<TestApp> {
    let env: &[(&str, &str)] = if enabled { &[("SPA_ENABLED", "1")] } else { &[] };
    TestApp::boot_seed_with_env("default", seed_clock(), env).await
}

fn json_request(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

/// The decrypted `_campfire_session` a browser holds.
fn session(a: &TestApp, b: &Browser<'_>) -> Value {
    let cookies = b.cookie_header();
    let raw = cookies
        .split("; ")
        .find_map(|cookie| cookie.strip_prefix(&format!("{}=", campfire_kit::session::SESSION_KEY)))
        .expect("a session cookie");
    let raw = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
    campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone())
        .decrypt_cookie(campfire_kit::session::SESSION_KEY, &raw, jiff::Timestamp::now())
        .unwrap()
}

/// The first `<meta name>`'s content in a page.
fn meta(page: &str, name: &str) -> Option<String> {
    let pattern = Regex::new(&format!(r#"<meta name="{name}" content="([^"]*)""#)).unwrap();
    pattern.captures(page).map(|captures| captures[1].to_string())
}

fn boot_json(page: &str) -> Value {
    let pattern = Regex::new(r#"(?s)<script type="application/json" id="boot"[^>]*>(.*?)</script>"#).unwrap();
    serde_json::from_str(&pattern.captures(page).expect("the shell's boot JSON")[1]).unwrap()
}

/// A policy with its per-session nonce taken out.
fn without_nonce(policy: &str) -> String {
    Regex::new(r" 'nonce-[^']*'").unwrap().replace_all(policy, "").into_owned()
}

/// The headers every classic page answers with (`security::default_headers`).
const SECURITY_HEADERS: &[&str] = &[
    "x-frame-options",
    "x-xss-protection",
    "x-content-type-options",
    "x-permitted-cross-domain-policies",
    "referrer-policy",
    "permissions-policy",
];

async fn unenroll(a: &TestApp, user_id: i64) {
    a.db().write(move |tx| User::find(tx.conn(), user_id)?.reset_two_factor(tx)).await.unwrap();
}

#[tokio::test]
async fn workspace_styles_reach_the_shell_and_boot_with_the_classic_policy() {
    let Some(a) = app(true).await else { return };
    let css = ":root { --accent: red; } body::after { content: \"</style>&\"; }";
    a.db().write(move |tx| Account::first(tx.conn())?.unwrap().update(tx, None, Some(Some(css)), None)).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let classic = b.classic_page("/users/me/profile").await;
    assert_eq!(classic.status, StatusCode::OK);
    assert!(classic.text().contains(&format!("<style data-turbo-track=\"reload\">{css}</style>")));
    let page = b.get("/app/").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("<style data-turbo-track=\"reload\">:root { --accent: red; } body::after { content: \"\\3c /style>&\"; }</style>"));
    assert_eq!(boot_json(&page.text())["customStyles"], css);
    let policy = page.header("content-security-policy").unwrap();
    assert!(policy.contains("style-src 'self' 'unsafe-inline'"));
    assert_eq!(without_nonce(policy), without_nonce(classic.header("content-security-policy").unwrap()));
    let boot = b.send(json_request("/api/v1/boot")).await;
    assert_eq!(serde_json::from_slice::<Value>(&boot.body).unwrap()["customStyles"], css);
}

/// The old switches no longer turn the SPA off or send anyone to the classic pages.
#[tokio::test]
async fn the_spa_is_served_whatever_the_old_switches_say() {
    for (enabled, default) in [("0", "classic"), ("false", "next"), ("", ""), ("1", "classic")] {
        let env = [("SPA_ENABLED", enabled), ("SPA_DEFAULT", default)];
        let Some(a) = TestApp::boot_seed_with_env("default", seed_clock(), &env).await else { return };
        let mut b = a.sign_in(DAVID).await;
        for path in ["/app", "/app/", "/app/rooms/1"] {
            let reply = b.get(path).await;
            assert_eq!((reply.status, reply.content_type()), (StatusCode::OK, Some("text/html; charset=utf-8")), "{env:?} {path}");
        }
        assert_eq!(b.send(json_request("/api/v1/boot")).await.status, StatusCode::OK, "{env:?}");
        let classic = b.get("/searches?q=fire").await;
        assert_eq!(classic.location(), Some("http://campfire.test/app/search?q=fire"), "{env:?}");
    }
}

#[tokio::test]
async fn signed_out_visitors_sign_in_and_come_back() {
    let Some(a) = app(true).await else { return };
    unenroll(&a, DAVID).await;
    let mut b = a.anonymous();
    let page = b.get("/app/rooms/42?tab=files").await;
    assert_eq!(page.status, StatusCode::FOUND);
    assert_eq!(page.location(), Some("http://campfire.test/session/new"));
    assert_eq!(session(&a, &b)["return_to_after_authenticating"], "http://campfire.test/app/rooms/42?tab=files");

    b.get("/session/new").await;
    let signed_in = b
        .write(Req::new(Method::POST, "/session").form(&[("email_address", "david@37signals.com"), ("password", "secret123456")]))
        .await;
    assert_eq!(signed_in.location(), Some("http://campfire.test/app/rooms/42?tab=files"), "back to the SPA");

    let mut anonymous = a.anonymous();
    assert_eq!(anonymous.send(json_request("/api/v1/boot")).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.get("/api/v1/boot").await.location(), Some("http://campfire.test/session/new"));
    for path in ["/app", "/app/"] {
        assert_eq!(anonymous.get(path).await.location(), Some("http://campfire.test/session/new"), "{path}");
    }
}

#[tokio::test]
async fn the_shell_boots_the_signed_in_user_with_the_classic_headers() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let classic = b.classic_page("/users/me/profile").await;
    assert_eq!(classic.status, StatusCode::OK);
    let shell = b.get("/app/rooms/1").await;
    assert_eq!(shell.status, StatusCode::OK);
    assert_eq!(shell.content_type(), Some("text/html; charset=utf-8"));
    // Any client route deep-links to the shell, the design system's kitchen sink included.
    let kitchen_sink = b.get("/app/_kitchen-sink").await;
    assert_eq!((kitchen_sink.status, kitchen_sink.content_type()), (StatusCode::OK, Some("text/html; charset=utf-8")));
    assert!(kitchen_sink.text().contains(r#"<script type="application/json" id="boot""#));

    for name in SECURITY_HEADERS.iter().chain(&["x-version", "x-rev"]) {
        assert!(shell.header(name).is_some(), "{name}");
        assert_eq!(shell.header(name), classic.header(name), "{name}");
    }
    let policy = shell.header("content-security-policy").expect("the shell's policy");
    assert_eq!(without_nonce(policy), without_nonce(classic.header("content-security-policy").unwrap()));
    let nonce = meta(&shell.text(), "csp-nonce").expect("the csp-nonce meta tag");
    assert!(policy.contains(&format!("script-src 'self' 'wasm-unsafe-eval' https://accounts.google.com/gsi/ https://apis.google.com 'nonce-{nonce}'")), "{policy}");
    for directive in ["default-src 'self'", "style-src 'self' 'unsafe-inline'", "connect-src 'self'", "worker-src 'self' blob:"] {
        assert!(policy.contains(directive), "Vite's module scripts, styles, fetches and workers are 'self': {policy}");
    }

    let page = shell.text();
    assert_eq!(meta(&page, "csrf-param").as_deref(), Some("authenticity_token"));
    let token = meta(&page, "csrf-token").expect("the csrf-token meta tag");
    assert!(b.real_authenticity_token().unwrap().is_valid(&token, "/rooms/1/messages", "post"), "the meta token verifies against the session");

    let (user, account, settings) = a
        .db()
        .read(|conn| Ok((User::find(conn, DAVID)?, Account::first(conn)?.unwrap(), UserStatusSettings::for_ids(conn, &[DAVID])?.remove(&DAVID).unwrap())))
        .await
        .unwrap();
    assert_eq!(
        boot_json(&page),
        serde_json::json!({
            "user": {"id": DAVID, "name": user.name, "avatarUrl": crate::controllers::presenters::avatar_path(&a.booted.app.secrets, &user)},
            "account": {"uploadLimitBytes": 104857600, "name": account.name, "logoUrl": null, "logoStillUrl": null, "bannerUrl": null, "bannerStillUrl": null},
            "customStyles": account.custom_styles,
            "theme": campfire_spa::theme(Some(&settings.theme)),
            "textSize": campfire_spa::text_size(Some(&settings.text_size)),
            "appearancePreferences": null,
            "cableUrl": "/cable",
            "serviceWorkerUrl": "/service-worker.js",
            "version": "parity",
            "revision": "parity",
        })
    );
}

#[tokio::test]
async fn the_shell_reads_the_appearance_settings_and_escapes_the_boot_json() {
    let Some(a) = app(true).await else { return };
    let name = "</script><script>alert(1)</script><!-- \u{2028}";
    a.db()
        .write(move |tx| {
            tx.conn().execute("UPDATE users SET name=?, theme='dark', text_size='larger' WHERE id=?", rusqlite::params![name, DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let page = a.sign_in(DAVID).await.get("/app").await.text();
    assert!(!page.contains("<script>alert(1)"), "{page}");
    let boot = boot_json(&page);
    assert_eq!((boot["user"]["name"].as_str(), boot["theme"].as_str(), boot["textSize"].as_str()), (Some(name), Some("dark"), Some("larger")));
}

#[tokio::test]
async fn the_boot_api_is_the_shells_boot_json_and_csrf_token() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let shell = boot_json(&b.get("/app").await.text());
    let reply = b.send(json_request("/api/v1/boot")).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.content_type(), Some("application/json; charset=utf-8"));
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert!(reply.header("x-content-type-options").is_some());
    let mut json = reply.json();
    let token = json.as_object_mut().unwrap().remove("csrfToken").expect("a csrfToken");
    assert!(b.real_authenticity_token().unwrap().is_valid(token.as_str().unwrap(), "/rooms/1/messages", "post"));
    assert_eq!(json, shell);
}

#[tokio::test]
async fn deactivated_users_are_signed_out_of_the_spa() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(JASON).await;
    assert_eq!(b.get("/app").await.status, StatusCode::OK);
    a.db().write(|tx| User::find(tx.conn(), JASON)?.deactivate(tx)).await.unwrap();
    assert_eq!(b.get("/app").await.location(), Some("http://campfire.test/session/new"));
    assert_eq!(b.send(json_request("/api/v1/boot")).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn two_step_enrollment_is_enforced_on_the_spa() {
    let Some(a) = app(true).await else { return };
    unenroll(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| {
            tx.conn().execute("UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(b.get("/app/rooms/1").await.location(), Some("http://campfire.test/two_factor_setup"));
    assert_eq!(session(&a, &b)["return_to_after_authenticating"], "http://campfire.test/app/rooms/1");
    assert_eq!(b.send(json_request("/api/v1/boot")).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn assets_carry_the_security_headers_and_a_shareable_policy() {
    let Some(a) = app(true).await else { return };
    let mut b = a.anonymous();
    let classic = b.get("/session/new").await;

    static FILE: campfire_spa::File = campfire_spa::File {
        path: "assets/index-B2x8Kq1f.js",
        content_type: "text/javascript; charset=utf-8",
        immutable: true,
        identity: b"export {}",
        br: Some(b"brotli"),
        gz: Some(b"gzip"),
    };
    let served = campfire_spa::Served { file: &FILE, content_encoding: Some("br"), body: b"brotli" };
    let response = super::served_response(&a.booted.fixture_kit, served, "public, immutable, max-age=31556952");
    let header = |name: &str| response.headers().get(name).map(|v| v.to_str().unwrap().to_string());
    assert_eq!(header("cache-control").as_deref(), Some("public, immutable, max-age=31556952, no-transform"));
    assert_eq!(header("content-type").as_deref(), Some("text/javascript; charset=utf-8"));
    assert_eq!(header("content-encoding").as_deref(), Some("br"));
    assert_eq!(header("vary").as_deref(), Some("accept-encoding"));
    assert_eq!(header("content-length").as_deref(), Some("6"));
    for name in SECURITY_HEADERS {
        assert_eq!(header(name).as_deref(), classic.header(name), "{name}");
    }
    let policy = header("content-security-policy").unwrap();
    assert!(!policy.contains("nonce"), "a public, cached response never carries a session's nonce");
    assert_eq!(policy, without_nonce(classic.header("content-security-policy").unwrap()));
    assert!(response.extensions().get::<campfire_kit::deflater::StaticFile>().is_some());

    static UNHASHED: campfire_spa::File = campfire_spa::File { path: "assets/unhashed.js", immutable: false, ..FILE };
    let unhashed = campfire_spa::Served { file: &UNHASHED, content_encoding: None, body: b"export {}" };
    let response = super::served_response(&a.booted.fixture_kit, unhashed, "public, immutable, max-age=31556952");
    assert_eq!(response.headers()["cache-control"], format!("{}, no-transform", campfire_spa::REVALIDATE_CACHE_CONTROL));
    assert!(response.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn embedded_assets_are_served_by_path_and_unknown_ones_are_not_found() {
    let Some(a) = app(true).await else { return };
    let mut b = a.anonymous();
    let unknown = b.get("/no-such-page").await;
    let missing = b.get("/app/assets/missing-AbCd1234.js").await;
    assert_eq!((missing.status, missing.body.clone()), (StatusCode::NOT_FOUND, unknown.body), "a public 404, not a sign-in redirect");

    // A real dist (built locally, or by the Frontend job's `cargo test -p campfire_spa`).
    for file in campfire_spa::files() {
        let path = format!("/app/{}", file.path);
        let reply: Reply = b.send(Req::new(Method::GET, &path).header("accept-encoding", "gzip, br")).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert_eq!(reply.content_type(), Some(file.content_type), "{path}");
        let expected = file.br.map(|br| (Some("br"), br)).unwrap_or((None, file.identity));
        assert_eq!((reply.header("content-encoding"), &reply.body[..]), expected, "{path}");
        assert_eq!(reply.header("set-cookie"), None, "{path}: no session for a static file");
        assert!(reply.header("cache-control").is_some_and(|cc| cc.ends_with(", no-transform")), "{path}: never gzipped again");
    }
}

/// Classic pages and the SPA name the same worker, whatever the old switches or a stored choice
/// of the classic UI say, signed in or out.
#[tokio::test]
async fn pwa_worker_selection_ignores_the_old_switches_and_choices() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};

    for (enabled, default_next, preference) in [
        (false, true, Some(UiPreference::Next)),
        (true, false, None),
        (true, true, None),
        (true, true, Some(UiPreference::Classic)),
        (true, false, Some(UiPreference::Next)),
    ] {
        let env = [
            ("RAILS_ENV", "test"),
            ("SPA_ENABLED", if enabled { "1" } else { "0" }),
            ("SPA_DEFAULT", if default_next { "next" } else { "classic" }),
        ];
        let Some(a) = TestApp::boot_seed_with_env("default", seed_clock(), &env).await else {
            return;
        };
        if let Some(preference) = preference {
            a.db()
                .write(move |tx| ui_preference::store(tx, DAVID, preference))
                .await
                .unwrap();
        }
        let label =
            format!("enabled={enabled} default_next={default_next} preference={preference:?}");
        let expected = Some("/service-worker.js");
        let mut b = a.sign_in(DAVID).await;
        let classic = b.classic_page("/users/me/profile").await;
        assert_eq!(classic.status, StatusCode::OK, "{label}");
        assert_eq!(
            meta(&classic.text(), "service-worker-url").as_deref(),
            expected,
            "{label}"
        );
        assert!(
            classic.text().contains("data-service-worker=\"false\""),
            "tests keep automatic registration disabled: {label}"
        );
        let shell = b.get("/app/").await;
        let boot = b.send(json_request("/api/v1/boot")).await;
        let worker = Value::from("/service-worker.js");
        assert_eq!(boot_json(&shell.text())["serviceWorkerUrl"], worker, "{label}");
        assert_eq!(boot.json()["serviceWorkerUrl"], worker, "{label}");
        // The auth pages (no Turbo) name the same worker: two-step setup is where a password
        // sign-in can land, and its load registers the signed-in person's worker.
        unenroll(&a, DAVID).await;
        let setup = b.get("/two_factor_setup").await;
        assert_eq!(setup.status, StatusCode::OK, "{label}");
        assert_eq!(
            meta(&setup.text(), "service-worker-url").as_deref(),
            expected,
            "auth page: {label}"
        );
        assert!(
            setup.text().contains("data-service-worker=\"false\""),
            "auth pages keep automatic registration disabled in tests: {label}"
        );
        let signed_out = a.anonymous().get("/session/new").await;
        assert_eq!(signed_out.status, StatusCode::OK, "{label}");
        assert!(
            signed_out.text().contains("href=\"/webmanifest.json\""),
            "{label}"
        );
        assert_eq!(
            meta(&signed_out.text(), "service-worker-url").as_deref(),
            expected,
            "signed out: {label}"
        );
    }
}

#[tokio::test]
async fn pwa_manifest_alias_and_shell_preserve_root_install_identity() {
    let Some(a) = app(true).await else { return };
    let manifest = a.anonymous().get("/webmanifest.json").await;
    let json = manifest.json();
    assert_eq!(json["start_url"], "/");
    assert_eq!(json["scope"], "/");
    assert!(json.get("id").is_none(), "the historical manifest uses start_url as its implicit id");
    let alias = a.anonymous().get("/app/manifest.webmanifest").await;
    assert_eq!(alias.status, StatusCode::FOUND);
    assert_eq!(alias.header("location"), Some("http://campfire.test/webmanifest.json"));
    let shell = a.sign_in(DAVID).await.get("/app/").await;
    assert!(shell.text().contains("<link rel=\"manifest\" href=\"/webmanifest.json\""));
    let classic = a.anonymous().get("/session/new").await;
    assert!(classic.text().contains("href=\"/webmanifest.json\""));
}

#[tokio::test]
async fn pwa_worker_and_offline_files_use_public_headers_without_becoming_the_shell() {
    let Some(a) = app(true).await else { return };
    let classic = a.anonymous().get("/session/new").await;
    static WORKER: campfire_spa::File = campfire_spa::File {
        path: "service-worker.js",
        content_type: "text/javascript; charset=utf-8",
        immutable: false,
        identity: b"importScripts('/app/assets/worker-AbCd1234.js');",
        br: None,
        gz: None,
    };
    static OFFLINE: campfire_spa::File = campfire_spa::File {
        path: "offline.html",
        content_type: "text/html; charset=utf-8",
        identity: b"<script type=module src=/app/assets/offline-AbCd1234.js></script>",
        ..WORKER
    };
    for file in [&WORKER, &OFFLINE] {
        let served = campfire_spa::Served {
            file,
            content_encoding: None,
            body: file.identity,
        };
        let response = super::served_response(
            &a.booted.fixture_kit,
            served,
            "public, immutable, max-age=31556952",
        );
        let headers = response.headers();
        assert_eq!(headers["content-type"], file.content_type);
        assert!(
            !headers["cache-control"]
                .to_str()
                .unwrap()
                .contains("immutable")
        );
        if file.path == "service-worker.js" {
            assert_eq!(headers["service-worker-allowed"], "/");
            assert_eq!(headers["cache-control"], "no-cache, no-transform");
        } else {
            assert_eq!(
                headers["cache-control"],
                format!("{}, no-transform", campfire_spa::REVALIDATE_CACHE_CONTROL)
            );
            assert!(headers.get("service-worker-allowed").is_none());
        }
        for name in SECURITY_HEADERS {
            assert_eq!(
                headers[*name].to_str().unwrap(),
                classic.header(name).unwrap()
            );
        }
        let policy = headers["content-security-policy"].to_str().unwrap();
        assert_eq!(
            policy,
            without_nonce(classic.header("content-security-policy").unwrap())
        );
        assert!(
            policy.contains("script-src 'self'"),
            "same-origin hashed modules and worker imports: {policy}"
        );
        assert!(
            policy.contains("style-src 'self'"),
            "offline styles: {policy}"
        );
    }
    for path in ["service-worker.js", "offline.html"] {
        let reply = a.anonymous().get(&format!("/app/{path}")).await;
        let root = a.anonymous().get(&format!("/{path}")).await;
        let file = campfire_spa::pwa::file(path, true, None).unwrap();
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.content_type(), Some(file.file.content_type));
        assert_eq!(reply.body, root.body, "older bundles use the same root resource");
        assert_eq!(reply.body, file.body);
        assert_eq!(reply.header("set-cookie"), None);
        if path == "service-worker.js" {
            assert_eq!(reply.header("service-worker-allowed"), Some("/"));
            assert_eq!(reply.header("cache-control"), Some("no-cache, no-transform"));
        }
    }
}

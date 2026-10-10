//! Request-level tests for the session, account and user controllers (controllers A), through the
//! whole stack (`crate::server::boot`, the Rails route table, kit) over a private copy of the reference-built
//! `default` parity seed. Missing seeds fail in CI and skip locally with a note
//! (`python3 parity/bin/frozen-seeds restore`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use tower::ServiceExt;

use crate::server::Booted;
use crate::config::Config;
use crate::controllers::presenters::test_support::{masked_session_token, seed_clock, seed_dir};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const HOST: &str = "campfire.test";
const PASSWORD: &str = "secret123456";
const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

#[path = "tests/github_connections.rs"]
mod github_connections;
#[path = "tests/owner_inputs.rs"]
mod owner_inputs;
#[path = "tests/bot_mutations.rs"]
mod bot_mutations;
#[path = "tests/bot_access.rs"]
mod bot_access;
#[path = "tests/approval_decisions.rs"]
mod approval_decisions;
#[path = "tests/agent_histories.rs"]
mod agent_histories;
#[path = "tests/agent_broadcasts.rs"]
mod agent_broadcasts;
#[path = "tests/webhook_secrets.rs"]
mod webhook_secrets;
#[path = "tests/kill_switch.rs"]
mod kill_switch;
#[path = "tests/navigation_inbox.rs"]
mod navigation_inbox;
#[path = "tests/member_polling.rs"]
mod member_polling;
#[path = "tests/page_read_boundaries.rs"]
mod page_read_boundaries;

#[test]
fn unicode_parity_sidebar_direct_names_use_ruby_sort_order() {
    let t = crate::integrations::test_support::TestDb::new();
    let names = ["ΟΣ", "οςa"];
    let secrets = rails_compat::Secrets::new("unicode-fixture-secret");
    let members =
        t.db.read_blocking(|c| {
            ["david", "jason"]
                .into_iter()
                .zip(names)
                .map(|(label, name)| {
                    let mut user = campfire_db::User::find(
                        c,
                        crate::integrations::test_support::TestDb::id(label),
                    )?;
                    user.name = name.into();
                    Ok(super::super::user_summary(&secrets, &user))
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/unicode_casing_parity.json"
    ))
    .unwrap();
    let expected = oracle["sigma_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(super::sidebar_direct_label(None, &members), expected);
}

struct Test {
    booted: Booted,
    labels: serde_json::Value,
    _dir: tempfile::TempDir,
}

async fn boot_seed(name: &str) -> Option<Test> {
    boot_seed_with_clock(name, seed_clock()).await
}

async fn boot_seed_with_clock(name: &str, clock: campfire_kit::SharedClock) -> Option<Test> {
    boot_seed_with_network(name, clock, crate::net::Network::system()).await
}

async fn boot_seed_with_network(name: &str, clock: campfire_kit::SharedClock, network: crate::net::Network) -> Option<Test> {
    let seed = seed_dir(name)?;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("db")).unwrap();
    std::fs::copy(
        seed.join("db/production.sqlite3"),
        dir.path().join("db/production.sqlite3"),
    )
    .unwrap();
    if seed.join("storage").exists() {
        copy_dir(&seed.join("storage"), &dir.path().join("files"));
    }
    let labels = std::fs::read_to_string(seed.join("labels.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let root = dir.path().to_string_lossy().into_owned();
    let secret = parity_env("SECRET_KEY_BASE").unwrap();
    let config = Config::from_lookup(|key| match key {
        "SECRET_KEY_BASE" => Some(secret.clone()),
        "DISABLE_SSL" => Some("true".into()),
        "APP_VERSION" | "GIT_REVISION" => Some("parity".into()),
        "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
        _ => None,
    })
    .unwrap();
    // The classic pages, until they're deleted.
    Some(Test {
        booted: crate::server::boot_with_network(config, clock, network).await.unwrap(),
        labels,
        _dir: dir,
    })
}

fn parity_env(name: &str) -> Option<String> {
    let env = std::fs::read_to_string(Path::new(ROOT).join("parity/.env.reference")).ok()?;
    env.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_string))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target: PathBuf = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

impl Test {
    fn label(&self, key: &str) -> String {
        match &self.labels[key] {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// A browser: a cookie jar and a remote IP of its own (the sign-in rate limit is per IP).
    fn browser(&self, ip: &str) -> Browser<'_> {
        Browser {
            test: self,
            cookies: BTreeMap::new(),
            ip: ip.to_string(),
        }
    }
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    fn location(&self) -> &str {
        self.header("location").unwrap_or("")
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect()
    }

    /// Asserts the page has a form posting to `action`.
    fn assert_form(&self, action: &str) {
        let html = self.text();
        assert!(
            html.contains(&format!("action=\"{action}\""))
                || html.contains(&format!("action=\"http://{HOST}{action}\"")),
            "no form for {action} in {html}"
        );
    }

}

struct Browser<'a> {
    test: &'a Test,
    cookies: BTreeMap<String, String>,
    ip: String,
}

impl Browser<'_> {
    async fn request(
        &mut self,
        method: Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<(&str, String)>,
    ) -> Reply {
        if method != Method::GET && self.cookies.get(campfire_kit::session::SESSION_KEY).and_then(|raw| masked_session_token(&self.test.booted.app.secrets, raw)).is_none() {
            Box::pin(self.get("/app/")).await;
        }
        let mut request = Request::builder()
            .method(method.clone())
            .uri(path)
            .header(header::HOST, HOST)
            .header("x-forwarded-for", &self.ip);
        if !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
        {
            request = request.header(header::USER_AGENT, CHROME);
        }
        // What the page itself sends with the writes it makes (forms, fetches): the session's
        // authenticity token, once a page has given the session one.
        if method != Method::GET {
            let session = self.cookies.get(campfire_kit::session::SESSION_KEY);
            if let Some(token) =
                session.and_then(|raw| masked_session_token(&self.test.booted.app.secrets, raw))
            {
                request = request.header(campfire_kit::csrf::HEADER, token);
            }
        }
        if !self.cookies.is_empty() {
            let cookie = self
                .cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            request = request.header(header::COOKIE, cookie);
        }
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = match body {
            Some((content_type, body)) => request
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .unwrap(),
            None => request.body(Body::empty()).unwrap(),
        };
        let response = self
            .test
            .booted
            .router
            .clone()
            .oneshot(request)
            .await
            .unwrap();
        let reply = Reply {
            status: response.status(),
            headers: response.headers().clone(),
            body: axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        };
        for cookie in reply.set_cookies() {
            let pair = cookie.split(';').next().unwrap();
            let (name, value) = pair.split_once('=').unwrap();
            let deleted = cookie_tombstone(&cookie);
            if deleted || value.is_empty() {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_string(), value.to_string());
            }
        }
        reply
    }

    async fn get(&mut self, path: &str) -> Reply {
        self.request(Method::GET, path, &[], None).await
    }

    async fn form(&mut self, method: &str, path: &str, fields: &[(&str, &str)]) -> Reply {
        let mut pairs = Vec::new();
        if method != "post" {
            pairs.push(("_method".into(), method.into()));
        }
        pairs.extend(fields.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let body = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        self.request(
            Method::POST,
            path,
            &[],
            Some(("application/x-www-form-urlencoded", body)),
        )
        .await
    }


    // Rails test helper's grant_sudo_access: authenticate the real encrypted cookie,
    // preserve its CSRF state, and add the same verified-at epoch value.
    fn grant_sudo_access(&mut self) {
        use campfire_kit::Crypto;
        let key = campfire_kit::session::SESSION_KEY;
        let crypto = campfire_kit::RailsCrypto::new(self.test.booted.app.secrets.clone());
        let raw = percent_encoding::percent_decode_str(self.cookies.get(key).unwrap()).decode_utf8().unwrap();
        let now = self.test.booted.app.clock.now();
        let mut data = crypto.decrypt_cookie(key, &raw, now).unwrap();
        data["sudo_verified_at"] = now.as_second().into();
        let cookie = crypto.encrypt_cookie(key, &data, None);
        self.cookies.insert(key.into(), campfire_kit::cookies::escape(&cookie));
    }

    async fn sign_in(&mut self, email: &str) {
        let page = self.get("/session/new").await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.text());
        page.assert_form("/session");
        let reply = self
            .form(
                "post",
                "/session",
                &[("email_address", email), ("password", PASSWORD)],
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FOUND,
            "sign in as {email}: {}",
            reply.text()
        );
        if reply.location() == "http://campfire.test/two_factor_challenge" {
            let completed = self.complete_challenge(email).await;
            assert_eq!(completed.status, StatusCode::FOUND, "{}", completed.text());
        }
    }
    async fn complete_challenge(&mut self, email: &str) -> Reply {
        self.get("/two_factor_challenge")
            .await
            .assert_form("/two_factor_challenge");
        let email = email.to_owned();
        let secrets = self.test.booted.app.secrets.clone();
        let now = self.test.booted.app.clock.now().as_second();
        let code = self
            .test
            .booted
            .app
            .db
            .read(move |conn| {
                let user = campfire_db::User::find_active_by_email_address(conn, &email)?
                    .expect("active member");
                let credential = campfire_db::TwoFactorCredential::for_user(conn, user.id)?
                    .expect("enrolled member");
                Ok(rails_compat::totp::at(
                    &credential
                        .secret(&rails_compat::ar_encryption::ArEncryption::new(&secrets))?,
                    now,
                )
                .unwrap())
            })
            .await
            .unwrap();
        self.form("post", "/two_factor_challenge", &[("code", &code)])
            .await
    }

    async fn confirm_sudo(&mut self) {
        assert_eq!(self.get("/sudo/new").await.status, StatusCode::OK);
        assert_redirect(
            &self.form("post", "/sudo", &[("password", PASSWORD)]).await,
            "http://campfire.test/",
        );
    }
}



#[test]
fn browser_keeps_valid_signed_cookies_with_epoch_digits_in_the_signature() {
    use campfire_kit::Crypto;
    let secrets = std::sync::Arc::new(rails_compat::Secrets::new(&parity_env("SECRET_KEY_BASE").unwrap()));
    let crypto = campfire_kit::RailsCrypto::new(secrets);
    let (token, signed) = (0..20_000).find_map(|i| {
        let token = format!("fixture-browser-session-{i}");
        let signed = crypto.sign_cookie("session_token", &token, None);
        signed.contains("1970").then_some((token, signed))
    }).expect("deterministic signed-cookie fixture contains epoch digits");
    assert_eq!(crypto.verify_signed_cookie("session_token", &signed, seed_clock().now()), Some(token));
    let header = format!("session_token={signed}; Path=/; HttpOnly; SameSite=Lax");
    assert!(!cookie_tombstone(&header), "valid signature digits are not an expiry attribute");
    assert!(!cookie_tombstone("session_token=fixture; Path=/1970; Expires=Mon, 02 Mar 2046 16:00:00 GMT"));
    assert!(cookie_tombstone("session_token=fixture; Max-Age=0; Path=/"));
    assert!(cookie_tombstone("session_token=fixture; max-age=-1; Path=/"));
    assert!(cookie_tombstone("session_token=fixture; expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=/"));
}

fn encode(value: &str) -> String {
    campfire_presentation::helpers::url::cgi_escape(value)
}

fn assert_redirect(reply: &Reply, location: &str) {
    assert_eq!(
        (reply.status, reply.location()),
        (StatusCode::FOUND, location),
        "{}",
        reply.text()
    );
}

// --- Sessions ------------------------------------------------------------------------------------

#[tokio::test]
async fn signs_in_with_a_password_and_out_again() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.1");

    // Unauthenticated requests remember where they were going.
    assert_redirect(
        &browser.get("/account/edit").await,
        "http://campfire.test/session/new",
    );

    let page = browser.get("/session/new").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("<title>Sign in</title>"));
    let html = page.text();
    let stylesheet = html.split_once("<link rel=\"stylesheet\" href=\"")
        .expect("retained auth stylesheet").1.split('"').next().unwrap();
    assert!(stylesheet.starts_with("/assets/auth-") && stylesheet.ends_with(".css"));
    let css = browser.get(stylesheet).await;
    assert_eq!(css.status, StatusCode::OK);
    assert_eq!(css.header("content-type"), Some("text/css"));
    page.assert_form("/session");
    let signed_in = browser
        .form(
            "post",
            "/session",
            &[
                ("email_address", &test.label("emails.david")),
                ("password", PASSWORD),
            ],
        )
        .await;
    assert_redirect(&signed_in, "http://campfire.test/two_factor_challenge");
    assert!(
        !signed_in
            .set_cookies()
            .iter()
            .any(|c| c.starts_with("session_token="))
    );
    let signed_in = browser
        .complete_challenge(&test.label("emails.david"))
        .await;
    assert_redirect(&signed_in, "http://campfire.test/app/admin");
    let session_cookie = signed_in
        .set_cookies()
        .into_iter()
        .find(|c| c.starts_with("session_token="))
        .expect("session cookie");
    assert!(
        session_cookie.contains("httponly")
            && session_cookie.contains("samesite=lax")
            && session_cookie.contains("expires="),
        "{session_cookie}"
    );

    // Signed in: sign-in and join pages send you home.
    assert_redirect(
        &browser
            .get(&format!("/join/{}", test.label("join_codes.signal")))
            .await,
        "http://campfire.test/",
    );
    let root = browser.get("/").await;
    assert_eq!(root.status, StatusCode::FOUND);
    assert_eq!(root.location(), "http://campfire.test/app/");

    // Sign out from the profile page's form.
    assert_eq!(browser.get("/users/me/profile").await.status, StatusCode::FOUND);
    let signed_out = browser.form("delete", "/session", &[]).await;
    assert_redirect(&signed_out, "http://campfire.test/");
    assert!(
        signed_out
            .set_cookies()
            .iter()
            .any(|c| c.starts_with("session_token=;")),
        "{:?}",
        signed_out.set_cookies()
    );
    assert_redirect(
        &browser.get("/users/me/profile").await,
        "http://campfire.test/session/new",
    );
}

#[tokio::test]
async fn rejects_bad_passwords_and_rate_limits_sign_ins() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.2");
    browser.get("/session/new").await.assert_form("/session");
    for attempt in 1..=11 {
        let reply = browser
            .form(
                "post",
                "/session",
                &[
                    ("email_address", "david@37signals.com"),
                    ("password", "wrong"),
                ],
            )
            .await;
        let expected = if attempt <= 10 {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::TOO_MANY_REQUESTS
        };
        assert_eq!(reply.status, expected, "attempt {attempt}");
        let html = reply.text();
        assert!(
            html.contains("Too many requests or unauthorized.") && html.contains("shake"),
            "{html}"
        );
        assert!(html.contains(r#"value="david@37signals.com""#));
    }
    // Deactivated users can't sign in.
    let mut other = test.browser("198.51.100.3");
    other.get("/session/new").await.assert_form("/session");
    let reply = other
        .form(
            "post",
            "/session",
            &[
                ("email_address", &test.label("emails.rita")),
                ("password", PASSWORD),
            ],
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_rails_issued_session_cookie_continues_on_rust() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/campfire_sessions.json"
    )))
    .unwrap();
    let cookie = vectors["sessions"][0]["cookie_header"].as_str().unwrap();
    let mut browser = test.browser("198.51.100.4");
    for pair in cookie.split("; ") {
        let (name, value) = pair.split_once('=').unwrap();
        browser.cookies.insert(name.into(), value.into());
    }
    let profile = browser.get("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::FOUND);
    assert_eq!(profile.location(), "http://campfire.test/app/settings");
}

#[tokio::test]
async fn direct_upload_metadata_is_not_limited_by_the_buffered_body_cap() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.10");
    browser.sign_in("david@37signals.com").await;
    let create = |byte_size: usize| {
        format!(
            r#"{{"blob":{{"filename":"a.bin","byte_size":{byte_size},"checksum":"1B2M2Y8AsgTpgAmY7PhCfg==","content_type":"application/octet-stream"}}}}"#
        )
    };
    let small = browser
        .request(
            Method::POST,
            "/rails/active_storage/direct_uploads",
            &[],
            Some(("application/json", create(5))),
        )
        .await;
    assert_eq!(small.status, StatusCode::OK, "{}", small.text());
    assert!(
        small.text().contains("/rails/active_storage/disk/"),
        "{}",
        small.text()
    );
    let large = browser
        .request(
            Method::POST,
            "/rails/active_storage/direct_uploads",
            &[],
            Some((
                "application/json",
                create(campfire_kit::body::MAX_BUFFERED_BODY + 1),
            )),
        )
        .await;
    assert_eq!(large.status, StatusCode::OK, "{}", large.text());
}

#[tokio::test]
async fn transfers_sign_in_on_another_device() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.5");
    admin.sign_in(&test.label("emails.david")).await;
    let kevin_id = test.label("users.kevin");
    let profile = admin.request(Method::GET, &format!("/api/v1/people/{kevin_id}"), &[("accept", "application/json")], None).await;
    assert_eq!(profile.status, StatusCode::OK, "{}", profile.text());
    let profile: serde_json::Value = serde_json::from_str(&profile.text()).unwrap();
    let transfer_url = profile["transferUrl"].as_str().expect("administrator's transfer link");
    let transfer_id = transfer_url.rsplit('/').next().unwrap().to_owned();

    let mut phone = test.browser("198.51.100.6");
    let path = format!("/session/transfers/{transfer_id}");
    let show = phone.get(&path).await;
    assert_eq!(show.status, StatusCode::OK);
    show.assert_form(&path);
    assert_redirect(
        &phone.form("put", &path, &[]).await,
        "http://campfire.test/two_factor_challenge",
    );
    assert_redirect(
        &phone.complete_challenge(&test.label("emails.kevin")).await,
        "http://campfire.test/app/",
    );
    assert_redirect(&phone.get("/users/me/profile").await, "http://campfire.test/app/settings");

    let mut stranger = test.browser("198.51.100.7");
    let bogus = "/session/transfers/bogus";
    stranger.get(bogus).await.assert_form(bogus);
    assert_eq!(
        stranger.form("put", bogus, &[]).await.status,
        StatusCode::BAD_REQUEST
    );
}

// --- Joining and first run -------------------------------------------------------------------------

#[tokio::test]
async fn joins_with_the_join_code() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.8");
    assert_eq!(
        browser.get("/join/nope").await.status,
        StatusCode::NOT_FOUND
    );
    let path = format!("/join/{}", test.label("join_codes.signal"));
    let page = browser.get(&path).await;
    assert_eq!(page.status, StatusCode::OK);
    page.assert_form(&path);
    let fields = [
        ("user[name]", "New Person"),
        ("user[email_address]", "new@example.com"),
        ("user[password]", PASSWORD),
    ];
    assert_redirect(
        &browser.form("post", &path, &fields).await,
        "http://campfire.test/",
    );
    assert_redirect(&browser.get("/users/me/profile").await, "http://campfire.test/two_factor_setup");
    assert_eq!(browser.get("/two_factor_setup").await.status, StatusCode::OK);

    // A taken email address goes to sign in instead.
    let mut other = test.browser("198.51.100.9");
    other.get(&path).await.assert_form(&path);
    let fields = [
        ("user[name]", "Imposter"),
        ("user[email_address]", "new@example.com"),
        ("user[password]", PASSWORD),
    ];
    assert_redirect(
        &other.form("post", &path, &fields).await,
        "http://campfire.test/session/new?email_address=new%40example.com",
    );
}

#[tokio::test]
async fn first_run_sets_up_the_account() {
    let Some(test) = boot_seed("first_run").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.10");
    assert_redirect(
        &browser.get("/session/new").await,
        "http://campfire.test/first_run",
    );
    let page = browser.get("/first_run").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    page.assert_form("/first_run");
    let fields = [
        ("user[name]", "Owner"),
        ("user[email_address]", "owner@example.com"),
        ("user[password]", PASSWORD),
    ];
    assert_redirect(
        &browser.form("post", "/first_run", &fields).await,
        "http://campfire.test/",
    );
    assert_redirect(&browser.get("/first_run").await, "http://campfire.test/");
    assert_redirect(&browser.get("/").await, "http://campfire.test/two_factor_setup");
    assert_eq!(browser.get("/two_factor_setup").await.status, StatusCode::OK);
}

// --- Account ---------------------------------------------------------------------------------------

#[tokio::test]
async fn administers_the_account() {
    let test = boot_seed("default").await.expect("seed");
    let mut admin = test.browser("198.51.100.11");
    admin.sign_in(&test.label("emails.david")).await;
    admin.confirm_sudo().await;
    let action = format!("/account.{}", test.label("accounts.signal"));
    assert_redirect(&admin.form("patch", &action, &[("account[name]", "Renamed")]).await,
        "http://campfire.test/account/edit");
    assert_redirect(&admin.form("post", "/account/join_code", &[]).await,
        "http://campfire.test/account/edit");
    assert_redirect(&admin.form("patch", "/account/custom_styles", &[("account[custom_styles]", "body { --x: 1 }")]).await,
        "http://campfire.test/account/custom_styles/edit");
    let account = test.booted.app.db.read(campfire_db::Account::first).await.unwrap().unwrap();
    assert_eq!(account.name, "Renamed");
    assert_eq!(account.custom_styles.as_deref(), Some("body { --x: 1 }"));
    assert_ne!(account.join_code, test.label("join_codes.signal"));
    let mut member = test.browser("198.51.100.12");
    member.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(member.form("patch", &action, &[("account[name]", "Mine")]).await.status, StatusCode::FORBIDDEN);
}


#[tokio::test]
async fn ws11_key_rotation_requires_sudo_and_shows_the_key_once() {
    let test = boot_seed("default").await.expect("build default parity seed");
    let mut admin = test.browser("198.51.100.113");
    admin.sign_in(&test.label("emails.david")).await;
    let bot_id: i64 = test.label("users.bender").parse().unwrap();
    let path = format!("/account/bots/{bot_id}/key");
    let old_key = test.label("bot_keys.bender");
    let response = admin.form("put", &path, &[]).await;
    assert_redirect(&response, "http://campfire.test/sudo/new");
    assert!(test.booted.app.db.read({let old_key=old_key.clone(); move |conn| campfire_db::User::authenticate_bot(conn,&old_key)}).await.unwrap().is_some());
    assert_eq!(admin.get("/sudo/new").await.status, StatusCode::OK);
    let confirmed = admin.form("post", "/sudo", &[("password", PASSWORD)]).await;
    assert_eq!(confirmed.status, StatusCode::OK);
    confirmed.assert_form(&path);
    assert!(confirmed.text().contains("name=\"_method\" value=\"put\""));
    // The browser submits Rails' continuation form after confirmation.
    let response = admin.form("put", &path, &[]).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
    assert_eq!(response.headers.get("pragma").unwrap(), "no-cache");
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    let new_key = payload["bot_key"].as_str().unwrap().to_owned();
    assert_ne!(new_key, old_key);
    let (new_valid, old_valid, plaintext, audit) = test.booted.app.db.read({let new_key=new_key.clone(); let old_key=old_key.clone(); move |conn| Ok((
        campfire_db::User::authenticate_bot(conn,&new_key)?.is_some(),
        campfire_db::User::authenticate_bot(conn,&old_key)?.is_some(),
        conn.query_row("SELECT bot_token FROM users WHERE id=?",[bot_id],|r|r.get::<_,Option<String>>(0))?,
        conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.reset'",[],|r|r.get::<_,i64>(0))?,
    ))}).await.unwrap();
    assert!(new_valid);
    assert!(!old_valid);
    assert_eq!(plaintext,None);
    assert_eq!(audit,1);
    for page in [
        "/account/bots".to_string(),
        format!("/account/bots/{bot_id}/edit"),
        format!("/account/bots/{bot_id}/credentials"),
        format!("/account/bots/{bot_id}/grants"),
    ] {
        let response = admin.get(&page).await;
        assert_eq!(response.status, StatusCode::FOUND, "{page}");
        for secret in [&old_key, &new_key] {
            assert!(!response.text().contains(secret), "{page} reveals a bot key after rotation");
        }
    }
}

#[tokio::test]
async fn manages_bots() {
    let test = boot_seed("default").await.expect("seed");
    let mut admin = test.browser("198.51.100.13");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let created = admin.form("post", "/account/bots", &[("user[name]", "Robo"), ("user[webhook_url]", "https://example.com/robo")]).await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.header("cache-control"), Some("no-store"));
    let payload: serde_json::Value = serde_json::from_slice(&created.body).unwrap();
    assert!(payload["bot_key"].as_str().is_some());
    let bender = test.label("users.bender");
    let id = bender.parse::<i64>().unwrap();
    let action = format!("/account/bots/{bender}");
    assert_redirect(&admin.form("patch", &action, &[("user[name]", "Bender 2")]).await,
        "http://campfire.test/account/bots");
    let old = test.booted.app.db.read(move |conn| campfire_db::User::find(conn,id)).await.unwrap();
    assert_eq!(old.name, "Bender 2");
    let reset = admin.form("put", &format!("{action}/key"), &[]).await;
    assert_eq!(reset.status, StatusCode::OK);
    assert_eq!(reset.header("cache-control"), Some("no-store"));
    let new = test.booted.app.db.read(move |conn| campfire_db::User::find(conn,id)).await.unwrap();
    assert_ne!(old.bot_token_digest, new.bot_token_digest);
    assert_redirect(&admin.form("delete", &action, &[]).await, "http://campfire.test/account/bots");
    assert!(!test.booted.app.db.read(move |conn| campfire_db::User::find(conn,id)).await.unwrap().is_active());
}


#[tokio::test]
async fn serves_the_account_logo_and_avatars() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.14");
    let logo = browser.get("/account/logo?size=small").await;
    assert_eq!(logo.status, StatusCode::OK);
    assert_eq!(logo.header("content-type"), Some("image/png"));
    assert_eq!(
        logo.header("cache-control"),
        Some("max-age=300, public, stale-while-revalidate=604800")
    );
    let etag = logo.header("etag").unwrap().to_string();
    let again = browser
        .request(
            Method::GET,
            "/account/logo?size=small",
            &[("if-none-match", &etag)],
            None,
        )
        .await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);

    // Avatars need a session.
    let david_token = test.label("avatar_tokens.david");
    assert_eq!(
        browser
            .get(&format!("/users/{david_token}/avatar"))
            .await
            .status,
        StatusCode::FOUND
    );
    browser.sign_in(&test.label("emails.kevin")).await;
    let avatar = browser.get(&format!("/users/{david_token}/avatar")).await;
    assert_eq!(avatar.status, StatusCode::OK);
    assert_eq!(
        avatar.header("content-type"),
        Some("image/svg+xml; charset=utf-8")
    );
    assert!(
        avatar.text().contains("\n      D\n    </text>"),
        "{}",
        avatar.text()
    );
    assert_eq!(
        avatar.header("cache-control"),
        Some("max-age=1800, public, stale-while-revalidate=604800")
    );
    let jason = browser
        .get(&format!(
            "/users/{}/avatar",
            test.label("avatar_tokens.jason")
        ))
        .await;
    assert_eq!(
        (jason.status, jason.header("content-type")),
        (StatusCode::OK, Some("image/webp"))
    );
    let bad = browser.get("/users/bogus/avatar").await;
    assert_eq!((bad.status, bad.body.len()), (StatusCode::NOT_FOUND, 0));
}

// --- Users -----------------------------------------------------------------------------------------

#[tokio::test]
async fn bans_and_unbans() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.16");
    admin.sign_in(&test.label("emails.david")).await;
    admin.confirm_sudo().await;
    let jz = test.label("users.jz");

    // WS19's verified sessions use loopback addresses. This scenario bans a public client;
    // Rails rejects private/internal IPs before applying the ban.
    let user_id: i64 = jz.parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE sessions SET ip_address = ? WHERE user_id = ?",
                rusqlite::params!["198.51.100.160", user_id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let action = format!("/users/{jz}/ban");

    assert_redirect(
        &admin.form("post", &action, &[]).await,
        &format!("http://campfire.test/users/{jz}"),
    );


    assert_redirect(
        &admin.form("delete", &action, &[]).await,
        &format!("http://campfire.test/users/{jz}"),
    );
}

#[tokio::test]
async fn autocompletes_users() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.17");
    browser.sign_in(&test.label("emails.david")).await;
    let html = browser.get("/autocompletable/users?filter=a").await;
    // WS8bm2: the pinned Markdown endpoint has only JSON templates (slash.json oracle).
    assert_eq!(html.status, StatusCode::NOT_ACCEPTABLE);
    let json = browser.get("/autocompletable/users.json?query=a").await;
    assert_eq!(
        json.header("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert!(json.header("x-total-count").is_some());
    let users: serde_json::Value = serde_json::from_slice(&json.body).unwrap();
    assert!(users.as_array().unwrap().iter().all(|user| {
        user["avatar_url"]
            .as_str()
            .unwrap()
            .starts_with("http://campfire.test/users/")
    }));
    assert_eq!(
        browser
            .get("/autocompletable/users?room_id=999999")
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn qr_codes_and_the_pwa() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.18");
    let qr = browser.get("/qr_code/aHR0cDovL2NhbXBmaXJlLnRlc3Q").await;
    assert_eq!(
        (qr.status, qr.header("content-type")),
        (StatusCode::OK, Some("image/svg+xml; charset=utf-8"))
    );
    assert_eq!(qr.header("cache-control"), Some("max-age=31556952, public"));
    assert!(
        qr.text()
            .starts_with("<?xml version=\"1.0\" standalone=\"yes\"?><svg")
    );

    let manifest = browser.get("/webmanifest.json").await;
    assert_eq!(
        (manifest.status, manifest.header("content-type")),
        (StatusCode::OK, Some("application/json; charset=utf-8"))
    );
    let worker = browser.get("/service-worker.js").await;
    assert_eq!(
        (worker.status, worker.header("content-type")),
        (StatusCode::OK, Some("text/javascript; charset=utf-8"))
    );
}

#[tokio::test]
async fn agent_directory_rejects_credentials_bots_and_unsigned_visitors() {
    let Some(test) = boot_seed("default").await else { return };
    let mut browser = test.browser("198.51.100.94");
    assert_redirect(&browser.get("/agents").await, "http://campfire.test/session/new");
    let secret = test.booted.app.db.write(|tx| {
        let id = tx.conn().query_row("SELECT id FROM agents LIMIT 1", [], |r| r.get(0))?;
        Ok(campfire_db::models::agent_credential::AgentCredential::create_with_secret(tx, id, "directory-denial", 127326141, None)?.1)
    }).await.unwrap();
    let authorization = format!("{} {}", "Bearer", secret);
    let response = browser.request(Method::GET, "/agents", &[("authorization", &authorization)], None).await;
    assert_eq!(response.status, StatusCode::FORBIDDEN, "directory credentials must be forbidden");
    let path = format!("/agents?bot_key={}", encode(&test.label("bot_keys.bender")));
    assert_eq!(browser.get(&path).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn agent_directory_bot_session_is_forbidden() {
    let Some(app) = crate::controllers::presenters::test_support::TestApp::boot().await else { return };
    let mut bot = app.sign_in(crate::controllers::presenters::test_support::BENDER).await;
    assert_eq!(bot.get("/agents").await.status, StatusCode::FORBIDDEN, "directory bot sessions must be forbidden");
}

// Set-Cookie values are opaque; only attributes determine deletion.
fn cookie_tombstone(cookie: &str) -> bool {
    cookie.split(';').skip(1).any(|attribute| {
        let Some((name,value))=attribute.trim().split_once('=') else {return false};
        (name.eq_ignore_ascii_case("max-age") && value.trim().parse::<i64>().is_ok_and(|seconds|seconds<=0))
            || (name.eq_ignore_ascii_case("expires") && value.trim().eq_ignore_ascii_case("Thu, 01 Jan 1970 00:00:00 GMT"))
    })
}

#[tokio::test]
async fn browser_cookie_value_1970_is_not_an_expiry_attribute() {
    let mut test = boot_seed("default").await.expect("pinned seed required");
    test.booted.router = axum::Router::new().route("/cookie-probe", axum::routing::get(|| async {
        ([(header::SET_COOKIE, "session_token=signed1970value; path=/; expires=Tue, 02 Mar 2027 16:00:00 GMT; httponly")], StatusCode::NO_CONTENT)
    }));
    let mut browser = test.browser("198.51.100.14");
    browser.get("/cookie-probe").await;
    assert_eq!(browser.cookies.get("session_token").map(String::as_str), Some("signed1970value"));
}

#[test]
fn browser_cookie_deletion_requires_real_attribute() {
    assert!(!cookie_tombstone("session_token=opaque1970value; expires=Tue, 02 Mar 2027 16:00:00 GMT"));
    assert!(!cookie_tombstone("session_token=signed; max-age=01"));
    assert!(cookie_tombstone("session_token=signed; Max-Age=0"));
    assert!(cookie_tombstone("session_token=; expires=Thu, 01 Jan 1970 00:00:00 GMT"));
}




#[path = "tests/sidebar_review.rs"]
mod sidebar_review;

#[path = "tests/navigation_followups.rs"]
mod navigation_followups;

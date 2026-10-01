//! Request-level tests for the session, account and user controllers (controllers A), through the
//! whole stack (`app::boot`, the Rails route table, kit) over a private copy of the reference-built
//! `default` parity seed. Missing seeds fail in CI and skip locally with a note
//! (`parity/bin/seed build default`). Parity against the running reference lives in
//! `reference-tools/campfire/controllers_a/replay.py`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use tower::ServiceExt;

use crate::app::{Booted, boot_with_clock};
use crate::config::Config;
use crate::controllers::presenters::test_support::{masked_session_token, seed_clock, seed_dir};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const HOST: &str = "campfire.test";
const PASSWORD: &str = "secret123456";
const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

struct Test {
    booted: Booted,
    labels: serde_json::Value,
    _dir: tempfile::TempDir,
}

async fn boot_seed(name: &str) -> Option<Test> {
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
    Some(Test {
        booted: boot_with_clock(config, seed_clock()).await.unwrap(),
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

    /// Asserts the page has a `button_to` form for `action` that sends `method`.
    fn assert_button(&self, action: &str, method: &str) {
        let html = self.text();
        let method_field = format!("name=\"_method\" value=\"{method}\"");
        let found = html.split("<form").skip(1).any(|form| {
            let form = &form[..form.find("</form>").unwrap_or(form.len())];
            form.contains(&format!("action=\"{action}\"")) && form.contains(&method_field)
        });
        assert!(found, "no {method} button for {action}");
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

fn encode(value: &str) -> String {
    campfire_views::helpers::url::cgi_escape(value)
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
    assert!(
        page.header("link")
            .is_some_and(|link| link.contains("rel=preload; as=style"))
    );
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
    assert_redirect(&signed_in, "http://campfire.test/account/edit");
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
    assert!(root.location().starts_with("http://campfire.test/rooms/"));

    // Sign out from the profile page's form.
    let profile = browser.get("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::OK, "{}", profile.text());
    profile.assert_form("/session");
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
    assert_eq!(profile.status, StatusCode::OK);
    assert!(profile.text().contains("David"));
}

#[tokio::test]
async fn direct_uploads_are_refused_past_the_body_limit() {
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
    assert_eq!(large.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn edge_gets_its_install_instructions() {
    // EdgeHTML's token: the useragent gem reports Chromium Edge (`Edg/`) as Chrome.
    const EDGE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edge/124.0.0.0";
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.9");
    browser.sign_in("david@37signals.com").await;
    let profile = browser
        .request(
            Method::GET,
            "/users/me/profile",
            &[("user-agent", EDGE)],
            None,
        )
        .await;
    assert_eq!(profile.status, StatusCode::OK);
    assert!(
        profile
            .text()
            .contains(&campfire_assets::asset_path("external/install-edge.svg")),
        "{}",
        profile.text()
    );
}

#[tokio::test]
async fn transfers_sign_in_on_another_device() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.5");
    admin.sign_in(&test.label("emails.david")).await;
    let kevin_id = test.label("users.kevin");
    let page = admin.get(&format!("/users/{kevin_id}")).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    let html = page.text();
    let at = html.find("/session/transfers/").expect("transfer link") + "/session/transfers/".len();
    let transfer_id = html[at..].split('"').next().unwrap().to_string();

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
        "http://campfire.test/",
    );
    assert_eq!(phone.get("/users/me/profile").await.status, StatusCode::OK);

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
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.11");
    admin.sign_in(&test.label("emails.david")).await;
    admin.confirm_sudo().await;
    let account_id = test.label("accounts.signal");

    let edit = admin.get("/account/edit").await;
    assert_eq!(edit.status, StatusCode::OK, "{}", edit.text());
    let action = format!("/account.{account_id}");
    edit.assert_form(&action);
    let updated = admin
        .form("patch", &action, &[("account[name]", "Renamed")])
        .await;
    assert_redirect(&updated, "http://campfire.test/account/edit");
    let edit = admin.get("/account/edit").await;
    assert!(edit.text().contains("Renamed"));
    assert!(edit.text().contains("flash"), "the ✓ notice shows once");

    // Join code reset.
    edit.assert_form("/account/join_code");
    assert_redirect(
        &admin.form("post", "/account/join_code", &[]).await,
        "http://campfire.test/account/edit",
    );
    assert!(
        !admin
            .get("/account/edit")
            .await
            .text()
            .contains(&test.label("join_codes.signal"))
    );

    // Custom styles.
    let page = admin.get("/account/custom_styles/edit").await;
    assert_eq!(page.status, StatusCode::OK);
    page.assert_form("/account/custom_styles");
    let reply = admin
        .form(
            "patch",
            "/account/custom_styles",
            &[("account[custom_styles]", "body { --x: 1 }")],
        )
        .await;
    assert_redirect(&reply, "http://campfire.test/account/custom_styles/edit");
    assert!(
        admin
            .get("/account/custom_styles/edit")
            .await
            .text()
            .contains("<style data-turbo-track=\"reload\">body { --x: 1 }</style>")
    );

    // The next page of people, as a turbo stream.
    let page = admin
        .request(
            Method::GET,
            "/account/users?page=2",
            &[("accept", "text/vnd.turbo-stream.html")],
            None,
        )
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(
        page.header("content-type")
            .unwrap()
            .starts_with("text/vnd.turbo-stream.html")
    );
    assert_eq!(
        admin.get("/account/users").await.status,
        StatusCode::NOT_ACCEPTABLE
    );

    // Members can see the account but not change it.
    let mut member = test.browser("198.51.100.12");
    member.sign_in(&test.label("emails.kevin")).await;
    let edit = member.get("/account/edit").await;
    assert_eq!(edit.status, StatusCode::OK);
    assert!(
        !edit.text().contains(&format!("action=\"{action}\"")),
        "members get no account form"
    );
    assert_eq!(
        member
            .form("patch", &action, &[("account[name]", "Mine")])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        member.get("/account/bots").await.status,
        StatusCode::FORBIDDEN
    );
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
    let html = response.text();
    let value = html.split("aria-label=\"Bot key\"").next().unwrap();
    let new_key = value.rsplit("value=\"").next().unwrap().split('"').next().unwrap().to_string();
    assert_ne!(new_key, old_key);
    let (new_valid, old_valid, plaintext, audit) = test.booted.app.db.read({let new_key=new_key.clone(); move |conn| Ok((
        campfire_db::User::authenticate_bot(conn,&new_key)?.is_some(),
        campfire_db::User::authenticate_bot(conn,&old_key)?.is_some(),
        conn.query_row("SELECT bot_token FROM users WHERE id=?",[bot_id],|r|r.get::<_,Option<String>>(0))?,
        conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.reset'",[],|r|r.get::<_,i64>(0))?,
    ))}).await.unwrap();
    assert!(new_valid);
    assert!(!old_valid);
    assert_eq!(plaintext,None);
    assert_eq!(audit,1);
    assert!(!admin.get("/account/bots").await.text().contains(&new_key));
}

#[tokio::test]
async fn manages_bots() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.13");
    admin.sign_in(&test.label("emails.david")).await;
    admin.confirm_sudo().await;
    let index = admin.get("/account/bots").await;
    assert_eq!(index.status, StatusCode::OK);
    assert!(!index.text().contains(&test.label("bot_keys.bender")));
    assert!(index.text().contains("BOT_KEY"));

    let new = admin.get("/account/bots/new").await;
    new.assert_form("/account/bots");
    let reply = admin
        .form(
            "post",
            "/account/bots",
            &[
                ("user[name]", "Robo"),
                ("user[webhook_url]", "https://example.com/robo"),
            ],
        )
        .await;
    assert_redirect(&reply, "http://campfire.test/account/bots");
    assert!(admin.get("/account/bots").await.text().contains("Robo"));

    let bender = test.label("users.bender");
    let edit = admin.get(&format!("/account/bots/{bender}/edit")).await;
    assert_eq!(edit.status, StatusCode::OK);
    let action = format!("/account/bots/{bender}");
    edit.assert_form(&action);
    assert_redirect(
        &admin
            .form("patch", &action, &[("user[name]", "Bender 2")])
            .await,
        "http://campfire.test/account/bots",
    );

    let edit = admin.get(&format!("/account/bots/{bender}/edit")).await;
    let key_action = format!("/account/bots/{bender}/key");
    edit.assert_button(&key_action, "put");
    admin.confirm_sudo().await;
    let bender_id: i64 = bender.parse().unwrap();
    let old_digest = test.booted.app.db.read(move |conn| Ok(campfire_db::User::find(conn, bender_id)?.bot_token_digest)).await.unwrap();
    assert_eq!(admin.form("put", &key_action, &[]).await.status, StatusCode::OK);
    let new_digest = test.booted.app.db.read(move |conn| Ok(campfire_db::User::find(conn, bender_id)?.bot_token_digest)).await.unwrap();
    assert_ne!(old_digest, new_digest);
    assert!(admin.get("/account/bots").await.text().contains(campfire_db::user::BOT_KEY_PLACEHOLDER));

    assert!(!admin.get("/account/bots").await.text().contains(&test.label("bot_keys.bender")));

    admin
        .get(&format!("/account/bots/{bender}/edit"))
        .await
        .assert_button(&action, "delete");
    assert_redirect(
        &admin.form("delete", &action, &[]).await,
        "http://campfire.test/account/bots",
    );
    assert_eq!(
        admin
            .get(&format!("/account/bots/{bender}/edit"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
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
async fn profile_sidebar_and_user_pages() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut browser = test.browser("198.51.100.15");
    browser.sign_in(&test.label("emails.kevin")).await;

    let profile = browser.get("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::OK);
    profile.assert_form("/users/me/profile");
    let reply = browser
        .form(
            "patch",
            "/users/me/profile",
            &[("user[name]", "Kev"), ("user[bio]", "Hi")],
        )
        .await;
    assert_redirect(&reply, "http://campfire.test/users/me/profile");
    assert!(
        browser
            .get("/users/me/profile")
            .await
            .text()
            .contains("Kev")
    );

    let sidebar = browser.get("/users/me/sidebar").await;
    assert_eq!(sidebar.status, StatusCode::OK);
    assert!(sidebar.text().contains("<!DOCTYPE html>"));
    let frame = browser
        .request(
            Method::GET,
            "/users/me/sidebar",
            &[("turbo-frame", "user_sidebar")],
            None,
        )
        .await;
    assert_eq!(frame.status, StatusCode::OK);
    assert!(!frame.text().contains("<!DOCTYPE html>") && frame.text().contains("<turbo-frame"));

    assert_eq!(
        browser
            .get(&format!("/users/{}", test.label("users.david")))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        browser.get("/users/999999999").await.status,
        StatusCode::NOT_FOUND
    );

    let subscriptions = browser.get("/users/me/push_subscriptions").await;
    assert_eq!(subscriptions.status, StatusCode::OK);
    let body = r#"{"push_subscription":{"endpoint":"http://example.com/push","p256dh_key":"a","auth_key":"b"}}"#;
    let reply = browser
        .request(
            Method::POST,
            "/users/me/push_subscriptions",
            &[],
            Some(("application/json", body.into())),
        )
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "an http endpoint fails validation"
    );
}

#[tokio::test]
async fn bans_and_unbans() {
    let Some(test) = boot_seed("default").await else {
        return;
    };
    let mut admin = test.browser("198.51.100.16");
    admin.sign_in(&test.label("emails.david")).await;
    admin.confirm_sudo().await;
    let jz = test.label("users.jz");
    let page = admin.get(&format!("/users/{jz}")).await;
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
    page.assert_form(&action);
    assert_redirect(
        &admin.form("post", &action, &[]).await,
        &format!("http://campfire.test/users/{jz}"),
    );
    let page = admin.get(&format!("/users/{jz}")).await;
    page.assert_button(&action, "delete");
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

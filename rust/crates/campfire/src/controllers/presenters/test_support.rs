//! Request-level test support for these controllers: the whole app booted over a private copy of
//! the reference-built `default` parity seed, signed in with a Rails-issued session cookie
//! (`vectors/campfire_sessions.json`), with a tiny cookie jar and CSRF token handling.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use tower::ServiceExt;

use crate::app::{Booted, boot_with_clock};
use crate::config::Config;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

pub const DAVID: i64 = 127326141;
pub const JASON: i64 = 149087659;
pub const KEVIN: i64 = 712064548;
pub const BENDER: i64 = 394959859;
/// Bender's key as our Rails accepts it: the seed keeps the fixture's `bot_token_digest`
/// (`User.digest_bot_token("BenderToken1")`); the plaintext `bot_token` column is legacy and
/// no longer authenticates (`User::Bot.authenticate_bot`).
pub const BENDER_KEY: &str = "394959859-BenderToken1";
/// Rooms::Closed "All Talk" (David, Jason, Bender): 131 messages.
pub const ALL_TALK: i64 = 486777696;
/// Rooms::Open "HQ" (David can't see messages; no messages).
pub const HQ: i64 = 201306877;
/// Rooms::Closed "Quiet Corner", created by Kevin, David a member.
pub const QUIET_CORNER: i64 = 699448326;
/// A Rooms::Direct between David and Jason.
pub const DIRECT_DAVID_JASON: i64 = 186869642;
/// Kevin and Bender's direct room: David isn't in it.
pub const DIRECT_KEVIN_BENDER: i64 = 340026324;

/// `NOW` in the parity seeds (`parity/seeds/README.md`): the reference serves a seed with its
/// clock started there (`reference up --time 2026-03-02T16:00:00Z`).
pub const SEED_NOW: &str = "2026-03-02T16:00:00Z";

/// The clock a seeded app runs on: [`SEED_NOW`] when the app boots, then ticking, as the
/// reference's libfaketime clock does. (On the real clock, the seed's administrator sessions,
/// last active that afternoon, are long past `ADMIN_SESSION_IDLE_TIMEOUT_DAYS`.)
pub fn seed_clock() -> campfire_kit::SharedClock {
    #[derive(Debug)]
    struct SeedClock {
        start: jiff::Timestamp,
        booted: std::time::Instant,
    }
    impl campfire_kit::Clock for SeedClock {
        fn now(&self) -> jiff::Timestamp {
            let elapsed = jiff::SignedDuration::try_from(self.booted.elapsed()).unwrap();
            self.start.checked_add(elapsed).unwrap()
        }
    }
    std::sync::Arc::new(SeedClock {
        start: SEED_NOW.parse().unwrap(),
        booted: std::time::Instant::now(),
    })
}

pub fn seed_dir(name: &str) -> Option<PathBuf> {
    find_seed(Path::new(ROOT), name, std::env::var_os("CI").is_some())
}

fn find_seed(root: &Path, name: &str, ci: bool) -> Option<PathBuf> {
    let dir = root.join("parity/.seed").join(name);
    if dir.join("db/production.sqlite3").is_file() {
        return Some(dir);
    }
    assert!(
        !ci,
        "CI requires parity/.seed/{name}; run parity/bin/seed build {name} before the tests"
    );
    eprintln!("skipping locally: parity/.seed/{name} isn't built (parity/bin/seed build {name})");
    None
}

#[test]
#[should_panic(expected = "CI requires parity/.seed/default")]
fn missing_seed_fails_in_ci() {
    let root = tempfile::tempdir().unwrap();
    find_seed(root.path(), "default", true);
}

#[test]
fn missing_seed_may_skip_locally() {
    let root = tempfile::tempdir().unwrap();
    assert!(find_seed(root.path(), "default", false).is_none());
}

#[test]
fn built_seed_is_found_in_ci() {
    let root = tempfile::tempdir().unwrap();
    let seed = root.path().join("parity/.seed/default");
    std::fs::create_dir_all(seed.join("db")).unwrap();
    std::fs::write(seed.join("db/production.sqlite3"), []).unwrap();
    assert_eq!(find_seed(root.path(), "default", true), Some(seed));
}

fn parity_env(name: &str) -> Option<String> {
    let env = std::fs::read_to_string(Path::new(ROOT).join("parity/.env.reference")).ok()?;
    env.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// David's Rails-issued `session_token` cookie header.
pub fn david_cookie() -> String {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/campfire_sessions.json"
    )))
    .unwrap();
    vectors["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["user_name"] == "David")
        .unwrap()["cookie_header"]
        .as_str()
        .unwrap()
        .to_string()
}

pub struct TestApp {
    pub booted: Booted,
    _dir: tempfile::TempDir,
}

impl TestApp {
    /// `None` (and a note) locally when the seed hasn't been built; fails in CI.
    pub async fn boot() -> Option<TestApp> {
        Self::boot_with_clock(seed_clock()).await
    }

    pub async fn boot_with_clock(clock: campfire_kit::SharedClock) -> Option<TestApp> {
        Self::boot_with_clock_and_env(clock, &[]).await
    }

    pub async fn boot_with_clock_and_env(
        clock: campfire_kit::SharedClock,
        extra: &[(&str, &str)],
    ) -> Option<TestApp> {
        Self::boot_with_loops(clock, extra, None).await
    }

    /// Consumer fixture tests must not race an unrelated initial periodic sweep.
    /// This still starts the real durable runner with its normal worker counts.
    pub async fn boot_without_periodic() -> Option<TestApp> {
        Self::boot_with_loops(
            seed_clock(),
            &[],
            Some(crate::jobs::periodic::Loops {
                periodic: None,
                huddle: None,
            }),
        )
        .await
    }

    async fn boot_with_loops(
        clock: campfire_kit::SharedClock,
        extra: &[(&str, &str)],
        loops: Option<crate::jobs::periodic::Loops>,
    ) -> Option<TestApp> {
        let seed = seed_dir("default")?;
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("db")).unwrap();
        std::fs::copy(
            seed.join("db/production.sqlite3"),
            dir.path().join("db/production.sqlite3"),
        )
        .unwrap();
        copy_dir(&seed.join("storage"), &dir.path().join("files"));
        let root = dir.path().to_string_lossy().into_owned();
        let secret = parity_env("SECRET_KEY_BASE").unwrap();
        let config = Config::from_lookup(|name| match name {
            "SECRET_KEY_BASE" => Some(secret.clone()),
            "DISABLE_SSL" => Some("true".into()),
            "APP_VERSION" | "GIT_REVISION" => Some("parity".into()),
            "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
            _ => extra
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).into()),
        })
        .unwrap();
        Some(TestApp {
            booted: match loops {
                Some(loops) => crate::app::boot_with_clock_and_loops(config, clock, loops)
                    .await
                    .unwrap(),
                None => boot_with_clock(config, clock).await.unwrap(),
            },
            _dir: dir,
        })
    }

    pub fn db(&self) -> &campfire_db::Database {
        &self.booted.app.db
    }

    /// A browser signed in as David.
    pub fn david(&self) -> Browser<'_> {
        let mut browser = Browser {
            app: self,
            cookies: BTreeMap::new(),
        };
        browser.absorb_cookie_header(&david_cookie());
        browser
    }

    pub fn anonymous(&self) -> Browser<'_> {
        Browser {
            app: self,
            cookies: BTreeMap::new(),
        }
    }

    /// A browser signed in as `user_id` with a new session of its own (two-factor verified, as
    /// the seed's are), for anyone the session vectors don't cover.
    pub async fn sign_in(&self, user_id: i64) -> Browser<'_> {
        use campfire_kit::Crypto;

        let attributes = campfire_db::NewSession {
            user_agent: None,
            ip_address: Some("127.0.0.1"),
            device_id: None,
            two_factor_verified: true,
        };
        let session = self
            .db()
            .write(move |tx| campfire_db::Session::start_with(tx, user_id, attributes))
            .await
            .unwrap();
        let signed = campfire_kit::RailsCrypto::new(self.booted.app.secrets.clone()).sign_cookie(
            "session_token",
            &session.token,
            None,
        );
        let mut browser = self.anonymous();
        browser.cookies.insert(
            "session_token".into(),
            campfire_kit::cookies::escape(&signed),
        );
        browser
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[derive(Debug)]
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| panic!("{e}: {}", self.text()))
    }

    pub fn location(&self) -> Option<&str> {
        self.header("location")
    }

    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }
}

/// A client that keeps cookies between requests.
pub struct Browser<'a> {
    app: &'a TestApp,
    cookies: BTreeMap<String, String>,
}

pub struct Req {
    pub method: Method,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Req {
    pub fn new(method: Method, path: &str) -> Self {
        Req {
            method,
            path: path.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn form(mut self, pairs: &[(&str, &str)]) -> Self {
        let body: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect();
        self.body = body.join("&").into_bytes();
        self.header("content-type", "application/x-www-form-urlencoded")
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// A multipart body with text fields and one file field.
    pub fn multipart(mut self, fields: &[(&str, &str)], file: (&str, &str, &str, &[u8])) -> Self {
        let boundary = "----campfiretestboundary";
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        let (name, filename, content_type, data) = file;
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        self.body = body;
        self.header(
            "content-type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
    }
}

pub fn encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

impl Browser<'_> {
    /// Rails-compatible sudo session for controller tests; no confirmation endpoint shortcut.
    pub(crate) async fn grant_sudo(&mut self) {
        use campfire_kit::Crypto;
        self.authenticity_token().await;
        let key = campfire_kit::session::SESSION_KEY;
        let crypto = campfire_kit::RailsCrypto::new(self.app.booted.app.secrets.clone());
        let raw = percent_encoding::percent_decode_str(self.cookies.get(key).unwrap()).decode_utf8().unwrap();
        let mut session = crypto.decrypt_cookie(key, &raw, jiff::Timestamp::now()).unwrap();
        session["sudo_verified_at"] = serde_json::json!(self.app.booted.app.clock.now().as_second());
        let encrypted = crypto.encrypt_cookie(key, &session, None);
        self.cookies.insert(key.into(), campfire_kit::cookies::escape(&encrypted));
    }

    pub fn cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn absorb_cookie_header(&mut self, header: &str) {
        for pair in header.split(';') {
            if let Some((name, value)) = pair.trim().split_once('=') {
                self.cookies.insert(name.to_string(), value.to_string());
            }
        }
    }

    fn absorb_set_cookies(&mut self, headers: &HeaderMap) {
        for value in headers.get_all(header::SET_COOKIE) {
            let value = value.to_str().unwrap();
            let pair = value.split(';').next().unwrap();
            if let Some((name, value)) = pair.split_once('=') {
                if value.is_empty() {
                    self.cookies.remove(name);
                } else {
                    self.cookies.insert(name.to_string(), value.to_string());
                }
            }
        }
    }

    pub async fn send(&mut self, req: Req) -> Reply {
        let mut request = Request::builder()
            .method(req.method.clone())
            .uri(&req.path)
            .header(header::HOST, "campfire.test");
        if !self.cookies.is_empty() {
            let cookie = self
                .cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            request = request.header(header::COOKIE, cookie);
        }
        let has_accept = req
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("accept"));
        if !has_accept {
            request = request.header(header::ACCEPT, "text/html,application/xhtml+xml");
        }
        for (name, value) in &req.headers {
            request = request.header(name.as_str(), value.as_str());
        }
        let request = request.body(Body::from(req.body)).unwrap();
        let response = self
            .app
            .booted
            .router
            .clone()
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        self.absorb_set_cookies(&headers);
        Reply {
            status,
            headers,
            body,
        }
    }

    pub async fn get(&mut self, path: &str) -> Reply {
        self.send(Req::new(Method::GET, path)).await
    }

    /// A write as the app's own pages make it: with the session's authenticity token in
    /// `X-CSRF-Token`, as Turbo sends it from the `csrf-token` meta tag.
    pub async fn write(&mut self, req: Req) -> Reply {
        let token = self.authenticity_token().await;
        self.send(req.header(campfire_kit::csrf::HEADER, &token))
            .await
    }

    /// A masked global token for this browser's session, as `csrf_meta_tags` renders it. A
    /// session without one yet gets one from a page first.
    pub async fn authenticity_token(&mut self) -> String {
        if let Some(token) = self.session_token() {
            return token;
        }
        let mut path = "/".to_string();
        for _ in 0..3 {
            match self.get(&path).await.location() {
                Some(location) => path = location.to_string(),
                None => break,
            }
        }
        self.session_token()
            .expect("the session has an authenticity token after a page")
    }

    fn session_token(&self) -> Option<String> {
        masked_session_token(
            &self.app.booted.app.secrets,
            self.cookies.get(campfire_kit::session::SESSION_KEY)?,
        )
    }

    /// The real (unmasked) authenticity token in this browser's session, which every token its
    /// pages carry must verify against, or `None` before a page has given it one.
    pub fn real_authenticity_token(&self) -> Option<campfire_kit::csrf::RealToken> {
        use campfire_kit::Crypto;

        let raw = self.cookies.get(campfire_kit::session::SESSION_KEY)?;
        let raw = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
        let crypto = campfire_kit::RailsCrypto::new(self.app.booted.app.secrets.clone());
        let session = crypto.decrypt_cookie(
            campfire_kit::session::SESSION_KEY,
            &raw,
            jiff::Timestamp::now(),
        )?;
        campfire_kit::csrf::RealToken::decode(
            session.get(campfire_kit::csrf::SESSION_KEY)?.as_str()?,
        )
    }
}

/// A masked global authenticity token for the session in the `_campfire_session` cookie value
/// `raw` (as sent, still escaped), or `None` when the session hasn't been given one.
pub fn masked_session_token(
    secrets: &std::sync::Arc<rails_compat::Secrets>,
    raw: &str,
) -> Option<String> {
    use campfire_kit::Crypto;

    let raw = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
    let crypto = campfire_kit::RailsCrypto::new(secrets.clone());
    let session = crypto.decrypt_cookie(
        campfire_kit::session::SESSION_KEY,
        &raw,
        jiff::Timestamp::now(),
    )?;
    let real = session.get(campfire_kit::csrf::SESSION_KEY)?.as_str()?;
    Some(campfire_kit::csrf::RealToken::decode(real)?.masked(None))
}

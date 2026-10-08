//! Private fixtures for successful URL-contract boundary requests.
//!
//! Each server group holds its [`FixtureContext`] to keep provider doubles alive. The empty
//! profile leaves the original unconfigured cases untouched; separate Slack profiles isolate
//! mutations because a queued or undoing run blocks subsequent imports.
//!
//! Profiles: `""` stages nothing (existing unconfigured cases keep their behavior); `"api"`
//! holds the bot/agent/webhook/calendar successes; `"slack_apply"`, `"slack_undo"` and
//! `"slack_personal"` each hold one mutation chain on its own runs (an `undoing`/`queued`
//! run blocks every other mutation, so the undos cannot share a group); `"retained"` holds
//! the blob/disk/proxy/representation/avatar successes; `"oauth"` holds the configured
//! Google and GitHub callbacks (classic + SPA return destinations); `"oauth_slack_r3"`
//! and `"oauth_google_r3"` separately hold Slack linking and Google sign-in successes.
//!
//! GitHub's and Slack's networks are fixed at boot; Google's transport doubles are installed
//! after boot.

use std::collections::BTreeMap;

use campfire_db::{AgentCredential, NewCredential};
use sha2::{Digest, Sha256};

use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};

/// The seeded workspace agent (`{AGENT}` in `urls.json`), owned by Bender's user row.
const AGENT_ID: i64 = 773018776;
/// Local fake bearer secret for the agent-token cases; only its SHA-256 digest is stored.
const AGENT_TOKEN: &str = "url-contract-agent-token-1";
/// `GITHUB_WEBHOOK_SECRET` for the `"api"` profile; the case's `X-Hub-Signature-256` is the
/// HMAC-SHA256 of its exact JSON bytes under this secret (`sha256=`-prefixed).
const GITHUB_WEBHOOK_SECRET: &str = "url-contract-github-webhook-secret";
/// Calendar push channel identity for the `"api"` profile (seed has no push channels).
const PUSH_CHANNEL: &str = "url-contract-push-channel-1";
const PUSH_TOKEN: &str = "url-contract-push-token-1";
/// Distinct Slack team per profile group (each group owns a private seed copy).
const SLACK_TEAM: &str = "TCONTRACT-R2";
/// The mock GitHub login the `"oauth"` fake `GET /user` answers with.
const GITHUB_LOGIN: &str = "contract-octocat";
/// The mock Google account email the `"oauth"` fake `/token` answers with.
const GOOGLE_EMAIL: &str = "david.contract@gmail.test";
/// The `"oauth_slack_r3"` app and team: the fake `POST /api/oauth.v2.access` answers with
/// this team, and the workspace row is pre-named to match so no `team.info` fetch is needed.
const SLACK_TEAM_R3: &str = "TCONTRACT-R3";
const SLACK_TEAM_NAME_R3: &str = "URL contract R3";
const SLACK_CLIENT_ID_R3: &str = "url-contract-slack-client-r3";
const SLACK_CLIENT_SECRET_R3: &str = "url-contract-slack-secret-r3";
const SLACK_USER_R3: &str = "UCONTRACTR3";
const SLACK_GRANT_R3: &str = "xoxe-url-contract-r3-grant";
/// The `"oauth_google_r3"` identity: David's seeded address (so the verified claims
/// resolve to his row) under a fixed subject, in the fixture's allowed domain.
const GOOGLE_SIGNIN_SUB: &str = "url-contract-david-sub";
const GOOGLE_SIGNIN_EMAIL: &str = "david@37signals.com";
const GOOGLE_SIGNIN_DOMAIN: &str = "37signals.com";
const GOOGLE_SIGNIN_CLIENT_ID: &str = "test-client-id";
const GOOGLE_SIGNIN_CLIENT_SECRET: &str = "FAKE-google-client-secret";

/// How many queued Google `/token` answers the `"oauth"` profile stages: two callbacks run
/// (classic + SPA return), the rest is margin. `Recorded` panics on any unrecorded Google
/// call, so no other case in that group may touch the Google API.
const GOOGLE_TOKEN_ANSWERS: usize = 6;

pub struct FixtureContext {
    profile: String,
    env: Vec<(String, String)>,
    fixed: Option<FixedNetwork>,
}

/// A TLS fake-provider harness (`integrations::github::tests::fake` for `"oauth"`,
/// `integrations::slack::client::tests::fake` for `"oauth_slack_r3"`): canned answers over a
/// `Network` whose resolver and dialer send the provider hosts to the local server. Holding
/// the server keeps its listener alive for the group's lifetime.
struct FixedNetwork {
    _server: crate::integrations::test_support::FakeServer,
    network: crate::net::Network,
}

impl FixtureContext {
    /// Start the profile's provider doubles. `"oauth"` starts the fake GitHub App server;
    /// `"oauth_slack_r3"` starts the fake Slack server; Google clients are post-boot installs in
    /// [`Self::prepare`].
    pub async fn boot(profile: &str) -> Self {
        let fixed = if profile == "oauth" {
            let routes = vec![
                crate::integrations::test_support::Route::new(
                    "POST",
                    "github.com",
                    "/login/oauth/access_token",
                    200,
                )
                .header("Content-Type", "application/json")
                .body(
                    "{\"access_token\":\"url-contract-github-oauth-token\",\
                     \"token_type\":\"bearer\",\"scope\":\"\"}",
                ),
                crate::integrations::test_support::Route::new(
                    "GET",
                    "api.github.com",
                    "/user",
                    200,
                )
                .header("Content-Type", "application/json")
                .body(format!(
                    "{{\"login\":\"{GITHUB_LOGIN}\",\"id\":424242,\"type\":\"User\"}}"
                )),
            ];
            let (server, network) = crate::integrations::github::tests::fake(routes).await;
            Some(FixedNetwork {
                _server: server,
                network,
            })
        } else if profile == "oauth_slack_r3" {
            let scopes = crate::integrations::slack::oauth::USER_SCOPES.join(",");
            let exchange = serde_json::json!({
                "ok": true,
                "team": {"id": SLACK_TEAM_R3, "name": SLACK_TEAM_NAME_R3},
                "authed_user": {"id": SLACK_USER_R3, "access_token": SLACK_GRANT_R3, "scope": scopes},
            });
            let team = serde_json::json!({
                "ok": true,
                "team": {"id": SLACK_TEAM_R3, "name": SLACK_TEAM_NAME_R3, "domain": "contract-r3"},
            });
            let routes = vec![
                crate::integrations::test_support::Route::new(
                    "POST",
                    "slack.com",
                    "/api/oauth.v2.access",
                    200,
                )
                .header("Content-Type", "application/json")
                .body(exchange.to_string()),
                crate::integrations::test_support::Route::new(
                    "GET",
                    "slack.com",
                    "/api/team.info",
                    200,
                )
                .header("Content-Type", "application/json")
                .body(team.to_string()),
            ];
            let (server, network) = crate::integrations::slack::client::tests::fake(routes).await;
            Some(FixedNetwork {
                _server: server,
                network,
            })
        } else {
            None
        };
        let env = match profile {
            "api" => vec![(
                "GITHUB_WEBHOOK_SECRET".to_string(),
                GITHUB_WEBHOOK_SECRET.to_string(),
            )],
            _ => vec![],
        };
        Self {
            profile: profile.to_string(),
            env,
            fixed,
        }
    }

    /// Extra boot env, borrowed from this context (root stores the context in the group,
    /// so the refs outlive the boot call).
    pub fn env(&self) -> Vec<(&str, &str)> {
        self.env
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect()
    }

    /// Interface addition (see module docs): the profile's fixed provider network
    /// (GitHub for `"oauth"`, Slack for `"oauth_slack_r3"`), when the profile boots through
    /// `TestApp::boot_with_github_network_and_env`.
    pub fn github_network(&self) -> Option<crate::net::Network> {
        self.fixed.as_ref().map(|mock| mock.network.clone())
    }

    /// Stage the profile's rows, blobs and provider doubles; returns the runtime
    /// placeholders the profile's cases substitute. The app itself is untouched: every
    /// case still runs as real HTTP against the real router with real auth and state.
    pub async fn prepare(&self, app: &TestApp) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        match self.profile.as_str() {
            "api" => self.prepare_api(app, &mut out).await,
            "slack_apply" => self.prepare_slack_apply(app, &mut out).await,
            "slack_undo" => self.prepare_slack_undo(app, &mut out).await,
            "slack_personal" => self.prepare_slack_personal(app, &mut out).await,
            "retained" => self.prepare_retained(app, &mut out).await,
            "oauth" => self.prepare_oauth(app).await,
            "oauth_slack_r3" => self.prepare_slack_oauth(app).await,
            "oauth_google_r3" => self.prepare_google_signin(app, &mut out).await,
            _ => {}
        }
        out
    }

    /// Agent credential (additive: the seed's credential row stays, and no grants are
    /// added, so the legacy `read_messages`/`post_messages`/`react` capabilities keep
    /// applying) plus a calendar push channel for Kevin.
    async fn prepare_api(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let digest = format!("{:x}", Sha256::digest(AGENT_TOKEN));
        app.db()
            .write(move |tx| {
                AgentCredential::create(
                    tx,
                    NewCredential {
                        agent_id: AGENT_ID,
                        created_by_id: DAVID,
                        name: "URL contract".into(),
                        token_last_four: digest[..4].into(),
                        token_digest: digest,
                        ..Default::default()
                    },
                )?;
                Ok(())
            })
            .await
            .unwrap();
        out.insert("AGENT_TOKEN".into(), AGENT_TOKEN.into());
        app.db()
            .write(|tx| {
                tx.conn().execute(
                    "DELETE FROM calendar_push_channels WHERE user_id=?",
                    [KEVIN],
                )?;
                campfire_db::models::google_calendar::PushChannel::create(
                    tx,
                    KEVIN,
                    PUSH_CHANNEL,
                    &campfire_db::models::google_calendar::PushChannel::digest(PUSH_TOKEN),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        out.insert("PUSH_CHANNEL".into(), PUSH_CHANNEL.into());
        out.insert("PUSH_TOKEN".into(), PUSH_TOKEN.into());
    }

    /// One workspace, David's Slack connection, a completed dry run to apply and a
    /// completed import to undo. `SLACK_APPLIED` is the id the apply case's new run
    /// takes: `slack_imports` is `AUTOINCREMENT`, nothing else in this group inserts
    /// there, and the apply runs first, so it is `MAX(id) + 1`.
    async fn prepare_slack_apply(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let cipher = encryption(app);
        let (dry_run, import_done, applied) = app
            .db()
            .write(move |tx| {
                let workspace = insert_slack_workspace(tx)?;
                insert_slack_connection(tx, &cipher, workspace, DAVID)?;
                let dry_run = insert_slack_run(
                    tx,
                    workspace,
                    DAVID,
                    "workspace",
                    "dry_run",
                    dry_run_stats(),
                )?;
                let import_done = insert_slack_run(
                    tx,
                    workspace,
                    DAVID,
                    "workspace",
                    "import",
                    imported_stats(),
                )?;
                let applied: i64 = tx.conn().query_row(
                    "SELECT COALESCE(MAX(id), 0) + 1 FROM slack_imports",
                    [],
                    |row| row.get(0),
                )?;
                Ok((dry_run, import_done, applied))
            })
            .await
            .unwrap();
        out.insert("SLACK_DRYRUN".into(), dry_run.to_string());
        out.insert("SLACK_IMPORT_DONE".into(), import_done.to_string());
        out.insert("SLACK_APPLIED".into(), applied.to_string());
    }

    /// A completed workspace import for `admin_undo`. Undo needs no Slack connection.
    async fn prepare_slack_undo(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let run = app
            .db()
            .write(|tx| {
                let workspace = insert_slack_workspace(tx)?;
                insert_slack_run(
                    tx,
                    workspace,
                    DAVID,
                    "workspace",
                    "import",
                    imported_stats(),
                )
            })
            .await
            .unwrap();
        out.insert("SLACK_UNDO_RUN".into(), run.to_string());
    }

    /// A completed personal import for `personal_undo`.
    async fn prepare_slack_personal(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let run = app
            .db()
            .write(|tx| {
                let workspace = insert_slack_workspace(tx)?;
                insert_slack_run(tx, workspace, DAVID, "personal", "import", imported_stats())
            })
            .await
            .unwrap();
        out.insert("SLACK_PERSONAL_DONE".into(), run.to_string());
    }

    /// Private test blobs and stored variants: a text blob (redirect/proxy/disk), a PNG
    /// with a preprocessed webp representation, and Kevin's avatar with its preprocessed
    /// `:square` variant. Placeholders are the exact app-minted paths, so the cases
    /// assert the same URLs the app generates.
    async fn prepare_retained(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let storage = app.booted.app.storage.clone();
        let verifier = storage.verifier.clone();
        let disk_expiry = app.booted.app.clock.now() + jiff::SignedDuration::from_secs(3600);

        let text = b"url-contract retained blob alpha-7\n";
        let staged = storage
            .stage_bytes(
                text,
                campfire_storage::Filename::new("contract-note.txt"),
                Some("text/plain"),
            )
            .unwrap();
        let text_blob = store_staged(app, staged).await;
        out.insert(
            "TEXT_REDIRECT".into(),
            campfire_storage::paths::blob_redirect_path(&*verifier, &text_blob, None),
        );
        out.insert(
            "TEXT_PROXY".into(),
            campfire_storage::paths::blob_proxy_path(&*verifier, &text_blob, None),
        );
        out.insert(
            "TEXT_DISK".into(),
            disk_path(&storage, &text_blob, disk_expiry),
        );

        let earth = include_bytes!("../../../../fixtures/files/earth.png");
        let staged = storage
            .stage_bytes(
                earth,
                campfire_storage::Filename::new("contract-earth.png"),
                Some("image/png"),
            )
            .unwrap();
        let image_blob = store_staged(app, staged).await;
        let thumb = campfire_storage::Variation::resize_to_limit(64, 64, Some("webp"));
        let representation = crate::active_storage::processed_representation(
            &app.booted.app,
            image_blob.clone(),
            thumb.clone(),
        )
        .await
        .unwrap();
        out.insert(
            "REPRESENTATION_SHA256".into(),
            format!(
                "{:x}",
                Sha256::digest(storage.service.download(&representation.key).unwrap())
            ),
        );
        out.insert(
            "IMG_REDIRECT".into(),
            campfire_storage::paths::representation_redirect_path(&*verifier, &image_blob, &thumb),
        );
        out.insert(
            "IMG_PROXY".into(),
            campfire_storage::paths::representation_proxy_path(&*verifier, &image_blob, &thumb),
        );

        let staged = storage
            .stage_bytes(
                earth,
                campfire_storage::Filename::new("contract-avatar.png"),
                Some("image/png"),
            )
            .unwrap();
        app.db()
            .write(|tx| {
                crate::controllers::presenters::attachments::attach(
                    tx,
                    crate::controllers::presenters::attachments::Record::user(KEVIN),
                    "avatar",
                    staged,
                )
            })
            .await
            .unwrap();
        let square = campfire_storage::Variation::resize_to_limit(512, 512, Some("webp"));
        let stored = crate::controllers::presenters::attachments::processed_variant(
            &app.booted.app,
            crate::controllers::presenters::attachments::Record::user(KEVIN),
            "avatar",
            square,
        )
        .await
        .unwrap();
        let stored = stored.expect("the avatar variant is stored");
        out.insert(
            "AVATAR_SHA256".into(),
            format!(
                "{:x}",
                Sha256::digest(storage.service.download(&stored.key).unwrap())
            ),
        );
        out.insert(
            "AVATAR_TOKEN".into(),
            rails_compat::signed_id::avatar_token(&app.booted.app.secrets, KEVIN),
        );
    }

    /// Install the recorded Google API double (the `google_api_tests` harness): the
    /// callbacks' `POST /token` exchanges answer locally, and any other Google call
    /// panics instead of touching the network. The id token is unsigned (`alg: none`);
    /// `email_from_id_token` only reads its claims.
    async fn prepare_oauth(&self, app: &TestApp) {
        let recorded = crate::app::google_api_tests::Recorded::new(vec![]);
        let tokens = serde_json::json!({
            "access_token": "url-contract-google-access-token",
            "refresh_token": "url-contract-google-refresh-token",
            "scope": format!("openid email {}", campfire_db::models::google_account::CALENDAR_SCOPE),
            "expires_in": 3600,
            "id_token": google_id_token(),
        });
        for _ in 0..GOOGLE_TOKEN_ANSWERS {
            recorded.answer_for(hyper::Method::POST, "/token", 200, tokens.clone());
        }
        crate::app::google_api_tests::install(app, recorded).await;
    }

    /// App credentials let the real start handler issue signed state and the callback
    /// exchange it against the fake Slack server.
    async fn prepare_slack_oauth(&self, app: &TestApp) {
        let cipher = encryption(app);
        app.db()
            .write(move |tx| {
                let workspace = campfire_db::models::slack::SlackWorkspace::create(
                    tx,
                    &cipher,
                    SLACK_CLIENT_ID_R3,
                    SLACK_CLIENT_SECRET_R3,
                    Some(DAVID),
                )?;
                workspace.name_team(tx, &cipher, SLACK_TEAM_R3, Some(SLACK_TEAM_NAME_R3), None)?;
                Ok(())
            })
            .await
            .unwrap();
    }

    /// The transport signs RS256 tokens and serves matching JWKS; production verifies
    /// signature, issuer, audience and nonce. Keep David's seeded second factor and
    /// supply its current code for the real challenge after the callback.
    async fn prepare_google_signin(&self, app: &TestApp, out: &mut BTreeMap<String, String>) {
        let cipher = encryption(app);
        let secret = app
            .db()
            .write(move |tx| {
                tx.conn().execute("DELETE FROM google_identities", [])?;
                let claims = serde_json::json!({
                    "sub": GOOGLE_SIGNIN_SUB,
                    "email": GOOGLE_SIGNIN_EMAIL,
                    "hd": GOOGLE_SIGNIN_DOMAIN,
                });
                campfire_db::models::google_identity::GoogleIdentity::link_to_user(
                    tx,
                    claims.as_object().unwrap(),
                    DAVID,
                )?;
                campfire_db::TwoFactorCredential::for_user(tx.conn(), DAVID)?
                    .expect("David's frozen second factor")
                    .secret(&cipher)
            })
            .await
            .unwrap();
        let code = rails_compat::totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        out.insert("GOOGLE_SIGNIN_TOTP".into(), code);
        let sign_in = crate::integrations::google::sign_in::SignIn::with_client(
            crate::integrations::google::sign_in::Config {
                client_id: GOOGLE_SIGNIN_CLIENT_ID.into(),
                client_secret: GOOGLE_SIGNIN_CLIENT_SECRET.into(),
                domains: vec![GOOGLE_SIGNIN_DOMAIN.into()],
            },
            std::sync::Arc::new(GoogleSignInMock),
        );
        app.booted.app.google.install(sign_in);
    }
}

/// The `google_tests` transport double, signing on demand: `POST /token` answers with a
/// fresh RS256 id token whose `nonce` is the posted `code`, and `GET /oauth2/v3/certs`
/// serves the matching JWKS. The cases pass the before step's captured `nonce` as the
/// callback's `code`, so production verification sees the flow's own nonce. Any other
/// Google call panics instead of touching the network.
struct GoogleSignInMock;

impl crate::integrations::google::client::Client for GoogleSignInMock {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: hyper::Method,
        target: &'a str,
        _headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> crate::integrations::net::BoxFuture<
        'a,
        Result<(u16, Vec<u8>), crate::integrations::google::client::Unavailable>,
    > {
        Box::pin(async move {
            if host == "www.googleapis.com" && target == "/oauth2/v3/certs" {
                return Ok((
                    200,
                    include_bytes!("../integrations/google/test-jwks.json").to_vec(),
                ));
            }
            assert_eq!(
                (host, method, target),
                ("oauth2.googleapis.com", hyper::Method::POST, "/token")
            );
            let form: std::collections::BTreeMap<_, _> =
                url::form_urlencoded::parse(&body).collect();
            let nonce: &str = form.get("code").map(|value| value.as_ref()).unwrap_or("");
            let tokens = serde_json::json!({"id_token": google_signin_token(nonce)});
            Ok((200, serde_json::to_vec(&tokens).unwrap()))
        })
    }
}

/// Insert a staged upload's blob row and keep its bytes, as `attachments::attach`
/// does without the attachment row.
async fn store_staged(app: &TestApp, staged: campfire_storage::Staged) -> campfire_storage::Blob {
    let mut staged = Some(staged);
    app.db()
        .write(move |tx| {
            let staged = staged.take().expect("a single write");
            let blob = staged
                .insert(tx.conn(), tx.now().jiff())
                .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
            crate::active_storage::keep_after_commit(tx, staged);
            Ok(blob)
        })
        .await
        .unwrap()
}

/// `SlackConnection` encryption, as `slack::runs` builds it: `ArEncryption` over the app
/// secrets.
fn encryption(app: &TestApp) -> rails_compat::ar_encryption::ArEncryption {
    rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets)
}

/// A local-imports workspace row, mirroring `url_contract_tests::slack_fixtures` (team
/// identity only; no provider credentials).
fn insert_slack_workspace(tx: &mut campfire_db::Tx<'_>) -> campfire_db::Result<i64> {
    tx.conn().execute(
        "INSERT INTO slack_workspaces(team_id,team_name,created_at,updated_at) VALUES(?,?,?,?)",
        rusqlite::params![SLACK_TEAM, "URL contract fixtures", tx.now(), tx.now()],
    )?;
    Ok(tx.conn().last_insert_rowid())
}

fn insert_slack_connection(
    tx: &mut campfire_db::Tx<'_>,
    encryption: &rails_compat::ar_encryption::ArEncryption,
    workspace: i64,
    user_id: i64,
) -> campfire_db::Result<()> {
    campfire_db::models::slack::SlackConnection::create(
        tx,
        encryption,
        campfire_db::models::slack::NewConnection {
            workspace_id: workspace,
            user_id,
            slack_user_id: "UCONTRACT1",
            access_token: Some("xoxb-url-contract-token"),
            scopes: Some("channels:history"),
        },
    )?;
    Ok(())
}

/// A completed run, mirroring `url_contract_tests::slack_fixtures`.
fn insert_slack_run(
    tx: &mut campfire_db::Tx<'_>,
    workspace: i64,
    user_id: i64,
    kind: &str,
    mode: &str,
    stats: serde_json::Value,
) -> campfire_db::Result<i64> {
    tx.conn().execute(
        "INSERT INTO slack_imports(slack_workspace_id,user_id,kind,mode,status,options,stats,started_at,finished_at,created_at,updated_at) VALUES(?,?,?,?,'completed','{}',?,?,?,?,?)",
        rusqlite::params![
            workspace,
            user_id,
            kind,
            mode,
            stats.to_string(),
            tx.now(),
            tx.now(),
            tx.now(),
            tx.now()
        ],
    )?;
    Ok(tx.conn().last_insert_rowid())
}

/// A dry run that saw one conversation, as the apply form posts it.
fn dry_run_stats() -> serde_json::Value {
    serde_json::json!({
        "conversations": [{"id": "C1", "name": "general", "type": "public_channel", "messages": 1}],
        "users": {"total": 1, "matched": 1, "placeholders": 0},
        "counts": {"messages": 1}
    })
}

/// A completed import that touched nothing undoable (no conversation targets), so undo is
/// not blocked by a later overlapping import.
fn imported_stats() -> serde_json::Value {
    serde_json::json!({
        "conversations": [{"id": "C9", "name": "random", "type": "public_channel", "messages": 2}],
        "users": {"total": 1, "matched": 1, "placeholders": 0},
        "counts": {"messages": 2}
    })
}

/// `blob.url(disposition:)` on the disk service, as `active_storage::blob_url` mints it
/// (that helper is private to the redirect controllers).
fn disk_path(
    storage: &campfire_storage::Storage,
    blob: &campfire_storage::Blob,
    expires_at: jiff::Timestamp,
) -> String {
    let content_type = campfire_storage::content_types::for_serving(blob.content_type());
    let disposition = campfire_storage::content_types::forced_disposition(blob.content_type())
        .unwrap_or("inline");
    storage.service.url_path(
        &*storage.verifier,
        &blob.key,
        Some(expires_at),
        &blob.filename,
        Some(content_type),
        disposition,
    )
}

/// An unsigned JWT whose claims pass `email_from_id_token` against the installed test
/// config (`aud` must be its `client_id`).
fn google_id_token() -> String {
    use base64::Engine as _;
    let encode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload = serde_json::json!({
        "iss": "https://accounts.google.com",
        "aud": "test-client-id",
        "exp": 1893456000,
        "email": GOOGLE_EMAIL,
    });
    format!(
        "{}.{}.{}",
        encode.encode(r#"{"alg":"none","typ":"JWT"}"#),
        encode.encode(payload.to_string()),
        encode.encode("url-contract-signature")
    )
}

/// An RS256 JWT over the house fixture key (`kid: fixture`, as `google_tests` signs),
/// whose claims pass production `SignIn::verify` against the installed config when the
/// caller echoes the flow's `nonce`.
fn google_signin_token(nonce: &str) -> String {
    use base64::Engine as _;
    use ring::signature::{RSA_PKCS1_SHA256, RsaKeyPair};
    let key = RsaKeyPair::from_pkcs8(include_bytes!("../integrations/google/signing.der")).unwrap();
    let encode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = serde_json::json!({"alg": "RS256", "kid": "fixture"});
    let payload = serde_json::json!({
        "iss": "https://accounts.google.com",
        "aud": GOOGLE_SIGNIN_CLIENT_ID,
        "sub": GOOGLE_SIGNIN_SUB,
        "email": GOOGLE_SIGNIN_EMAIL,
        "email_verified": true,
        "hd": GOOGLE_SIGNIN_DOMAIN,
        "name": "David",
        "nonce": nonce,
        "exp": 1893456000,
        "iat": 1772467200,
        "auth_time": 1772467200,
    });
    let input = format!(
        "{}.{}",
        encode.encode(serde_json::to_vec(&header).unwrap()),
        encode.encode(serde_json::to_vec(&payload).unwrap()),
    );
    let mut signature = vec![0; key.public().modulus_len()];
    key.sign(
        &RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        input.as_bytes(),
        &mut signature,
    )
    .unwrap();
    format!("{input}.{}", encode.encode(signature))
}

//! Boot: configuration, the database (`db:prepare`), storage, the cable server, jobs and the HTTP
//! stack, wired the way the reference's middleware and initializers are.
//!
//! [`AppState`] is what controllers, channels, jobs and integrations share. Actions reach it with
//! `c.app()` ([`AppCtx`]).
//!
//! Request flow (mirroring the Rails middleware order): `Rack::Deflater` (config.ru) → kit's
//! pre-routing middleware (`ActionDispatch::SSL`, request id, `_method` override) → public files
//! (`ActionDispatch::Static`, from `campfire_assets`) → `/cable` (Action Cable) or the Rails route
//! table (`controllers::dispatch`).

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::middleware::Next;
use campfire_db::Database;
use campfire_kit::exceptions::ErrorPages;
use campfire_kit::{Ctx, Kit, KitConfig, RailsCrypto, SharedClock, SharedCrypto};
use campfire_storage::{DiskService, Storage};
use campfire_views::fragment_cache::{FragmentCache, Scoped};
use rails_compat::{MessageVerifier, Secrets};

use crate::config::Config;
use crate::rich_text::AppRichText;
use crate::{channels, controllers, jobs};

pub use crate::channels::Cable;

/// Everything that outlives a request. Cheap to share as [`App`].
pub struct AppState {
    pub config: Config,
    pub secrets: Arc<Secrets>,
    pub ar_encryption: Arc<rails_compat::ar_encryption::ArEncryption>,
    pub clock: SharedClock,
    pub db: Database,
    pub storage: Arc<Storage>,
    pub cable: Cable,
    pub broadcasts: channels::Broadcasts,
    pub jobs: jobs::Jobs,
    pub mail: crate::mail::State,
    pub fizzy: crate::integrations::fizzy::State,
    pub agent_message_payload: crate::controllers::presenters::agent_payload::State,
    pub agent_repositories: crate::integrations::agent_repositories::State,
    pub sudo: crate::concerns::sudo::State,
    pub two_factor: crate::concerns::two_factor::State,
    pub google: crate::integrations::google::State,
    pub errors: crate::errors::Reporter,
    /// `config.x.web_push_pool`; `None` when Web Push is off (no valid VAPID keys).
    pub web_push: Option<crate::integrations::web_push::Pool>,
    pub github_accounts: crate::integrations::github::accounts::Accounts,
    pub github_app: crate::integrations::github::client::AppClient,
    pub github_read: crate::integrations::github::client::ReadClient,
    pub subscription_network: crate::integrations::net::Network,
    pub slack_network: crate::integrations::net::Network,
    /// `Rails.cache` for view fragments (`cache message do`), current during every request
    /// and every render outside one.
    pub fragment_cache: Arc<FragmentCache>,
}

impl AppState {
    /// The original system helpers change LiveKit ENV inside a test body.
    /// Rebind only that real configuration, keeping the booted model/service
    /// dependencies and all installed adapter contents in the private host.
    #[cfg(test)]
    pub(crate) fn fixture_huddle_config(&self, lookup: impl Fn(&str) -> Option<String>) -> App {
        let mut config = self.config.clone();
        config.livekit_url = lookup("LIVEKIT_URL");
        config.huddle = crate::huddle::Config::from_lookup(&lookup);
        config.huddles_configured = crate::huddle_readiness::huddles_configured(&lookup);
        Arc::new(Self {
            config,
            secrets: self.secrets.clone(),
            ar_encryption: self.ar_encryption.clone(),
            clock: self.clock.clone(),
            db: self.db.clone(),
            storage: self.storage.clone(),
            cable: self.cable.clone(),
            broadcasts: self.broadcasts.clone(),
            jobs: self.jobs.clone(),
            mail: self.mail.fixture_snapshot(),
            fizzy: crate::integrations::fizzy::State {
                network: self.fizzy.network.clone(), base: self.fizzy.base.clone(),
            },
            agent_message_payload: self.agent_message_payload.fixture_snapshot(),
            agent_repositories: self.agent_repositories.fixture_snapshot(),
            sudo: self.sudo.fixture_snapshot(),
            two_factor: self.two_factor.fixture_snapshot(),
            google: self.google.clone(),
            errors: self.errors.clone(),
            web_push: self.web_push.clone(),
            github_accounts: self.github_accounts.clone(),
            github_app: self.github_app.clone(),
            github_read: self.github_read.clone(),
            subscription_network: self.subscription_network.clone(),
            slack_network: self.slack_network.clone(),
            fragment_cache: self.fragment_cache.clone(),
        })
    }

    /// The key pages offer browsers to subscribe with: none while Web Push is off, so that browsers
    /// don't subscribe to notifications that would never be sent.
    pub fn vapid_public_key(&self) -> Option<String> {
        self.web_push
            .as_ref()
            .and(self.config.vapid_public_key.clone())
    }
}

pub type App = Arc<AppState>;

/// `c.app()` in actions.
pub trait AppCtx {
    fn app(&self) -> &App;
}

impl AppCtx for Ctx {
    fn app(&self) -> &App {
        self.state::<App>()
    }
}

/// A booted app: its state, the HTTP service, and the job runner (with the periodic loops).
pub struct Booted {
    pub app: App,
    pub router: Router,
    pub jobs: jobs::Runner,
    #[cfg(test)]
    pub(crate) fixture_kit: Kit,
}

/// Boots the app from `config`: prepares the database, restores the reference's boot-time
/// side effects, and builds the HTTP stack. Must run inside a Tokio runtime.
pub async fn boot(config: Config) -> anyhow::Result<Booted> {
    boot_with_clock(config, campfire_kit::clock::from_env()?).await
}

/// [`boot`] with its clock given rather than read from `CAMPFIRE_FROZEN_TIME`: the seeded tests
/// run at the parity seed's instant, as the reference does (`parity/seeds/README.md`).
pub async fn boot_with_clock(config: Config, clock: SharedClock) -> anyhow::Result<Booted> {
    boot_with_network(config, clock, crate::integrations::net::Network::system()).await
}

pub(crate) async fn boot_with_network(config: Config, clock: SharedClock, subscription_network: crate::integrations::net::Network) -> anyhow::Result<Booted> {
    boot_with_services(config, clock, subscription_network, jobs::periodic::Intervals::from_env()).await
}

/// Service dependencies, including which periodic hosts run beside HTTP. The Rails parity
/// server does not run bin/periodic; its seeded HTTP tests invoke due tasks explicitly.
pub(crate) async fn boot_with_services(config: Config, clock: SharedClock, subscription_network: crate::integrations::net::Network, intervals: jobs::periodic::Intervals) -> anyhow::Result<Booted> {
    let network = subscription_network.clone();
    let github_app = crate::integrations::github::client::AppClient::with_network(
        std::env::var("GITHUB_APP_CLIENT_ID").ok(),
        std::env::var("GITHUB_APP_CLIENT_SECRET").ok(),
        network.clone(),
    );
    boot_with_all_services(config, clock, crate::integrations::github::client::ReadClient::from_env(), github_app, network, subscription_network, intervals).await
}

/// Injects the fixed-host GitHub client for runtime acceptance tests.
#[cfg(test)]
pub(crate) async fn boot_with_github_read(config: Config, clock: SharedClock, github_read: crate::integrations::github::client::ReadClient) -> anyhow::Result<Booted> {
    boot_with_github_network(config, clock, github_read, crate::integrations::net::Network::system()).await
}

#[cfg(test)]
pub(crate) async fn boot_with_github_network(config: Config, clock: SharedClock, github_read: crate::integrations::github::client::ReadClient, github_network: crate::integrations::net::Network) -> anyhow::Result<Booted> {
    let github_app = crate::integrations::github::client::AppClient::with_network(
        std::env::var("GITHUB_APP_CLIENT_ID").ok(),
        std::env::var("GITHUB_APP_CLIENT_SECRET").ok(),
        github_network.clone(),
    );
    boot_with_github_clients(config, clock, github_read, github_app, github_network).await
}

#[cfg(test)]
pub(crate) async fn boot_with_github_clients(config: Config, clock: SharedClock, github_read: crate::integrations::github::client::ReadClient, github_app: crate::integrations::github::client::AppClient, github_network: crate::integrations::net::Network) -> anyhow::Result<Booted> {
    boot_with_all_services(config, clock, github_read, github_app, github_network, crate::integrations::net::Network::system(), jobs::periodic::Intervals::from_env()).await
}

pub(crate) async fn boot_with_all_services(config: Config, clock: SharedClock, github_read: crate::integrations::github::client::ReadClient, github_app: crate::integrations::github::client::AppClient, github_network: crate::integrations::net::Network, subscription_network: crate::integrations::net::Network, intervals: jobs::periodic::Intervals) -> anyhow::Result<Booted> {
    boot_with_integrations(config, clock, BootIntegrations { github_read, github_app, github_network, subscription_network, fizzy: crate::integrations::fizzy::State::system() }, intervals).await
}

/// Per-app transports, including fixture transports; production uses the shared clients.
pub(crate) struct BootIntegrations {
    pub github_read: crate::integrations::github::client::ReadClient,
    pub github_app: crate::integrations::github::client::AppClient,
    pub github_network: crate::integrations::net::Network,
    pub subscription_network: crate::integrations::net::Network,
    pub fizzy: crate::integrations::fizzy::State,
}

pub(crate) async fn boot_with_integrations(config: Config, clock: SharedClock, integrations: BootIntegrations, intervals: jobs::periodic::Intervals) -> anyhow::Result<Booted> {
    let BootIntegrations { github_read, github_app, github_network, subscription_network, fizzy } = integrations;
    config.storage.create_dirs()?;
    let secrets = Arc::new(Secrets::new(&config.secret_key_base));
    let ar_encryption = Arc::new(rails_compat::ar_encryption::ArEncryption::new(&secrets));
    let crypto: SharedCrypto = Arc::new(RailsCrypto::new(secrets.clone()));

    // The job classes first: the database's sink enqueues them on their queues.
    let mail = crate::mail::State::new(config.mail.clone());
    let registry = jobs::registry();
    let runner_config = jobs::runner_config(&config);
    let (jobs, ad_hoc) = jobs::Jobs::new(&registry, &runner_config)?;
    let loops = jobs::periodic::Loops::new(intervals);
    let rich_text = Arc::new(AppRichText::new(secrets.clone(), clock.clone()));
    // Mail's preflight and Message::create use the same room-aware, fallible renderer.
    mail.install_renderer(Arc::new({
        let rich_text = rich_text.clone();
        move |conn: &campfire_db::Connection, room: &campfire_db::Room, source: &str| {
            campfire_db::RichText::render_markdown(&*rich_text, conn, source, room.id)
                .map_err(campfire_db::Error::Other)
        }
    }));
    let db = open_database(&config, clock.clone(), jobs.clone(), rich_text.clone()).await?;
    jobs::huddle::recover_unregistered(&db).await?;

    // config/puma.rb: `Membership.disconnect_all` when the server boots.
    db.write(|tx| campfire_db::Membership::disconnect_all(tx).map(|_| ()))
        .await?;

    let storage = Arc::new(Storage::new(
        DiskService::new(&config.storage.files, "local"),
        Arc::new(ActiveStorageVerifier(rails_compat::app_verifier(
            &secrets,
            "ActiveStorage",
        ))),
    ));

    let cable_config = campfire_cable::Config {
        assume_ssl: !config.disable_ssl,
        ..campfire_cable::Config::default()
    };
    let deps = channels::Deps {
        db: db.clone(),
        secrets: secrets.clone(),
        crypto: crypto.clone(),
        clock: clock.clone(),
        admin_session_idle_timeout: config.admin_session_idle_timeout,
    };
    let cable = channels::server(deps, cable_config);

    let mut kit_config = KitConfig::production(config.disable_ssl);
    kit_config.error_pages = error_pages();
    kit_config.default_headers = crate::security::default_headers();
    kit_config.content_security_policy = Some(Arc::new(crate::security::content_security_policy(
        config.livekit_url.clone(),
    )));
    campfire_kit::param_filter::install(crate::security::parameter_filter());

    let fragment_cache = FragmentCache::new(config.fragment_cache_bytes);
    let web_push = crate::integrations::web_push_pool(&config, &db);
    let github_accounts = crate::integrations::github::accounts::Accounts::with_network(
        db.clone(), ar_encryption.clone(), github_app.clone(), github_network,
    );
    let google = crate::integrations::google::State::from_config(&config);
    let agent_repositories = crate::integrations::agent_repositories::State::live(github_accounts.clone());
    let app = Arc::new(AppState {
        config,
        secrets,
        ar_encryption,
        clock: clock.clone(),
        db,
        storage,
        broadcasts: channels::Broadcasts::new(cable.clone()),
        cable,
        jobs,
        mail,
        fizzy,
        agent_message_payload: crate::controllers::presenters::agent_payload::State::live(),
        agent_repositories,
        sudo: crate::concerns::sudo::State::default(),
        two_factor: crate::concerns::two_factor::State::default(),
        google,
        errors: crate::errors::Reporter::default(),
        web_push,
        github_read,
        github_app,
        github_accounts,
        slack_network: subscription_network.clone(),
        subscription_network,
        fragment_cache,
    });

    app.sudo.install_google(Arc::new(app.google.clone()));
    app.two_factor.install_google(Arc::new(app.google.clone()));

    let runner = jobs::start(app.clone(), registry, ad_hoc, runner_config, loops);

    let kit = Kit::new(kit_config, crypto, clock, app.clone());
    let router = router(&app, kit.clone());
    Ok(Booted {
        app,
        router,
        jobs: runner,
        #[cfg(test)]
        fixture_kit: kit,
    })
}

async fn open_database(
    config: &Config,
    clock: SharedClock,
    jobs: jobs::Jobs,
    rich_text: Arc<AppRichText>,
) -> anyhow::Result<Database> {
    let mut db_config = campfire_db::Config::new(&config.storage.database);
    db_config.readers = config.db_readers;
    db_config.environment = config.environment.clone();
    let env = campfire_db::Env {
        clock: Arc::new(DbClock(clock)),
        sink: Arc::new(jobs),
        rich_text,
        bcrypt_cost: 12,
        default_url_origin: config.mail.url_origin().to_owned(),
        message_reference_syncs: vec![crate::integrations::github::references::sync],
        user_deactivation_hooks: vec![crate::integrations::github::accounts::on_user_deactivation],
        ..Default::default()
    };
    #[cfg(test)]
    let env = campfire_db::Env {
        fixture_inputs: crate::test_support::message_inputs(),
        // WS16 flagged, per-database entropy seam for real first-login enrollment.
        fixture_auth_inputs: crate::test_support::auth_inputs(),
        ..env
    };
    Ok(tokio::task::spawn_blocking(move || Database::open(db_config, env)).await??)
}

/// The HTTP service: public files, then `/cable`, then the Rails route table.
fn router(app: &App, kit: Kit) -> Router {
    let github_webhook = || {
        axum::routing::post(campfire_kit::unparsed_action(controllers::github::webhooks::create))
            .fallback(campfire_kit::unparsed_action(controllers::github::webhooks::not_found))
    };
    let dispatch = || axum::routing::any(
        campfire_kit::action(dispatch_with_fragment_cache)
            .json_body_parser(controllers::accounts::bots::github_connections::scoped_json_body_params),
    );
    let routes = Router::new()
        .merge(
            app.cable
                .router::<Kit>(campfire_cable::protocol::DEFAULT_MOUNT_PATH),
        )
        // `post "csp_reports"`: an `ActionController::API`, outside the ApplicationController
        // routes, which reads its own body after its rate limit.
        .route(
            "/csp_reports",
            axum::routing::post(campfire_kit::unparsed_action(
                controllers::csp_reports::create,
            )),
        )
        .route(
            "/csp_reports.{format}",
            axum::routing::post(campfire_kit::unparsed_action(
                controllers::csp_reports::create,
            )),
        )
        .route(
            "/google/calendar/notifications",
            axum::routing::post(campfire_kit::unparsed_action(
                controllers::google_calendar::notifications,
            )),
        )
        .route(
            "/google/calendar/notifications.{format}",
            axum::routing::post(campfire_kit::unparsed_action(
                controllers::google_calendar::notifications,
            )),
        )
        .route("/github/webhooks", github_webhook())
        .route("/github/webhooks.{format}", github_webhook())
        .route("/agents/mcp", axum::routing::any(campfire_kit::unparsed_action(dispatch_with_fragment_cache)))
        .route("/agents/mcp.{format}", axum::routing::any(campfire_kit::unparsed_action(dispatch_with_fragment_cache)))
        // DiskController reads params before the token, but file bytes remain spooled.
        .route("/rails/active_storage/disk/{encoded_token}", axum::routing::put(campfire_kit::spooled_action(dispatch_with_fragment_cache)).fallback(campfire_kit::action(dispatch_with_fragment_cache)))
        .route("/", dispatch())
        .route("/{*path}", dispatch())
        .layer(axum::middleware::from_fn(public_files));
    #[cfg(test)]
    let routes = if app.config.environment == "test" && std::env::var("WS11UI_LEDGER_HOST").as_deref() == Ok("1") {
        routes.route("/test_session", axum::routing::get(campfire_kit::action(
            controllers::ledger_browser_tests::original_test_session,
        )))
    } else {
        routes
    };
    // config.ru: `use Rack::Deflater` around the whole app.
    campfire_kit::app(routes, kit)
        .layer(axum::middleware::from_fn(campfire_kit::deflater::deflater))
}

/// Rebuild the unchanged real HTTP stack for the original helper's late
/// configuration input, without booting another DB, Cable server or job runner.
#[cfg(test)]
pub(crate) fn fixture_router(app: &App, original: &Kit) -> (Kit, Router) {
    let mut kit_config = KitConfig::production(app.config.disable_ssl);
    kit_config.error_pages = error_pages();
    kit_config.default_headers = crate::security::default_headers();
    kit_config.content_security_policy = Some(Arc::new(crate::security::content_security_policy(
        app.config.livekit_url.clone(),
    )));
    let kit = original.fixture_rebind(kit_config, app.clone());
    let router = router(app, kit.clone());
    (kit, router)
}

/// The Rails route table, with the app's fragment cache current while the action runs.
async fn dispatch_with_fragment_cache(c: &mut Ctx) -> campfire_kit::Result {
    let cache = c.app().fragment_cache.clone();
    Scoped::new(cache, controllers::dispatch(c)).await
}

/// `ActionDispatch::Static`: serve `public/` (including digested `/assets`) before routing.
async fn public_files(request: axum::extract::Request, next: Next) -> axum::response::Response {
    match static_response(&request) {
        Some(response) => response,
        None => next.run(request).await,
    }
}

fn static_response(request: &axum::extract::Request) -> Option<axum::response::Response> {
    let header = |name| request.headers().get(name).and_then(|v| v.to_str().ok());
    let served = campfire_assets::serve(&campfire_assets::StaticRequest {
        method: request.method().as_str(),
        path: request.uri().path(),
        accept_encoding: header(axum::http::header::ACCEPT_ENCODING),
        range: header(axum::http::header::RANGE),
        if_modified_since: header(axum::http::header::IF_MODIFIED_SINCE),
    })?;
    let immutable = immutable_asset(request.uri().path(), served.status);
    let mut response =
        axum::response::Response::new(axum::body::Body::from(served.body.into_owned()));
    *response.status_mut() =
        axum::http::StatusCode::from_u16(served.status).unwrap_or(axum::http::StatusCode::OK);
    response
        .extensions_mut()
        .insert(campfire_kit::deflater::StaticFile);
    for (name, value) in served.headers {
        if let (Ok(name), Ok(value)) = (
            axum::http::HeaderName::from_bytes(name.as_bytes()),
            axum::http::HeaderValue::from_str(&value),
        ) {
            response.headers_mut().append(name, value);
        }
    }
    if immutable {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static(IMMUTABLE_CACHE_CONTROL),
        );
    }
    Some(response)
}

/// `RailsExt::ImmutableAssetHeaders::IMMUTABLE_CACHE_CONTROL`: `"public, immutable, max-age=#{1.year.to_i}"`.
const IMMUTABLE_CACHE_CONTROL: &str = "public, immutable, max-age=31556952";

/// `RailsExt::ImmutableAssetHeaders` (reference/lib/rails_ext/immutable_asset_headers.rb), which
/// wraps `ActionDispatch::Static`: a digest-stamped asset the static server found (200 or 304) can
/// never change, so it's cached as immutable. Anything else under `/assets/` falls through to the
/// app's 404 and stays correctable.
fn immutable_asset(path: &str, status: u16) -> bool {
    path.starts_with("/assets/") && matches!(status, 200 | 304)
}

/// The error pages kit renders (`ActionDispatch::PublicExceptions`), from the embedded `public/`.
fn error_pages() -> ErrorPages {
    ErrorPages::new([404, 422, 500, 502].into_iter().filter_map(|status| {
        let path = format!("/{status}.html");
        let request = campfire_assets::StaticRequest {
            method: "GET",
            path: &path,
            ..Default::default()
        };
        campfire_assets::serve(&request).map(|page| (status, page.body.into_owned().into()))
    }))
}

/// `Rails.application.message_verifier("ActiveStorage")` for campfire_storage.
struct ActiveStorageVerifier(MessageVerifier);

impl campfire_storage::Verifier for ActiveStorageVerifier {
    fn generate(
        &self,
        data_json: &str,
        purpose: &str,
        expires_at: Option<jiff::Timestamp>,
    ) -> String {
        self.0.generate_raw(data_json, Some(purpose), expires_at)
    }

    fn verified(&self, message: &str, purpose: &str, now: jiff::Timestamp) -> Option<String> {
        self.0.verify_raw(message, Some(purpose), now).ok()
    }
}

/// The process clock (frozen with `CAMPFIRE_FROZEN_TIME`) as the models' clock.
struct DbClock(SharedClock);

impl campfire_db::Clock for DbClock {
    fn now(&self) -> campfire_db::Timestamp {
        campfire_db::Timestamp::from_jiff(self.0.now())
    }
}

// --- Commands --------------------------------------------------------------------------------------

const USAGE: &str = "usage: campfire [server|backup|db-check [--immutable] DATABASE|db-migrate DATABASE MIGRATIONS_DIR|verify-additive-sqlite-migration BEFORE AFTER|twitter-backfill-references DATABASE]";

/// The binary's entry point.
///
/// - `campfire` / `campfire server`: serve the app behind the front server, as `bin/boot` did
///   with Thruster in front of `bin/start-app` (see `campfire_kit::front`).
/// - `campfire backup`: the ONCE `pre-backup` hook (`script/admin/prepare-backup`): snapshot the
///   live database into `storage/backups/` with SQLite's online backup API.
///
/// The ONCE `post-restore` hook (`ops/post-restore`) follows the reference: copy
/// `storage/backups/<env>.sqlite3` over `storage/db/<env>.sqlite3` and delete its `-wal` and
/// `-shm` files; the next boot checks the schema. Rust leaves Redis persistence for the lead's
/// explicit cutover step and supports the app's storage overrides.
pub fn run() -> anyhow::Result<()> {
    let command = std::env::args().nth(1);
    if matches!(command.as_deref(), Some("-h" | "--help")) {
        println!("{USAGE}");
        return Ok(());
    }
    let config = Config::from_env()?;
    init_logging(&config);
    match command.as_deref() {
        None | Some("server") => {
            if let Some(limit) = campfire_kit::server::raise_open_file_limit() {
                tracing::info!(limit, "open files");
            }
            tokio::runtime::Runtime::new()?.block_on(serve(config))
        }
        Some("backup") => backup(&config),
        Some(other) => anyhow::bail!("unknown command {other:?}\n{USAGE}"),
    }
}

fn init_logging(config: &Config) {
    let level = match config.log_level.to_ascii_lowercase().as_str() {
        "debug" => "debug",
        "warn" => "warn",
        "error" | "fatal" | "unknown" => "error",
        _ => "info",
    };
    // The front server logs on its own terms, as Thruster did: requests at info, more with DEBUG.
    let front = if campfire_kit::front::FrontConfig::from_env().debug {
        "debug"
    } else {
        "info"
    };
    let default = format!("{level},thruster={front},campfire_kit::front={front}");
    let filter = tracing_subscriber::EnvFilter::try_from_env("CAMPFIRE_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    // `LogScrubbingFormatter`: bot keys in paths never reach the log.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(crate::security::ScrubbingStdout)
        .try_init();
}

/// How long in-flight requests and running jobs get after SIGTERM/SIGINT. Jobs still waiting
/// stay in the queue for the next process.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// `bin/boot`'s `thrust bin/start-app`: the front server (kit's Thruster) on HTTP_PORT and, with
/// TLS_DOMAIN, HTTPS_PORT, and the app itself on TARGET_PORT.
async fn serve(config: Config) -> anyhow::Result<()> {
    let front = campfire_kit::front::FrontConfig::from_env();
    let Booted { app, router, jobs, .. } = boot(config).await?;

    let (stopping_tx, stopping) = tokio::sync::watch::channel(false);
    let cable = app.cable.clone();
    let signal = async move {
        campfire_kit::server::shutdown_signal().await;
        tracing::info!("shutting down");
        // Close every WebSocket (`server_restart`, so clients reconnect) or they'd hold the
        // graceful shutdown open.
        cable.restart();
        let _ = stopping_tx.send(true);
    };
    let server = campfire_kit::front::serve(front, router, signal);
    let deadline = async move {
        let mut stopping = stopping;
        let _ = stopping.wait_for(|stopping| *stopping).await;
        tokio::time::sleep(SHUTDOWN_GRACE).await;
    };
    tokio::select! {
        result = server => result?,
        _ = deadline => tracing::warn!("requests still running at shutdown were abandoned"),
    }
    // Jobs first: pushing a message queues its notifications on the Web Push pool.
    jobs.shutdown(SHUTDOWN_GRACE).await;
    if let Some(web_push) = &app.web_push {
        web_push.shutdown().await;
    }
    Ok(())
}

/// `script/admin/prepare-backup`: `SQLite3::Backup` of the live database, all pages in one step,
/// into `storage/backups/<database file name>`.
pub fn backup(config: &Config) -> anyhow::Result<()> {
    let destination = config.storage.backup_file();
    if let Some(dir) = destination.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Written to a file of its own beside the destination and renamed over it, so a failed,
    // interrupted or concurrent backup never leaves a torn file where ONCE (and `post-restore`)
    // expect the last good one. A failed one's file is deleted when `partial` drops.
    let dir = destination.parent().unwrap_or(std::path::Path::new("."));
    let partial = tempfile::Builder::new()
        .prefix(".backup-")
        .suffix(".sqlite3")
        .tempfile_in(dir)?;
    copy_database(&config.storage.database, partial.path())?;
    partial.persist(&destination)?;
    tracing::info!(path = %destination.display(), "backup written");
    Ok(())
}

/// SQLite's online backup of the live database at `source` into a new file at `target`.
fn copy_database(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    let source =
        rusqlite::Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    source.busy_timeout(Duration::from_secs(5))?;
    let mut target = rusqlite::Connection::open(target)?;
    let backup = rusqlite::backup::Backup::new(&source, &mut target)?;
    // `backup.step(-1)`: every page in one step; a busy or locked source is retried.
    let mut attempts = 0;
    loop {
        match backup.step(-1)? {
            rusqlite::backup::StepResult::Done => return Ok(()),
            _ if attempts < 50 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            other => anyhow::bail!("backup did not finish: {other:?}"),
        }
    }
}

#[cfg(test)]
mod security_tests;

#[cfg(test)]
mod sudo_tests;

#[cfg(test)]
mod two_factor_tests;

#[cfg(test)]
mod challenge_tests;

#[cfg(test)]
mod enforcement_tests;
#[cfg(test)]
mod direct_upload_tests;

#[cfg(test)]
mod session_management_tests;

#[cfg(test)]
mod admin_two_factor_tests;
#[cfg(test)]
mod full_page_tests;
#[cfg(test)]
mod profile_security_tests;
#[cfg(test)]
mod round_four_security_tests;
#[cfg(test)]
mod round_three_security_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod google_tests;

#[cfg(test)]
mod google_webhook_tests;

#[cfg(test)]
pub(crate) mod google_api_tests;

#[cfg(test)]
mod google_connection_tests;

#[cfg(test)]
mod google_drive_tests;

#[cfg(test)]
mod google_calendar_job_tests;
#[cfg(test)]
mod google_meeting_refresh_tests;
#[cfg(test)]
mod google_push_channel_tests;

#[cfg(test)]
pub(crate) mod google_test_support;

#[cfg(test)]
#[path = "../../../test-support/asset_goldens.rs"]
pub(crate) mod asset_goldens;

#[cfg(test)]
mod google_review_tests;
#[cfg(test)]
mod google_consumer_tests;

#[cfg(test)]
mod google_lifecycle_tests;

#[cfg(test)]
mod google_admin_tests;

#[cfg(test)]
mod google_page_tests;

#[cfg(test)]
mod google_reporting_tests;

#[cfg(test)]
mod google_message_tests;

#[cfg(test)]
pub(crate) mod cutover_c_tests;

#[cfg(test)]
mod cutover_d_tests;

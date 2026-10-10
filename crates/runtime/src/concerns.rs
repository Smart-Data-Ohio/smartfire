//! `ApplicationController`'s concerns as before-action functions on [`Ctx`].
//!
//! `ApplicationController` does `include AllowBrowser, Authentication, Authorization,
//! BlockBannedRequests, SetCurrentRequest, SetPlatform, TrackedRoomVisit, VersionHeaders`. Ruby
//! includes a list right to left, so the `included` hooks (and their `before_action`s) run in
//! reverse, and the chain the reference actually runs is (dumped with
//! `reference-tools/campfire/callbacks.rb`):
//!
//! 1. `set_version_headers` (VersionHeaders)
//! 2. `Current.request = request` (SetCurrentRequest)
//! 3. `reject_banned_ip`, unless GET/HEAD (BlockBannedRequests)
//! 4. `require_authentication` (Authentication)
//! 5. `deny_bots` (Authentication)
//! 6. `deny_agent_tokens` (Authentication)
//! 7. `verify_authenticity_token`, unless authenticated by bot key, bot reply token or agent token
//!    (Authentication's `protect_from_forgery with: :exception, unless: -> { ... }`)
//! 8. `allow_browser` (AllowBrowser)
//!
//! then the controller's own before-actions. [`before_actions`] runs 1–8 with a controller's
//! skips ([`Before`]); controllers then call their own (`set_room`, `ensure_can_administer`, ...)
//! in declaration order. Everything returns `Err(Error::Halt(..))` to stop the chain the way a
//! Rails callback that renders or redirects does, so actions just use `?`:
//!
//! ```ignore
//! pub async fn show(c: &mut Ctx) -> Result {
//!     before_actions(c, Before::default()).await?;
//!     let room = set_room(c).await?;          // RoomScoped
//!     ...
//! }
//! ```
//!
//! Our Rails app's `ApplicationController` also includes `SetTimeZone`, `SudoMode` and
//! `TwoFactorEnforcement` (`require_two_factor_enrollment`). Sudo gates and the second-factor
//! controllers and global enrollment enforcement are ported. The session state they keep is in
//! [`session_keys`].
//!
//! Current attributes (`Current.user`, `Current.session`) live in the `Ctx` extensions; read them
//! with [`current_user`] / [`current_session`].

// The frame later controller ports build on; parts are unused until they land.

pub mod platform;
pub mod session_keys;
pub mod sudo;
pub mod two_factor;
pub mod user_agent;
pub mod agent_api;

#[derive(Clone, Copy)]
pub struct CurrentAgent {
    pub agent_id: i64,
    pub credential_id: i64,
}

use campfire_db::{Ban, Membership, NewSession, PasswordDigest, Room, Session, User};
use campfire_kit::{Cookie, Ctx, Error, Result, SameSite, StatusCode, halt};

use crate::app::AppCtx;
pub use crate::ruby::ruby_to_i;

/// The route that matched the current request (`request.path_parameters` plus the endpoint),
/// available to actions as `c.current::<MatchedRoute>()`.
#[derive(Debug, Clone)]
pub struct MatchedRoute {
    pub endpoint: &'static str,
}

// --- Current -----------------------------------------------------------------------------------

/// `Current.user`
#[derive(Debug, Clone)]
pub struct CurrentUser(pub User);

/// `Current.session`
#[derive(Debug, Clone)]
pub struct CurrentSession(pub Session);

/// `authenticated_by` (`"".inquiry`, `"session"`, `"bot_key"`, `"bot_reply"` or `"agent_token"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthenticatedBy {
    #[default]
    Nothing,
    Session,
    BotKey,
    BotReply,
    AgentToken,
}

impl AuthenticatedBy {
    /// The key-authenticated callers `protect_from_forgery` leaves alone: they send no cookies, so
    /// there's no session to forge.
    pub fn skips_forgery_protection(self) -> bool {
        matches!(self, Self::BotKey | Self::BotReply | Self::AgentToken)
    }
}

pub fn current_user(c: &Ctx) -> Option<&User> {
    c.current::<CurrentUser>().map(|current| &current.0)
}

/// `Current.user` where a before-action guarantees one (after `require_authentication`).
pub fn require_current_user(c: &Ctx) -> Result<&User> {
    current_user(c).ok_or_else(|| Error::internal(anyhow::anyhow!("no Current.user")))
}

pub fn current_session(c: &Ctx) -> Option<&Session> {
    c.current::<CurrentSession>().map(|current| &current.0)
}

/// `signed_in?`
pub fn signed_in(c: &Ctx) -> bool {
    current_user(c).is_some()
}

pub fn authenticated_by(c: &Ctx) -> AuthenticatedBy {
    c.current::<AuthenticatedBy>().copied().unwrap_or_default()
}

fn set_authenticated_by(c: &mut Ctx, by: AuthenticatedBy) {
    c.set_current(by);
}

// --- The ApplicationController chain ---------------------------------------------------------

/// How a controller's `allow_unauthenticated_access` / `require_unauthenticated_access` /
/// `allow_bot_access` / `allow_agent_access` / `skip_forgery_protection` change the chain for one
/// action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Before {
    pub authentication: Authentication,
    /// `deny_bots` (skipped by `allow_bot_access`).
    pub deny_bots: bool,
    /// `deny_agent_tokens` (skipped by `allow_agent_access`).
    pub deny_agent_tokens: bool,
    /// `verify_authenticity_token` (skipped by `skip_forgery_protection`).
    pub forgery_protection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authentication {
    /// `require_authentication`
    Required,
    /// MembersController overrides request_authentication with an empty JSON 401.
    JsonUnauthorized,
    /// `allow_unauthenticated_access`: skip `require_authentication`.
    Skipped,
    /// `require_unauthenticated_access`: skip `require_authentication`, then (after the rest of
    /// the chain, as it's declared in the subclass) `restore_authentication` and
    /// `redirect_signed_in_user_to_root`.
    RequireUnauthenticated,
}

impl Default for Before {
    fn default() -> Self {
        Self {
            authentication: Authentication::Required,
            deny_bots: true,
            deny_agent_tokens: true,
            forgery_protection: true,
        }
    }
}

impl Before {
    pub fn allow_unauthenticated_access(self) -> Self {
        Self {
            authentication: Authentication::Skipped,
            ..self
        }
    }

    pub fn require_unauthenticated_access(self) -> Self {
        Self {
            authentication: Authentication::RequireUnauthenticated,
            ..self
        }
    }

    pub fn allow_bot_access(self) -> Self {
        Self {
            deny_bots: false,
            ..self
        }
    }

    #[allow(dead_code)] // for the agent API controllers
    pub fn allow_agent_access(self) -> Self {
        Self {
            deny_agent_tokens: false,
            ..self
        }
    }

    pub fn skip_forgery_protection(self) -> Self {
        Self {
            forgery_protection: false,
            ..self
        }
    }
}

/// `ApplicationController`'s before-actions, in the order the reference runs them.
pub async fn before_actions(c: &mut Ctx, before: Before) -> Result<()> {
    before_actions_with_authentication(c, before, None, None).await
}

/// Subclass overrides of `request_authentication` and `deny_bots`, preserving their
/// positions in the callback chain (including authentication before CSRF).
pub async fn before_actions_with_authentication(
    c: &mut Ctx,
    before: Before,
    request_authentication_override: Option<fn(&mut Ctx) -> Result<()>>,
    deny_bots_override: Option<fn(&mut Ctx) -> Result<()>>,
) -> Result<()> {
    set_version_headers(c);
    set_current_request(c);
    reject_banned_ip(c).await?;
    if before.authentication == Authentication::Required {
        if let Some(request_authentication) = request_authentication_override {
            if !(restore_authentication(c).await?
                || bot_authentication(c).await?
                || agent_authentication(c).await?)
            {
                request_authentication(c)?;
            }
        } else {
            require_authentication(c).await?;
        }
    }
    if before.authentication == Authentication::JsonUnauthorized
        && !(restore_authentication(c).await? || bot_authentication(c).await? || agent_authentication(c).await?)
    {
        if c.format()?.is_some_and(|format| format.is("json")) {
            return halt(head(StatusCode::UNAUTHORIZED));
        }
        request_authentication(c).await?;
    }
    if before.deny_bots {
        deny_bots_override.unwrap_or(deny_bots)(c)?;
    }
    if before.deny_agent_tokens {
        deny_agent_tokens(c)?;
    }
    let forgery_protection = before.forgery_protection;
    if forgery_protection && !authenticated_by(c).skips_forgery_protection() {
        c.verify_authenticity_token()?;
    }
    allow_browser(c).await?;
    enforce_two_factor_for_restored_session(c).await?;
    if before.authentication == Authentication::RequireUnauthenticated {
        restore_authentication(c).await?;
        redirect_signed_in_user_to_root(c)?;
    }
    redirect_to_spa(c).await?;
    Ok(())
}

// --- The new UI ----------------------------------------------------------------------------------

/// Whether this person uses the new UI: everyone does, whatever `ui_preference` once stored.
/// Only a test of the classic pages (`Config::spa_enabled` off) says no.
pub async fn next_ui(c: &Ctx, _user: &User) -> Result<bool> {
    Ok(c.app().config.spa_enabled)
}

/// The UI this request uses: the SPA, unless a classic page test turned it off.
pub async fn effective_ui(c: &Ctx) -> Result<campfire_db::models::user::ui_preference::UiPreference> {
    use campfire_db::models::user::ui_preference::UiPreference;
    Ok(if c.app().config.spa_enabled { UiPreference::Next } else { UiPreference::Classic })
}

/// Both UIs update the same registration at scope `/`.
pub fn service_worker_url(_ui: campfire_db::models::user::ui_preference::UiPreference) -> String {
    "/service-worker.js".into()
}

/// The SPA URL for `endpoint` at `path`.
pub fn ported_page(endpoint: &str, path: &str, query: Option<&str>) -> Option<String> {
    campfire_spa::screens::spa_url(endpoint, path, query)
}

/// A signed-in person who opens a classic page the SPA has (`campfire_spa::screens`) goes to that
/// page's SPA URL, with a 302: the old URLs (bookmarks, push and mail links) are aliases. Only for
/// a signed-in person's `GET` or `HEAD` that navigates to an HTML page ([`navigates`]): not a
/// Turbo frame's, a script's, a JSON request or a bare `fetch()`. A `classic` query parameter is
/// dropped, not obeyed. A notice or alert left for the page is kept for the SPA, whose shell shows
/// it. It runs last in the chain, so signing in, two-step enforcement and the rest come first, and
/// a page's own checks (room access) are the SPA's to make.
pub async fn redirect_to_spa(c: &mut Ctx) -> Result<()> {
    use campfire_kit::format;

    let config = &c.app().config;
    if !config.spa_enabled || !(c.request.is_get() || c.request.is_head()) {
        return Ok(());
    }
    if authenticated_by(c) != AuthenticatedBy::Session || c.is_turbo_frame_request() || c.request.is_xhr() {
        return Ok(());
    }
    let Some(endpoint) = c.current::<MatchedRoute>().map(|route| route.endpoint) else {
        return Ok(());
    };
    let query = Some(c.request.query_string()).filter(|query| !query.is_empty());
    let screen_path = screen_path(c.request.path());
    let location = if endpoint == "rooms#show" {
        let confirmed = confirmed_room_query(c, screen_path, query).await?;
        campfire_spa::screens::spa_url_confirmed(endpoint, screen_path, query, confirmed)
    } else {
        campfire_spa::screens::profile_url(endpoint, screen_path, query, require_current_user(c)?.id)
    };
    let Some(location) = location else {
        return Ok(());
    };
    if !matches!(c.format()?, Some(f) if f == &format::HTML || f == &format::ALL) || !navigates(c) {
        return Ok(());
    }
    keep_waiting_flash(c);
    let location = c.url_for(&location);
    halt(c.redirect_to(&location)?)
}

/// `path` as the screen map spells it: the router takes `/rooms/7.html` for `/rooms/7` (the
/// `(.:format)` suffix), so a `.html` suffix is dropped. Other suffixes (`.json`) stay, so they
/// match no screen, and the format check would refuse them anyway.
fn screen_path(path: &str) -> &str {
    path.strip_suffix(".html").unwrap_or(path)
}

/// A notice or alert a classic action left for this page (`redirect_to ..., notice:`) carries on
/// to the SPA page this request is sent to, whose shell shows it.
pub fn keep_waiting_flash(c: &mut Ctx) {
    if !c.peek_flash().is_empty() {
        c.flash().keep(None);
    }
}

/// Whether this signed-in HTML navigation goes to the SPA, apart from which screen it is. The same
/// gates as [`redirect_to_spa`]: the request navigates, by a person's session. A waiting flash is
/// kept for the next hop.
pub async fn coexistence_wants_spa(c: &mut Ctx) -> Result<bool> {
    if authenticated_by(c) != AuthenticatedBy::Session || !coexistence_navigation(c)? {
        return Ok(false);
    }
    next_ui(c, require_current_user(c)?).await
}

/// The navigation gates for the SPA, before checking a person's session. A waiting flash is kept
/// for the next hop.
pub fn coexistence_navigation(c: &mut Ctx) -> Result<bool> {
    use campfire_kit::format;

    let config = &c.app().config;
    if !config.spa_enabled || !(c.request.is_get() || c.request.is_head()) {
        return Ok(false);
    }
    if c.is_turbo_frame_request() || c.request.is_xhr() {
        return Ok(false);
    }
    if !matches!(c.format()?, Some(kind) if kind == &format::HTML || kind == &format::ALL) || !navigates(c) {
        return Ok(false);
    }
    keep_waiting_flash(c);
    Ok(true)
}

/// `thread` and `message_id` the viewer can open in the room `path` names. Anything else (another
/// room, a room they aren't in, a deleted row) is left off, so the redirect stays on the plain
/// room route.
async fn confirmed_room_query(
    c: &Ctx,
    path: &str,
    query: Option<&str>,
) -> Result<campfire_spa::screens::ConfirmedRoomQuery> {
    let none = campfire_spa::screens::ConfirmedRoomQuery { thread: None, message: None };
    let Some(room_id) = campfire_spa::screens::room_show_id(path) else {
        return Ok(none);
    };
    let ids = campfire_spa::screens::room_query_ids(query);
    if ids.thread.is_none() && ids.message.is_none() {
        return Ok(none);
    }
    let Some(user_id) = current_user(c).map(|user| user.id) else {
        return Ok(none);
    };
    let thread_id = ids.thread.map(|id| id as i64);
    let message_id = ids.message.map(|id| id as i64);
    c.app().db.read(move |conn| {
        if Room::find_for_user(conn, user_id, room_id)?.is_none() {
            return Ok(none);
        }
        let thread = match thread_id {
            Some(id) => campfire_db::ChannelThread::find_by_id(conn, id)?
                .filter(|thread| thread.room_id == room_id)
                .map(|thread| thread.id as u64),
            None => None,
        };
        let message = match message_id {
            Some(id) => campfire_db::Message::find_by_id(conn, id)?.and_then(|message| {
                if message.room_id != room_id {
                    return None;
                }
                let on_thread = thread.is_some_and(|thread| message.thread_id == Some(thread as i64));
                let on_timeline = thread.is_none() && message.thread_id.is_none();
                (on_thread || on_timeline).then_some(message.id as u64)
            }),
            None => None,
        };
        Ok(campfire_spa::screens::ConfirmedRoomQuery { thread, message })
    }).await.map_err(Error::internal)
}

/// A browser opening a page: `Sec-Fetch-Mode: navigate` outside a frame, or, from a client that
/// sends no Fetch Metadata, an `Accept` naming `text/html`. A `fetch()` or `curl` with the session
/// cookie and `Accept: */*`, or a script's `fetch()` of HTML (`Sec-Fetch-Mode: cors`), isn't one,
/// so it isn't redirected.
fn navigates(c: &Ctx) -> bool {
    // A browser that sends Fetch Metadata says what the request is: only a top-level navigation
    // counts, whatever it accepts (a script's `fetch()` or a frame's load doesn't). The SPA's
    // service worker forwards a navigation with `Sec-Fetch-Dest: empty`, so any destination but a
    // nested browsing context's counts.
    if let Some(mode) = c.request.header("sec-fetch-mode") {
        let nested = c.request.header("sec-fetch-dest").is_some_and(|dest| {
            ["iframe", "frame", "fencedframe", "embed", "object"]
                .iter()
                .any(|nested| dest.eq_ignore_ascii_case(nested))
        });
        return mode.eq_ignore_ascii_case("navigate") && !nested;
    }
    c.request.header("accept").is_some_and(|accept| accept.to_ascii_lowercase().contains("text/html"))
}

// --- VersionHeaders ----------------------------------------------------------------------------

/// `set_version_headers`: `X-Version` and `X-Rev` (`config/initializers/version.rb`).
pub fn set_version_headers(c: &mut Ctx) {
    let config = &c.app().config;
    let (version, revision) = (config.app_version.clone(), config.git_revision.clone());
    c.set_header("x-version", &version);
    // Rails drops a header set to nil.
    if let Some(revision) = revision {
        c.set_header("x-rev", &revision);
    }
}

// --- SetCurrentRequest -------------------------------------------------------------------------

/// `Current.request = request`. `default_url_options` then carry the request's host and
/// protocol, which is what `Ctx::url_for` already does, so there's nothing to store.
pub fn set_current_request(_c: &mut Ctx) {}

// --- BlockBannedRequests -----------------------------------------------------------------------

/// `reject_banned_ip`, unless the request is safe (GET/HEAD): 429 for a banned `remote_ip`.
pub async fn reject_banned_ip(c: &mut Ctx) -> Result<()> {
    if c.request.is_get() || c.request.is_head() {
        return Ok(());
    }
    let ip = c.request.remote_ip()?.to_string();
    let banned = c
        .app()
        .db
        .read(move |conn| Ban::banned(conn, &ip))
        .await
        .map_err(Error::internal)?;
    if banned {
        return halt(head(StatusCode::TOO_MANY_REQUESTS));
    }
    Ok(())
}

// --- Authentication ----------------------------------------------------------------------------

/// `Authentication::SessionLookup#find_session_by_cookie`
pub async fn find_session_by_cookie(c: &Ctx) -> Result<Option<Session>> {
    let Some(token) = c.cookies.signed("session_token") else {
        return Ok(None);
    };
    c.app()
        .db
        .read(move |conn| Session::find_by_token(conn, &token))
        .await
        .map_err(Error::internal)
}

/// `require_authentication`: `restore_authentication || bot_authentication ||
/// agent_authentication || request_authentication`.
pub async fn require_authentication(c: &mut Ctx) -> Result<()> {
    if restore_authentication(c).await?
        || bot_authentication(c).await?
        || agent_authentication(c).await?
    {
        return Ok(());
    }
    request_authentication(c).await
}

/// `restore_authentication`: resume the session named by the `session_token` cookie, unless it
/// has expired ([`session_expired`]), in which case it's destroyed and its cookie dropped.
pub async fn restore_authentication(c: &mut Ctx) -> Result<bool> {
    let Some(token) = c.cookies.signed("session_token") else {
        return Ok(false);
    };
    let now = c.now();
    let preload_layout = matches!(
        c.request.method,
        campfire_kit::Method::GET | campfire_kit::Method::HEAD
    ) && (c.request.path().starts_with("/agents")
        || c.request.path().starts_with("/account/bots")
        || c.request.path().starts_with("/users/"));
    let found = c
        .app()
        .db
        .read(move |conn| {
            let Some(session) = Session::find_by_token(conn, &token)? else {
                return Ok(None);
            };
            let user = User::find_by_id_with(conn, session.user_id, |row| {
                if preload_layout {
                    crate::controllers::presenters::layout_preferences::LoadedPreferences::from_row(
                        row, now,
                    )
                    .map(Some)
                } else {
                    Ok(None)
                }
            })?;
            Ok(Some((session, user)))
        })
        .await
        .map_err(Error::internal)?;
    let Some((session, user)) = found else {
        return Ok(false);
    };
    let (user, preferences) = match user {
        Some((user, preferences)) => (Some(user), preferences),
        None => (None, None),
    };
    let now = campfire_db::Timestamp::from_jiff(c.now());
    if user.as_ref().is_some_and(|user| {
        session_expired(
            &session,
            user,
            c.app().config.admin_session_idle_timeout,
            now,
        )
    }) {
        c.app()
            .db
            .write(move |tx| session.destroy(tx))
            .await
            .map_err(Error::internal)?;
        c.cookies.delete("session_token");
        return Ok(false);
    }
    if let Some(preferences) = preferences {
        c.set_current(preferences);
    }
    resume_session(c, session, user).await?;
    enforce_two_factor_for_restored_session(c).await?;
    Ok(true)
}

/// `Session#expired?`: administrators' sessions end after `ADMIN_SESSION_IDLE_TIMEOUT_DAYS`
/// without activity (`config/initializers/session_lifetimes.rb`); members' never do. Checked when
/// a request restores the session and when a cable connects.
pub fn session_expired(
    session: &Session,
    user: &User,
    idle_timeout: jiff::SignedDuration,
    now: campfire_db::Timestamp,
) -> bool {
    user.is_administrator() && session.last_active_at < now.ago(idle_timeout)
}

/// What a presence heartbeat does first (`WorkspacePresenceChannel#heartbeat`): end the session
/// if it has idled past [`session_expired`]. A session that's already gone is left alone.
pub fn expire_idle_timed_out_session(
    tx: &mut campfire_db::Tx<'_>,
    session_id: i64,
    timeout: jiff::SignedDuration,
) -> campfire_db::Result<()> {
    let fresh = match Session::find(tx.conn(), session_id) {
        Ok(session) => session,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(()),
        Err(error) => return Err(error),
    };
    let user = User::find(tx.conn(), fresh.user_id)?;
    if session_expired(&fresh, &user, timeout, tx.now()) {
        fresh.destroy(tx)?;
    }
    Ok(())
}

/// `TwoFactorEnforcement#require_two_factor_enrollment`. The callback and every late restore
/// share this gate. Verified sessions do not query the credential; key authentication is exempt.
pub async fn enforce_two_factor_for_restored_session(c: &mut Ctx) -> Result<()> {
    let Some(session) = current_session(c) else { return Ok(()) };
    let Some(user) = current_user(c) else { return Ok(()) };
    if authenticated_by(c) != AuthenticatedBy::Session || !user.requires_two_factor() {
        return Ok(());
    }
    // Use the trusted dispatcher endpoint, never query/body controller or action parameters.
    let endpoint = c.current::<MatchedRoute>().map(|r| r.endpoint);
    if endpoint.is_some_and(|endpoint| {
        let (controller, action) = endpoint.split_once('#').unwrap_or((endpoint, ""));
        matches!(controller, "two_factor/challenges" | "pwa")
            || (controller == "two_factor/setups" && matches!(action, "show" | "create"))
            || (controller == "sessions" && action == "destroy")
    }) || session.two_factor_verified() {
        return Ok(());
    }
    let user = user.clone();
    let enabled = c.app().db.read(move |conn| user.two_factor_enabled(conn)).await.map_err(Error::internal)?;
    let html = c.format()?.is_some_and(|f| f.symbol == "html" || f.string.contains("html"));
    if enabled {
        terminate_current_session(c).await?;
        if html {
            return halt(c.redirect_to_with(&c.url_for("/session/new"), campfire_kit::Redirect {
                alert: Some("Sign in again to verify two-step sign-in.".into()),
                ..Default::default()
            })?);
        }
        halt(head(StatusCode::UNAUTHORIZED))
    } else if html {
        if c.request.is_get() {
            let url = c.request.url();
            c.session().insert(session_keys::RETURN_TO_KEY, url);
        }
        halt(c.redirect_to(&c.url_for("/two_factor_setup"))?)
    } else {
        halt(head(StatusCode::FORBIDDEN))
    }
}

/// `bot_authentication`: `params[:bot_key].present?` and a matching active bot.
pub async fn bot_authentication(c: &mut Ctx) -> Result<bool> {
    let Some(param) = c.params.get("bot_key").filter(|p| p.is_present()) else {
        return Ok(false);
    };
    // `params[:bot_key].strip` raises NoMethodError for a hash or array.
    let Some(bot_key) = param.as_str().map(|key| ruby_strip(key).to_string()) else {
        return Err(Error::internal(anyhow::anyhow!(
            "undefined method 'strip' for bot_key"
        )));
    };
    // `params[:room_id]` for `authenticate_bot_reply_token`, compared `to_s`: nil is "", and a
    // hash or array never matches a signed room id.
    let room_id = match c.params.get("room_id") {
        None => Some(String::new()),
        Some(param) => param.as_str().map(str::to_string),
    };
    let (secrets, now) = (c.app().secrets.clone(), c.now());
    let bot = c
        .app()
        .db
        .read(move |conn| {
            if let Some(bot) = User::authenticate_bot(conn, &bot_key)? {
                return Ok(Some((bot, AuthenticatedBy::BotKey)));
            }
            let Some(room_id) = room_id else {
                return Ok(None);
            };
            Ok(
                authenticate_bot_reply_token(conn, &secrets, &bot_key, &room_id, now)?
                    .map(|bot| (bot, AuthenticatedBy::BotReply)),
            )
        })
        .await
        .map_err(Error::internal)?;
    match bot {
        Some((bot, by)) => {
            c.set_current(CurrentUser(bot));
            set_authenticated_by(c, by);
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `User.authenticate_bot_reply_token(token, room_id:)`: a webhook reply token that verifies for
/// this room (`rails_compat::verifiers::bot_reply`), naming an active bot (`active_bots.find_by(id:)`)
/// that is still a member of the room (`bot.rooms.exists?(room_id)`).
pub fn authenticate_bot_reply_token(
    conn: &campfire_db::Connection,
    secrets: &rails_compat::Secrets,
    token: &str,
    room_id: &str,
    now: jiff::Timestamp,
) -> campfire_db::Result<Option<User>> {
    let Some(bot_id) = rails_compat::verifiers::bot_reply::verify(secrets, token, room_id, now)
    else {
        return Ok(None);
    };
    // Active Record casts the id: an integer, or a string of digits.
    let bot_id = bot_id
        .as_i64()
        .or_else(|| bot_id.as_str().and_then(|id| id.trim().parse().ok()));
    let (Some(bot_id), Ok(room_id)) = (bot_id, room_id.parse::<i64>()) else {
        return Ok(None);
    };
    let bot = match User::find_active_bot(conn, bot_id) {
        Ok(bot) => bot,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Membership::find_by_room_and_user(conn, room_id, bot.id)?.map(|_| bot))
}

/// Bearer authentication uses the persisted credential and the agent's current active state.
/// Unknown, revoked, expired and suspended credentials halt with 401; endpoint policy then
/// denies a valid agent token on every controller that hasn't opted into agent access.
pub async fn agent_authentication(c: &mut Ctx) -> Result<bool> {
    let Some(secret) = agent_bearer_secret(c.request.header("authorization")).map(str::to_string) else { return Ok(false) };
    let ip = c.request.remote_ip()?.to_string();
    let identity = c.app().db.write(move |tx| campfire_db::models::agent_access::authenticate_identity(tx, &secret, &ip)).await.map_err(Error::internal)?;
    let Some(identity) = identity else { return halt(head(StatusCode::UNAUTHORIZED)) };
    c.set_current(CurrentAgent { agent_id: identity.agent_id, credential_id: identity.credential_id });
    c.set_current(CurrentUser(identity.user));
    set_authenticated_by(c, AuthenticatedBy::AgentToken);
    Ok(true)
}

/// AgentAuthorization: membership is checked by each controller first, with 404. Missing
/// capabilities return the same JSON 403 for bot keys, agent tokens and bot sessions.
pub async fn ensure_agent_capability(c: &mut Ctx, capability: &'static str, room_id: i64) -> Result<()> {
    let user = require_current_user(c)?;
    if !user.is_bot() && !matches!(authenticated_by(c), AuthenticatedBy::BotKey | AuthenticatedBy::AgentToken) {
        return Ok(());
    }
    let user_id = user.id;
    let allowed = c.app().db.read(move |conn| campfire_db::models::agent_access::capability_for_user(conn,user_id,capability,room_id)).await.map_err(Error::internal)?;
    if allowed == Some(false) {
        let body = serde_json::json!({"error":format!("Forbidden: agent lacks {capability} capability")}).to_string();
        return halt(c.render(StatusCode::FORBIDDEN, &campfire_kit::format::JSON, body));
    }
    Ok(())
}

/// `agent_bearer_secret`: `scheme, token = request.authorization.to_s.split(" ", 2)`, then the
/// token (stripped) if the scheme is `Bearer` in any case.
pub fn agent_bearer_secret(authorization: Option<&str>) -> Option<&str> {
    // `split(" ", 2)` is awk-style: leading whitespace skipped, then one run of it separates.
    let authorization = authorization?.trim_start_matches(ruby_space);
    let (scheme, token) = authorization.split_once(ruby_space)?;
    let token = token.trim_start_matches(ruby_space);
    if !scheme.eq_ignore_ascii_case("Bearer") || token.chars().all(char::is_whitespace) {
        return None;
    }
    Some(ruby_strip(token)).filter(|token| !token.is_empty())
}

/// The whitespace Ruby's awk-style `split(" ")` splits on.
fn ruby_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

/// `request_authentication`: someone waiting on their second factor goes back to the challenge;
/// anyone else to sign in, remembering the page they were after.
pub async fn request_authentication(c: &mut Ctx) -> Result<()> {
    if two_factor_pending_user(c).await?.is_some() {
        let location = c.url_for("/two_factor_challenge");
        return halt(c.redirect_to(&location)?);
    }
    // Only page navigations bounce back after sign in: not background polls (JSON, Turbo Stream
    // refreshes) or form submissions.
    if bounce_back_after_sign_in(c)? {
        let url = c.request.url();
        c.session().insert(session_keys::RETURN_TO_KEY, url);
    }
    let location = c.url_for(&campfire_routes::new_session());
    halt(c.redirect_to(&location)?)
}

/// `bounce_back_after_sign_in?`: `request.get? && request.format.html? &&
/// !request.format.turbo_stream?`. (Turbo Stream's MIME type contains "html", which is what
/// `html?` checks.)
fn bounce_back_after_sign_in(c: &mut Ctx) -> Result<bool> {
    if !c.request.is_get() {
        return Ok(false);
    }
    let format = c.format()?;
    Ok(format.is_some_and(|format| {
        (format.symbol == "html" || format.string.contains("html"))
            && format.symbol != "turbo_stream"
    }))
}

/// `two_factor_pending_user`: the active user a still-valid pending second factor names.
pub async fn two_factor_pending_user(c: &mut Ctx) -> Result<Option<User>> {
    let now = c.now();
    let Some(user_id) = session_keys::two_factor_pending_user_id(c.session(), now) else {
        return Ok(None);
    };
    c.app()
        .db
        .read(move |conn| match User::find_active(conn, user_id) {
            Ok(user) => Ok(Some(user)),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)
}

/// `redirect_signed_in_user_to_root`
pub fn redirect_signed_in_user_to_root(c: &mut Ctx) -> Result<()> {
    if signed_in(c) {
        let root = c.url_for(&campfire_routes::root());
        return halt(c.redirect_to(&root)?);
    }
    Ok(())
}

/// `has_secure_password`'s `password=`, hashed on the blocking pool ahead of the write that saves
/// it, so bcrypt (about 250 ms) holds neither an async thread nor the database writer.
pub async fn password_digest(c: &Ctx, password: Option<String>) -> Result<Option<PasswordDigest>> {
    let Some(password) = password else {
        return Ok(None);
    };
    PasswordDigest::hash(password, c.app().db.env().bcrypt_cost)
        .await
        .map(Some)
        .map_err(Error::internal)
}

/// `User.active.authenticate_by(email_address:, password:)`: the user is looked up on a reader,
/// and the password checked once the reader is released.
pub async fn authenticate_by(
    c: &Ctx,
    email_address: String,
    password: String,
) -> Result<Option<User>> {
    // `authenticate_by` returns nil for a blank password before looking anything up.
    if password.is_empty() {
        return Ok(None);
    }
    let candidate = c
        .app()
        .db
        .read(move |conn| User::find_active_by_email_address(conn, &email_address))
        .await
        .map_err(Error::internal)?;
    tokio::task::spawn_blocking(move || User::authenticated(candidate, &password))
        .await
        .map_err(Error::internal)
}

/// `start_new_session_for(user)`: drop the previous member's confirmations, make sure the browser
/// has its `device_id`, start the session, and settle the CSRF token before any page renders (so
/// concurrent first renders don't each start their own).
///
/// Rails' `two_factor_verified:` defaults to false; the completed challenge and remembered-device
/// flows pass true through `start_new_verified_session_for`. Every real session records its
/// device and alerts on a new device, using WS10's typed mail API when configured.
pub async fn start_new_session_for(c: &mut Ctx, user: User) -> Result<Session> {
    start_session(c, user, false).await
}

/// A completed second factor (or valid remembered device) creates a verified session.
pub async fn start_new_verified_session_for(c: &mut Ctx, user: User) -> Result<Session> {
    start_session(c, user, true).await
}

async fn start_session(c: &mut Ctx, user: User, two_factor_verified: bool) -> Result<Session> {
    session_keys::clear_confirmations(c.session());
    let device_id = ensure_device_cookie(c)?;
    let (user_agent, ip) = (
        c.request.user_agent().map(str::to_string),
        c.request.remote_ip()?.to_string(),
    );
    let user_id = user.id;
    let notify = c.app().mail.config.security_configured();
    let session = c
        .app()
        .db
        .write(move |tx| {
            let attributes = NewSession {
                user_agent: user_agent.as_deref(),
                ip_address: Some(&ip),
                device_id: Some(&device_id),
                two_factor_verified,
            };
            crate::authentication::start_session(tx, user_id, attributes, notify)
        })
        .await
        .map_err(Error::internal)?;
    authenticated_as(c, session.clone(), Some(user), true).await?;
    c.form_authenticity_token();
    Ok(session)
}

/// `ensure_device_cookie`: the browser's `device_id` (new-device sign-in alerts key on it), minted
/// on its first sign-in as `SecureRandom.hex(16)` in a signed, permanent, HttpOnly, SameSite=Lax
/// cookie.
pub fn ensure_device_cookie(c: &mut Ctx) -> Result<String> {
    if let Some(device_id) = c.cookies.signed("device_id") {
        return Ok(device_id);
    }
    let device_id: String = rand::random::<[u8; 16]>()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let cookie = Cookie::new(device_id.clone())
        .permanent()
        .httponly()
        .same_site(Some(SameSite::Lax));
    c.cookies.set_signed("device_id", cookie)?;
    Ok(device_id)
}

/// `resume_session(session)`: refresh its activity (at most hourly), then authenticate as it.
///
/// Only a session due for its refresh goes to the database writer, so other requests don't queue
/// behind every write for nothing. The `session_token` cookie is re-signed on the same schedule
/// rather than on every request as Rails does, which keeps its 20-year expiry rolling without a
/// cookie on every response.
async fn resume_session(c: &mut Ctx, session: Session, user: Option<User>) -> Result<()> {
    let refresh = session.needs_resume(campfire_db::Timestamp::from_jiff(c.now()));
    let session = if refresh {
        let (user_agent, ip) = (
            c.request.user_agent().map(str::to_string),
            c.request.remote_ip()?.to_string(),
        );
        c.app()
            .db
            .write(move |tx| {
                let mut session = session;
                session.resume(tx, user_agent.as_deref(), Some(&ip))?;
                Ok(session)
            })
            .await
            .map_err(Error::internal)?
    } else {
        session
    };
    authenticated_as(c, session, user, refresh).await
}

/// `authenticated_as(session)`: `Current.session = session` (which sets `Current.user` to
/// `session.user`), `authenticated_by` session, and, with `set_cookie`, a fresh `session_token` cookie.
pub async fn authenticated_as(
    c: &mut Ctx,
    session: Session,
    user: Option<User>,
    set_cookie: bool,
) -> Result<()> {
    let user = match user {
        Some(user) => Some(user),
        None => {
            let user_id = session.user_id;
            c.app()
                .db
                .read(move |conn| User::find_by_id(conn, user_id))
                .await
                .map_err(Error::internal)?
        }
    };
    if set_cookie {
        set_authentication_cookie(c, &session)?;
    }
    c.set_current(CurrentSession(session));
    if let Some(user) = user {
        c.set_current(CurrentUser(user));
    }
    set_authenticated_by(c, AuthenticatedBy::Session);
    Ok(())
}

/// `cookies.signed.permanent[:session_token] = { value: session.token, httponly: true, same_site: :lax }`
fn set_authentication_cookie(c: &mut Ctx, session: &Session) -> Result<()> {
    let cookie = Cookie::new(session.token.clone())
        .permanent()
        .httponly()
        .same_site(Some(SameSite::Lax));
    c.cookies.set_signed("session_token", cookie)
}

/// `terminate_current_session`: destroy the session, reset the Rails session, drop the cookie,
/// and disconnect the user's sockets (`reset_remote_connections`, errors only logged).
pub async fn terminate_current_session(c: &mut Ctx) -> Result<()> {
    if let Some(session) = current_session(c).cloned() {
        c.app()
            .db
            .write(move |tx| session.destroy(tx))
            .await
            .map_err(Error::internal)?;
    }
    c.reset_session();
    c.cookies.delete("session_token");
    if let Some(user) = current_user(c).cloned()
        && let Err(error) = c
            .app()
            .db
            .write(move |tx| {
                user.reset_remote_connections(tx);
                Ok(())
            })
            .await
    {
        tracing::warn!("Could not disconnect remote connections on sign out: {error}");
    }
    Ok(())
}

/// `post_authenticating_url`: `session.delete(:return_to_after_authenticating) || root_url`.
pub async fn post_authenticating_url(c: &mut Ctx) -> Result<String> {
    let stored = c.session().remove(session_keys::RETURN_TO_KEY);
    let url = match stored {
        Some(serde_json::Value::String(url)) => url,
        Some(serde_json::Value::Null) | None => c.url_for(&campfire_routes::root()),
        Some(other) => other.to_string(),
    };
    post_authentication_destination(c, url).await
}

/// Map a saved local destination after the final factor, preserving an existing SPA return.
pub async fn post_authentication_destination(c: &Ctx, url: String) -> Result<String> {
    let Some(user) = current_user(c) else { return Ok(url) };
    if !next_ui(c, user).await? {
        return Ok(url);
    }
    let Some(local) = campfire_app::integrations::google::sign_in::safe_return_path(Some(&url), &c.request.host()) else {
        return Ok(url);
    };
    let (path, query) = local.split_once('?').map_or((local.as_str(), None), |(path, query)| (path, Some(query)));
    let confirmed = if campfire_spa::screens::room_show_id(path).is_some() {
        Some(confirmed_room_query(c, path, query).await?)
    } else {
        None
    };
    for screen in campfire_spa::screens::SCREENS {
        let spa = match confirmed {
            Some(confirmed) if screen.endpoint == "rooms#show" => {
                campfire_spa::screens::spa_url_confirmed(screen.endpoint, path, query, confirmed)
            }
            _ => campfire_spa::screens::spa_url(screen.endpoint, path, query),
        };
        if let Some(spa) = spa {
            return Ok(c.url_for(&spa));
        }
    }
    Ok(url)
}

// --- Sudo mode (app/controllers/concerns/sudo_mode.rb) ---------------------------------------------

/// `SudosController`: `rate_limit to: 10, within: 3.minutes, only: %i[ create google ], with: ->
/// { render_sudo_rejection }` (a 429 with "Too many confirmation attempts. Try again in a few
/// minutes."). Counted per `request.remote_ip` in the shared store.
#[allow(dead_code)]
pub fn sudo_rate_limit() -> campfire_kit::RateLimit {
    campfire_kit::RateLimit::new("sudos", 10, jiff::SignedDuration::from_mins(3))
}

/// `require_sudo_mode`: through when the session confirmed its member less than
/// [`session_keys::SUDO_TIMEOUT`] ago; otherwise stashes this request to continue after
/// confirming (`store_sudo_pending_request`) and redirects to `new_sudo_url`.
#[allow(dead_code)]
pub fn require_sudo_mode(c: &mut Ctx) -> Result<()> {
    sudo::require_sudo_mode(c)
}

/// `deny_bots`: 403 for bot-key and bot-reply-token requests.
pub fn deny_bots(c: &mut Ctx) -> Result<()> {
    if matches!(
        authenticated_by(c),
        AuthenticatedBy::BotKey | AuthenticatedBy::BotReply
    ) {
        return halt(head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// `deny_agent_tokens`: 403 for agent-token requests.
pub fn deny_agent_tokens(c: &mut Ctx) -> Result<()> {
    if authenticated_by(c) == AuthenticatedBy::AgentToken {
        return halt(head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

// --- Authorization -----------------------------------------------------------------------------

/// `ensure_can_administer`: 403 unless `Current.user.can_administer?` (no record).
pub fn ensure_can_administer(c: &mut Ctx) -> Result<()> {
    let allowed = current_user(c).is_some_and(|user| user.can_administer(None, false));
    if !allowed {
        return halt(head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

// --- AllowBrowser --------------------------------------------------------------------------------

/// `allow_browser versions: VERSIONS, block: -> { render template: "sessions/incompatible_browser" }`
pub async fn allow_browser(c: &mut Ctx) -> Result<()> {
    if platform::browser_blocked(c.request.user_agent()) {
        return halt(render_incompatible_browser(c).await?);
    }
    Ok(())
}

/// `render template: "sessions/incompatible_browser"` (200). Rendered from a before-action, so
/// it's HTML whatever the request format. The layout is the controller's: turbo-rails' frame
/// layout for a Turbo-Frame request, except in controllers that declare their own layout
/// (`MessagesController` and its `Messages::ByBotsController`), which always use the application
/// layout.
async fn render_incompatible_browser(c: &mut Ctx) -> Result {
    use askama::Template;
    use campfire_retained::sessions::IncompatibleBrowser;

    let own_layout = c
        .current::<MatchedRoute>()
        .is_some_and(|route| {
            route.endpoint.starts_with("messages#")
                || route.endpoint.starts_with("messages/by_bots#")
        });
    use crate::request_context::{
        retained_document, retained_page_or_frame_in_any_format,
    };

    // An explicit `render template:`, so no format lookup: a blocked browser gets this page for
    // /webmanifest.json, /service-worker.js or `Accept: application/json` alike (verified against
    // the reference), never a 406. The document is the retained shell. Message controllers still
    // force that full document when the request asks for a frame.
    let response = if own_layout {
        retained_document(c, StatusCode::OK, |ctx| {
            IncompatibleBrowser { ctx }.render()
        })
        .await?
    } else {
        retained_page_or_frame_in_any_format(
            c,
            StatusCode::OK,
            |ctx| IncompatibleBrowser { ctx }.render(),
            |ctx| {
                let page = IncompatibleBrowser { ctx };
                campfire_retained::layouts::frame(ctx, page.as_head(), page.as_content())
            },
        )
        .await?
    };
    Ok(response.content_type(campfire_kit::response::HTML_UTF8))
}

// --- SetPlatform -------------------------------------------------------------------------------

/// `platform` (`helper_method`): `ApplicationPlatform.new(request.user_agent)`.
pub fn platform(c: &Ctx) -> platform::ApplicationPlatform {
    platform::ApplicationPlatform::new(c.request.user_agent())
}

// --- TrackedRoomVisit ----------------------------------------------------------------------------

/// `remember_last_room_visited`: `cookies.permanent[:last_room] = @room.id`.
///
/// Only when it changes: Rails sets it on every room page.
pub fn remember_last_room_visited(c: &mut Ctx, room_id: i64) {
    let room_id = room_id.to_string();
    if c.cookies.get("last_room") != Some(room_id.as_str()) {
        c.cookies.set("last_room", Cookie::new(room_id).permanent());
    }
}

/// `last_room_visited`: the `last_room` cookie's room if the user is in it, else
/// `Current.user.rooms.original`.
pub async fn last_room_visited(c: &Ctx) -> Result<Option<Room>> {
    let Some(user_id) = current_user(c).map(|user| user.id) else {
        return Ok(None);
    };
    // `find_by(id: cookies[:last_room])` casts the cookie like an integer column would.
    let last_room = c.cookies.get("last_room").and_then(cast_integer);
    c.app()
        .db
        .read(move |conn| {
            if let Some(room_id) = last_room
                && let Some(room) = Room::find_for_user(conn, user_id, room_id)?
            {
                return Ok(Some(room));
            }
            Room::original_for_user(conn, user_id)
        })
        .await
        .map_err(Error::internal)
}

// --- RoomScoped ----------------------------------------------------------------------------------

/// `RoomScoped#set_room`: memberships joined to `Room.alive`, 404
/// otherwise. Returns the membership and its room.
pub async fn set_room(c: &mut Ctx) -> Result<(Membership, Room)> {
    let user_id = require_current_user(c)?.id;
    let Some(room_id) = c.param_str("room_id").and_then(cast_integer) else {
        return Err(Error::NotFound);
    };
    c.app()
        .db
        .read(move |conn| {
            let Some(membership) = Membership::find_by_room_and_user(conn, room_id, user_id)?
            else {
                return Ok(None);
            };
            let room = membership.room(conn)?;
            if room.deleted() { return Ok(None) }
            Ok(Some((membership, room)))
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)
}

// --- Helpers -------------------------------------------------------------------------------------

/// `head status` from a before-action. Rails sets the controller's `formats` only after the
/// before-callbacks have run (`AbstractController::Callbacks#process_action` wraps
/// `ActionController::Rendering#process_action`), so a `head` there falls back to `Mime[:html]`
/// whatever the request format: `text/html`, no charset. (A `head` inside the action itself uses
/// the request format: that's `c.head`.)
pub fn head(status: StatusCode) -> campfire_kit::Response {
    let response = campfire_kit::Response::new(status);
    if matches!(status.as_u16(), 100..=199 | 204 | 205 | 304) {
        response
    } else {
        response.content_type("text/html")
    }
}

/// ActiveModel's integer cast of a string attribute value (`"12abc"` → 12, `"abc"` → nil).
pub fn cast_integer(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let (sign, digits) = match value.as_bytes().first() {
        Some(b'-') => (-1, &value[1..]),
        Some(b'+') => (1, &value[1..]),
        _ => (1, value),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<i64>().ok().map(|n| sign * n)
}

/// Ruby's `String#strip` (ASCII whitespace and NUL).
fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c: char| c == '\0' || c.is_ascii_whitespace() || c == '\u{b}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casts_integers_like_active_model() {
        assert_eq!(cast_integer("12"), Some(12));
        assert_eq!(cast_integer("12abc"), Some(12));
        assert_eq!(cast_integer(" -3"), Some(-3));
        assert_eq!(cast_integer("abc"), None);
        assert_eq!(cast_integer(""), None);
    }

    #[test]
    fn to_i_like_ruby() {
        assert_eq!(ruby_to_i("1717243200000"), 1717243200000);
        assert_eq!(ruby_to_i(" +12abc"), 12);
        assert_eq!(ruby_to_i("\t\n\u{b}\u{c}\r 7"), 7);
        assert_eq!(ruby_to_i("\u{a0}5"), 0);
        assert_eq!(ruby_to_i("abc"), 0);
        assert_eq!(ruby_to_i("-5"), -5);
        assert_eq!(ruby_to_i("--5"), 0);
        assert_eq!(ruby_to_i("5_6"), 56);
        assert_eq!(ruby_to_i("5__6"), 5);
        assert_eq!(ruby_to_i("_5"), 0);
        assert_eq!(ruby_to_i("0__5"), 0);
        assert_eq!(ruby_to_i("-0d5"), -5);
        assert_eq!(ruby_to_i("0d_5"), 0);
        assert_eq!(ruby_to_i("0x5"), 0);
        assert_eq!(ruby_to_i("99999999999999999999"), i64::MAX);
    }

    #[test]
    fn before_builders() {
        let before = Before::default()
            .allow_bot_access()
            .skip_forgery_protection();
        assert_eq!(before.authentication, Authentication::Required);
        assert!(!before.deny_bots);
        assert!(!before.forgery_protection);
        assert_eq!(
            Before::default()
                .require_unauthenticated_access()
                .authentication,
            Authentication::RequireUnauthenticated
        );
    }
}

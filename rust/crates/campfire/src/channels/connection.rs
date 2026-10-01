//! `ApplicationCable::Connection` (reference/app/channels/application_cable/connection.rb):
//! `current_user` from the signed `session_token` cookie (`Authentication::SessionLookup`), or
//! `reject_unauthorized_connection`. An expired session (`Session#expired?`) is destroyed and
//! rejected, and so is a session that never completed a required second factor
//! (`user.requires_two_factor? && !session.two_factor_verified?`, where only active humans require
//! one).
//!
//! Password/Google/transfer first factors remain pending until verification; unenrolled sessions
//! cannot connect before setup confirms them. Remembered devices also start verified sessions.
use campfire_cable::{Authenticate, ConnectRequest};
use campfire_db::{Database, Session, User};
use campfire_kit::{CookieJar, SharedClock, SharedCrypto};
use jiff::SignedDuration;

use super::CableUser;
use crate::concerns::session_expired;

pub struct SessionAuthenticator {
    db: Database,
    crypto: SharedCrypto,
    clock: SharedClock,
    admin_session_idle_timeout: SignedDuration,
}

impl SessionAuthenticator {
    pub fn new(db: Database, crypto: SharedCrypto, clock: SharedClock, admin_session_idle_timeout: SignedDuration) -> Self {
        Self { db, crypto, clock, admin_session_idle_timeout }
    }

    /// `cookies.signed[:session_token]`.
    fn session_token(&self, request: &ConnectRequest) -> Option<String> {
        let headers = request.headers.get_all("cookie").iter().filter_map(|value| value.to_str().ok());
        CookieJar::from_headers(headers, self.crypto.clone(), self.clock.clone()).signed("session_token")
    }
}

#[async_trait::async_trait]
impl Authenticate<CableUser> for SessionAuthenticator {
    async fn connect(&self, request: &ConnectRequest) -> Option<CableUser> {
        let token = self.session_token(request)?;
        let found = self
            .db
            .read(move |conn| {
                let Some(session) = Session::find_by_token(conn, &token)? else { return Ok(None) };
                Ok(User::find_by_id(conn, session.user_id)?.map(|user| (session, user)))
            })
            .await;
        let (session, user) = match found {
            Ok(found) => found?,
            Err(error) => {
                tracing::error!(%error, "Could not look up the cable session");
                return None;
            }
        };
        let now = campfire_db::Timestamp::from_jiff(self.clock.now());
        if session_expired(&session, &user, self.admin_session_idle_timeout, now) {
            if let Err(error) = self.db.write(move |tx| session.destroy(tx)).await {
                tracing::error!(%error, "Could not destroy the expired cable session");
            }
            return None;
        }
        if user.requires_two_factor() && !session.two_factor_verified() {
            return None;
        }
        Some(CableUser { id: user.id, name: user.name, role: user.role, status: user.status, session_id: session.id })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use campfire_db::{NewSession, NewUser, Role, TestClock, Timestamp};
    use campfire_kit::testing::crypto;
    use campfire_kit::FrozenClock;

    use super::*;

    const NOW: i64 = 1_767_268_800;
    const DAY: i64 = 86_400;

    async fn database(dir: &std::path::Path) -> Database {
        let clock = TestClock::frozen_at(Timestamp::from_second(NOW));
        let env = campfire_db::Env { clock: Arc::new(clock), bcrypt_cost: 4, ..Default::default() };
        let mut config = campfire_db::Config::new(dir.join("test.sqlite3"));
        config.environment = "test".into();
        Database::open(config, env).unwrap()
    }

    /// A session for a new user, last active `idle` seconds ago.
    async fn session(db: &Database, role: Role, idle: i64) -> Session {
        db.write(move |tx| {
            let attributes = NewUser { name: format!("{role:?} {idle}"), email_address: None, password_digest: None, role, bio: None, icon_name: None, bot_token_digest: None };
            let user = User::create(tx, attributes)?;
            let attributes = NewSession { user_agent: Some("test"), ip_address: Some("8.8.8.8"), two_factor_verified: true, ..Default::default() };
            let session = Session::start_with(tx, user.id, attributes)?;
            tx.conn().execute("UPDATE sessions SET last_active_at = ? WHERE id = ?", rusqlite::params![Timestamp::from_second(NOW - idle), session.id])?;
            Ok(session)
        })
        .await
        .unwrap()
    }

    fn request(token: &str) -> ConnectRequest {
        let signed = crypto().sign_cookie("session_token", token, None);
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("cookie", format!("session_token={}", campfire_kit::cookies::escape(&signed)).parse().unwrap());
        ConnectRequest { uri: "/cable".parse().unwrap(), headers }
    }

    #[tokio::test]
    async fn expired_administrator_sessions_are_destroyed_and_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db = database(dir.path()).await;
        let clock: SharedClock = Arc::new(FrozenClock::new(jiff::Timestamp::from_second(NOW).unwrap()));
        let authenticator = SessionAuthenticator::new(db.clone(), crypto(), clock, SignedDuration::from_hours(7 * 24));

        let stale_admin = session(&db, Role::Administrator, 7 * DAY + 1).await;
        let idle_admin = session(&db, Role::Administrator, 7 * DAY).await;
        let stale_member = session(&db, Role::Member, 400 * DAY).await;

        assert!(authenticator.connect(&request(&stale_admin.token)).await.is_none());
        let token = stale_admin.token.clone();
        assert!(db.read(move |conn| Session::find_by_token(conn, &token)).await.unwrap().is_none(), "destroyed");
        assert!(authenticator.connect(&request(&idle_admin.token)).await.is_some(), "exactly the timeout isn't past it");
        assert!(authenticator.connect(&request(&stale_member.token)).await.is_some(), "members' sessions don't expire");
    }

    #[tokio::test]
    async fn human_verification_is_required_and_reloaded_on_every_connection() {
        let dir = tempfile::tempdir().unwrap();
        let db = database(dir.path()).await;
        let clock: SharedClock = Arc::new(FrozenClock::new(jiff::Timestamp::from_second(NOW).unwrap()));
        let authenticator = SessionAuthenticator::new(db.clone(), crypto(), clock, SignedDuration::from_hours(7 * 24));
        let human = session(&db, Role::Member, 0).await;
        let request = request(&human.token);
        // Verification is enough even without a credential (the reference test-only sign-in).
        assert!(authenticator.connect(&request).await.is_some());
        let id = human.id;
        db.write(move |tx| { tx.conn().execute("UPDATE sessions SET two_factor_verified_at=NULL WHERE id=?", [id])?; Ok(()) }).await.unwrap();
        assert!(authenticator.connect(&request).await.is_none());
        assert_eq!(db.read(move |c| Session::find(c,id)).await.unwrap().id, id, "2FA rejection does not destroy the session on cable");
        db.write(move |tx| { let mut s = Session::find(tx.conn(),id)?; s.mark_two_factor_verified(tx) }).await.unwrap();
        assert!(authenticator.connect(&request).await.is_some());
        db.write(move |tx| { let mut s = Session::find(tx.conn(),id)?; s.clear_two_factor_verified(tx) }).await.unwrap();
        assert!(authenticator.connect(&request).await.is_none(), "clearing the persisted stamp blocks reconnect");
    }

    #[tokio::test]
    async fn cable_exempts_unverified_bots_and_inactive_users_from_two_factor() {
        let dir = tempfile::tempdir().unwrap();
        let db = database(dir.path()).await;
        let clock: SharedClock = Arc::new(FrozenClock::new(jiff::Timestamp::from_second(NOW).unwrap()));
        let authenticator = SessionAuthenticator::new(db.clone(), crypto(), clock, SignedDuration::from_hours(7 * 24));
        for role in [Role::Bot, Role::Member] {
            let session = session(&db, role, 0).await;
            let id = session.id;
            let user_id = session.user_id;
            db.write(move |tx| {
                tx.conn().execute("UPDATE sessions SET two_factor_verified_at=NULL WHERE id=?", [id])?;
                if role == Role::Member { tx.conn().execute("UPDATE users SET status=1 WHERE id=?", [user_id])?; }
                Ok(())
            }).await.unwrap();
            assert!(authenticator.connect(&request(&session.token)).await.is_some());
        }
    }
}

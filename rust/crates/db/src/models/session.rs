//! `reference/app/models/session.rb`

use jiff::SignedDuration;
use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{OptionalExt, Result};
use crate::sql::{self, CachedStatements, query_all, query_one};

use crate::time::Timestamp;

/// `Session::ACTIVITY_REFRESH_RATE`
pub const ACTIVITY_REFRESH_RATE: SignedDuration = SignedDuration::from_hours(1);

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    /// The signed device cookie the session started on (`UserDevice`), if any.
    pub device_id: Option<String>,
    /// When the session passed two-factor verification; nil until it does.
    pub two_factor_verified_at: Option<Timestamp>,
    pub last_active_at: Timestamp,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// The keyword arguments of `Session.start!(user_agent:, ip_address:, device_id: nil,
/// two_factor_verified: false)`.
#[derive(Debug, Clone, Default)]
pub struct NewSession<'a> {
    pub user_agent: Option<&'a str>,
    pub ip_address: Option<&'a str>,
    pub device_id: Option<&'a str>,
    pub two_factor_verified: bool,
}

impl Session {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            token: row.get("token")?,
            ip_address: row.get("ip_address")?,
            user_agent: row.get("user_agent")?,
            device_id: row.get("device_id")?,
            two_factor_verified_at: row.get("two_factor_verified_at")?,
            last_active_at: row.get("last_active_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT * FROM "sessions" WHERE "sessions"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("Session")
    }

    /// `Session.find_by(token:)`
    pub fn find_by_token(conn: &Connection, token: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "sessions" WHERE "sessions"."token" = ? LIMIT 1"#,
            [token],
            Self::from_row,
        )
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "sessions" WHERE "sessions"."user_id" = ?"#,
            [user_id],
            Self::from_row,
        )
    }

    pub fn count_for_user(conn: &Connection, user_id: i64) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "sessions" WHERE "sessions"."user_id" = ?"#,
            [user_id],
        )
    }

    /// `user.sessions.start!(user_agent:, ip_address:)`, with no device and unverified.
    pub fn start(
        tx: &mut Tx<'_>,
        user_id: i64,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<Self> {
        Self::start_with(
            tx,
            user_id,
            NewSession {
                user_agent,
                ip_address,
                ..Default::default()
            },
        )
    }

    /// `user.sessions.start!(user_agent:, ip_address:, device_id:, two_factor_verified:)`
    /// (`app/models/session.rb`): a new 24-character base58 `has_secure_token`,
    /// `two_factor_verified_at: (Time.current if two_factor_verified)`, and
    /// `last_active_at ||= Time.now` in `before_create`.
    pub fn start_with(tx: &mut Tx<'_>, user_id: i64, attributes: NewSession<'_>) -> Result<Self> {
        let now = tx.now();
        let last_active_at = tx.now();
        let two_factor_verified_at = attributes.two_factor_verified.then_some(now);
        let token = sql::base58(24);
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "sessions" ("created_at", "device_id", "ip_address", "last_active_at", "token", "two_factor_verified_at", "updated_at", "user_agent", "user_id") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, attributes.device_id, attributes.ip_address, last_active_at, token, two_factor_verified_at, now, attributes.user_agent, user_id],
            |r| r.get(0),
        )?;
        Ok(Self {
            id,
            user_id,
            token,
            ip_address: attributes.ip_address.map(Into::into),
            user_agent: attributes.user_agent.map(Into::into),
            device_id: attributes.device_id.map(Into::into),
            two_factor_verified_at,
            last_active_at,
            created_at: now,
            updated_at: now,
        })
    }

    /// `two_factor_verified?`
    pub fn two_factor_verified(&self) -> bool {
        self.two_factor_verified_at.is_some()
    }

    /// `mark_two_factor_verified!`: stamps the session unless it's already verified.
    pub fn mark_two_factor_verified(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self.two_factor_verified() {
            return Ok(());
        }
        self.set_two_factor_verified_at(tx, Some(tx.now()))
    }

    /// `clear_two_factor_verified!`
    pub fn clear_two_factor_verified(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if !self.two_factor_verified() {
            return Ok(());
        }
        self.set_two_factor_verified_at(tx, None)
    }

    fn set_two_factor_verified_at(&mut self, tx: &mut Tx<'_>, at: Option<Timestamp>) -> Result<()> {
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "sessions" SET "two_factor_verified_at" = ?, "updated_at" = ? WHERE "sessions"."id" = ?"#,
            params![at, now, self.id],
        )?;
        self.two_factor_verified_at = at;
        self.updated_at = now;
        Ok(())
    }

    /// Whether [`Self::resume`] would refresh the session at `now`: its activity is over an hour old.
    pub fn needs_resume(&self, now: Timestamp) -> bool {
        self.last_active_at < now.ago(ACTIVITY_REFRESH_RATE)
    }

    /// `resume`: refreshes activity, user agent and IP at most once an hour.
    pub fn resume(
        &mut self,
        tx: &mut Tx<'_>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<()> {
        let now = tx.now();
        if !self.needs_resume(now) {
            return Ok(());
        }
        self.user_agent = user_agent.map(Into::into);
        self.ip_address = ip_address.map(Into::into);
        self.last_active_at = now;
        self.updated_at = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "sessions" SET "ip_address" = ?, "last_active_at" = ?, "updated_at" = ?, "user_agent" = ? WHERE "sessions"."id" = ?"#,
            params![self.ip_address, self.last_active_at, self.updated_at, self.user_agent, self.id],
        )?;
        Ok(())
    }

    /// `destroy!`: revoke huddle grants before deleting the session and its dependents.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        crate::models::huddle_grant::HuddleGrant::revoke_for_session(tx, self.id, &crate::models::room_delete::HuddleConfig::from_env())?;
        tx.conn().execute_cached(
            r#"DELETE FROM "workspace_presence_leases" WHERE "workspace_presence_leases"."session_id" = ?"#,
            [self.id],
        )?;
        tx.conn().execute_cached(
            r#"DELETE FROM "two_factor_setup_secrets" WHERE "two_factor_setup_secrets"."session_id" = ?"#,
            [self.id],
        )?;
        tx.conn().execute_cached(
            r#"DELETE FROM "sessions" WHERE "sessions"."id" = ?"#,
            [self.id],
        )?;
        Ok(())
    }
}

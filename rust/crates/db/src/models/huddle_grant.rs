//! Authorization and liveness from `app/models/huddle_grant.rb`.
//! Invitation fan-out and rendered callback effects are separate WS13 slices.
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::models::huddle_cleanup::HuddleCleanup;
use crate::models::room_delete::HuddleConfig;
use crate::sql::query_all;
use crate::{CachedStatements, Connection, Errors, Event, Job, Result, Timestamp, Tx};

pub const IN_CALL_WINDOW: i64 = 20;
pub const SEEN_TOUCH_INTERVAL: i64 = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct HuddleGrant {
    pub id: i64,
    pub identity: String,
    pub room_name: String,
    pub session_id: i64,
    pub user_id: i64,
    pub membership_id: i64,
    pub room_id: i64,
    pub stage_role: Option<String>,
    pub server_muted: bool,
    pub last_issued_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
    pub revoked_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PresenceJob {
    pub grant_id: i64,
}
impl Job for PresenceJob {
    const CLASS: &'static str = "Huddle::BroadcastPresenceJob";
}
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinNoticeJob {
    pub grant_id: i64,
}
impl Job for JoinNoticeJob {
    const CLASS: &'static str = "Huddle::JoinNoticeJob";
}

impl HuddleGrant {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            identity: r.get("identity")?,
            room_name: r.get("room_name")?,
            session_id: r.get("session_id")?,
            user_id: r.get("user_id")?,
            membership_id: r.get("membership_id")?,
            room_id: r.get("room_id")?,
            stage_role: r.get("stage_role")?,
            server_muted: r.get("server_muted")?,
            last_issued_at: r.get("last_issued_at")?,
            last_seen_at: r.get("last_seen_at")?,
            revoked_at: r.get("revoked_at")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row_cached(
                "SELECT * FROM huddle_grants WHERE id=?",
                [id],
                Self::from_row,
            )
            .optional()?)
    }
    pub fn find_by_coordinates(
        conn: &Connection,
        identity: &str,
        room_name: &str,
    ) -> Result<Option<Self>> {
        Ok(conn
            .query_row_cached(
                "SELECT * FROM huddle_grants WHERE identity=? AND room_name=? LIMIT 1",
                params![identity, room_name],
                Self::from_row,
            )
            .optional()?)
    }
    pub fn authorization_payload(&self) -> serde_json::Value {
        serde_json::json!({"grant_id": self.id, "room_name": self.room_name, "identity": self.identity})
    }
    pub fn revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
    pub fn in_call(&self, now: Timestamp) -> bool {
        self.last_seen_at
            .is_some_and(|at| at > now.ago(SignedDuration::from_secs(IN_CALL_WINDOW)))
    }
    fn validate(&self, conn: &Connection) -> Result<()> {
        let mut errors = Errors::default();
        if self.identity.chars().all(char::is_whitespace) {
            errors.add("identity", "can't be blank");
        }
        if self.room_name.chars().all(char::is_whitespace) {
            errors.add("room_name", "can't be blank");
        }
        if conn.query_row_cached(
            "SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE identity=? AND id!=?)",
            params![self.identity, self.id],
            |r| r.get::<_, bool>(0),
        )? {
            errors.add("identity", "has already been taken");
        }
        errors.into_result()
    }
    /// Re-read all coordinates under SQLite's immediate write transaction. Callers must run
    /// post-issuance invitation/presence work separately; it is not part of this slice.
    pub fn issue(
        tx: &mut Tx<'_>,
        session_id: i64,
        membership_id: i64,
        room_id: i64,
        config: &HuddleConfig,
    ) -> Result<Self> {
        for attempt in 0..3 {
            let result =
                tx.savepoint(|tx| Self::issue_once(tx, session_id, membership_id, room_id, config));
            if result
                .as_ref()
                .is_err_and(|error| error.is_record_not_unique())
                && attempt < 2
            {
                continue;
            }
            return result;
        }
        unreachable!()
    }

    fn issue_once(
        tx: &mut Tx<'_>,
        session_id: i64,
        membership_id: i64,
        expected_room_id: i64,
        config: &HuddleConfig,
    ) -> Result<Self> {
        let candidate: Option<(i64, i64, Option<String>, bool)> = tx.conn().query_row_cached(
            "SELECT u.id,r.id,CASE WHEN r.type='Rooms::Stage' THEN m.stage_role END,m.server_muted_at IS NOT NULL FROM sessions s JOIN users u ON u.id=s.user_id AND u.status=0 AND u.role!=2 JOIN memberships m ON m.id=? AND m.room_id=? AND m.user_id=u.id JOIN rooms r ON r.id=m.room_id AND r.deleted_at IS NULL WHERE s.id=?",
            params![membership_id, expected_room_id, session_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).optional()?;
        let Some((user_id, room_id, stage_role, server_muted)) = candidate else {
            return Err(crate::Error::Other("HuddleGrant::Ineligible".into()));
        };
        let grants = query_all(
            tx.conn(),
            "SELECT * FROM huddle_grants WHERE session_id=? AND revoked_at IS NULL AND ((room_id=? AND membership_id!=?) OR (room_id!=? AND last_seen_at>?)) ORDER BY id",
            params![
                session_id,
                room_id,
                membership_id,
                room_id,
                tx.now().ago(SignedDuration::from_secs(IN_CALL_WINDOW))
            ],
            Self::from_row,
        )?;
        for mut grant in grants {
            grant.revoke(tx, true, config)?;
        }
        let existing = tx.conn().query_row_cached("SELECT * FROM huddle_grants WHERE session_id=? AND membership_id=? AND revoked_at IS NULL LIMIT 1", params![session_id, membership_id], Self::from_row).optional()?;
        if let Some(mut grant) = existing {
            if grant.stage_role == stage_role && grant.server_muted == server_muted {
                grant.validate(tx.conn())?;
                let now = tx.now();
                tx.conn().execute_cached(
                    "UPDATE huddle_grants SET last_issued_at=?,updated_at=? WHERE id=?",
                    params![now, now, grant.id],
                )?;
                return Ok(Self::find_by_id(tx.conn(), grant.id)?.unwrap());
            }
            grant.revoke(tx, true, config)?;
        }
        let secret = config
            .api_secret
            .as_deref()
            .ok_or_else(|| crate::Error::Other("LIVEKIT_API_SECRET is missing".into()))?;
        let bytes: [u8; 32] = rand::random();
        let identity = format!(
            "campfire-participant-{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        let room_name = rails_compat::jwt::livekit::room_name(secret, room_id);
        let now = tx.now();
        let id = tx.conn().query_row_cached("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,stage_role,server_muted,last_issued_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?) RETURNING id", params![identity, room_name, session_id, user_id, membership_id, room_id, stage_role, server_muted, now, now, now], |r| r.get(0))?;
        Ok(Self::find_by_id(tx.conn(), id)?.unwrap())
    }
    pub fn authorized(&self, conn: &Connection) -> Result<bool> {
        if self.revoked() {
            return Ok(false);
        }
        Ok(conn.query_row_cached(
            "SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id AND s.id=? JOIN memberships m ON m.user_id=u.id AND m.id=? AND m.room_id=? JOIN rooms r ON r.id=m.room_id AND r.deleted_at IS NULL WHERE u.id=? AND u.status=0 AND u.role!=2 AND (r.type!='Rooms::Stage' OR m.stage_role IS ?) AND (m.server_muted_at IS NOT NULL)=?)",
            params![self.session_id, self.membership_id, self.room_id, self.user_id, self.stage_role, self.server_muted], |r| r.get(0),
        )?)
    }
    /// Fast reads belong to the caller; re-read here before any write, as Rails' with_lock does.
    pub fn authorize_or_revoke(
        tx: &mut Tx<'_>,
        id: i64,
        config: &HuddleConfig,
    ) -> Result<Option<Self>> {
        let Some(mut grant) = Self::find_by_id(tx.conn(), id)? else {
            return Ok(None);
        };
        if grant.authorized(tx.conn())? {
            return Ok(Some(grant));
        }
        grant.revoke(tx, true, config)?;
        Ok(None)
    }
    pub fn revoke(
        &mut self,
        tx: &mut Tx<'_>,
        create_cleanup: bool,
        config: &HuddleConfig,
    ) -> Result<()> {
        // Re-read for idempotence even if the caller retained a stale object.
        let Some(current) = Self::find_by_id(tx.conn(), self.id)? else {
            return Ok(());
        };
        *self = current;
        if self.revoked() {
            return Ok(());
        }
        self.validate(tx.conn())?;
        let now = tx.now();
        tx.conn().execute_cached(
            "UPDATE huddle_grants SET revoked_at=?,updated_at=? WHERE id=?",
            params![now, now, self.id],
        )?;
        self.revoked_at = Some(now);
        self.updated_at = now;
        if create_cleanup {
            HuddleCleanup::create_participant_removal(
                tx,
                self.id,
                &self.room_name,
                &self.identity,
                config.admin_configured,
            )?;
        }
        // Stage's last-grant callback runs synchronously inside this revocation transaction.
        let last_stage_grant: bool = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM rooms r WHERE r.id=? AND r.type='Rooms::Stage' AND r.deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM huddle_grants g WHERE g.room_id=r.id AND g.membership_id=? AND g.revoked_at IS NULL))", params![self.room_id, self.membership_id], |r| r.get(0))?;
        if last_stage_grant {
            Self::end_streams_for_membership(tx, self.room_id, self.membership_id)?;
        }
        Ok(())
    }
    pub(crate) fn end_streams_for_membership(
        tx: &mut Tx<'_>,
        room_id: i64,
        membership_id: i64,
    ) -> Result<()> {
        let qualities: Vec<String> = query_all(
            tx.conn(),
            "SELECT quality FROM streams WHERE room_id=? AND membership_id=? AND ended_at IS NULL",
            params![room_id, membership_id],
            |r| r.get(0),
        )?;
        for quality in qualities {
            if !["720p15", "1080p15", "1080p30"].contains(&quality.as_str()) {
                let mut errors = Errors::default();
                errors.add("quality", "is not included in the list");
                errors.into_result()?;
            }
        }
        let now = tx.now();
        tx.conn().execute_cached("UPDATE streams SET ended_at=?,updated_at=? WHERE room_id=? AND membership_id=? AND ended_at IS NULL", params![now, now, room_id, membership_id])?;
        Ok(())
    }
    fn revoke_scope(
        tx: &mut Tx<'_>,
        column: &str,
        id: i64,
        create_cleanup: bool,
        config: &HuddleConfig,
    ) -> Result<()> {
        // Column names are constants from the four public scoped operations below.
        let grants = query_all(
            tx.conn(),
            &format!(
                "SELECT * FROM huddle_grants WHERE {column}=? AND revoked_at IS NULL ORDER BY id"
            ),
            [id],
            Self::from_row,
        )?;
        for mut grant in grants {
            grant.revoke(tx, create_cleanup, config)?;
        }
        Ok(())
    }
    pub fn revoke_for_membership(tx: &mut Tx<'_>, id: i64, config: &HuddleConfig) -> Result<()> {
        Self::revoke_scope(tx, "membership_id", id, true, config)
    }
    pub fn revoke_for_session(tx: &mut Tx<'_>, id: i64, config: &HuddleConfig) -> Result<()> {
        Self::revoke_scope(tx, "session_id", id, true, config)
    }
    pub fn revoke_for_user(tx: &mut Tx<'_>, id: i64, config: &HuddleConfig) -> Result<()> {
        Self::revoke_scope(tx, "user_id", id, true, config)
    }
    pub fn revoke_for_room(tx: &mut Tx<'_>, id: i64, config: &HuddleConfig) -> Result<()> {
        let name: Option<String> = tx
            .conn()
            .query_row_cached(
                "SELECT room_name FROM huddle_grants WHERE room_id=? LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let name = name.or_else(|| {
            config
                .api_secret
                .as_deref()
                .map(|secret| rails_compat::jwt::livekit::room_name(secret, id))
        });
        Self::revoke_scope(tx, "room_id", id, false, config)?;
        if let Some(name) = name.filter(|s| !s.chars().all(char::is_whitespace)) {
            HuddleCleanup::create_room_deletion(tx, &name, config.admin_configured)?;
        }
        Ok(())
    }
    pub fn record_seen(&mut self, tx: &mut Tx<'_>) -> Result<bool> {
        let now = tx.now();
        if self
            .last_seen_at
            .is_some_and(|at| at > now.ago(SignedDuration::from_secs(SEEN_TOUCH_INTERVAL)))
        {
            return Ok(false);
        }
        let first_seen = !self.in_call(now);
        tx.conn().execute_cached(
            "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
            params![now, self.id],
        )?;
        self.last_seen_at = Some(now);
        if first_seen {
            tx.emit_after_commit(Event::job(&PresenceJob { grant_id: self.id }));
            let another: bool = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE id!=? AND room_id=? AND user_id=? AND revoked_at IS NULL AND last_seen_at>?)", params![self.id, self.room_id, self.user_id, now.ago(SignedDuration::from_secs(IN_CALL_WINDOW))], |r| r.get(0))?;
            if !another {
                tx.emit_after_commit(Event::job(&JoinNoticeJob { grant_id: self.id }));
            }
        }
        Ok(true)
    }
    pub fn mark_out_of_call(
        &mut self,
        tx: &mut Tx<'_>,
        seen_after: Option<Timestamp>,
    ) -> Result<bool> {
        let Some(seen) = self.last_seen_at else {
            return Ok(false);
        };
        if seen_after.is_some_and(|floor| seen > floor) {
            return Ok(false);
        }
        tx.conn().execute_cached(
            "UPDATE huddle_grants SET last_seen_at=NULL WHERE id=?",
            [self.id],
        )?;
        self.last_seen_at = None;
        Ok(true)
    }
    pub fn participants_for(
        conn: &Connection,
        room_id: i64,
        now: Timestamp,
    ) -> Result<Vec<crate::User>> {
        query_all(conn, "SELECT DISTINCT u.* FROM users u JOIN huddle_grants g ON g.user_id=u.id WHERE g.room_id=? AND g.revoked_at IS NULL AND g.last_seen_at>?", params![room_id, now.ago(SignedDuration::from_secs(IN_CALL_WINDOW))], crate::models::user::User::from_row).map(|mut users| { users.sort_by_key(|u| u.name.to_lowercase()); users })
    }
    pub fn participant_identities_for(
        conn: &Connection,
        room_id: i64,
        now: Timestamp,
    ) -> Result<std::collections::BTreeMap<i64, Vec<String>>> {
        let rows: Vec<(i64, String)> = query_all(
            conn,
            "SELECT user_id,identity FROM huddle_grants WHERE room_id=? AND revoked_at IS NULL AND last_seen_at>? ORDER BY id",
            params![room_id, now.ago(SignedDuration::from_secs(IN_CALL_WINDOW))],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut result = std::collections::BTreeMap::new();
        for (id, identity) in rows {
            result.entry(id).or_insert_with(Vec::new).push(identity);
        }
        Ok(result)
    }
}

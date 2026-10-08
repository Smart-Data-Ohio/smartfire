//! AgentApproval: validation, expiry, inbox fanout and atomic decision callbacks.
//! Human controllers call `decide_authorized`; trusted domain callers may use the
//! Rails model's `decide` operation, whose permission check lives in its controller.
use crate::models::agent_delivery::{NewEvent, create_delivered, enqueue_delivered_webhook};
use crate::models::agent_payloads::{compact, json_time};
use crate::sql::{exists, query_all, query_one};
use crate::{ActivityItem, Connection, Errors, Event, Job, Result, Timestamp, Tx, User};
use rails_compat::unicode;
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const STATUSES: [&str; 5] = ["pending", "approved", "denied", "cancelled", "expired"];

#[derive(Debug, Clone)]
pub struct AgentApproval {
    pub id: i64,
    pub agent_id: i64,
    pub agent_credential_id: Option<i64>,
    pub room_id: Option<i64>,
    pub action: String,
    pub summary: String,
    pub payload: Option<String>,
    pub external_id: Option<String>,
    pub status: String,
    pub expires_at: Timestamp,
    pub decided_by_id: Option<i64>,
    pub decided_at: Option<Timestamp>,
    pub decision_note: Option<String>,
    pub github_account_id: Option<i64>,
    pub github_login: Option<String>,
    pub fizzy_connected_account_id: Option<i64>,
    pub fizzy_user_id: Option<String>,
    pub fizzy_user_name: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone)]
pub struct NewApproval {
    pub agent_id: i64,
    pub agent_credential_id: Option<i64>,
    pub room_id: Option<i64>,
    pub action: String,
    pub summary: String,
    pub payload: Option<String>,
    pub external_id: Option<String>,
    pub status: String,
    pub expires_at: Option<Timestamp>,
    pub decided_by_id: Option<i64>,
    pub decided_at: Option<Timestamp>,
    pub decision_note: Option<String>,
    pub github_account_id: Option<i64>,
    pub github_login: Option<String>,
    pub fizzy_connected_account_id: Option<i64>,
    pub fizzy_user_id: Option<String>,
    pub fizzy_user_name: Option<String>,
}
impl Default for NewApproval {
    fn default() -> Self {
        Self {
            agent_id: 0,
            agent_credential_id: None,
            room_id: None,
            action: String::new(),
            summary: String::new(),
            payload: None,
            external_id: None,
            status: "pending".into(),
            expires_at: None,
            decided_by_id: None,
            decided_at: None,
            decision_note: None,
            github_account_id: None,
            github_login: None,
            fizzy_connected_account_id: None,
            fizzy_user_id: None,
            fizzy_user_name: None,
        }
    }
}
impl NewApproval {
    fn normalized(&self, now: Timestamp) -> Self {
        let mut a = self.clone();
        a.external_id = a
            .external_id
            .filter(|s| !campfire_richtext::ruby::is_blank(s));
        a.expires_at
            .get_or_insert(now.since(jiff::SignedDuration::from_hours(24)));
        a
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Applied,
    Forbidden,
    Invalid(Errors),
}

#[derive(Serialize, Deserialize)]
struct GithubAction {
    approval_id: i64,
}
impl Job for GithubAction {
    const CLASS: &'static str = "Github::PerformAgentActionJob";
}
#[derive(Serialize, Deserialize)]
struct FizzyAction {
    approval_id: i64,
}
impl Job for FizzyAction {
    const CLASS: &'static str = "Fizzy::PerformAgentActionJob";
}

/// The request was approved, denied, cancelled or settled as expired. The classic app has no
/// broadcast for it; the cable sink publishes the single-page app's `approval.updated` to the
/// people who received its activity item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalChange {
    pub approval_id: i64,
}
impl crate::events::Broadcast for ApprovalChange {
    const KIND: &'static str = "AgentApproval#sync_change";
}
impl ApprovalChange {
    pub fn emit(tx: &mut Tx<'_>, approval_id: i64) {
        tx.emit_after_commit(Event::broadcast(&ApprovalChange { approval_id }));
    }
}

impl AgentApproval {
    pub(crate) fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            agent_credential_id: r.get("agent_credential_id")?,
            room_id: r.get("room_id")?,
            action: r.get("action")?,
            summary: r.get("summary")?,
            payload: r.get("payload")?,
            external_id: r.get("external_id")?,
            status: r.get("status")?,
            expires_at: r.get("expires_at")?,
            decided_by_id: r.get("decided_by_id")?,
            decided_at: r.get("decided_at")?,
            decision_note: r.get("decision_note")?,
            github_account_id: r.get("github_account_id")?,
            github_login: r.get("github_login")?,
            fizzy_connected_account_id: r.get("fizzy_connected_account_id")?,
            fizzy_user_id: r.get("fizzy_user_id")?,
            fizzy_user_name: r.get("fizzy_user_name")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    /// The management history's bounded 50+1 window, with complete rows.
    pub fn history_page(
        conn: &Connection,
        agent_id: i64,
        status: Option<&str>,
        now: Timestamp,
        offset: i64,
    ) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM agent_approvals WHERE agent_id=?1 AND (?2 IS NULL OR (?2='pending' AND status='pending' AND expires_at>?3) OR (?2='expired' AND (status='expired' OR (status='pending' AND expires_at<=?3))) OR (?2 NOT IN ('pending','expired') AND status=?2)) ORDER BY id DESC LIMIT 51 OFFSET ?4",
            params![agent_id, status, now, offset],
            Self::from_row,
        )
    }

    /// [`Self::history_page`] by keyset rather than offset: the 50+1 newest rows with an id
    /// below `before` (the single-page app's opaque cursor), under the same `status` filter.
    pub fn history_page_before(
        conn: &Connection,
        agent_id: i64,
        status: Option<&str>,
        now: Timestamp,
        before: Option<i64>,
    ) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM agent_approvals WHERE agent_id=?1 AND (?4 IS NULL OR id<?4) AND (?2 IS NULL OR (?2='pending' AND status='pending' AND expires_at>?3) OR (?2='expired' AND (status='expired' OR (status='pending' AND expires_at<=?3))) OR (?2 NOT IN ('pending','expired') AND status=?2)) ORDER BY id DESC LIMIT 51",
            params![agent_id, status, now, before],
            Self::from_row,
        )
    }

    /// Rows by id, for batch preloads (missing ids are skipped).
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            "SELECT * FROM agent_approvals WHERE id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(ids).to_string()],
            Self::from_row,
        )
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_approvals WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn find_by_external_id(
        conn: &Connection,
        agent_id: i64,
        external_id: &str,
    ) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_approvals WHERE agent_id=? AND external_id=? LIMIT 1",
            params![agent_id, external_id],
            Self::from_row,
        )
    }
    pub fn validate(
        conn: &Connection,
        a: &NewApproval,
        now: Timestamp,
        exclude: Option<i64>,
    ) -> Result<Errors> {
        let normalized;
        let a = if exclude.is_none() {
            normalized = a.normalized(now);
            &normalized
        } else {
            a
        };
        let mut errors = Errors::default();
        if !exists(conn, "SELECT 1 FROM agents WHERE id=?", [a.agent_id])? {
            errors.add("agent", "must exist");
        }
        for (field, text, max) in [("action", &a.action, 60), ("summary", &a.summary, 500)] {
            if campfire_richtext::ruby::is_blank(text) {
                errors.add(field, "can't be blank");
            }
            if text.chars().count() > max {
                errors.add(field, format!("is too long (maximum is {max} characters)"));
            }
            if field == "action"
                && (text.is_empty()
                    || !text.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b)
                    }))
            {
                errors.add(field, "is invalid");
            }
        }
        if campfire_richtext::ruby::is_blank(&a.status) {
            errors.add("status", "can't be blank");
        }
        if !STATUSES.contains(&a.status.as_str()) {
            errors.add("status", "is not included in the list");
        }
        if a.expires_at.is_none() {
            errors.add("expires_at", "can't be blank");
        }
        if a.decision_note
            .as_ref()
            .is_some_and(|s| s.chars().count() > 200)
        {
            errors.add("decision_note", "is too long (maximum is 200 characters)");
        }
        if let Some(external) = &a.external_id
            && exists(
                conn,
                "SELECT 1 FROM agent_approvals WHERE agent_id=? AND external_id=? AND (? IS NULL OR id!=?)",
                params![a.agent_id, external, exclude, exclude],
            )?
        {
            errors.add("external_id", "has already been taken");
        }
        if a.payload.as_ref().is_some_and(|s| s.len() > 4096) {
            errors.add("payload", "is too large (maximum is 4 KB)");
        }
        if exclude.is_none()
            && let Some(expires) = a.expires_at
        {
            if expires < now.since(jiff::SignedDuration::from_mins(4)) {
                errors.add("expires_at", "must be at least 5 minutes from now");
            } else if expires > now.since(jiff::SignedDuration::from_secs(7 * 86400 + 60)) {
                errors.add("expires_at", "must be within 7 days from now");
            }
        }
        Ok(errors)
    }
    pub fn create(tx: &mut Tx<'_>, a: NewApproval) -> Result<Self> {
        let now = tx.now();
        let a = a.normalized(now);
        Self::validate(tx.conn(), &a, now, None)?.into_result()?;
        let id=tx.conn().query_row("INSERT INTO agent_approvals(agent_id,agent_credential_id,room_id,action,summary,payload,external_id,status,expires_at,decided_by_id,decided_at,decision_note,github_account_id,github_login,fizzy_connected_account_id,fizzy_user_id,fizzy_user_name,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",params![a.agent_id,a.agent_credential_id,a.room_id,a.action,a.summary,a.payload,a.external_id,a.status,a.expires_at,a.decided_by_id,a.decided_at,a.decision_note,a.github_account_id,a.github_login,a.fizzy_connected_account_id,a.fizzy_user_id,a.fizzy_user_name,now,now],|r|r.get(0))?;
        let record = Self::find(tx.conn(), id)?.expect("inserted approval");
        let approval = record.clone();
        tx.after_commit(move |tx| approval.fan_out_inbox_items_after_commit(tx));
        Ok(record)
    }
    fn attributes(&self) -> NewApproval {
        NewApproval {
            agent_id: self.agent_id,
            agent_credential_id: self.agent_credential_id,
            room_id: self.room_id,
            action: self.action.clone(),
            summary: self.summary.clone(),
            payload: self.payload.clone(),
            external_id: self.external_id.clone(),
            status: self.status.clone(),
            expires_at: Some(self.expires_at),
            decided_by_id: self.decided_by_id,
            decided_at: self.decided_at,
            decision_note: self.decision_note.clone(),
            github_account_id: self.github_account_id,
            github_login: self.github_login.clone(),
            fizzy_connected_account_id: self.fizzy_connected_account_id,
            fizzy_user_id: self.fizzy_user_id.clone(),
            fizzy_user_name: self.fizzy_user_name.clone(),
        }
    }
    pub fn effective_status(&self, now: Timestamp) -> &str {
        if self.status == "pending" && self.expires_at <= now {
            "expired"
        } else {
            &self.status
        }
    }
    pub fn expire_if_due(&mut self, tx: &mut Tx<'_>) -> Result<bool> {
        *self =
            Self::find(tx.conn(), self.id)?.ok_or(crate::Error::RecordNotFound("AgentApproval"))?;
        let now = tx.now();
        if self.status != "pending" || self.expires_at > now {
            return Ok(false);
        };
        let mut a = self.attributes();
        a.status = "expired".into();
        Self::validate(tx.conn(), &a, now, Some(self.id))?.into_result()?;
        let revision = tx.revision_after(self.updated_at);
        tx.conn().execute(
            "UPDATE agent_approvals SET status='expired',updated_at=? WHERE id=?",
            params![revision, self.id],
        )?;
        self.status = "expired".into();
        self.updated_at = revision;
        ActivityItem::handle_for_source(tx, "AgentApproval", self.id)?;
        ApprovalChange::emit(tx, self.id);
        Ok(true)
    }
    fn pending_errors(&self, now: Timestamp) -> Errors {
        let mut errors = Errors::default();
        if self.effective_status(now) != "pending" {
            errors.add(
                "base",
                if self.status == "expired" {
                    "Request has expired".into()
                } else {
                    format!("Request is already {}", self.status)
                },
            );
        }
        errors
    }
    /// Expected validation failures are values, allowing lazy expiry to commit even
    /// when the requested decision is rejected. SQL failures still roll back.
    pub fn decide(
        &mut self,
        tx: &mut Tx<'_>,
        decision: &str,
        by: &User,
        note: Option<&str>,
    ) -> Result<Errors> {
        if !["approved", "denied"].contains(&decision) {
            return Err(crate::Error::Other(format!("Unknown decision: {decision}")));
        };
        self.expire_if_due(tx)?;
        let errors = self.pending_errors(tx.now());
        if !errors.is_empty() {
            return Ok(errors);
        };
        let note = note.filter(|s| !campfire_richtext::ruby::is_blank(s));
        let mut a = self.attributes();
        a.status = decision.into();
        a.decided_by_id = Some(by.id);
        let now = tx.now();
        a.decided_at = Some(now);
        a.decision_note = note.map(str::to_owned);
        let errors = Self::validate(tx.conn(), &a, now, Some(self.id))?;
        if !errors.is_empty() {
            return Ok(errors);
        };
        let revision = tx.revision_after(self.updated_at);
        tx.conn().execute("UPDATE agent_approvals SET status=?,decided_by_id=?,decided_at=?,decision_note=?,updated_at=? WHERE id=?",params![decision,by.id,now,note,revision,self.id])?;
        *self = Self::find(tx.conn(), self.id)?.expect("updated approval");
        ActivityItem::handle_for_source(tx, "AgentApproval", self.id)?;
        ApprovalChange::emit(tx, self.id);
        let event = create_delivered(
            tx,
            NewEvent {
                agent_id: self.agent_id,
                room_id: self.room_id,
                actor_id: Some(by.id),
                agent_approval_id: Some(self.id),
                event_type: "approval_decided".into(),
                metadata: json!({"approval_id":self.id,"status":self.status,"decided_by":by.name,"note":self.decision_note}),
                ..Default::default()
            },
        )?;
        if decision == "approved" {
            if self.github_action() {
                tx.emit_after_commit(Event::job(&GithubAction {
                    approval_id: self.id,
                }));
            }
            if self.fizzy_action() {
                tx.emit_after_commit(Event::job(&FizzyAction {
                    approval_id: self.id,
                }));
            }
        }
        enqueue_delivered_webhook(tx, &event);
        Ok(Errors::default())
    }
    pub fn decide_authorized(
        &mut self,
        tx: &mut Tx<'_>,
        decision: &str,
        by: &User,
        note: Option<&str>,
    ) -> Result<ApprovalDecision> {
        *self =
            Self::find(tx.conn(), self.id)?.ok_or(crate::Error::RecordNotFound("AgentApproval"))?;
        if !self.decidable_by(tx.conn(), by)?
            || (decision == "approved" && !self.approvable_by(tx.conn(), by)?)
        {
            return Ok(ApprovalDecision::Forbidden);
        };
        let errors = self.decide(tx, decision, by, note)?;
        Ok(if errors.is_empty() {
            ApprovalDecision::Applied
        } else {
            ApprovalDecision::Invalid(errors)
        })
    }
    pub fn cancel_by_agent(&mut self, tx: &mut Tx<'_>) -> Result<Errors> {
        self.expire_if_due(tx)?;
        let errors = self.pending_errors(tx.now());
        if !errors.is_empty() {
            return Ok(errors);
        };
        let mut a = self.attributes();
        a.status = "cancelled".into();
        let errors = Self::validate(tx.conn(), &a, tx.now(), Some(self.id))?;
        if !errors.is_empty() {
            return Ok(errors);
        };
        let revision = tx.revision_after(self.updated_at);
        tx.conn().execute(
            "UPDATE agent_approvals SET status='cancelled',updated_at=? WHERE id=?",
            params![revision, self.id],
        )?;
        self.status = "cancelled".into();
        self.updated_at = revision;
        ActivityItem::handle_for_source(tx, "AgentApproval", self.id)?;
        ApprovalChange::emit(tx, self.id);
        Ok(Errors::default())
    }
    pub fn github_action(&self) -> bool {
        self.action.starts_with("github.")
    }
    pub fn fizzy_action(&self) -> bool {
        self.action.starts_with("fizzy.")
    }
    pub fn github_identity_matches(&self, id: i64, login: &str) -> bool {
        self.github_account_id == Some(id)
            && self.github_login.as_deref().is_some_and(|s| {
                !campfire_richtext::ruby::is_blank(s) && unicode::fold(s) == unicode::fold(login)
            })
    }
    pub fn fizzy_identity_matches(&self, id: i64, user_id: &str) -> bool {
        self.fizzy_connected_account_id == Some(id)
            && self
                .fizzy_user_id
                .as_deref()
                .is_some_and(|s| !campfire_richtext::ruby::is_blank(s) && s == user_id)
    }
    pub fn decidable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        // Re-read the actor and agent user so cached request objects cannot preserve
        // authority after a concurrent deactivation or ownership change.
        exists(
            conn,
            "SELECT 1 FROM agents a JOIN users bot ON bot.id=a.user_id JOIN users actor ON actor.id=? WHERE a.id=? AND bot.status=0 AND actor.status=0 AND actor.role!=2 AND (actor.role=1 OR a.owner_id=actor.id)",
            params![user.id, self.agent_id],
        )
    }
    pub fn approvable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(self.decidable_by(conn, user)?
            && (!(self.github_action() || self.fizzy_action())
                || exists(conn, "SELECT 1 FROM users WHERE id=? AND role=1", [user.id])?))
    }
    pub fn decider_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        query_all(
            conn,
            "SELECT u.id FROM users u WHERE u.status=0 AND u.role!=2 AND (u.role=1 OR u.id=(SELECT owner_id FROM agents WHERE id=?)) ORDER BY CASE WHEN u.id=(SELECT owner_id FROM agents WHERE id=?) THEN 0 ELSE 1 END,u.id",
            params![self.agent_id, self.agent_id],
            |r| r.get(0),
        )
    }
    pub fn fan_out_inbox_items(&self, tx: &mut Tx<'_>) -> Result<()> {
        for user in self.decider_ids(tx.conn())? {
            self.fan_out_inbox_item(tx, user)?;
        }
        Ok(())
    }
    fn fan_out_inbox_items_after_commit(&self, tx: &mut Tx<'_>) -> Result<()> {
        for user in self.decider_ids(tx.conn())? {
            // Rails' after_create_commit loops over create_or_find_by!: a later
            // recipient's failure does not roll back earlier recipients' items.
            crate::database::run_write(tx.conn(), tx.env(), |tx| {
                self.fan_out_inbox_item(tx, user)
            })?;
        }
        Ok(())
    }
    fn fan_out_inbox_item(&self, tx: &mut Tx<'_>, user: i64) -> Result<()> {
        let raw: Option<Value> = tx.conn().query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |r| r.get(0),
        )?;
        let enabled = raw
            .as_ref()
            .and_then(|v| v.get("agent_approvals"))
            .is_none_or(|v| {
                !matches!(v, Value::Bool(false))
                    && *v != json!(0)
                    && *v != json!("0")
                    && *v != json!("false")
            });
        if enabled
            && ActivityItem::find_by_user_and_source(tx.conn(), user, "AgentApproval", self.id)?
                .is_none()
        {
            ActivityItem::refresh_unread(
                tx,
                user,
                "AgentApproval",
                self.id,
                "agent_approval_request",
            )?;
        }
        Ok(())
    }
    pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()> {
        let ids = query_all(
            tx.conn(),
            "SELECT DISTINCT source_id FROM activity_items WHERE source_type='AgentApproval' AND event_type='agent_approval_request' AND handled_at IS NULL AND (? IS NULL OR user_id=?)",
            params![user_id, user_id],
            |r| r.get::<_, i64>(0),
        )?;
        for id in ids {
            if let Some(mut approval) = Self::find(tx.conn(), id)? {
                approval.expire_if_due(tx)?;
            }
        }
        Ok(())
    }
    pub fn created_payload(&self, now: Timestamp) -> Value {
        json!({"id":self.id,"status":self.effective_status(now),"expires_at":json_time(self.expires_at)})
    }
    pub fn payload(&self, conn: &Connection, now: Timestamp) -> Result<Value> {
        let room = self
            .room_id
            .map(|id| {
                query_one(conn, "SELECT name FROM rooms WHERE id=?", [id], |r| {
                    r.get::<_, Option<String>>(0)
                })
            })
            .transpose()?
            .flatten()
            .flatten();
        let decided_by = self
            .decided_by_id
            .map(|id| {
                query_one(conn, "SELECT name FROM users WHERE id=?", [id], |r| {
                    r.get::<_, String>(0)
                })
            })
            .transpose()?
            .flatten();
        let payload = self
            .payload
            .as_ref()
            .map(|s| serde_json::from_str::<Value>(s).unwrap_or_else(|_| json!(s)));
        Ok(compact(
            json!({"id":self.id,"action":self.action,"summary":self.summary,"payload":payload,"room_id":self.room_id,"room_name":room,"external_id":self.external_id,"status":self.effective_status(now),"expires_at":json_time(self.expires_at),"created_at":json_time(self.created_at),"decided_by":decided_by,"decided_by_id":self.decided_by_id,"decided_at":self.decided_at.map(json_time),"decision_note":self.decision_note,"note":self.decision_note}),
        ))
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        ActivityItem::destroy_for_source(tx, "AgentApproval", self.id)?;
        tx.conn()
            .execute("DELETE FROM agent_approvals WHERE id=?", [self.id])?;
        Ok(())
    }
}

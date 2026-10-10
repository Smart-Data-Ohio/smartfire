//! Agent records, live read policies, profile status and working presence. Rendering
//! adapters publish the status change described by `AgentSyncChange`.
use crate::sql::{exists, query_all, query_one};
use crate::{AgentGrant, Connection, Errors, Event, Result, Timestamp, Tx, User};
use rails_compat::unicode;
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};

pub mod cap_input;
pub use cap_input::BudgetCapInput;

pub const STATUSES: [&str; 4] = ["idle", "working", "waiting", "failed"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentKind {
    #[default]
    Personal,
    Workspace,
}
impl AgentKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Workspace => "workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: i64,
    pub user_id: i64,
    pub owner_id: Option<i64>,
    pub kind: AgentKind,
    pub description: Option<String>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub status: String,
    pub status_note: Option<String>,
    pub status_changed_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
    pub suspended_at: Option<Timestamp>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub working_presence: Option<String>,
    pub working_presence_expires_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone)]
pub struct NewAgent {
    pub user_id: i64,
    pub owner_id: Option<i64>,
    pub kind: AgentKind,
    pub description: Option<String>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub status: String,
    pub status_note: Option<String>,
    pub suspended_at: Option<Timestamp>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub working_presence: Option<String>,
    pub working_presence_expires_at: Option<Timestamp>,
}
impl Default for NewAgent {
    fn default() -> Self {
        Self {
            user_id: 0,
            owner_id: None,
            kind: AgentKind::Personal,
            description: None,
            provider: None,
            runtime: None,
            status: "idle".into(),
            status_note: None,
            suspended_at: None,
            daily_message_cap: None,
            daily_board_post_cap: None,
            daily_external_action_cap: None,
            working_presence: None,
            working_presence_expires_at: None,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct AgentChanges {
    pub owner_id: Option<Option<i64>>,
    pub kind: Option<AgentKind>,
    pub description: Option<Option<String>>,
    pub provider: Option<Option<String>>,
    pub runtime: Option<Option<String>>,
    pub status: Option<String>,
    pub status_note: Option<Option<String>>,
    pub suspended_at: Option<Option<Timestamp>>,
    pub daily_message_cap: Option<Option<i64>>,
    pub daily_board_post_cap: Option<Option<i64>>,
    pub daily_external_action_cap: Option<Option<i64>>,
    /// Original permitted scalar, including null. None means no raw input was submitted.
    pub daily_message_cap_before_type_cast: Option<serde_json::Value>,
    pub daily_board_post_cap_before_type_cast: Option<serde_json::Value>,
    pub daily_external_action_cap_before_type_cast: Option<serde_json::Value>,
}

impl AgentChanges {
    /// The cast value and errors are available even when update rejects the input;
    /// callers keep this value to redisplay Rails' before-type-cast form input.
    pub fn budget_cap_input(&self, field: &str) -> Option<BudgetCapInput> {
        let (raw, typed) = match field {
            "daily_message_cap" => (
                &self.daily_message_cap_before_type_cast,
                self.daily_message_cap,
            ),
            "daily_board_post_cap" => (
                &self.daily_board_post_cap_before_type_cast,
                self.daily_board_post_cap,
            ),
            "daily_external_action_cap" => (
                &self.daily_external_action_cap_before_type_cast,
                self.daily_external_action_cap,
            ),
            _ => return None,
        };
        raw.clone()
            .or_else(|| typed.map(|v| serde_json::json!(v)))
            .map(BudgetCapInput::new)
    }
}

/// One capability's active grants ([`Agent::grant_lines`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantLine {
    pub capability: &'static str,
    /// A grant with no room is active.
    pub workspace_wide: bool,
    /// Rooms with an active grant; 0 when `workspace_wide`.
    pub room_count: i64,
}

/// The last 24 hours of an agent's deliveries ([`Agent::activity_counts`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityCounts {
    pub delivered: i64,
    pub acknowledged: i64,
    pub posted: i64,
    pub suppressed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSyncChange {
    pub agent_id: i64,
}
impl crate::events::Broadcast for AgentSyncChange {
    const KIND: &'static str = "Agent#sync_status";
}

impl Agent {
    pub(crate) fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        let kind: String = r.get("kind")?;
        let kind = match kind.as_str() {
            "personal" => AgentKind::Personal,
            "workspace" => AgentKind::Workspace,
            _ => {
                return Err(rusqlite::Error::InvalidColumnType(
                    r.as_ref().column_index("kind")?,
                    "kind".into(),
                    rusqlite::types::Type::Text,
                ));
            }
        };
        Ok(Self {
            id: r.get("id")?,
            user_id: r.get("user_id")?,
            owner_id: r.get("owner_id")?,
            kind,
            description: r.get("description")?,
            provider: r.get("provider")?,
            runtime: r.get("runtime")?,
            status: r.get("status")?,
            status_note: r.get("status_note")?,
            status_changed_at: r.get("status_changed_at")?,
            last_seen_at: r.get("last_seen_at")?,
            suspended_at: r.get("suspended_at")?,
            daily_message_cap: r.get("daily_message_cap")?,
            daily_board_post_cap: r.get("daily_board_post_cap")?,
            daily_external_action_cap: r.get("daily_external_action_cap")?,
            working_presence: r.get("working_presence")?,
            working_presence_expires_at: r.get("working_presence_expires_at")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agents WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    /// Read the encrypted attribute without generating or touching a secret.
    pub fn webhook_signing_secret(
        &self,
        conn: &Connection,
        encryption: &rails_compat::ar_encryption::ArEncryption,
    ) -> Result<Option<String>> {
        let encrypted: Option<String> = conn.query_row(
            "SELECT webhook_signing_secret FROM agents WHERE id=?",
            [self.id],
            |r| r.get(0),
        )?;
        encrypted
            .map(|value| {
                encryption
                    .decrypt(&value)
                    .map_err(|e| crate::Error::Other(e.to_string()))
            })
            .transpose()
    }
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agents WHERE user_id=? LIMIT 1",
            [user_id],
            Self::from_row,
        )
    }
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        crate::sql::query_all(
            conn,
            "SELECT * FROM agents WHERE id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(ids).to_string()],
            Self::from_row,
        )
    }
    /// `users.preload(:agent)`: one read for the bot members of a room.
    pub fn for_users(conn: &Connection, user_ids: &[i64]) -> Result<Vec<Self>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        crate::sql::query_all(
            conn,
            "SELECT * FROM agents WHERE user_id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(user_ids).to_string()],
            Self::from_row,
        )
    }
    pub fn validate(conn: &Connection, a: &NewAgent, exclude: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if !exists(conn, "SELECT 1 FROM users WHERE id=?", [a.user_id])? {
            errors.add("user", "must exist");
        }
        if exists(
            conn,
            "SELECT 1 FROM agents WHERE user_id=? AND (? IS NULL OR id!=?)",
            params![a.user_id, exclude, exclude],
        )? {
            errors.add("user_id", "has already been taken");
        }
        if a.owner_id.is_none() && (a.kind == AgentKind::Personal || exclude.is_none()) {
            errors.add("owner_id", "can't be blank");
        }
        if !STATUSES.contains(&a.status.as_str()) {
            errors.add("status", "is not included in the list");
        }
        for (field, text, limit) in [
            ("description", &a.description, 500),
            ("status_note", &a.status_note, 200),
            ("working_presence", &a.working_presence, 140),
        ] {
            if text.as_ref().is_some_and(|s| s.chars().count() > limit) {
                errors.add(
                    field,
                    format!("is too long (maximum is {limit} characters)"),
                );
            }
        }
        for (field, cap) in [
            ("daily_message_cap", a.daily_message_cap),
            ("daily_board_post_cap", a.daily_board_post_cap),
            ("daily_external_action_cap", a.daily_external_action_cap),
        ] {
            if cap.is_some_and(|c| c <= 0) {
                errors.add(field, "must be greater than 0");
            }
        }
        Ok(errors)
    }
    pub fn create(tx: &Tx<'_>, a: NewAgent) -> Result<Self> {
        Self::validate(tx.conn(), &a, None)?.into_result()?;
        let id=tx.conn().query_row("INSERT INTO agents(user_id,owner_id,kind,description,provider,runtime,status,status_note,suspended_at,daily_message_cap,daily_board_post_cap,daily_external_action_cap,working_presence,working_presence_expires_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",params![a.user_id,a.owner_id,a.kind.name(),a.description,a.provider,a.runtime,a.status,a.status_note,a.suspended_at,a.daily_message_cap,a.daily_board_post_cap,a.daily_external_action_cap,a.working_presence,a.working_presence_expires_at,tx.now(),tx.now()],|r|r.get(0))?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted agent"))
    }
    fn attributes(&self) -> NewAgent {
        NewAgent {
            user_id: self.user_id,
            owner_id: self.owner_id,
            kind: self.kind,
            description: self.description.clone(),
            provider: self.provider.clone(),
            runtime: self.runtime.clone(),
            status: self.status.clone(),
            status_note: self.status_note.clone(),
            suspended_at: self.suspended_at,
            daily_message_cap: self.daily_message_cap,
            daily_board_post_cap: self.daily_board_post_cap,
            daily_external_action_cap: self.daily_external_action_cap,
            working_presence: self.working_presence.clone(),
            working_presence_expires_at: self.working_presence_expires_at,
        }
    }
    /// Read-only validation for forms; raw cap inputs are checked before any bot write.
    pub fn validate_changes(&self, conn: &Connection, changes: AgentChanges) -> Result<Errors> {
        Ok(self.changed_candidate(conn, changes)?.1)
    }
    fn changed_candidate(
        &self,
        conn: &Connection,
        changes: AgentChanges,
    ) -> Result<(Self, Errors, bool)> {
        let mut candidate =
            Self::find(conn, self.id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
        let raw_caps = [
            (
                "daily_message_cap",
                changes.daily_message_cap_before_type_cast.clone(),
            ),
            (
                "daily_board_post_cap",
                changes.daily_board_post_cap_before_type_cast.clone(),
            ),
            (
                "daily_external_action_cap",
                changes.daily_external_action_cap_before_type_cast.clone(),
            ),
        ];
        macro_rules! assign {($($field:ident),*)=>{$(if let Some(value)=changes.$field {candidate.$field=value;})*};}
        assign!(
            owner_id,
            kind,
            description,
            provider,
            runtime,
            status,
            status_note,
            suspended_at,
            daily_message_cap,
            daily_board_post_cap,
            daily_external_action_cap
        );
        let mut raw_errors = Errors::default();
        let mut out_of_range = false;
        for (field, raw) in &raw_caps {
            if let Some(raw) = raw {
                let input = BudgetCapInput::new(raw.clone());
                for message in input.errors {
                    raw_errors.add(field, message);
                }
                let value = input.value.as_i64();
                out_of_range |= !input.value.is_null() && value.is_none();
                match *field {
                    "daily_message_cap" => candidate.daily_message_cap = value,
                    "daily_board_post_cap" => candidate.daily_board_post_cap = value,
                    _ => candidate.daily_external_action_cap = value,
                }
            }
        }
        let mut errors = Self::validate(conn, &candidate.attributes(), Some(candidate.id))?;
        errors.0.retain(|(field, _)| {
            !raw_caps
                .iter()
                .any(|(raw_field, raw)| raw.is_some() && field == raw_field)
        });
        errors.0.extend(raw_errors.0);
        // Keep cap messages in the model's declared field order.
        let mut caps = Vec::new();
        errors.0.retain(|(field, message)| {
            if raw_caps.iter().any(|(cap, _)| field == cap) {
                caps.push((*field, message.clone()));
                false
            } else {
                true
            }
        });
        for (field, _) in &raw_caps {
            errors
                .0
                .extend(caps.iter().filter(|(cap, _)| cap == field).cloned());
        }
        Ok((candidate, errors, out_of_range))
    }
    pub fn update(&mut self, tx: &mut Tx<'_>, changes: AgentChanges) -> Result<()> {
        let (mut candidate, errors, out_of_range) = self.changed_candidate(tx.conn(), changes)?;
        errors.into_result()?;
        if out_of_range {
            return Err(crate::Error::Other(
                "budget cap is out of range for SQLite integer storage".into(),
            ));
        }
        candidate.save(tx)?;
        *self = candidate;
        Ok(())
    }
    pub fn save(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let before =
            Self::find(tx.conn(), self.id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
        Self::validate(tx.conn(), &self.attributes(), Some(self.id))?.into_result()?;
        let mut sets: Vec<(&str, Box<dyn rusqlite::ToSql + '_>)> = vec![];
        macro_rules! changed {($($field:ident),*)=>{$(if self.$field!=before.$field {sets.push((stringify!($field),Box::new(&self.$field)));})*};}
        if self.kind != before.kind {
            sets.push(("kind", Box::new(self.kind.name())));
        }
        changed!(
            owner_id,
            description,
            provider,
            runtime,
            status,
            status_note,
            suspended_at,
            daily_message_cap,
            daily_board_post_cap,
            daily_external_action_cap,
            working_presence,
            working_presence_expires_at
        );
        if sets.is_empty() {
            return Ok(());
        };
        if self.suspended_at.is_some() && self.suspended_at != before.suspended_at {
            AgentGrant::revoke_for_agent(tx, self.id)?;
        }
        if self.status != before.status {
            let now = tx.now();
            self.status_changed_at = Some(now);
            sets.push(("status_changed_at", Box::new(now)));
        }
        let revision = tx.revision_after(before.updated_at);
        self.updated_at = revision;
        sets.push(("updated_at", Box::new(revision)));
        let assignments = sets
            .iter()
            .map(|(field, _)| format!("{field}=?"))
            .collect::<Vec<_>>()
            .join(",");
        let mut values: Vec<&dyn rusqlite::ToSql> =
            sets.iter().map(|(_, value)| value.as_ref()).collect();
        values.push(&self.id);
        tx.conn().execute(
            &format!("UPDATE agents SET {assignments} WHERE id=?"),
            values.as_slice(),
        )?;
        drop(values);
        drop(sets);
        *self = Self::find(tx.conn(), self.id)?.expect("updated agent");
        if self.status != before.status
            || self.status_note != before.status_note
            || self.suspended_at != before.suspended_at
            || self.working_presence != before.working_presence
            || self.working_presence_expires_at != before.working_presence_expires_at
        {
            tx.emit_after_commit(Event::broadcast(&AgentSyncChange { agent_id: self.id }));
        }
        Ok(())
    }
    /// Agent's declared dependent destroys, in Rails declaration order. This is
    /// different from User::Bot's has_one :agent, dependent: :delete.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.savepoint(|tx| self.destroy_inner(tx))
    }
    fn destroy_inner(&self, tx: &mut Tx<'_>) -> Result<()> {
        for table in [
            "agent_credentials",
            "agent_grants",
            "agent_slash_commands",
            "agent_events",
        ] {
            tx.conn()
                .execute(&format!("DELETE FROM {table} WHERE agent_id=?"), [self.id])?;
        }
        let approvals = crate::sql::query_all(
            tx.conn(),
            "SELECT id FROM agent_approvals WHERE agent_id=? ORDER BY id",
            [self.id],
            |r| r.get::<_, i64>(0),
        )?;
        for id in approvals {
            if let Some(approval) = crate::AgentApproval::find(tx.conn(), id)? {
                approval.destroy(tx)?;
            }
        }
        // Budget notices, steps and handoffs are not declared Agent dependents.
        let notices = crate::sql::query_all(
            tx.conn(),
            "SELECT id FROM agent_budget_notices WHERE agent_id=?",
            [self.id],
            |r| r.get::<_, i64>(0),
        )?;
        crate::ActivityItem::emit_hidden_for_sources(tx, "AgentBudgetNotice", &notices)?;
        tx.conn()
            .execute("DELETE FROM agents WHERE id=?", [self.id])?;
        Ok(())
    }
    /// The migration's one-shot data operation, not a schema migration or boot hook.
    /// Repeating it with existing Agent rows fails the same unique index as Rails.
    pub fn backfill_existing_bots(tx: &Tx<'_>) -> Result<usize> {
        Ok(tx.conn().execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) SELECT id,NULL,'workspace',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP FROM users WHERE role=2",[])?)
    }
    pub fn active(&self, conn: &Connection) -> Result<bool> {
        exists(
            conn,
            "SELECT 1 FROM agents a JOIN users u ON u.id=a.user_id WHERE a.id=? AND a.suspended_at IS NULL AND u.status=0",
            [self.id],
        )
    }
    pub fn suspended(&self) -> bool {
        self.suspended_at.is_some()
    }
    pub fn legacy_capabilities(&self, conn: &Connection) -> Result<bool> {
        Ok(!exists(
            conn,
            "SELECT 1 FROM agent_grants WHERE agent_id=?",
            [self.id],
        )?)
    }
    pub fn can(&self, conn: &Connection, capability: &str, room_id: Option<i64>) -> Result<bool> {
        super::agent_access::capability_for_agent(conn, self.id, capability, room_id)
    }
    pub fn capabilities_for_rooms(
        conn: &Connection,
        capability: &str,
        requests: &[(i64, Option<i64>)],
    ) -> Result<std::collections::HashMap<(i64, Option<i64>), bool>> {
        super::agent_access::capabilities_for_agents(conn, capability, requests)
    }
    pub fn has_capability_anywhere(&self, conn: &Connection, capability: &str) -> Result<bool> {
        super::agent_access::has_capability_anywhere(conn, self.id, capability)
    }
    pub fn touch_last_seen(&mut self, tx: &Tx<'_>) -> Result<()> {
        let now = tx.now();
        if tx.conn().execute("UPDATE agents SET last_seen_at=? WHERE id=? AND (last_seen_at IS NULL OR last_seen_at<=?)",params![now,self.id,now.ago(jiff::SignedDuration::from_mins(1))])?>0 {self.last_seen_at=Some(now);}
        Ok(())
    }
    pub fn working_presence_text(&self, now: Timestamp) -> Option<&str> {
        self.working_presence.as_deref().filter(|s| {
            !campfire_richtext::ruby::is_blank(s)
                && self
                    .working_presence_expires_at
                    .is_none_or(|expires| expires > now)
        })
    }
    pub fn assign_working_presence(&mut self, text: Option<&str>, now: Timestamp) {
        self.working_presence = text
            .map(campfire_richtext::ruby::strip)
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .map(str::to_owned);
        self.working_presence_expires_at = self
            .working_presence
            .as_ref()
            .map(|_| now.since(jiff::SignedDuration::from_mins(5)));
    }
    pub fn set_working_presence(&mut self, tx: &mut Tx<'_>, text: Option<&str>) -> Result<()> {
        self.assign_working_presence(text, tx.now());
        self.save(tx)
    }
    pub fn clear_working_presence(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self
            .working_presence
            .as_deref()
            .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
        {
            self.working_presence = None;
            self.working_presence_expires_at = None;
            self.save(tx)?;
        }
        Ok(())
    }
    pub fn kind_description(&self, conn: &Connection) -> Result<String> {
        let owner = self
            .owner_id
            .map(|id| User::find_by_id(conn, id))
            .transpose()?
            .flatten();
        Ok(match owner {
            None => "no owner recorded".into(),
            Some(owner) => match self.kind {
                AgentKind::Personal => format!("Personal agent of {}", owner.name),
                AgentKind::Workspace => format!("Workspace agent, managed by {}", owner.name),
            },
        })
    }
    /// The active grants by capability, in capability name order: `None` for legacy access (no
    /// grant rows recorded). [`Self::grants_summary`] words them.
    pub fn grant_lines(&self, conn: &Connection) -> Result<Option<Vec<GrantLine>>> {
        if self.legacy_capabilities(conn)? {
            return Ok(None);
        };
        let mut capabilities = super::agent_access::CAPABILITIES;
        capabilities.sort();
        let mut lines = vec![];
        for capability in capabilities {
            let rooms = query_all(
                conn,
                "SELECT room_id FROM agent_grants WHERE agent_id=? AND capability=? AND revoked_at IS NULL",
                params![self.id, capability],
                |r| r.get::<_, Option<i64>>(0),
            )?;
            if rooms.contains(&None) {
                lines.push(GrantLine {
                    capability,
                    workspace_wide: true,
                    room_count: 0,
                });
            } else if !rooms.is_empty() {
                lines.push(GrantLine {
                    capability,
                    workspace_wide: false,
                    room_count: rooms.len() as i64,
                });
            }
        }
        Ok(Some(lines))
    }
    pub fn grants_summary(&self, conn: &Connection) -> Result<String> {
        let Some(lines) = self.grant_lines(conn)? else {
            return Ok("legacy access (no grants recorded)".into());
        };
        let summaries = lines
            .iter()
            .map(|line| {
                if line.workspace_wide {
                    format!("{} workspace-wide", line.capability)
                } else {
                    format!(
                        "{} in {} {}",
                        line.capability,
                        line.room_count,
                        if line.room_count == 1 {
                            "room"
                        } else {
                            "rooms"
                        }
                    )
                }
            })
            .collect::<Vec<_>>();
        Ok(if summaries.is_empty() {
            "no active grants".into()
        } else {
            summaries.join(", ")
        })
    }
    /// The last 24 hours of deliveries, as [`Self::activity_summary`] counts them.
    pub fn activity_counts(&self, conn: &Connection, now: Timestamp) -> Result<ActivityCounts> {
        Ok(conn.query_row("SELECT COUNT(CASE WHEN outcome='delivered' AND event_type IN ('mention','direct_message','reply','approval_decided','github_action_completed','fizzy_action_completed','work_assigned','work_unassigned','work_handed_off','slash_command') THEN 1 END),COUNT(CASE WHEN outcome='acknowledged' THEN 1 END),COUNT(CASE WHEN event_type='posted' THEN 1 END),COUNT(CASE WHEN outcome='suppressed' THEN 1 END) FROM agent_events WHERE agent_id=? AND created_at>=?",params![self.id,now.ago(jiff::SignedDuration::from_hours(24))],|r|Ok(ActivityCounts { delivered: r.get(0)?, acknowledged: r.get(1)?, posted: r.get(2)?, suppressed: r.get(3)? }))?)
    }
    pub fn activity_summary(&self, conn: &Connection, now: Timestamp) -> Result<String> {
        let ActivityCounts {
            delivered,
            acknowledged,
            posted,
            suppressed,
        } = self.activity_counts(conn, now)?;
        Ok(format!(
            "{delivered} delivered, {acknowledged} acknowledged, {posted} posted, {suppressed} suppressed"
        ))
    }
    pub fn for_directory(conn: &Connection) -> Result<Vec<Self>> {
        Ok(Self::directory_rows(conn)?
            .into_iter()
            .map(|(agent, _, _)| agent)
            .collect())
    }

    /// Rails' eager-loaded directory: complete bot and owner facts travel with each agent.
    pub(crate) fn directory_rows(conn: &Connection) -> Result<Vec<(Self, User, Option<User>)>> {
        let sql = format!(
            "SELECT a.*,{},{},(a.suspended_at IS NULL AND u.status=0) AS active_sort FROM agents a JOIN users u ON u.id=a.user_id LEFT JOIN users owner ON owner.id=a.owner_id WHERE u.status!=1",
            User::projection("u", "directory_user_"),
            User::projection("owner", "directory_owner_")
        );
        let mut rows = query_all(conn, &sql, [], |r| {
            let user = User::from_prefixed_row(r, "directory_user_")?;
            let owner = r
                .get::<_, Option<i64>>("directory_owner_id")?
                .map(|_| User::from_prefixed_row(r, "directory_owner_"))
                .transpose()?;
            let name = unicode::downcase(&user.name);
            Ok((
                Self::from_row(r)?,
                user,
                owner,
                r.get::<_, bool>("active_sort")?,
                name,
            ))
        })?;
        rows.sort_by(|a, b| (!a.3, &a.4).cmp(&(!b.3, &b.4)));
        Ok(rows
            .into_iter()
            .map(|(agent, user, owner, _, _)| (agent, user, owner))
            .collect())
    }
}

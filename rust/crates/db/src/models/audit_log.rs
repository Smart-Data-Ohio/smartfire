//! `app/models/audit_log.rb`. Request owners supply the equivalent of Rails `Current` explicitly.
use crate::sql::{count, exists, query_one};
use crate::{Account, Connection, Result, Room, Timestamp, Tx, User};
use campfire_richtext::ruby::{is_blank, strip, truncate};
use regex::Regex;
use rusqlite::{Row, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
#[derive(Debug, Clone)]
pub struct Actor {
    pub id: i64,
    pub label: String,
}
impl From<&User> for Actor {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            label: format!(
                "{} <{}>",
                user.name,
                user.email_address.as_deref().unwrap_or("")
            ),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Target {
    pub record_type: String,
    pub id: i64,
    pub label: Option<String>,
}
impl From<&User> for Target {
    fn from(user: &User) -> Self {
        Self {
            record_type: "User".into(),
            id: user.id,
            label: Some(Actor::from(user).label),
        }
    }
}
impl From<&Room> for Target {
    fn from(room: &Room) -> Self {
        Self {
            record_type: "Room".into(),
            id: room.id,
            label: room.name.clone(),
        }
    }
}
impl From<&Account> for Target {
    fn from(account: &Account) -> Self {
        Self {
            record_type: "Account".into(),
            id: account.id,
            label: Some(account.name.clone()),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub actor: Option<Actor>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct NewAuditLog {
    pub action: String,
    pub actor: Option<Actor>,
    pub actor_label: Option<String>,
    pub target: Option<Target>,
    pub target_label: Option<String>,
    pub changes: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: i64,
    pub action: String,
    pub actor_id: Option<i64>,
    pub actor_label: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub target_label: Option<String>,
    pub details: Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl AuditLog {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let details: Option<String> = row.get("details")?;
        Ok(Self {
            id: row.get("id")?,
            action: row.get("action")?,
            actor_id: row.get("actor_id")?,
            actor_label: row.get("actor_label")?,
            target_type: row.get("target_type")?,
            target_id: row.get("target_id")?,
            target_label: row.get("target_label")?,
            details: details
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Value::Null),
            ip_address: row.get("ip_address")?,
            user_agent: row.get("user_agent")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            "SELECT * FROM audit_logs WHERE id=?",
            [id],
            Self::from_row,
        )?
        .ok_or_else(|| crate::Error::RecordNotFound("AuditLog".into()))
    }
    pub fn snapshot(&self) -> Value {
        json!({"action":self.action,"actor_id":self.actor_id,"actor_label":self.actor_label,"target_type":self.target_type,"target_id":self.target_id,"target_label":self.target_label,"details":self.details,"ip_address":self.ip_address,"user_agent":self.user_agent})
    }
    pub fn record(tx: &Tx<'_>, input: NewAuditLog, ctx: &Context) -> Result<Self> {
        let mut errors = crate::Errors::default();
        if is_blank(&input.action) {
            errors.add("action", "can't be blank");
        }
        errors.into_result()?;
        let actor = input.actor.as_ref().or(ctx.actor.as_ref());
        let target = input.target.as_ref();
        let actor_label = input
            .actor_label
            .as_deref()
            .or(actor.map(|a| a.label.as_str()));
        let target_label = input
            .target_label
            .as_deref()
            .or(target.and_then(|t| t.label.as_deref()));
        let ip = input.ip_address.as_ref().or(ctx.ip_address.as_ref());
        let agent = input
            .user_agent
            .as_ref()
            .or(ctx.user_agent.as_ref())
            .map(|s| truncate(s, 512, "..."));
        let details = filter_secrets(input.changes.as_ref()).to_string();
        tx.conn().execute("INSERT INTO audit_logs(action,actor_id,actor_label,target_type,target_id,target_label,details,ip_address,user_agent,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![input.action,actor.map(|a|a.id),actor_label,target.map(|t|t.record_type.as_str()),target.map(|t|t.id),target_label,details,ip,agent,tx.now(),tx.now()])?;
        Self::find(tx.conn(), tx.conn().last_insert_rowid())
    }
    pub fn save(&self, _tx: &Tx<'_>) -> Result<()> {
        Err(crate::Error::Other("AuditLog is append-only".into()))
    }
    pub fn update(&self, tx: &Tx<'_>, _action: &str) -> Result<()> {
        self.save(tx)
    }
    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        self.save(tx)
    }
    pub fn delete(&self, tx: &Tx<'_>) -> Result<()> {
        self.save(tx)
    }
    pub fn actor_record(&self, conn: &Connection) -> Result<Option<User>> {
        match self.actor_id {
            Some(id) => User::find_by_id(conn, id),
            None => Ok(None),
        }
    }
    pub fn record_sign_in_failure(
        tx: &Tx<'_>,
        email: &str,
        method: &str,
        ctx: &Context,
    ) -> Result<Option<Self>> {
        let label = failure_actor_label(tx.conn(), Some(email))?;
        if let Some(ip) = ctx.ip_address.as_deref().filter(|s| !is_blank(s)) {
            let cutoff = tx.now().ago(jiff::SignedDuration::from_secs(300));
            if exists(
                tx.conn(),
                "SELECT 1 FROM audit_logs WHERE action='session.sign_in.failure' AND ip_address=? AND created_at>=? AND LOWER(actor_label)=?",
                params![ip, cutoff, label.to_lowercase()],
            )? {
                return Ok(None);
            }
            if count(
                tx.conn(),
                "SELECT COUNT(*) FROM audit_logs WHERE action='session.sign_in.failure' AND ip_address=? AND created_at>=?",
                params![ip, cutoff],
            )? >= 20
            {
                let latest = query_one(
                    tx.conn(),
                    "SELECT * FROM audit_logs WHERE action='session.sign_in.failure' AND ip_address=? AND created_at>=? ORDER BY id DESC LIMIT 1",
                    params![ip, cutoff],
                    Self::from_row,
                )?;
                let Some(mut latest) = latest else {
                    return Ok(None);
                };
                if latest.details.is_null() {
                    latest.details = json!({});
                }
                let n = ruby_integer(&latest.details["suppressed_count"])? + 1;
                latest.details["suppressed_count"] = json!(n);
                // The only update exception: Rails update_all intentionally leaves updated_at unchanged.
                tx.conn().execute(
                    "UPDATE audit_logs SET details=? WHERE id=?",
                    params![latest.details.to_string(), latest.id],
                )?;
                return Ok(Some(latest));
            }
        }
        Self::record(
            tx,
            NewAuditLog {
                action: "session.sign_in.failure".into(),
                actor_label: Some(label),
                changes: Some(json!({"method":method})),
                ..Default::default()
            },
            ctx,
        )
        .map(Some)
    }
}

pub fn actions() -> Vec<String> {
    ACTIONS.iter().map(|s| (*s).into()).collect()
}
static SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)passw|passwd|pwd|secret|token|api[-_]?key|_key$|credential|authorization|cookie|session|join[-_]?code|transfer[-_]?id").unwrap()
});
pub fn filter_secrets(input: Option<&Value>) -> Value {
    fn deep(value: &Value) -> Value {
        match value {
            Value::Object(hash) => Value::Object(
                hash.iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            if SECRET.is_match(key) {
                                json!("[FILTERED]")
                            } else {
                                deep(value)
                            },
                        )
                    })
                    .collect(),
            ),
            Value::Array(values) => Value::Array(values.iter().map(deep).collect()),
            other => other.clone(),
        }
    }
    match input {
        None | Some(Value::Null) => json!({}),
        Some(value @ Value::Object(_)) => deep(value),
        Some(value) => deep(&json!({"value":value})),
    }
}
pub fn failure_email_shape(value: &str) -> bool {
    if value.chars().count() > 1000 || value.matches('@').count() != 1 {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.contains('.')
        && !value
            .chars()
            .any(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\x0b' | '\x0c'))
}
pub fn failure_actor_label(conn: &Connection, email: Option<&str>) -> Result<String> {
    let typed = strip(email.unwrap_or(""));
    if is_blank(typed) {
        return Ok("[unrecognized]".into());
    }
    let known = failure_email_shape(typed)
        || exists(
            conn,
            "SELECT 1 FROM users WHERE LOWER(email_address)=?",
            [typed.to_lowercase()],
        )?;
    Ok(if known {
        truncate(typed, 254, "...")
    } else {
        "[unrecognized]".into()
    })
}
pub fn webhook_origin_summary(url: Option<&str>) -> Result<Option<Value>> {
    let Some(url) = url.filter(|s| !is_blank(s)) else {
        return Ok(None);
    };
    let origin = match campfire_richtext::uri::parse(strip(url)) {
        Ok(mut uri)
            if uri.scheme.as_deref().is_some_and(|s| !is_blank(s))
                && uri.host.as_deref().is_some_and(|s| !is_blank(s)) =>
        {
            uri.scheme = uri.scheme.map(|s|s.to_ascii_lowercase());
            uri.userinfo = None;
            uri.path = None;
            uri.query = None;
            uri.fragment = None;
            uri.opaque = None;
            uri.to_s()
        }
        Err(campfire_richtext::uri::UriError::InvalidComponent) => {
            return Err(crate::Error::Other("URI::InvalidComponentError".into()));
        }
        _ => "[invalid]".into(),
    };
    Ok(Some(
        json!({"origin":origin,"digest":hex::encode(Sha256::digest(url.as_bytes()))[..12]}),
    ))
}
pub fn pair(before: Value, after: Value) -> Value {
    json!({"before":before,"after":after})
}
fn ruby_integer(value: &Value) -> Result<i64> {
    static INTEGER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[ \t\r\n\x0b\x0c]*([+-]?[0-9]+)").unwrap());
    match value {
        Value::Null => Ok(0),
        Value::Number(n) => Ok(n
            .as_i64()
            .or_else(|| n.as_f64().map(|n| n as i64))
            .unwrap_or(0)),
        Value::String(s) => Ok(INTEGER
            .captures(s)
            .and_then(|c| c[1].parse().ok())
            .unwrap_or(0)),
        _ => Err(crate::Error::Other(
            "suppressed_count does not implement to_i".into(),
        )),
    }
}

/// The administrator filter vocabulary; unknown action strings remain valid.
pub const ACTIONS: &[&str] = &[
    "session.sign_in.success",
    "session.sign_in.failure",
    "sign_in.two_factor.failure",
    "sign_in.two_factor.lockout",
    "two_factor.enable",
    "two_factor.disable",
    "two_factor.reset",
    "two_factor.backup_codes.regenerate",
    "two_factor.reauthenticate",
    "two_factor.devices.revoke_all",
    "session.revoke",
    "session.revoke_others",
    "sudo.confirm.success",
    "sudo.confirm.failure",
    "user.create",
    "user.email.change",
    "user.password.change",
    "user.role.change",
    "user.ban",
    "user.unban",
    "user.deactivate",
    "google.sign_in.link",
    "google.sign_in.link_allow",
    "google.sign_in.unlink",
    "google.account.connect",
    "google.account.disconnect",
    "github.account.connect",
    "github.account.disconnect",
    "fizzy.account.connect",
    "fizzy.account.disconnect",
    "slack.account.connect",
    "slack.account.disconnect",
    "slack.workspace.configure",
    "slack.workspace.remove_credentials",
    "slack.import.start",
    "slack.import.cancel",
    "slack.import.undo",
    "account.join_code.reset",
    "account.settings.change",
    "account.custom_styles.change",
    "agent.create",
    "agent.update",
    "agent.suspend",
    "agent.credential.create",
    "agent.credential.revoke",
    "agent.credential.reset",
    "agent.grant.create",
    "agent.grant.revoke",
    "agent.webhook_url.change",
    "agent.webhook_secret.reset",
    "agent.github.connect",
    "agent.github.disconnect",
    "agent.approval.decide",
    "agent.github_action.execute",
    "agent.fizzy_action.execute",
    "agent.kill_switch",
    "room.create",
    "room.destroy",
    "room.membership.change",
    "board.automation.change",
    "work.handoff",
    "workspace_icon.create",
    "workspace_icon.destroy",
];

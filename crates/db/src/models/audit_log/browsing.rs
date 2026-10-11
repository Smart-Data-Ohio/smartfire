//! Read-only selection for `Accounts::AuditLogsController#filtered_entries`.
use super::AuditLog;
use crate::{Connection, Result, Timestamp};
use rusqlite::types::Value;

pub const PAGE_SIZE: i64 = 50;
pub const CSV_EXPORT_LIMIT: i64 = 5000;
pub const TARGET_TYPES: &[&str] = &[
    "User",
    "Account",
    "Room",
    "Agent",
    "AgentCredential",
    "AgentGrant",
    "AgentApproval",
    "WorkspaceIcon",
    "WorkspaceInvite",
];

#[derive(Clone, Debug, Default)]
pub struct Filters {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}
impl Filters {
    fn conditions(&self) -> (String, Vec<Value>) {
        let mut clauses = Vec::new();
        let mut values = Vec::new();
        if let Some(actor) = &self.actor {
            // Rails sanitize_sql_like escapes the characters even on SQLite, whose LIKE
            // expression here has no ESCAPE clause. Preserve that observable behavior.
            clauses.push("actor_label LIKE ?");
            values.push(
                format!(
                    "%{}%",
                    actor
                        .replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_")
                )
                .into(),
            );
        }
        for (clause, value) in [
            ("action = ?", &self.action),
            ("target_type = ?", &self.target_type),
        ] {
            if let Some(value) = value {
                clauses.push(clause);
                values.push(value.clone().into());
            }
        }
        for (clause, value) in [("created_at >= ?", self.from), ("created_at <= ?", self.to)] {
            if let Some(value) = value {
                clauses.push(clause);
                values.push(value.to_db().into());
            }
        }
        (
            if clauses.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", clauses.join(" AND "))
            },
            values,
        )
    }
    pub fn count(&self, conn: &Connection) -> Result<i64> {
        let (conditions, values) = self.conditions();
        Ok(conn.query_row(
            &format!("SELECT COUNT(*) FROM audit_logs{conditions}"),
            rusqlite::params_from_iter(values),
            |r| r.get(0),
        )?)
    }
    pub fn entries(&self, conn: &Connection, limit: i64, offset: i64) -> Result<Vec<AuditLog>> {
        let (conditions, mut values) = self.conditions();
        values.extend([limit.into(), offset.into()]);
        let mut statement = conn.prepare(&format!(
            "SELECT * FROM audit_logs{conditions} ORDER BY id DESC LIMIT ? OFFSET ?"
        ))?;
        Ok(statement
            .query_map(rusqlite::params_from_iter(values), AuditLog::from_row)?
            .collect::<rusqlite::Result<_>>()?)
    }
}

//! The fixed sudo-confirmation writes from `AuditLog.record!`. WS12 owns the general audit
//! vocabulary/filter/reader; this API accepts no arbitrary changes or credential material.
use crate::{Result, Tx, User};
use rusqlite::params;

#[derive(Clone)]
pub struct AuthAudit {
    pub actor: User,
    pub ip_address: String,
    pub user_agent: Option<String>,
}

#[derive(Clone, Copy)]
pub enum SudoVerifier {
    Password,
    Totp,
    Google,
}

impl SudoVerifier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Totp => "totp",
            Self::Google => "google",
        }
    }
}

impl AuthAudit {
    pub fn sudo_confirmation(
        &self,
        tx: &mut Tx<'_>,
        verifier: SudoVerifier,
        success: bool,
        subject_mismatch: bool,
    ) -> Result<()> {
        let mut details = serde_json::json!({ "verifier": verifier.as_str() });
        if subject_mismatch {
            details["reason"] = "subject_mismatch".into();
        }
        let label = format!(
            "{} <{}>",
            self.actor.name,
            self.actor.email_address.as_deref().unwrap_or("")
        );
        let ua = self.user_agent.as_ref().map(|ua| {
            if ua.chars().count() > 512 {
                format!("{}...", ua.chars().take(509).collect::<String>())
            } else {
                ua.clone()
            }
        });
        tx.conn().execute("INSERT INTO audit_logs (action, actor_id, actor_label, details, ip_address, user_agent, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![if success { "sudo.confirm.success" } else { "sudo.confirm.failure" }, self.actor.id, label, details.to_string(), self.ip_address, ua, tx.now(), tx.now()])?;
        Ok(())
    }
}

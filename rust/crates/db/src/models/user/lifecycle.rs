//! Authority revoked by Rails User#deactivate, User::Bannable#ban and User's status callback.
//! WS11 can replace the narrow agent suspension primitive with its model; the stored values,
//! grant revocation, audit and after-commit finalization must remain the same.
use crate::{Result, Tx};
use crate::models::audit_log::{AuditLog, NewAuditLog, Context, Target};
use rusqlite::params;

pub(super) fn revoke_agent_grants(tx: &Tx<'_>, user: i64) -> Result<()> {
    tx.conn().execute("UPDATE agent_grants SET revoked_at=?1,updated_at=?1 WHERE revoked_at IS NULL AND agent_id=(SELECT id FROM agents WHERE user_id=?2 LIMIT 1)", params![tx.now(), user])?;
    Ok(())
}

pub(super) fn deactivate(tx: &mut Tx<'_>, user: i64, context: &Context) -> Result<()> {
    let sink = tx.env().sink.clone();
    sink.disconnect_user_accounts(tx, user)?;
    suspend_owned_agents(tx, user, context)?;
    tx.conn().execute("UPDATE users SET ooo_until=NULL,ooo_note=NULL,ooo_broadcast=NULL WHERE id=?", [user])?;
    Ok(())
}

pub(super) fn suspend_owned_agents(tx: &mut Tx<'_>, owner: i64, context: &Context) -> Result<()> {
    let agents = {
        let mut query = tx.conn().prepare("SELECT g.id,g.user_id,u.name FROM agents g LEFT JOIN users u ON u.id=g.user_id WHERE owner_id=? AND suspended_at IS NULL ORDER BY g.id")?;
        query.query_map([owner], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<String>>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, user, name) in agents {
        tx.conn().execute("UPDATE agent_grants SET revoked_at=?1,updated_at=?1 WHERE agent_id=?2 AND revoked_at IS NULL", params![tx.now(),id])?;
        tx.conn().execute("UPDATE agents SET suspended_at=?1,updated_at=?1 WHERE id=?2", params![tx.now(),id])?;
        AuditLog::record(tx, NewAuditLog {
            action: "agent.suspend".into(),
            target: Some(Target {record_type:"Agent".into(), id, label:Some(name.map(|n|format!("Agent {n}")).unwrap_or_else(||format!("Agent #{id}")))}),
            ..Default::default()
        }, context)?;
        // Never finalize a draft while its owner's outer write can still roll back.
        tx.after_commit(move |tx| {
            let messages = crate::Message::by_creator(tx.conn(), user)?;
            for mut message in messages.into_iter().filter(|m| m.streaming) {
                let result = (|| -> Result<()> {
                    if message.claim_stream_finalized(tx)? {
                        tx.conn().execute("UPDATE agents SET working_presence=NULL,working_presence_expires_at=NULL,updated_at=?1 WHERE user_id=?2 AND working_presence IS NOT NULL AND trim(working_presence)!=''", params![tx.now(), user])?;
                        tx.emit_after_commit(crate::Event::broadcast(&QuietStreamFinal {message_id:message.id}));
                    }
                    Ok(())
                })();
                if let Err(error) = result { tracing::error!(%error, message_id=message.id, "Quiet stream finalize failed"); }
            }
            Ok(())
        });
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct QuietStreamFinal { pub message_id: i64 }
impl crate::Broadcast for QuietStreamFinal {
    const KIND: &'static str = "Message#broadcast_stream_final";
}

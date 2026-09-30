//! Connected-account deactivation uses WS15 callbacks; WS11 owns agent suspension.
use crate::{Result, Tx};
use crate::models::audit_log::Context;
pub(super) fn deactivate(tx: &mut Tx<'_>, user: i64, context: &Context) -> Result<()> {
    let sink=tx.env().sink.clone();
    sink.disconnect_user_accounts(tx, user)?;
    crate::models::agent_lifecycle::suspend_owned(tx,user,context)?;
    tx.conn().execute("UPDATE users SET ooo_until=NULL,ooo_note=NULL,ooo_broadcast=NULL WHERE id=?",[user])?;
    Ok(())
}

/// Compatibility with WS15's earlier broadcast seam; suspension now uses the
/// WS11 finalizer, which also clears working presence and emits the thread frame.
#[derive(serde::Serialize,serde::Deserialize)]
pub struct QuietStreamFinal {pub message_id:i64}
impl crate::Broadcast for QuietStreamFinal {const KIND: &'static str="Message#broadcast_stream_final";}

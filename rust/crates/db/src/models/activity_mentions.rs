//! The mention branch of ActivityItems::Recorder. Huddle inbox preferences
//! apply to invitations only; they never suppress neighboring message items.
//! Other inbox winners and grouping remain WS12's recorder integration.
use crate::{
    ActivityItem, Involvement, Membership, Message, Result, ThreadInvolvement, ThreadMembership, Tx,
};

pub fn record(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    if message.system_note || message.streaming {
        return Ok(());
    }
    let recipients = message.mentionees(tx.conn(), tx.rich_text())?;
    for recipient in recipients {
        if recipient.id == message.creator_id || !recipient.is_active() || recipient.is_bot() {
            continue;
        }
        let Some(member) =
            Membership::find_by_room_and_user(tx.conn(), message.room_id, recipient.id)?
        else {
            continue;
        };
        if member.involvement == Some(Involvement::Invisible) {
            continue;
        }
        if let Some(thread_id) = message.thread_id {
            let member =
                ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, recipient.id)?;
            if !member.is_some_and(|m| {
                matches!(
                    m.involvement,
                    ThreadInvolvement::Mentions | ThreadInvolvement::Everything
                )
            }) {
                continue;
            }
        }
        // Rails create_or_find_by! keeps an existing item's read/type state.
        if ActivityItem::find_by_user_and_source(tx.conn(), recipient.id, "Message", message.id)?
            .is_none()
        {
            ActivityItem::refresh_unread(tx, recipient.id, "Message", message.id, "mention")?;
        }
    }
    Ok(())
}

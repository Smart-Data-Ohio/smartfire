//! Saved-item reminder policy through WS17's shared, preloaded status readers.
use crate::{Connection, NotificationKind, NotificationPolicy, Result, Timestamp, UserStatusSettings};
pub fn allows(conn: &Connection, user_id: i64, now: Timestamp) -> Result<bool> {
    let users = UserStatusSettings::for_ids(conn, &[user_id])?;
    Ok(NotificationPolicy {
        recipient: users.get(&user_id), kind: NotificationKind::Reminder,
        room_involvement: None, thread_involvement: None, mentioned: false,
        reply_to_recipient: false, keyword_matched: false, dnd_exception: false, now,
    }.push())
}

//! The message-owned list slot inside WS8b-r's room shell. Unread metadata is per viewer.
use super::{Presenter, Result, page};
use campfire_db::Message;
use askama::Template;

impl Presenter<'_> {
    #[allow(dead_code, reason = "WS8b-r's separately owned room shell supplies the unread facts after merge")]
    pub fn room_message_list(&self, records: &[Message], divider_id: Option<i64>, unread_count: i64) -> Result<String> {
        let base = self.cache_base_url.as_deref().ok_or(campfire_db::Error::Other("room list needs request origin".into()))?;
        let items = self.messages(records)?;
        let account = campfire_db::Account::first(self.conn)?;
        page::render_detached_at(self.app, account.as_ref(), base, |ctx| {
            campfire_views::messages::RoomIndex { ctx, messages: &items,
                unread_index: divider_id.and_then(|id| records.iter().position(|record| record.id == id)), unread_count }.render().map(|list| format!("\n    \n{list}"))
                .map_err(|error| campfire_db::Error::Other(error.to_string()))
        })
    }
}

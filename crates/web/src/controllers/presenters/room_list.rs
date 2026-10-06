//! The message-owned list slot inside WS8b-r's room shell. Unread metadata is per viewer.
use super::{Presenter, Result, page};
use askama::Template;
use campfire_db::Message;

impl Presenter<'_> {
    #[allow(
        dead_code,
        reason = "WS8b-r's separately owned room shell supplies the unread facts after merge"
    )]
    pub fn room_message_list(
        &self,
        records: &[Message],
        divider_id: Option<i64>,
        unread_count: i64,
    ) -> Result<String> {
        let base = self
            .cache_base_url
            .as_deref()
            .ok_or(campfire_db::Error::Other(
                "room list needs request origin".into(),
            ))?;
        let account = campfire_db::Account::first(self.conn)?;
        // Detached fragments and the list use the same already-required account.
        // A page with new messages must cost the same reads as a warm page.
        *self.render_account.borrow_mut() = Some(account.clone());
        let items = self.messages(records)?;
        page::render_detached_in_zone(self.app, account.as_ref(), base, &self.render_zone, |ctx| {
            campfire_views::messages::RoomIndex {
                ctx,
                messages: &items,
                unread_index: divider_id
                    .and_then(|id| records.iter().position(|record| record.id == id)),
                unread_count,
            }
            .render()
            // The room's invitation expression contributes its empty line before
            // the unread branch. Keep this outside the shared message fragments.
            .map(|list| format!("\n    \n{list}"))
            .map_err(|error| campfire_db::Error::Other(error.to_string()))
        })
    }
}

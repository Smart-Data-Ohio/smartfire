//! The aggregate facts in Github::PullRequestsHelper#message_with_pr_cards_cache_key.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, Timestamp};
use campfire_views::fragment_cache::keys::{self, MessageKey};
use rusqlite::OptionalExtension;

impl Presenter<'_> {
    pub fn message_collection_cache_key(&self, message: &Message) -> Result<String> {
        let stamp = |sql: &str| -> Result<Option<jiff::Timestamp>> {
            Ok(self
                .conn
                .query_row(sql, [message.id], |row| row.get::<_, Option<Timestamp>>(0))?
                .map(|time| time.jiff()))
        };
        let cards = stamp("SELECT MAX(updated_at) FROM (
            SELECT c.updated_at FROM github_pull_request_references r JOIN github_pull_requests c ON c.id=r.github_pull_request_id WHERE r.message_id=?1
            UNION ALL SELECT c.updated_at FROM fizzy_card_references r JOIN fizzy_cards c ON c.id=r.fizzy_card_id WHERE r.message_id=?1
            UNION ALL SELECT c.updated_at FROM twitter_post_references r JOIN twitter_posts c ON c.id=r.twitter_post_id WHERE r.message_id=?1
            UNION ALL SELECT c.updated_at FROM event_references r JOIN events c ON c.id=r.event_id WHERE r.message_id=?1)")?;
        let embeds = self.conn.prepare("SELECT r.id, c.updated_at FROM link_embed_references r LEFT JOIN link_embeds c ON c.id=r.link_embed_id WHERE r.message_id=? ORDER BY r.id")?
            .query_map([message.id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<Timestamp>>(1)?.map(|time| time.jiff()))))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let has_pull_requests = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM github_pull_request_references WHERE message_id=?)",
            [message.id],
            |row| row.get(0),
        )?;
        let pr_threads_stamp = self
            .conn
            .query_row(
                "SELECT MAX(updated_at) FROM github_pull_request_threads WHERE room_id=?",
                [message.room_id],
                |row| row.get::<_, Option<Timestamp>>(0),
            )?
            .map(|time| time.jiff());
        let pins = stamp("SELECT MAX(updated_at) FROM message_pins WHERE message_id=?")?;
        let count = self
            .conn
            .query_row(
                "SELECT messages_count FROM channel_threads WHERE parent_message_id=?",
                [message.id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let poll = stamp("SELECT MAX(updated_at) FROM polls WHERE message_id=?")?;
        let steps = stamp("SELECT MAX(updated_at) FROM agent_steps WHERE message_id=?")?;
        let quotes = self.conn.prepare("SELECT m.updated_at, m.edited_at, u.name, r.name FROM message_references ref JOIN messages m ON m.id=ref.referenced_message_id JOIN users u ON u.id=m.creator_id JOIN rooms r ON r.id=m.room_id WHERE ref.message_id=? ORDER BY ref.id")?
            .query_map([message.id], |row| {
                let updated: Timestamp = row.get(0)?;
                let edited: Option<Timestamp> = row.get(1)?;
                Ok((edited.map_or(updated, |edited| updated.max(edited)).jiff(), row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?))
            })?.collect::<std::result::Result<Vec<_>, _>>()?;
        // MessageKey's existing string adapter covers named-room quotes. Keep nil room names
        // distinct too: Ruby's digest uses nil rather than an empty string for direct rooms.
        let data = MessageKey {
            record: cache_key_with_version("messages", message.id, message.updated_at.jiff()),
            cards: cards.into_iter().collect(),
            embeds,
            has_pull_requests,
            pr_threads_stamp,
            pins: pins.into_iter().collect(),
            thread_messages_count: count,
            poll,
            system_note: message.system_note,
            streaming: message.streaming,
            agent_steps: steps.into_iter().collect(),
            quotes: quotes
                .iter()
                .map(|(time, user, room)| (*time, user.clone(), room.clone().unwrap_or_default()))
                .collect(),
        };
        let mut key = keys::message_with_pr_cards(&data);
        if !quotes.is_empty() {
            let names = quotes
                .into_iter()
                .map(|(_, user, room)| (user, room))
                .collect::<Vec<_>>();
            let keys::Key::Array(parts) = &mut key else {
                unreachable!()
            };
            let slot = parts.len() - 2;
            parts[slot] = keys::Key::Text(
                keys::quote_names_digest_nullable(&names)
                    .map_err(|error| campfire_db::Error::Other(error.into()))?,
            );
        }
        // Rails expands TimeWithZone components in the viewer's zone. Plain record
        // versions remain UTC; equivalent zone aliases therefore share the same key.
        Ok(keys::expand(&key, &self.render_zone))
    }
}

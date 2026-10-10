//! The aggregate facts in Github::PullRequestsHelper#message_with_pr_cards_cache_key.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, Timestamp};
use campfire_views::fragment_cache::keys::{self, MessageKey};
use rusqlite::OptionalExtension;
pub trait MessageCache {
    fn message_fragment_cache_key(&self, message: &Message, base: &str) -> Result<String>;
    fn message_collection_cache_key(&self, message: &Message) -> Result<String>;
}

impl MessageCache for Presenter<'_> {
    fn message_fragment_cache_key(&self, message: &Message, base: &str) -> Result<String> {
        // Rails' initial room list is uncached. Keep both original Rails key
        // compositions, then expand the rendered records' individual Rails versions.
        // Include the preview's source so its author's updates are covered too;
        // reply edits and creator updates do not touch the reply itself.
        let collection = self.message_collection_cache_key(message)?;
        if let Some(data) = &self.search_preloads {
            let validator =
                data.cache.validators.get(&message.id).ok_or_else(|| {
                    campfire_db::Error::Other("missing preloaded validator".into())
                })?;
            let rendered = self.message_rendered_cache_key(message)?;
            return Ok(campfire_views::messages::collection_fragment_key(
                &format!("{collection}/{validator}/{rendered}"),
                base,
            ));
        }
        let mut dependencies = vec![message.clone()];
        if let Some(source) = message
            .reply_to_message_id
            .map(|id| Message::find_by_id(self.conn, id))
            .transpose()?
            .flatten()
        {
            dependencies.push(source);
        }
        let validator = crate::controllers::presenters::message_freshness::etag(self.conn, &dependencies)?;
        let rendered = self.message_rendered_cache_key(message)?;
        Ok(campfire_views::messages::collection_fragment_key(
            &format!("{collection}/{validator}/{rendered}"),
            base,
        ))
    }

    fn message_collection_cache_key(&self, message: &Message) -> Result<String> {
        let blob = if let Some(data) = &self.search_preloads {
            data.attachments.get(&message.id).cloned()
        } else {
            campfire_storage::Blob::attached(self.conn, "Message", message.id, "attachment").map_err(super::storage_error)?
        };
        if let Some(blob) = blob { self.recover_attachment_preview(message, &blob)?; }
        if let Some(data) = &self.search_preloads {
            let records = &data.records;
            let cache = records.cache.get(&message.id);
            let quotes = records
                .quotes
                .get(&message.id)
                .into_iter()
                .flatten()
                .filter_map(|(_, id)| {
                    let source = records.sources.get(id)?;
                    let user = data.users.get(&source.creator_id)?;
                    let room = records.rooms.get(&source.room_id)?;
                    Some((
                        source
                            .edited_at
                            .map_or(source.updated_at, |t| t.max(source.updated_at))
                            .jiff(),
                        user.user.name.clone(),
                        room.name.clone(),
                    ))
                })
                .collect::<Vec<_>>();
            let key = MessageKey {
                record: cache_key_with_version("messages", message.id, message.updated_at.jiff()),
                cards: cache
                    .into_iter()
                    .flat_map(|c| c.cards.iter().map(|t| t.jiff()))
                    .collect(),
                embeds: cache
                    .into_iter()
                    .flat_map(|c| c.embeds.iter().map(|(id, t)| (*id, t.map(|t| t.jiff()))))
                    .collect(),
                has_pull_requests: cache.is_some_and(|c| c.has_pull_requests),
                pr_threads_stamp: records
                    .pr_thread_stamps
                    .get(&message.room_id)
                    .map(|t| t.jiff()),
                pins: cache
                    .into_iter()
                    .flat_map(|c| c.pins.iter().map(|t| t.jiff()))
                    .collect(),
                thread_messages_count: records.reply_counts.get(&message.id).map(|c| *c as i64),
                poll: records.polls.get(&message.id).map(|p| p.updated_at.jiff()),
                system_note: message.system_note,
                streaming: message.streaming,
                agent_steps: records
                    .steps
                    .get(&message.id)
                    .into_iter()
                    .flatten()
                    .map(|s| s.updated_at.jiff())
                    .collect(),
                quotes: quotes
                    .iter()
                    .map(|(t, u, r)| (*t, u.clone(), r.clone().unwrap_or_default()))
                    .collect(),
            };
            return collection_key(key, quotes, &self.render_zone);
        }
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
        collection_key(data, quotes, &self.render_zone)
    }
}

fn collection_key(
    data: MessageKey,
    quotes: Vec<(jiff::Timestamp, String, Option<String>)>,
    zone: &campfire_views::time::Zone,
) -> Result<String> {
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
    Ok(keys::expand(&key, zone))
}

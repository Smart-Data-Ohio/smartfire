//! Record dependencies of Rails' uncached rooms/show message tree. Keep the collection
//! helper separate: its aggregates alone cannot safely cache an initial room render.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, Timestamp, models::message_rendering::RenderingRecords};
use campfire_views::fragment_cache::keys::{self, Key};
use std::collections::BTreeSet;

impl Presenter<'_> {
    pub fn message_rendered_cache_key(&self, message: &Message) -> Result<String> {
        let records = RenderingRecords::load(self.conn, std::slice::from_ref(message))?;
        let mentions = records
            .bodies
            .values()
            .flatten()
            .flat_map(|body| crate::controllers::searches::preloads::mention_ids(body, 0))
            .collect::<Vec<_>>();
        let users = records.users(self.conn, std::slice::from_ref(message), &mentions)?;
        let mut versions = BTreeSet::new();
        let mut user_ids = BTreeSet::new();
        for user in users
            .values()
            .map(|u| &u.user)
            .chain(records.direct_members.values().flatten())
        {
            user_ids.insert(user.id);
            versions.insert(cache_key_with_version(
                "users",
                user.id,
                user.updated_at.jiff(),
            ));
        }
        for (vote, _) in records.votes.values().flatten() {
            if let Some(user) = campfire_db::User::find_by_id(self.conn, vote.user_id)? {
                user_ids.insert(user.id);
                versions.insert(cache_key_with_version(
                    "users",
                    user.id,
                    user.updated_at.jiff(),
                ));
            }
        }
        for room in records.rooms.values() {
            let stem = match room.room_type {
                campfire_db::RoomType::Open => "rooms/opens",
                campfire_db::RoomType::Closed => "rooms/closeds",
                campfire_db::RoomType::Direct => "rooms/directs",
                campfire_db::RoomType::Voice => "rooms/voices",
                campfire_db::RoomType::Stage => "rooms/stages",
                campfire_db::RoomType::Board => "rooms/boards",
            };
            versions.insert(cache_key_with_version(
                stem,
                room.id,
                room.updated_at.jiff(),
            ));
        }
        // Record versions are independent, not timestamp maxima. Attachments/blobs and
        // Drive rows have no updated_at: Rails' key is just their model name and id;
        // their touch chain supplies the changing owner version (see the review ledger).
        let ids = records.body_ids(std::slice::from_ref(message));
        let id_list = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        let user_list = user_ids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let attachments = format!(
            "(record_type='Message' AND name='attachment' AND record_id IN ({id_list})) OR (record_type='User' AND name='avatar' AND record_id IN ({user_list})) OR (record_type='ActionText::RichText' AND name='embeds' AND record_id IN (SELECT id FROM action_text_rich_texts WHERE record_type='Message' AND record_id IN ({id_list}) AND name='body'))"
        );
        let queries = [
            ("active_storage/attachments", format!("SELECT id,NULL FROM active_storage_attachments WHERE {attachments}")),
            ("active_storage/blobs", format!("SELECT id,NULL FROM active_storage_blobs WHERE id IN (SELECT blob_id FROM active_storage_attachments WHERE {attachments})")),
            ("messages", format!("SELECT id,updated_at FROM messages WHERE id IN ({id_list})")),
            ("action_text/rich_texts", format!("SELECT id,updated_at FROM action_text_rich_texts WHERE record_type='Message' AND record_id IN ({id_list}) AND name='body'")),
            ("boosts", "SELECT id,updated_at FROM boosts WHERE message_id=?1".into()),
            ("message_pins", "SELECT id,updated_at FROM message_pins WHERE message_id=?1".into()),
            ("agent_steps", "SELECT id,updated_at FROM agent_steps WHERE message_id=?1".into()),
            ("drive_attachments", "SELECT id,NULL FROM drive_attachments WHERE message_id=?1".into()),
            ("polls", "SELECT id,updated_at FROM polls WHERE message_id=?1".into()),
            ("poll_options", "SELECT id,updated_at FROM poll_options WHERE poll_id IN (SELECT id FROM polls WHERE message_id=?1)".into()),
            ("poll_votes", "SELECT id,updated_at FROM poll_votes WHERE poll_id IN (SELECT id FROM polls WHERE message_id=?1)".into()),
            ("channel_threads", "SELECT id,updated_at FROM channel_threads WHERE parent_message_id=?1 OR id IN (SELECT thread_id FROM messages WHERE id=?1)".into()),
            ("message_references", "SELECT id,updated_at FROM message_references WHERE message_id=?1".into()),
            ("workspace_icons", "SELECT id,updated_at FROM workspace_icons".into()),
            ("github/pull_request_threads", "SELECT id,updated_at FROM github_pull_request_threads WHERE room_id IN (SELECT room_id FROM messages WHERE id=?1) AND EXISTS(SELECT 1 FROM github_pull_request_references WHERE message_id=?1)".into()),
        ];
        for (stem, query) in queries {
            let mut statement = self.conn.prepare(&query)?;
            let parameters = (statement.parameter_count() > 0).then_some(message.id);
            let rows = statement.query_map(rusqlite::params_from_iter(parameters), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<Timestamp>>(1)?))
            })?;
            for row in rows {
                let (id, stamp) = row?;
                versions.insert(stamp.map_or_else(
                    || format!("{stem}/{id}"),
                    |t| cache_key_with_version(stem, id, t.jiff()),
                ));
            }
        }
        // These rows don't touch messages. Keep every reference/card version, including
        // reference URL/position changes, alongside Rails' original helper aggregates.
        for (reference, card, foreign_key, namespace) in [
            (
                "github_pull_request_references",
                "github_pull_requests",
                "github_pull_request_id",
                "github/",
            ),
            (
                "fizzy_card_references",
                "fizzy_cards",
                "fizzy_card_id",
                "fizzy/",
            ),
            (
                "twitter_post_references",
                "twitter_posts",
                "twitter_post_id",
                "twitter/",
            ),
            ("event_references", "events", "event_id", ""),
            ("link_embed_references", "link_embeds", "link_embed_id", ""),
        ] {
            for (table, query) in [
                (
                    reference,
                    format!("SELECT id,updated_at FROM {reference} WHERE message_id=?"),
                ),
                (
                    card,
                    format!(
                        "SELECT c.id,c.updated_at FROM {reference} r JOIN {card} c ON c.id=r.{foreign_key} WHERE r.message_id=?"
                    ),
                ),
            ] {
                let stem = if namespace.is_empty() {
                    table.to_owned()
                } else {
                    table.replacen('_', "/", 1)
                };
                for row in self.conn.prepare(&query)?.query_map([message.id], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, Timestamp>(1)?))
                })? {
                    let (id, stamp) = row?;
                    versions.insert(cache_key_with_version(&stem, id, stamp.jiff()));
                }
            }
        }
        let mut key = versions.into_iter().map(Key::Text).collect::<Vec<_>>();
        if let Some(poll) = records.polls.get(&message.id) {
            key.push(Key::Bool(
                poll.closed_at.is_some() || poll.closes_at.is_some_and(|t| t.jiff() <= self.now),
            ));
        }
        Ok(keys::expand(&Key::Array(key), &self.render_zone))
    }
}

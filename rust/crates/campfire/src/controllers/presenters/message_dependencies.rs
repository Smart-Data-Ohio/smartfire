//! Record dependencies of Rails' uncached rooms/show message tree. Keep the collection
//! helper separate: its aggregates alone cannot safely cache an initial room render.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, Timestamp, models::message_rendering::RenderingRecords};
use campfire_richtext::markdown::{self, Icon, IconCatalog, IconResolver};
use campfire_views::fragment_cache::keys::{self, Key};
use std::{cell::RefCell, collections::BTreeSet};

/// Observe the same icon lookups as Markdown presentation, including missing names
/// which may resolve after an upload. Brands keep precedence over workspace icons.
struct RenderedIcons<'a> {
    catalog: &'a IconCatalog,
    custom_names: RefCell<BTreeSet<String>>,
}
impl IconResolver for RenderedIcons<'_> {
    fn find(&self, name: &str) -> Option<Icon> {
        let icon = self.catalog.find(name);
        if let Some(Icon::Custom { name, .. }) = &icon {
            self.custom_names.borrow_mut().insert(name.clone());
        }
        icon
    }
}

impl Presenter<'_> {
    pub(crate) fn message_room_icon_cache_key(&self, message: &Message) -> Result<String> {
        use campfire_views::helpers::{AvatarIcon, IconSource};
        let name: Option<String> = self.conn.query_row(
            "SELECT icon_name FROM rooms WHERE id=?",
            [message.room_id],
            |r| r.get(0),
        )?;
        let value = match name
            .as_deref()
            .and_then(|name| self.resolve_avatar_icon(name))
        {
            Some(AvatarIcon::Emoji { title, character }) => vec![
                Key::Text("emoji".into()),
                Key::Text(title),
                Key::Text(character),
            ],
            Some(AvatarIcon::Image { title, url, brand }) => vec![
                Key::Text("image".into()),
                Key::Text(title),
                Key::Text(url),
                Key::Bool(brand),
            ],
            None => vec![Key::Null],
        };
        Ok(keys::expand(&Key::Array(value), &self.render_zone))
    }

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
        for user in users.values().map(|u| &u.user) {
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
        let room_ids = std::iter::once(message.room_id)
            .chain(
                records
                    .quotes
                    .values()
                    .flatten()
                    .filter_map(|(_, source)| records.sources.get(source).map(|m| m.room_id)),
            )
            .collect::<BTreeSet<_>>();
        // Message belongs_to :room, touch: true: an unrelated post changes its
        // timestamp, but not _meta's label. Quote cards use a neutral direct label.
        for room in records.rooms.values().filter(|r| room_ids.contains(&r.id)) {
            let stem = match room.room_type {
                campfire_db::RoomType::Open => "rooms/opens",
                campfire_db::RoomType::Closed => "rooms/closeds",
                campfire_db::RoomType::Direct => "rooms/directs",
                campfire_db::RoomType::Voice => "rooms/voices",
                campfire_db::RoomType::Stage => "rooms/stages",
                campfire_db::RoomType::Board => "rooms/boards",
            };
            let label = if room.id == message.room_id {
                if room.direct() {
                    room.direct_display_name(
                        self.conn,
                        None,
                        Some(
                            records
                                .direct_members
                                .get(&room.id)
                                .map(Vec::as_slice)
                                .unwrap_or_default(),
                        ),
                    )?
                    .unwrap_or_default()
                } else {
                    room.name.clone().unwrap_or_default()
                }
            } else if room.direct() {
                "a direct message".into()
            } else {
                room.name.clone().unwrap_or_default()
            };
            versions.insert(keys::expand(
                &Key::Array(vec![
                    Key::Text(format!("{stem}/{}", room.id)),
                    Key::Text(label),
                ]),
                &self.render_zone,
            ));
        }
        let catalog = crate::rich_text::icons(self.conn).map_err(campfire_db::Error::Other)?;
        let icons = RenderedIcons {
            catalog: &catalog,
            custom_names: RefCell::default(),
        };
        for source in std::iter::once(message).chain(records.sources.values()) {
            if (source.markdown() || source.forwarded_markdown)
                && let Some(body) = records.bodies.get(&source.id).and_then(Option::as_deref)
            {
                // app/models/message/markdown.rb:75,97: resolve only image alt names
                // consulted by the existing sanitizer, never the global icon stamp.
                let _ = markdown::sanitize_presentation(body, &icons, None);
            }
        }
        for boost in records.boosts.values().flatten() {
            if let Some(name) = campfire_views::messages::reactions::shortcode_name(
                campfire_richtext::ruby::strip(&boost.content),
            ) {
                icons.find(name);
            }
        }
        let avatar_users = std::iter::once(message.creator_id)
            .chain(records.boosts.values().flatten().map(|b| b.booster_id));
        for id in avatar_users {
            if let Some(user) = users.get(&id)
                && user.user.is_bot()
                && !user.uploaded_avatar
                && let Some(name) = &user.icon_name
            {
                icons.find(name);
            }
        }
        for name in icons.custom_names.into_inner() {
            let (id, stamp): (i64, Timestamp) = self.conn.query_row(
                "SELECT id,updated_at FROM workspace_icons WHERE name=?",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            versions.insert(cache_key_with_version("workspace_icons", id, stamp.jiff()));
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
            ("github/pull_request_threads", "SELECT id,updated_at FROM github_pull_request_threads WHERE room_id IN (SELECT room_id FROM messages WHERE id=?1) AND github_pull_request_id IN (SELECT github_pull_request_id FROM github_pull_request_references WHERE message_id=?1)".into()),
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

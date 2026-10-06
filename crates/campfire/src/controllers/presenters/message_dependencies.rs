//! Record dependencies of Rails' uncached rooms/show message tree. Keep the collection
//! helper separate: its aggregates alone cannot safely cache an initial room render.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, models::message_rendering::RenderingRecords};
use campfire_richtext::markdown::{self, Icon, IconCatalog, IconResolver};
use campfire_views::fragment_cache::keys::{self, Key};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};

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
        let name: Option<String> = if let Some(data) = &self.search_preloads {
            data.records
                .room_icons
                .get(&message.room_id)
                .cloned()
                .flatten()
        } else {
            self.conn.query_row(
                "SELECT icon_name FROM rooms WHERE id=?",
                [message.room_id],
                |r| r.get(0),
            )?
        };
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
        if let Some(data) = &self.search_preloads {
            let records = data.records.scoped(message);
            return self.rendered_cache_key(
                message,
                &records,
                &data.users,
                &data.icons,
                &data.cache,
            );
        }
        let records = RenderingRecords::load(self.conn, std::slice::from_ref(message))?;
        let mentions = records
            .bodies
            .values()
            .flatten()
            .flat_map(|body| crate::controllers::presenters::search_preloads::mention_ids(body, 0))
            .collect::<Vec<_>>();
        let users = records.users(self.conn, std::slice::from_ref(message), &mentions)?;
        let catalog = crate::rich_text::icons(self.conn).map_err(campfire_db::Error::Other)?;
        let facts = super::message_cache_preloads::CacheFacts::load_rendered(
            self,
            std::slice::from_ref(message),
            &records,
        )?;
        self.rendered_cache_key(message, &records, &users, &catalog, &facts)
    }

    fn rendered_cache_key(
        &self,
        message: &Message,
        records: &RenderingRecords,
        page_users: &HashMap<i64, campfire_db::models::message_rendering::RenderingUser>,
        catalog: &IconCatalog,
        facts: &super::message_cache_preloads::CacheFacts,
    ) -> Result<String> {
        let user_ids =
            std::iter::once(message)
                .chain(records.sources.values())
                .map(|m| m.creator_id)
                .chain(records.boosts.values().flatten().map(|b| b.booster_id))
                .chain(records.votes.values().flatten().map(|(v, _)| v.user_id))
                .chain(
                    records.bodies.values().flatten().flat_map(|body| {
                        crate::controllers::presenters::search_preloads::mention_ids(body, 0)
                    }),
                )
                .collect::<BTreeSet<_>>();
        let users = page_users
            .iter()
            .filter(|(id, _)| user_ids.contains(id))
            .map(|(id, u)| (*id, u))
            .collect::<HashMap<_, _>>();
        let mut versions = facts.versions.get(&message.id).cloned().unwrap_or_default();
        for user in users.values().map(|u| &u.user) {
            versions.insert(cache_key_with_version(
                "users",
                user.id,
                user.updated_at.jiff(),
            ));
            versions.extend(facts.avatars.get(&user.id).into_iter().flatten().cloned());
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
        let icons = RenderedIcons {
            catalog,
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
            if let Some((id, stamp)) = facts.icons.get(&name) {
                versions.insert(cache_key_with_version("workspace_icons", *id, stamp.jiff()));
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

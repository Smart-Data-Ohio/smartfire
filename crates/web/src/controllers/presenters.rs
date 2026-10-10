//! Maps database rows into the view models `campfire_views` renders: what the Rails views read
//! off the records (`message.creator`, `room_display_name`, `message_presentation`, the Jbuilder
//! partials) computed up front.

pub mod accounts;
pub use campfire_runtime::presenters::workspace_branding;
pub mod github;
pub use campfire_runtime::presenters::status_settings;
pub use campfire_runtime::presenters::activity;
pub use campfire_runtime::presenters::agents;
pub use campfire_runtime::presenters::agent_payload;
pub use campfire_runtime::presenters::attachments;
pub mod events;
pub mod fizzy_cards;
pub mod link_embeds;
mod message_cache;
pub use campfire_runtime::presenters::message_cache_preloads;
pub use campfire_runtime::presenters::message_dependencies;
pub mod page;
pub use campfire_runtime::presenters::pagination;
pub use campfire_runtime::presenters::people;
pub use campfire_runtime::presenters::rich_text;
mod room_list;
pub mod room_native;
pub use campfire_runtime::presenters::room_shell;
pub use campfire_runtime::presenters::rooms_directory;
pub use campfire_runtime::presenters::boards;
pub mod board_posts;
pub use campfire_runtime::presenters::work_threads;
pub use campfire_runtime::presenters::switcher;
#[cfg(any(test, feature = "test-support"))]
pub use campfire_runtime::presenters::render_secrets;
pub mod twitter_cards;
pub mod view_context;
pub use campfire_runtime::presenters::search_preloads;
pub use campfire_runtime::presenters::message_freshness;
pub use campfire_runtime::presenters::message_payload;
pub use campfire_runtime::presenters::bot_input_casts;
pub use campfire_runtime::presenters::call_navigation;
pub mod sidebar_composition;
pub use campfire_runtime::presenters::calls;
pub use campfire_runtime::presenters::message_parts;
pub use campfire_runtime::presenters::params;
pub use campfire_runtime::presenters::pins;

use campfire_db::Message;
use campfire_views::fragment_cache;
use campfire_views::messages::{
    MessageContent, MessageItem, MessageView, UserView,
};

pub use rich_text::DbResolver;

/// `record.cache_key_with_version`: `"messages/1-20240601120000000000"`.
pub use campfire_views::fragment_cache::cache_key_with_version;
pub use campfire_runtime::presenters::*;
pub trait Rendering {
    fn messages(&self, messages: &[Message]) -> Result<Vec<MessageItem>>;
    fn message_item(&self, message: &Message) -> Result<MessageItem>;
    fn search_message_item(&self, message: &Message) -> Result<MessageItem>;
    fn message_item_for(&self, message: &Message, search: bool) -> Result<MessageItem>;
    fn message(&self, message: &Message) -> Result<MessageView>;
    fn renderable_message(&self, message: &Message, room_name: &str) -> Result<MessageView>;
}
impl Rendering for Presenter<'_> {

    /// `render @messages, cached: message_with_pr_cards_cache_key`: collection hits skip
    /// rendering, while individual messages bypass the collection cache.
    fn messages(&self, messages: &[Message]) -> Result<Vec<MessageItem>> {
        if self.search_preloads.is_none() {
            return self.preload_search(messages)?.messages(messages);
        }
        messages
            .iter()
            .map(|message| self.message_item(message))
            .collect()
    }

    /// `render message`, as [`Self::messages`] does it.
    fn message_item(&self, message: &Message) -> Result<MessageItem> {
        self.message_item_for(message, false)
    }

    /// Room context belongs inside the shared fragment, so resolve its icon on a miss.
    fn search_message_item(&self, message: &Message) -> Result<MessageItem> {
        self.message_item_for(message, true)
    }

    fn message_item_for(&self, message: &Message, search: bool) -> Result<MessageItem> {
        // Fetch intent belongs to this request even if a prior render filled the cache
        // and its durable enqueue rolled back. Search uses the same bulk preloads.
        for post in self.twitter_posts(message)? {
            self.request_twitter_fetch(&post);
        }
        if !message.embeds_suppressed {
            for reference in self.link_references(message)? {
                self.request_link_fetch(&reference.embed);
            }
        }
        let view = || -> Result<MessageView> {
            let mut view = self.message(message)?;
            if search {
                use campfire_views::helpers::IconSource;
                let icon = self
                    .search_preloads
                    .as_ref()
                    .and_then(|data| data.records.room_icons.get(&message.room_id))
                    .and_then(Option::as_deref);
                view.details.room_icon = icon.and_then(|name| self.resolve_avatar_icon(name));
            }
            Ok(view)
        };
        // Event cards contain viewer-zone dates and must render in the request context.
        let has_events = if let Some(data) = &self.search_preloads {
            data.event_views.contains_key(&message.id)
        } else {
            self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM event_references WHERE message_id=?)",
                [message.id],
                |row| row.get::<_, bool>(0),
            )?
        };
        let Some(base) = self.cache_base_url.as_deref().filter(|_| !has_events) else { return Ok(MessageItem::View(Box::new(view()?))) };
        let mut key = self.message_fragment_cache_key(message, base)?;
        if search {
            key.push_str("/show-room-icon/");
            key.push_str(&self.message_room_icon_cache_key(message)?);
        }
        let html = fragment_cache::try_fetch_value(|| key, || {
            let view = view()?;
            // Detached rendering needs the same singleton account for every miss
            // in this page. Keep that read lazy so warm hits need no account query.
            let account = {
                let mut account = self.render_account.borrow_mut();
                if account.is_none() { *account = Some(campfire_db::Account::first(self.conn)?); }
                account.as_ref().unwrap().clone()
            };
            page::render_detached_in_zone(self.app, account.as_ref(), base, &self.render_zone, |ctx| {
                use askama::Template;
                campfire_views::messages::MessagePartial { ctx, message: &view }.render()
                    .map(std::sync::Arc::new).map_err(|error| campfire_db::Error::Other(error.to_string()))
            })
        })?;
        Ok(MessageItem::Fragment { client_message_id: message.client_message_id.clone(), room_id: message.room_id, html })
    }

    /// A message as `messages/_message` shows it.
    fn message(&self, message: &Message) -> Result<MessageView> {
        if self.search_preloads.is_none() {
            return self
                .preload_search(std::slice::from_ref(message))?
                .message(message);
        }
        let (_, room_name) = self.room_and_name(message.room_id)?;
        match self.renderable_message(message, &room_name) {
            // `message_tag` rescues whatever its block raises, e.g. `avatar_tag message.creator`
            // for a creator that's gone (nil), and renders `messages/_unrenderable` instead.
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(MessageView {
                id: message.id,
                client_message_id: message.client_message_id.clone(),
                room_id: message.room_id,
                room_name,
                creator: UserView {
                    id: message.creator_id,
                    name: String::new(),
                    title: String::new(),
                    avatar_url: String::new(),
                    icon: None,
                },
                created_at: message.created_at.jiff(),
                updated_at: message.updated_at.jiff(),
                all_emoji: false,
                content: MessageContent::Unrenderable,
                boosts: Vec::new(),
                details: Default::default(),
                components: Default::default(),
            }),
            rendered => rendered,
        }
    }

    fn renderable_message(&self, message: &Message, room_name: &str) -> Result<MessageView> {
        let (github_cards_html, github_cards_stamp) = if let Some(data) = &self.search_preloads {
            if let Some(github) = data.github.get(&message.id) {
                self.github_refreshes
                    .borrow_mut()
                    .extend(github.refreshes.iter().copied());
                (Some(github::render_message_cards(self.app, message, &self.render_zone, github.account.as_ref(), &github.cards)), github.stamp.clone())
            } else {
                (None, String::new())
            }
        } else {
            let html = github::message_cards_in_zone(self.conn, self.app, message, &self.render_zone)?;
            self.github_refreshes.borrow_mut().extend(
                crate::integrations::github::pull_requests::PullRequest::for_message(
                    self.conn, message.id,
                )?
                .into_iter()
                .filter(|pr| pr.stale(campfire_db::Timestamp::from_jiff(self.now)))
                .map(|pr| pr.id),
            );
            (Some(html), github::cache_stamp(self.conn, message)?)
        };
        let plain_text = self.plain_text_body(message)?;
        Ok(MessageView {
            id: message.id,
            client_message_id: message.client_message_id.clone(),
            room_id: message.room_id,
            room_name: room_name.to_string(),
            creator: self.user_view(message.creator_id)?,
            created_at: message.created_at.jiff(),
            updated_at: message.updated_at.jiff(),
            all_emoji: all_emoji_with_icons(&plain_text, |name| {
                use campfire_views::helpers::{AvatarIcon, IconSource};
                crate::rich_text::builtin_icon(name)
                    || matches!(self.resolve_avatar_icon(name), Some(AvatarIcon::Image { .. }))
            }),
            content: self.content(message, &plain_text)?,
            boosts: self.boosts(message)?,
            details: self.message_details(message)?,
            components: {
                let mut components = link_embeds::components(self, message)?;
                components.event_views = if let Some(data) = &self.search_preloads {
                    data.event_views
                        .get(&message.id)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    events::for_message(self.conn, message)?
                };
                components.quote_references = self.quote_components(message)?.quote_references;
                components.github_cards_html = github_cards_html;
                components.github_cards_stamp = github_cards_stamp;
                components
            },
        })
    }
}

pub use message_cache::MessageCache;
pub use room_list::RoomList;

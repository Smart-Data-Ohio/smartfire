//! Maps database rows into the view models `campfire_views` renders: what the Rails views read
//! off the records (`message.creator`, `room_display_name`, `message_presentation`, the Jbuilder
//! partials) computed up front.

pub mod accounts;
pub mod agent_profile;
pub mod people;
pub mod rooms_directory;
pub mod room_shell;
pub mod switcher;
pub mod github;
pub mod status_settings;
pub mod attachments;
pub mod link_embeds;
pub mod fizzy_cards;
pub mod twitter_cards;
pub mod page;
pub mod pagination;
pub mod rich_text;
mod layout_preferences;
#[cfg(test)]
pub mod test_support;
pub mod view_context;

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use campfire_db::{Boost, Connection, Membership, Message, RichText, Room, RoomType, User};
use campfire_richtext::Presentation;
use campfire_storage::{Storage, Variation};
use campfire_views::fragment_cache;
use campfire_views::messages::json::{BoostJson, BoostMessageJson, IdJson, MessageBodyJson, MessageJson, UserJson};
use campfire_views::messages::support::RubyNumber;
use campfire_views::messages::support::json_time;
use campfire_views::messages::{
    AttachmentPreview, AttachmentView, BoostView, MessageContent, MessageItem, MessageView, RoomKind, SoundImage, SoundView, UserView,
};
use campfire_views::rooms::RoomView;
use rails_compat::Secrets;
use regex::Regex;
use rusqlite::OptionalExtension;

use crate::app::AppState;

pub use rich_text::DbResolver;

/// `Message::THUMBNAIL_MAX_WIDTH` / `THUMBNAIL_MAX_HEIGHT`.
const THUMBNAIL_MAX_WIDTH: i64 = 1200;
const THUMBNAIL_MAX_HEIGHT: i64 = 800;

pub type Result<T> = std::result::Result<T, campfire_db::Error>;

/// `String#all_emoji?` (reference/lib/rails_ext/string.rb).
pub fn all_emoji(text: &str) -> bool {
    static ALL_EMOJI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A(\p{Emoji_Presentation}|\p{Extended_Pictographic}|\x{FE0F})+\z").unwrap());
    ALL_EMOJI.is_match(text)
}

/// `Time#to_fs(:number)`: `%Y%m%d%H%M%S` in UTC (the app's time zone).
pub fn to_fs_number(time: jiff::Timestamp) -> String {
    time.strftime("%Y%m%d%H%M%S").to_string()
}

/// `record.cache_key_with_version`: `"messages/1-20240601120000000000"`.
pub use campfire_views::fragment_cache::cache_key_with_version;

/// `user.avatar_token`: `signed_id(purpose: :avatar)`.
pub fn avatar_token(secrets: &Secrets, user_id: i64) -> String {
    rails_compat::signed_id::generate(secrets, "User", user_id, Some("avatar"), None)
}

/// `fresh_user_avatar_path(user)`.
pub fn avatar_path(secrets: &Secrets, user: &User) -> String {
    campfire_routes::fresh_user_avatar(avatar_token(secrets, user.id), to_fs_number(user.updated_at.jiff()))
}

pub fn room_kind(room_type: RoomType) -> RoomKind {
    match room_type {
        RoomType::Open => RoomKind::Open,
        RoomType::Closed => RoomKind::Closed,
        RoomType::Direct => RoomKind::Direct,
        // The views' RoomKind has no voice, stage or board rooms yet (their screens aren't
        // ported); they're explicit-membership rooms like closed ones.
        RoomType::Voice | RoomType::Stage | RoomType::Board => RoomKind::Closed,
    }
}

pub fn user_view(secrets: &Secrets, user: &User) -> UserView {
    UserView { id: user.id, name: user.name.clone(), title: user.title(), avatar_url: avatar_path(secrets, user), icon: None }
}

impl campfire_views::helpers::IconSource for Presenter<'_> {
    fn resolve_avatar_icon(&self, name: &str) -> Option<campfire_views::helpers::AvatarIcon> {
        use campfire_views::{helpers::AvatarIcon, messages::reactions::static_icon};
        let icon = static_icon(name);
        if matches!(icon, Some(AvatarIcon::Image { brand: true, .. })) { return icon; }
        let custom: Option<String> = self.conn.query_row("SELECT title FROM workspace_icons WHERE name = ?1", [name], |row| row.get(0)).optional().ok().flatten();
        custom.map(|title| AvatarIcon::Image { title, url: format!("/icons/{name}"), brand: false }).or(icon)
    }
}

/// `users/_user.json.jbuilder` (`json.cache! user`).
fn cached_user_json(secrets: &Secrets, base_url: &str, user: &User) -> UserJson {
    let key = || jbuilder_key("users/_user", &cache_key_with_version("users", user.id, user.updated_at.jiff()), base_url);
    fragment_cache::try_fetch_value(key, || Ok::<_, std::convert::Infallible>(user_json(secrets, base_url, user)))
        .unwrap_or_else(|never| match never {})
}

/// Jbuilder's `json.cache!` key: `jbuilder/views/<template>:<digest>/<record key>`. The digest
/// is the Rust build's (templates can't change while the process runs). The JSON carries absolute
/// URLs built from the request's `base_url`, which comes from its Host header, so the key does too:
/// Rails' key doesn't, and one request with a forged Host fed its URLs to every bot.
fn jbuilder_key(template: &str, record: &str, base_url: &str) -> String {
    format!("jbuilder/views/{template}:{}/{record}/{base_url}", env!("CARGO_PKG_VERSION"))
}

/// `users/_user.json.jbuilder`.
pub fn user_json(secrets: &Secrets, base_url: &str, user: &User) -> UserJson {
    UserJson {
        id: user.id,
        name: user.name.clone(),
        role: user.role.name().to_string(),
        avatar_url: format!("{base_url}{}", avatar_path(secrets, user)),
    }
}

/// Everything a page of messages needs, with the rows it looks up along the way remembered
/// (Rails preloads them with `with_creator`, `with_boosts` and friends).
pub struct Presenter<'a> {
    app: &'a AppState,
    pub conn: &'a Connection,
    pub secrets: &'a Secrets,
    pub storage: &'a Storage,
    pub rich_text: &'a dyn RichText,
    pub now: jiff::Timestamp,
    /// `Current.request_host`, which opengraph embeds are checked against.
    pub request_host: Option<String>,
    pub cache_base_url: Option<String>,
    github_refreshes: RefCell<BTreeSet<i64>>,
    users: RefCell<HashMap<i64, User>>,
    room_names: RefCell<HashMap<i64, (Room, String)>>,
    link_fetches: RefCell<std::collections::BTreeSet<i64>>,
    twitter_fetches: RefCell<std::collections::BTreeSet<i64>>,
    twitter_posts: RefCell<HashMap<i64, Vec<crate::integrations::twitter::post::Post>>>,
    twitter_existence: RefCell<HashMap<String, bool>>,
}

impl<'a> Presenter<'a> {
    pub fn new(conn: &'a Connection, app: &'a AppState, request_host: Option<String>) -> Self {
        Self {
            app,
            conn,
            secrets: &app.secrets,
            storage: &app.storage,
            rich_text: &*app.db.env().rich_text,
            now: app.clock.now(),
            request_host,
            cache_base_url: None,
            github_refreshes: RefCell::default(),
            users: RefCell::default(),
            room_names: RefCell::default(),
            link_fetches: RefCell::default(),
            twitter_fetches: RefCell::default(),
            twitter_posts: RefCell::default(),
            twitter_existence: RefCell::default(),
        }
    }

    /// Collected only when a card partial actually renders (never on a fragment-cache hit).
    /// Callers enqueue on the writer after releasing this read-only connection.
    pub fn take_github_refreshes(&self) -> Vec<i64> {
        self.github_refreshes.take().into_iter().collect()
    }

    pub fn resolver(&self) -> DbResolver<'_> {
        DbResolver::with_twitter_cache(self.conn, self.secrets, self.now, &self.twitter_existence)
    }

    /// A read records stale cards; its caller claims/enqueues them on the writer after rendering.
    pub fn request_link_fetch(&self, embed: &crate::integrations::link_embed::Embed) {
        if embed.needs_fetch(campfire_db::Timestamp::from_jiff(self.now)) {
            self.link_fetches.borrow_mut().insert(embed.id);
        }
    }

    pub fn pending_link_fetches(&self) -> Vec<i64> {
        self.link_fetches.borrow().iter().copied().collect()
    }

    pub fn pending_twitter_fetches(&self) -> Vec<i64> { self.twitter_fetches.borrow().iter().copied().collect() }
    pub fn twitter_posts(&self, message: &Message) -> Result<Vec<crate::integrations::twitter::post::Post>> {
        if let Some(posts) = self.twitter_posts.borrow().get(&message.id) { return Ok(posts.clone()); }
        let mut posts = crate::integrations::twitter::post::Post::for_message(self.conn, message.id)?;
        crate::integrations::twitter::post::Post::order_cards(&mut posts);
        self.twitter_posts.borrow_mut().insert(message.id, posts.clone());
        Ok(posts)
    }
    pub fn request_twitter_fetch(&self, post: &crate::integrations::twitter::post::Post) {
        if post.fetch_pending() { self.twitter_fetches.borrow_mut().insert(post.id); }
    }

    pub fn user(&self, id: i64) -> Result<User> {
        if let Some(user) = self.users.borrow().get(&id) {
            return Ok(user.clone());
        }
        let user = User::find(self.conn, id)?;
        self.users.borrow_mut().insert(id, user.clone());
        Ok(user)
    }

    pub fn user_view(&self, id: i64) -> Result<UserView> {
        let user = self.user(id)?;
        let mut view = user_view(self.secrets, &user);
        let uploaded: bool = self.conn.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type = 'User' AND record_id = ?1 AND name = 'avatar')", [id], |row| row.get(0))?;
        if user.is_bot() && !uploaded {
            let icon_name: Option<String> = self.conn.query_row("SELECT icon_name FROM users WHERE id = ?1", [id], |row| row.get(0))?;
            view.icon = icon_name.as_deref().and_then(|name| campfire_views::helpers::IconSource::resolve_avatar_icon(self, name));
        }
        Ok(view)
    }

    /// `room_display_name(room, for_user:)`.
    pub fn room_display_name(&self, room: &Room, for_user: Option<&User>) -> Result<String> {
        Ok(if room.direct() {
            room.direct_display_name(self.conn, for_user, None)?.unwrap_or_default()
        } else {
            room.name.clone().unwrap_or_default()
        })
    }

    pub fn room_view(&self, room: &Room, for_user: &User) -> Result<RoomView> {
        let header = rooms_directory::header(self.conn, room, for_user)?;
        Ok(RoomView {
            involvement: campfire_db::Membership::find_by_room_and_user(self.conn,room.id,for_user.id)?.and_then(|m|m.involvement).map(|i|i.name().to_string()).unwrap_or_else(||room.default_involvement().to_string()),
            id: room.id,
            kind: room_kind(room.room_type),
            name: room.name.clone(),
            display_name: header.display_name.clone(),
            header: Some(header),
        })
    }

    /// `message.room` with `room_display_name(message.room, for_user: nil)`.
    fn room_and_name(&self, room_id: i64) -> Result<(Room, String)> {
        if let Some(entry) = self.room_names.borrow().get(&room_id) {
            return Ok(entry.clone());
        }
        let room = Room::find(self.conn, room_id)?;
        let name = self.room_display_name(&room, None)?;
        self.room_names.borrow_mut().insert(room_id, (room.clone(), name.clone()));
        Ok((room, name))
    }

    pub fn plain_text_body(&self, message: &Message) -> Result<String> {
        if message.markdown_source.is_some() || message.forwarded_markdown {
            let body = message.body_html(self.conn)?.unwrap_or_default();
            let resolver = self.resolver();
            return campfire_richtext::markdown::plain_text(&body, &resolver.render_context(self.request_host.clone()), &resolver)
                .map_err(|error| campfire_db::Error::Other(error.to_string()));
        }
        message.plain_text_body(self.conn, self.rich_text)
    }

    /// `render @messages`: each message's cached fragment when the current store has its version
    /// (`cache [ message, "presentation-v3" ]` wraps the whole partial, so Rails evaluates none of
    /// it on a hit), else its view.
    pub fn messages(&self, messages: &[Message]) -> Result<Vec<MessageItem>> {
        let ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
        let mut posts = crate::integrations::twitter::post::Post::for_messages(self.conn, &ids)?;
        for posts in posts.values_mut() { crate::integrations::twitter::post::Post::order_cards(posts); }
        self.twitter_posts.borrow_mut().extend(posts);
        messages.iter().map(|message| self.message_item(message)).collect()
    }

    /// `render message`, as [`Self::messages`] does it.
    pub fn message_item(&self, message: &Message) -> Result<MessageItem> {
        let stamp = github::cache_stamp(self.conn, message)?;
        Ok(match self.cache_base_url.as_deref().and_then(|base| campfire_views::messages::cached_message_fragment_with_cards(message.id, message.updated_at.jiff(), base, &stamp)) {
            Some(html) => MessageItem::Fragment { client_message_id: message.client_message_id.clone(), room_id: message.room_id, html },
            None => MessageItem::View(Box::new(self.message(message)?)),
        })
    }

    /// A message as `messages/_message` shows it.
    pub fn message(&self, message: &Message) -> Result<MessageView> {
        let (_, room_name) = self.room_and_name(message.room_id)?;
        match self.renderable_message(message, &room_name) {
            // `message_tag` rescues whatever its block raises, e.g. `avatar_tag message.creator`
            // for a creator that's gone (nil), and renders `messages/_unrenderable` instead.
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(MessageView {
                id: message.id,
                client_message_id: message.client_message_id.clone(),
                room_id: message.room_id,
                room_name,
                creator: UserView { id: message.creator_id, name: String::new(), title: String::new(), avatar_url: String::new(), icon: None },
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
        let github_cards_html = github::message_cards(self.conn, self.app, message)?;
        self.github_refreshes.borrow_mut().extend(
            crate::integrations::github::pull_requests::PullRequest::for_message(self.conn, message.id)?
                .into_iter().filter(|pr| pr.stale(campfire_db::Timestamp::from_jiff(self.now))).map(|pr| pr.id)
        );
        let plain_text = self.plain_text_body(message)?;
        Ok(MessageView {
            id: message.id,
            client_message_id: message.client_message_id.clone(),
            room_id: message.room_id,
            room_name: room_name.to_string(),
            creator: self.user_view(message.creator_id)?,
            created_at: message.created_at.jiff(),
            updated_at: message.updated_at.jiff(),
            all_emoji: all_emoji(&plain_text),
            content: self.content(message, &plain_text)?,
            boosts: self.boosts(message)?,
            details: self.message_details(message)?,
            components: campfire_views::messages::MessageComponents {
                github_cards_html: Some(github_cards_html),
                github_cards_stamp: github::cache_stamp(self.conn, message)?,
                ..link_embeds::components(self, message)?
            },
        })
    }

    fn message_details(&self, message: &Message) -> Result<campfire_views::messages::MessageDetails> {
        use campfire_views::messages::{MessageDetails, ReplyPreview, ReplySource};
        let pinned = self.conn.query_row("SELECT EXISTS(SELECT 1 FROM message_pins WHERE message_id = ?1)", [message.id], |row| row.get(0))?;
        let reply_count: u64 = self.conn.query_row("SELECT messages_count FROM channel_threads WHERE parent_message_id = ?1", [message.id], |row| row.get(0)).optional()?.unwrap_or(0);
        let drive_urls = self.conn.prepare("SELECT file_id FROM drive_attachments WHERE message_id = ?1 ORDER BY id")?
            .query_map([message.id], |row| Ok(format!("https://drive.google.com/open?id={}", row.get::<_, String>(0)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let reply = if message.reply_to_message_id.is_some() || message.reply_target_deleted_at.is_some() {
            let source = message.reply_to_message_id.map(|id| Message::find_by_id(self.conn, id)).transpose()?.flatten();
            let source = source.map(|source| -> Result<ReplySource> {
                let url = if let Some(thread) = source.thread_id {
                    campfire_routes::ROOM.path_with(&[&source.room_id], None, &[("thread", Some(&thread.to_string())), ("message_id", Some(&source.id.to_string()))])
                } else { campfire_routes::room_at_message(source.room_id, source.id) };
                Ok(ReplySource { id: source.id, author: self.user(source.creator_id)?.name, plain_text: self.plain_text_body(&source)?, url })
            }).transpose()?;
            Some(ReplyPreview { source })
        } else { None };
        let agent_steps = self.conn.prepare("SELECT name, status, duration_ms, input_summary, output_summary FROM agent_steps WHERE message_id = ?1 ORDER BY position, id")?
            .query_map([message.id], |row| Ok(campfire_views::messages::parts::AgentStep {
                name: row.get(0)?, status: row.get(1)?, duration_ms: row.get(2)?, input_summary: row.get(3)?, output_summary: row.get(4)?,
            }))?.collect::<std::result::Result<Vec<_>, _>>()?;
        let poll_row = self.conn.query_row("SELECT id, anonymous, multiple, closed_at, closes_at FROM polls WHERE message_id = ?1", [message.id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?, row.get::<_, bool>(2)?, row.get::<_, Option<campfire_db::Timestamp>>(3)?, row.get::<_, Option<campfire_db::Timestamp>>(4)?))
        }).optional()?;
        let poll = if let Some((id, anonymous, multiple, closed_at, closes_at)) = poll_row {
            use campfire_views::messages::parts::{Poll, PollOption, PollVote};
            let options = self.conn.prepare("SELECT id, label FROM poll_options WHERE poll_id = ?1 ORDER BY position, id")?
                .query_map([id], |row| Ok(PollOption { id: row.get(0)?, label: row.get(1)? }))?.collect::<std::result::Result<Vec<_>, _>>()?;
            let votes = self.conn.prepare("SELECT poll_option_id, user_id, users.name FROM poll_votes LEFT JOIN users ON users.id = poll_votes.user_id WHERE poll_id = ?1 ORDER BY poll_votes.id")?
                .query_map([id], |row| Ok(PollVote { option_id: row.get(0)?, user_id: row.get(1)?, user_name: row.get(2)? }))?.collect::<std::result::Result<Vec<_>, _>>()?;
            Some(Poll { id, room_id: message.room_id, anonymous, multiple,
                closed: closed_at.is_some() || closes_at.is_some_and(|time| time.jiff() <= self.now), closes_at: closes_at.map(|time| time.jiff()),
                options, votes, vote_error: None,
            })
        } else { None };
        Ok(MessageDetails {
            thread_id: message.thread_id, system_note: message.system_note, action: message.action,
            edited_at: message.edited_at.map(|time| time.jiff()), streaming: message.streaming,
            markdown: message.markdown_source.is_some(), forwarded: message.forwarded_at.is_some(),
            forward_note: message.forward_note.clone(), pinned, reply_count, drive_urls, reply, agent_steps, poll,
            ..Default::default()
        })
    }

    /// `message.boosts.ordered`.
    pub fn boosts(&self, message: &Message) -> Result<Vec<BoostView>> {
        Boost::for_message_ordered(self.conn, message.id)?.iter().map(|boost| self.boost(boost)).collect()
    }

    pub fn boost(&self, boost: &Boost) -> Result<BoostView> {
        Ok(BoostView {
            id: boost.id,
            updated_at: boost.updated_at.jiff(),
            message_id: boost.message_id,
            content: boost.content.clone(),
            all_emoji: all_emoji(&boost.content),
            booster: self.user_view(boost.booster_id)?,
            reaction: campfire_views::messages::reactions::resolve(&boost.content, self),
        })
    }

    /// `message.content_type`, with what `message_presentation` shows for it.
    fn content(&self, message: &Message, plain_text: &str) -> Result<MessageContent> {
        let body = message.body_html(self.conn)?.unwrap_or_default();
        let resolver = self.resolver();
        let ctx = resolver.render_context(self.request_host.clone());
        // `message_tag` evaluates `message.plain_text_body` first; where that raises, it rescues
        // and renders `messages/_unrenderable`, unless logging the exception raises again (a
        // message that isn't UTF-8): then the page fails (verified against the reference).
        match campfire_richtext::to_plain_text(&body, &ctx) {
            Err(campfire_richtext::Error::Unrenderable(error)) => {
                return Err(campfire_db::Error::Other(format!("message_tag's rescue raised logging {error}")));
            }
            Err(_) => return Ok(MessageContent::Unrenderable),
            Ok(_) => {}
        }
        if let Some(attachment) = self.attachment(message)? {
            return Ok(MessageContent::Attachment(attachment));
        }
        if let Some(sound) = campfire_db::message::sound_in(plain_text) {
            return Ok(MessageContent::Sound(SoundView {
                url: campfire_assets::asset_path(&sound.asset_path()),
                image: sound.image.map(|image| SoundImage {
                    src: campfire_assets::image_path(&image.asset_path()),
                    width: image.width,
                    height: image.height,
                }),
                text: sound.text.map(str::to_string),
            }));
        }
        if message.markdown_source.is_some() || message.forwarded_markdown {
            return Ok(MessageContent::Text {
                html: campfire_richtext::markdown::presentation(&body, &ctx, &resolver, None).unwrap_or_default(),
            });
        }
        Ok(match campfire_richtext::present_message(&body, &ctx) {
            Presentation::Html(html) => MessageContent::Text { html },
            Presentation::Unrenderable => MessageContent::Unrenderable,
        })
    }

    /// `message.attachment` as `Messages::AttachmentPresentation` needs it.
    fn attachment(&self, message: &Message) -> Result<Option<AttachmentView>> {
        let blob = campfire_storage::Blob::attached(self.conn, "Message", message.id, "attachment").map_err(storage_error)?;
        let Some(blob) = blob else { return Ok(None) };
        let verifier = &*self.storage.verifier;
        let preview = if blob.is_previewable() || blob.is_variable() {
            if blob.is_video() {
                // `attachment.preview(format: :webp, resize_to_limit: [...])`
                let poster = Variation::new(vec![
                    ("format".into(), campfire_storage::marshal::Value::Symbol("webp".into())),
                    (
                        "resize_to_limit".into(),
                        campfire_storage::marshal::Value::Array(vec![
                            campfire_storage::marshal::Value::Int(THUMBNAIL_MAX_WIDTH),
                            campfire_storage::marshal::Value::Int(THUMBNAIL_MAX_HEIGHT),
                        ]),
                    ),
                ]);
                AttachmentPreview::Video {
                    poster_url: campfire_storage::paths::representation_redirect_path(verifier, &blob, &poster),
                }
            } else {
                AttachmentPreview::Image { thumb_url: self.thumb_path(&blob)? }
            }
        } else {
            AttachmentPreview::File
        };
        Ok(Some(AttachmentView {
            filename: blob.filename.to_string(),
            filename_base: blob.filename.base().to_string(),
            blob_path: campfire_storage::paths::blob_redirect_path(verifier, &blob, None),
            download_path: campfire_storage::paths::blob_redirect_path(verifier, &blob, Some("attachment")),
            preview,
            width: dimension(&blob, "width"),
            height: dimension(&blob, "height"),
        }))
    }

    /// `polymorphic_url(attachment.representation(:thumb), only_path: true)`.
    fn thumb_path(&self, blob: &campfire_storage::Blob) -> Result<String> {
        let thumb = Variation::resize_to_limit(THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT, None);
        let variation = if blob.is_previewable() { thumb } else { self.storage.variation_for(blob, &thumb).map_err(storage_error)? };
        Ok(campfire_storage::paths::representation_redirect_path(&*self.storage.verifier, blob, &variation))
    }

    /// `message.body.to_s`: the stored rich text rendered inside its layout.
    pub fn body_html(&self, message: &Message) -> Result<String> {
        let Some(body) = message.body_html(self.conn)? else { return Ok(String::new()) };
        let resolver = self.resolver();
        let ctx = resolver.render_context(self.request_host.clone());
        Ok(campfire_richtext::Content::load(&body, &ctx)
            .and_then(|content| content.to_rendered_html_with_layout(&ctx))
            .unwrap_or_default())
    }

    /// `editable_body(message)` as the editor's `value`.
    pub fn editable_body(&self, message: &Message) -> Result<String> {
        let body = message.body_html(self.conn)?.unwrap_or_default();
        let resolver = self.resolver();
        let ctx = resolver.render_context(self.request_host.clone());
        // An `Err` is where the edit page raises in Rails (a missing attachment, say).
        campfire_richtext::editable_value(&body, &ctx)
            .map(Option::unwrap_or_default)
            .map_err(|error| campfire_db::Error::Other(format!("editable_body raised: {error}")))
    }

    /// `messages/_message.json.jbuilder` (`json.cache! message`).
    pub fn message_json(&self, message: &Message, base_url: &str) -> Result<MessageJson> {
        let key = || jbuilder_key("messages/_message", &cache_key_with_version("messages", message.id, message.updated_at.jiff()), base_url);
        fragment_cache::try_fetch_value(key, || self.render_message_json(message, base_url))
    }

    fn render_message_json(&self, message: &Message, base_url: &str) -> Result<MessageJson> {
        Ok(MessageJson {
            id: message.id,
            created_at: json_time(message.created_at.jiff()),
            body: MessageBodyJson { plain_text: self.plain_text_body(message)?, html: self.body_html(message)? },
            creator: cached_user_json(self.secrets, base_url, &self.user(message.creator_id)?),
            room: IdJson { id: message.room_id },
            url: format!("{base_url}{}", campfire_routes::room_message(message.room_id, message.id)),
        })
    }

    /// `messages/boosts/_boost.json.jbuilder` (`json.cache! boost`).
    pub fn boost_json(&self, boost: &Boost, message: &Message, base_url: &str) -> Result<BoostJson> {
        let key = || jbuilder_key("messages/boosts/_boost", &cache_key_with_version("boosts", boost.id, boost.updated_at.jiff()), base_url);
        fragment_cache::try_fetch_value(key, || self.render_boost_json(boost, message, base_url))
    }

    fn render_boost_json(&self, boost: &Boost, message: &Message, base_url: &str) -> Result<BoostJson> {
        Ok(BoostJson {
            id: boost.id,
            content: boost.content.clone(),
            created_at: json_time(boost.created_at.jiff()),
            booster: cached_user_json(self.secrets, base_url, &self.user(boost.booster_id)?),
            message: BoostMessageJson {
                id: boost.message_id,
                url: format!("{base_url}{}", campfire_routes::room_message(message.room_id, message.id)),
            },
        })
    }

    /// `users/sidebars/rooms/_shared` locals.
    pub fn sidebar_room(&self, room: &Room) -> campfire_views::users::SidebarRoom {
        campfire_views::users::SidebarRoom {
            id: room.id,
            param_key: room_kind(room.room_type).param_key().to_string(),
            name: room.name.clone().unwrap_or_default(),
            unread: false,
            menu:accounts::room_menu(room,None,None,0,None),
            icon:accounts::resolve_room_icon(self.conn,room.icon_name.as_deref()),
            huddle_participants:None,
        }
    }

    /// `users/sidebars/rooms/_direct` locals for `membership`.
    pub fn sidebar_direct(&self, membership: &Membership) -> Result<campfire_views::users::SidebarDirect> {
        let room = Room::find(self.conn,membership.room_id)?;
        accounts::sidebar_direct(self.conn,self.secrets,membership,&room)
    }

}

/// A `User` row as the users views see it.
pub fn user_summary(secrets: &Secrets, user: &User) -> campfire_views::users::UserSummary {
    use campfire_views::users::{Role, Status};
    campfire_views::users::UserSummary {
        id: user.id,
        name: user.name.clone(),
        bio: user.bio.clone(),
        email_address: user.email_address.clone(),
        role: match user.role {
            campfire_db::Role::Member => Role::Member,
            campfire_db::Role::Administrator => Role::Administrator,
            campfire_db::Role::Bot => Role::Bot,
        },
        status: match user.status {
            campfire_db::Status::Active => Status::Active,
            campfire_db::Status::Deactivated => Status::Deactivated,
            campfire_db::Status::Banned => Status::Banned,
        },
        avatar_path: avatar_path(secrets, user),
        two_factor_enabled: false,
        google_identity_email: None,
        email_self_changed: false,
        google_email_link_allowed: false,
    }
}

/// The account list additionally offers administrator recovery for enrolled humans.
pub fn account_user_summary(conn: &Connection, secrets: &Secrets, user: &User) -> campfire_db::Result<campfire_views::users::UserSummary> {
    let (google_identity_email,email_self_changed,google_email_link_allowed) = conn.query_row(
        "SELECT google_identities.email,users.email_self_changed_at IS NOT NULL,users.google_email_link_allowed FROM users LEFT JOIN google_identities ON google_identities.user_id=users.id WHERE users.id=?",[user.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    )?;
    Ok(campfire_views::users::UserSummary { two_factor_enabled: user.two_factor_enabled(conn)?, google_identity_email,email_self_changed,google_email_link_allowed, ..user_summary(secrets,user) })
}

/// `to_fs(:epoch)` as a string (milliseconds).
pub fn epoch_string(time: jiff::Timestamp) -> String {
    campfire_views::messages::support::epoch_ms(time).to_string()
}

/// `attachment.metadata[:width]`: an Integer for images, a Float for videos.
fn dimension(blob: &campfire_storage::Blob, name: &str) -> Option<RubyNumber> {
    match blob.metadata.get(name)? {
        campfire_storage::Json::Int(value) => Some(RubyNumber::Int(*value)),
        campfire_storage::Json::Float(value) => Some(RubyNumber::Float(*value)),
        _ => None,
    }
}

pub fn storage_error(error: campfire_storage::Error) -> campfire_db::Error {
    campfire_db::Error::Other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_emoji_matches_ruby() {
        assert!(all_emoji("👍"));
        assert!(all_emoji("❤️"));
        assert!(!all_emoji("hi 👍"));
        assert!(!all_emoji(""));
    }

    #[test]
    fn cache_versions_use_usec() {
        let time: jiff::Timestamp = "2024-06-01T12:00:00.000123Z".parse().unwrap();
        assert_eq!(cache_key_with_version("messages", 1, time), "messages/1-20240601120000000123");
        assert_eq!(to_fs_number(time), "20240601120000");
    }
}

pub(crate) mod profile_sections;

pub mod fizzy_profile;

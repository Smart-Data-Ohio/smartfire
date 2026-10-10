pub mod accounts;
pub mod workspace_branding;
pub mod status_settings;
pub mod activity;
pub mod agents;
pub mod agent_payload;
pub mod attachments;
pub mod layout_preferences;
pub mod pagination;
pub mod people;
pub mod rich_text;
pub mod switcher;
pub mod message_parts;
pub mod params;
pub mod profile_sections;
pub mod fizzy_profile;
pub mod room_unread;
pub mod calls;
pub mod message_payload;
pub mod bot_input_casts;
pub mod message_freshness;
pub mod github;
pub mod events;
pub mod twitter_cards;
pub mod search_preloads;

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use campfire_db::{Boost, Connection, Message, RichText, Room, RoomType, User};
use campfire_richtext::Presentation;
use campfire_storage::{Storage, Variation};
use campfire_app::cache as fragment_cache;
use campfire_presentation::messages::json::{
    BoostJson, BoostMessageJson, UserJson,
};
use campfire_presentation::messages::support::RubyNumber;
use campfire_presentation::messages::support::json_time;
use campfire_presentation::messages::{
    AttachmentPreview, AttachmentView, BoostView, MessageContent,
    RoomKind, SoundImage, SoundView, UserView,
};
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
#[cfg(any(test, feature = "test-support"))]
pub fn all_emoji(text: &str) -> bool {
    all_emoji_with_icons(text, crate::rich_text::builtin_icon)
}

pub fn all_emoji_with_icons(text: &str, is_icon: impl Fn(&str) -> bool) -> bool {
    static ALL_EMOJI: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\A((\p{Emoji_Presentation}|\p{Extended_Pictographic}|\x{FE0F})|(:[a-z0-9_]+:))+\z").unwrap()
    });
    static SHORTCODES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":([a-z0-9_]+):").unwrap());
    ALL_EMOJI.is_match(text) && SHORTCODES.captures_iter(text).all(|capture| is_icon(&capture[1]))
}

/// `Time#to_fs(:number)`: `%Y%m%d%H%M%S` in UTC (the app's time zone).
#[cfg(any(test, feature = "test-support"))]
pub fn to_fs_number(time: jiff::Timestamp) -> String {
    time.strftime("%Y%m%d%H%M%S").to_string()
}

/// `record.cache_key_with_version`: `"messages/1-20240601120000000000"`.
pub use campfire_presentation::cache_keys::cache_key_with_version;

/// `user.avatar_token`: `signed_id(purpose: :avatar)`.
pub fn avatar_token(secrets: &Secrets, user_id: i64) -> String {
    rails_compat::signed_id::generate(secrets, "User", user_id, Some("avatar"), None)
}

/// `fresh_user_avatar_path(user)`.
pub fn avatar_path(secrets: &Secrets, user: &User) -> String {
    avatar_path_in_zone(secrets, user, &campfire_presentation::time::Zone::utc())
}

pub fn avatar_path_in_zone(secrets: &Secrets, user: &User, zone: &campfire_presentation::time::Zone) -> String {
    campfire_routes::fresh_user_avatar(
        avatar_token(secrets, user.id),
        zone.to_fs(user.updated_at.jiff(), "number"),
    )
}

pub fn room_kind(room_type: RoomType) -> RoomKind {
    match room_type {
        RoomType::Open => RoomKind::Open,
        RoomType::Closed => RoomKind::Closed,
        RoomType::Direct => RoomKind::Direct,
        RoomType::Voice => RoomKind::Voice,
        RoomType::Stage => RoomKind::Stage,
        RoomType::Board => RoomKind::Board,
    }
}

pub fn user_view(secrets: &Secrets, user: &User) -> UserView {
    UserView {
        id: user.id,
        name: user.name.clone(),
        title: user.title(),
        avatar_url: avatar_path(secrets, user),
        icon: None,
    }
}

impl campfire_presentation::helpers::IconSource for Presenter<'_> {
    fn resolve_avatar_icon(&self, name: &str) -> Option<campfire_presentation::helpers::AvatarIcon> {
        use campfire_presentation::{helpers::AvatarIcon, messages::reactions::static_icon};
        let icon = static_icon(name);
        if matches!(icon, Some(AvatarIcon::Image { brand: true, .. })) {
            return icon;
        }
        if let Some(data) = &self.search_preloads {
            return data
                .custom_icons
                .get(name)
                .map(|title| AvatarIcon::Image {
                    title: title.clone(),
                    url: format!("/icons/{name}"),
                    brand: false,
                })
                .or(icon);
        }
        let custom: Option<String> = self
            .conn
            .query_row(
                "SELECT title FROM workspace_icons WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        custom
            .map(|title| AvatarIcon::Image {
                title,
                url: format!("/icons/{name}"),
                brand: false,
            })
            .or(icon)
    }
}

pub fn resolve_avatar_icon(conn: &Connection, name: &str) -> Option<campfire_presentation::helpers::AvatarIcon> {
        use campfire_presentation::{helpers::AvatarIcon, messages::reactions::static_icon};
        let icon = static_icon(name);
        if matches!(icon, Some(AvatarIcon::Image { brand: true, .. })) { return icon; }
        let custom: Option<String> = conn.query_row("SELECT title FROM workspace_icons WHERE name = ?1", [name], |row| row.get(0)).optional().ok().flatten();
        custom.map(|title| AvatarIcon::Image { title, url: format!("/icons/{name}"), brand: false }).or(icon)
}

/// `users/_user.json.jbuilder` (`json.cache! user`).
pub fn cached_user_json(secrets: &Secrets, base_url: &str, user: &User) -> UserJson {
    let key = || {
        jbuilder_key(
            "users/_user",
            &cache_key_with_version("users", user.id, user.updated_at.jiff()),
            base_url,
        )
    };
    fragment_cache::try_fetch_value(key, || {
        Ok::<_, std::convert::Infallible>(user_json(secrets, base_url, user))
    })
    .unwrap_or_else(|never| match never {})
}

/// Jbuilder's `json.cache!` key: `jbuilder/views/<template>:<digest>/<record key>`. The digest
/// is the Rust build's (templates can't change while the process runs). The JSON carries absolute
/// URLs built from the request's `base_url`, which comes from its Host header, so the key does too:
/// Rails' key doesn't, and one request with a forged Host fed its URLs to every bot.
pub fn jbuilder_key(template: &str, record: &str, base_url: &str) -> String {
    format!(
        "jbuilder/views/{template}:{}/{record}/{base_url}",
        env!("CARGO_PKG_VERSION")
    )
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
#[derive(Default)]
pub struct RenderRefreshes {
    github: Vec<i64>,
    attachments: Vec<(i64, i64)>,
}

/// Persist rendering's recovery requests after releasing the reader, just like PR refreshes.
pub async fn refresh_after_render(db: &campfire_db::Database, refreshes: RenderRefreshes) {
    if !refreshes.attachments.is_empty()
        && let Err(error) = db.write(move |tx| {
            for (message, blob) in refreshes.attachments {
                campfire_db::models::message_attachment_processing::recover(tx, message, blob)?;
            }
            Ok(())
        }).await
    {
        tracing::warn!(%error, "Skipping attachment preview recovery");
    }
    crate::integrations::github::pull_requests::refresh_after_render(db, refreshes.github).await;
}

pub struct Presenter<'a> {
    pub app: &'a AppState,
    pub conn: &'a Connection,
    pub secrets: &'a Secrets,
    pub storage: &'a Storage,
    pub rich_text: &'a dyn RichText,
    pub now: jiff::Timestamp,
    /// `Current.request_host`, which opengraph embeds are checked against.
    pub request_host: Option<String>,
    pub cache_base_url: Option<String>,
    pub current_user_id: Option<i64>,
    #[allow(dead_code)] // WS11-api calls agent_message_payload after its branch merges.
    agent_payload: &'a agent_payload::State,
    /// The viewer's zone for token-free request partials; detached jobs default to UTC.
    pub render_zone: campfire_presentation::time::Zone,
    pub github_refreshes: std::rc::Rc<RefCell<BTreeSet<i64>>>,
    pub attachment_recoveries: std::rc::Rc<RefCell<BTreeSet<(i64, i64)>>>,
    users: RefCell<HashMap<i64, User>>,
    pub render_account: RefCell<Option<Option<campfire_db::Account>>>,
    // WS8bm2 shared rendering-details seam for root and search pages.
    pub search_preloads: Option<search_preloads::Preloads>,
    link_fetches: std::rc::Rc<RefCell<std::collections::BTreeSet<i64>>>,
    twitter_fetches: std::rc::Rc<RefCell<std::collections::BTreeSet<i64>>>,
    twitter_posts:
        std::rc::Rc<RefCell<HashMap<i64, Vec<crate::integrations::twitter::post::Post>>>>,
    twitter_existence: std::rc::Rc<RefCell<HashMap<String, bool>>>,
}
impl<'a> Presenter<'a> {
    pub fn app(&self) -> &AppState {
        self.app
    }

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
            current_user_id: None,
            agent_payload: &app.agent_message_payload,
            render_zone: crate::context::renderer_time_zone(),
            github_refreshes: Default::default(),
            attachment_recoveries: Default::default(),
            users: RefCell::default(),
            render_account: RefCell::default(),
            search_preloads: None,
            link_fetches: Default::default(),
            twitter_fetches: Default::default(),
            twitter_posts: Default::default(),
            twitter_existence: Default::default(),
        }
    }

    pub fn use_viewer_zone(&mut self, user_id: i64) -> Result<()> {
        let zone: Option<String> = self.conn.query_row("SELECT time_zone FROM users WHERE id=?", [user_id], |row| row.get(0))?;
        self.render_zone = campfire_presentation::time::Zone::for_user(zone.as_deref());
        Ok(())
    }

    /// PR refreshes require a rendered card; attachment recovery also runs on cache hits.
    /// Callers enqueue on the writer after releasing this read-only connection.
    pub fn take_render_refreshes(&self) -> RenderRefreshes {
        RenderRefreshes {
            github: self.github_refreshes.take().into_iter().collect(),
            attachments: self.attachment_recoveries.take().into_iter().collect(),
        }
    }

    pub fn remember_github_refresh(&self, id: i64) {
        self.github_refreshes.borrow_mut().insert(id);
    }

    pub fn remember_message_refreshes(&self, message: &Message) -> Result<()> {
        let now = self.app.db.env().now();
        for pr in crate::integrations::github::pull_requests::PullRequest::for_message(self.conn, message.id)? {
            if pr.stale(now) { self.remember_github_refresh(pr.id); }
        }
        let _ = self.attachment(message)?;
        Ok(())
    }

    pub fn resolver(&self) -> search_preloads::PageResolver<'_> {
        search_preloads::PageResolver {
            db: DbResolver::with_twitter_cache(
                self.conn,
                self.secrets,
                self.now,
                &self.twitter_existence,
            ),
            preloads: self.search_preloads.as_ref(),
        }
    }
    pub fn preload_payload(&self,messages:&[Message]) -> Result<Self> {
        self.with_preloads(search_preloads::Preloads::load_payload(self,messages)?,messages)
    }
    pub fn preload_plain_text(&self,messages:&[Message]) -> Result<Self> {
        Ok(self.with_preloaded_facts(search_preloads::Preloads::load_plain_text(self,messages)?))
    }
    fn with_preloads(&self,data:search_preloads::Preloads,messages:&[Message]) -> Result<Self> {
        let ids = data.records.body_ids(messages);
        let mut posts = crate::integrations::twitter::post::Post::for_messages(self.conn, &ids)?;
        for id in &ids {
            posts.entry(*id).or_default();
        }
        for rows in posts.values_mut() {
            crate::integrations::twitter::post::Post::order_cards(rows);
        }
        self.twitter_existence.borrow_mut().extend(
            posts
                .values()
                .flatten()
                .map(|post| (post.post_id.clone(), true)),
        );
        self.twitter_posts.borrow_mut().extend(posts);
        Ok(self.with_preloaded_facts(data))
    }
    fn with_preloaded_facts(&self,data:search_preloads::Preloads) -> Self {
        Self {
            app: self.app,
            conn: self.conn,
            secrets: self.secrets,
            storage: self.storage,
            rich_text: self.rich_text,
            now: self.now,
            request_host: self.request_host.clone(),
            cache_base_url: self.cache_base_url.clone(),
            current_user_id: self.current_user_id,
            agent_payload: self.agent_payload,
            render_zone: self.render_zone.clone(),
            users: RefCell::default(),
            render_account: self.render_account.clone(),
            search_preloads: Some(data),
            link_fetches: self.link_fetches.clone(),
            twitter_fetches: self.twitter_fetches.clone(),
            twitter_posts: self.twitter_posts.clone(),
            twitter_existence: self.twitter_existence.clone(),
            github_refreshes: self.github_refreshes.clone(),
            attachment_recoveries: self.attachment_recoveries.clone(),
        }
    }
    pub fn stored_body(&self, message: &Message) -> Result<Option<String>> {
        if let Some(data) = &self.search_preloads {
            return Ok(data.records.bodies.get(&message.id).cloned().flatten());
        }
        message.body_html(self.conn)
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

    pub fn pending_twitter_fetches(&self) -> Vec<i64> {
        self.twitter_fetches.borrow().iter().copied().collect()
    }
    pub fn twitter_posts(
        &self,
        message: &Message,
    ) -> Result<Vec<crate::integrations::twitter::post::Post>> {
        if let Some(posts) = self.twitter_posts.borrow().get(&message.id) {
            return Ok(posts.clone());
        }
        let mut posts =
            crate::integrations::twitter::post::Post::for_message(self.conn, message.id)?;
        crate::integrations::twitter::post::Post::order_cards(&mut posts);
        self.twitter_posts
            .borrow_mut()
            .insert(message.id, posts.clone());
        Ok(posts)
    }
    pub fn request_twitter_fetch(&self, post: &crate::integrations::twitter::post::Post) {
        if post.fetch_pending() {
            self.twitter_fetches.borrow_mut().insert(post.id);
        }
    }

    pub fn user(&self, id: i64) -> Result<User> {
        if let Some(user) = self.users.borrow().get(&id) {
            return Ok(user.clone());
        }
        if let Some(data) = &self.search_preloads {
            return data
                .users
                .get(&id)
                .map(|u| u.user.clone())
                .ok_or(campfire_db::Error::RecordNotFound("User"));
        }
        let user = User::find(self.conn, id)?;
        self.users.borrow_mut().insert(id, user.clone());
        Ok(user)
    }

    pub fn user_view(&self, id: i64) -> Result<UserView> {
        let user = self.user(id)?;
        let mut view = user_view(self.secrets, &user);
        view.avatar_url = avatar_path_in_zone(self.secrets, &user, &self.render_zone);
        if let Some(data) = &self.search_preloads {
            if let Some(row) = data.users.get(&id)
                && user.is_bot()
                && !row.uploaded_avatar
            {
                view.icon = row.icon_name.as_deref().and_then(|n| {
                    campfire_presentation::helpers::IconSource::resolve_avatar_icon(self, n)
                });
            }
            return Ok(view);
        }
        let uploaded: bool = self.conn.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type = 'User' AND record_id = ?1 AND name = 'avatar')", [id], |row| row.get(0))?;
        if user.is_bot() && !uploaded {
            let icon_name: Option<String> =
                self.conn
                    .query_row("SELECT icon_name FROM users WHERE id = ?1", [id], |row| {
                        row.get(0)
                    })?;
            view.icon = icon_name.as_deref().and_then(|name| {
                campfire_presentation::helpers::IconSource::resolve_avatar_icon(self, name)
            });
        }
        Ok(view)
    }

    /// `room_display_name(room, for_user:)`.
    pub fn room_display_name(&self, room: &Room, for_user: Option<&User>) -> Result<String> {
        Ok(if room.direct() {
            room.direct_display_name(self.conn, for_user, None)?
                .unwrap_or_default()
        } else {
            room.name.clone().unwrap_or_default()
        })
    }

    pub fn plain_text_body(&self, message: &Message) -> Result<String> {
        if let Some(data) = &self.search_preloads {
            return data.plain_text(self, message);
        }
        if !message.markdown() {
            return message.plain_text_body(self.conn, self.rich_text);
        }
        let body = message.body_html(self.conn)?.unwrap_or_default();
        let resolver = self.resolver();
        let mut text = campfire_richtext::markdown::plain_text(
            &body,
            &resolver.render_context(self.request_host.clone()),
            &resolver.db,
        )
        .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
        // Message#plain_text_body applies these after Markdown.plain_text, including
        // attachment-only Markdown and a forward note. forwarded_markdown is not markdown?.
        if campfire_presentation::helpers::is_blank(&text) {
            text = message.attachment_summary(self.conn)?;
        }
        Ok(
            match message
                .forward_note
                .as_deref()
                .filter(|note| !campfire_presentation::helpers::is_blank(note))
            {
                Some(note) if campfire_presentation::helpers::is_blank(&text) => note.to_string(),
                Some(note) => format!("{note}\n\n{text}"),
                None => text,
            },
        )
    }

    /// `message.boosts.ordered`.
    pub fn boosts(&self, message: &Message) -> Result<Vec<BoostView>> {
        if let Some(data) = &self.search_preloads {
            return data
                .records
                .boosts
                .get(&message.id)
                .into_iter()
                .flatten()
                .map(|b| self.boost(b))
                .collect();
        }
        Boost::for_message_ordered(self.conn, message.id)?
            .iter()
            .map(|boost| self.boost(boost))
            .collect()
    }

    pub fn boost(&self, boost: &Boost) -> Result<BoostView> {
        Ok(BoostView {
            id: boost.id,
            updated_at: boost.updated_at.jiff(),
            message_id: boost.message_id,
            content: boost.content.clone(),
            all_emoji: all_emoji_with_icons(&boost.content, |name| {
                use campfire_presentation::helpers::{AvatarIcon, IconSource};
                crate::rich_text::builtin_icon(name)
                    || matches!(self.resolve_avatar_icon(name), Some(AvatarIcon::Image { .. }))
            }),
            booster: self.user_view(boost.booster_id)?,
            reaction: campfire_presentation::messages::reactions::resolve(&boost.content, self),
        })
    }

    /// `message.content_type`, with what `message_presentation` shows for it.
    pub fn content(&self, message: &Message, plain_text: &str) -> Result<MessageContent> {
        let stored_body = self.stored_body(message)?;
        let missing_body = stored_body.is_none();
        let body = stored_body.unwrap_or_default();
        let resolver = self.resolver();
        let ctx = resolver.render_context(self.request_host.clone());
        // `message_tag` evaluates `message.plain_text_body` first; where that raises, it rescues
        // and renders `messages/_unrenderable`, unless logging the exception raises again (a
        // message that isn't UTF-8): then the page fails (verified against the reference).
        match campfire_richtext::to_plain_text(&body, &ctx) {
            Err(campfire_richtext::Error::Unrenderable(error)) => {
                return Err(campfire_db::Error::Other(format!(
                    "message_tag's rescue raised logging {error}"
                )));
            }
            Err(_) => return Ok(MessageContent::Unrenderable),
            Ok(_) => {}
        }
        if let Some(attachment) = self.attachment(message)? {
            return Ok(MessageContent::Attachment(attachment));
        }
        if let Some(sound) = campfire_db::message::sound_in(plain_text) {
            return Ok(MessageContent::Sound(SoundView {
                url: campfire_static_assets::asset_path(&sound.asset_path()),
                image: sound.image.map(|image| SoundImage {
                    src: campfire_static_assets::image_path(&image.asset_path()),
                    width: image.width,
                    height: image.height,
                }),
                text: sound.text.map(str::to_string),
            }));
        }
        // Drive-only messages can have no ActionText body. Rails' message_presentation
        // rescues the nil content and returns an empty string, without a trix wrapper.
        if missing_body {
            return Ok(MessageContent::Text {
                html: String::new(),
            });
        }
        if message.markdown() || message.forwarded_markdown {
            let html = if let Some(data) = &self.search_preloads {
                campfire_richtext::markdown::presentation(&body, &ctx, &data.icons, None)
                    .map_err(|e| e.to_string())
            } else {
                crate::rich_text::markdown_presentation(self.conn, &body, &ctx)
            };
            return Ok(match html {
                Ok(html) => MessageContent::Text { html },
                Err(_) => MessageContent::Unrenderable,
            });
        }
        Ok(match campfire_richtext::present_message(&body, &ctx) {
            Presentation::Html(html) => MessageContent::Text { html },
            Presentation::Unrenderable => MessageContent::Unrenderable,
        })
    }

    /// `message.attachment` as `Messages::AttachmentPresentation` needs it.
    pub fn attachment(&self, message: &Message) -> Result<Option<AttachmentView>> {
        let blob = self.message_files(message)?.blobs.into_iter().next();
        let Some(blob) = blob else { return Ok(None) };
        self.attachment_blob(message, &blob).map(Some)
    }

    pub fn message_files(&self, message: &Message) -> Result<campfire_storage::blob::MessageAttachments> {
        if let Some(data) = &self.search_preloads {
            return Ok(data.attachments.get(&message.id).cloned().unwrap_or_default());
        }
        Ok(campfire_storage::Blob::attached_messages(self.conn, &[message.id])
            .map_err(storage_error)?.remove(&message.id).unwrap_or_default())
    }

    pub fn attachment_blob(&self, message: &Message, blob: &campfire_storage::Blob) -> Result<AttachmentView> {
        if blob.is_video() && (blob.is_previewable() || blob.is_variable()) {
            self.recover_attachment_preview(message, blob)?;
        }
        self.attachment_file(blob)
    }

    pub fn attachment_file(&self, blob: &campfire_storage::Blob) -> Result<AttachmentView> {
        let verifier = &*self.storage.verifier;
        let preview = if blob.is_previewable() || blob.is_variable() {
            if blob.is_video() {
                // `attachment.preview(format: :webp, resize_to_limit: [...])`
                let poster = Variation::new(vec![
                    (
                        "format".into(),
                        campfire_storage::marshal::Value::Symbol("webp".into()),
                    ),
                    (
                        "resize_to_limit".into(),
                        campfire_storage::marshal::Value::Array(vec![
                            campfire_storage::marshal::Value::Int(THUMBNAIL_MAX_WIDTH),
                            campfire_storage::marshal::Value::Int(THUMBNAIL_MAX_HEIGHT),
                        ]),
                    ),
                ]);
                AttachmentPreview::Video {
                    poster_url: self.preview_attached(blob)?.then(|| campfire_storage::paths::representation_redirect_path(
                        verifier, blob, &poster,
                    )),
                }
            } else {
                AttachmentPreview::Image {
                    thumb_url: self.thumb_path(blob)?,
                }
            }
        } else {
            AttachmentPreview::File
        };
        Ok(AttachmentView {
            filename: blob.filename.to_string(),
            filename_base: blob.filename.base().to_string(),
            blob_path: campfire_storage::paths::blob_redirect_path(verifier, blob, None),
            download_path: campfire_storage::paths::blob_redirect_path(
                verifier,
                blob,
                Some("attachment"),
            ),
            preview,
            width: dimension(blob, "width"),
            height: dimension(blob, "height"),
        })
    }

    pub fn preview_attached(&self, blob: &campfire_storage::Blob) -> Result<bool> {
        if let Some(data) = &self.search_preloads {
            return Ok(data.previewed_blobs.contains(&blob.id));
        }
        Ok(self.conn.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='ActiveStorage::Blob' AND name='preview_image' AND record_id=?)", [blob.id], |r| r.get(0))?)
    }

    pub fn recover_attachment_preview(&self, message: &Message, blob: &campfire_storage::Blob) -> Result<()> {
        if blob.is_video() && !self.preview_attached(blob)? {
            self.attachment_recoveries.borrow_mut().insert((message.id, blob.id));
        }
        Ok(())
    }

    /// `polymorphic_url(attachment.representation(:thumb), only_path: true)`.
    fn thumb_path(&self, blob: &campfire_storage::Blob) -> Result<String> {
        let thumb = Variation::resize_to_limit(THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT, None);
        let variation = if blob.is_previewable() {
            thumb
        } else {
            self.storage
                .variation_for(blob, &thumb)
                .map_err(storage_error)?
        };
        Ok(campfire_storage::paths::representation_redirect_path(
            &*self.storage.verifier,
            blob,
            &variation,
        ))
    }

    /// Fallible ActionText::Content#to_s for human payloads and legacy conversion.
    pub fn rendered_body_html(&self, message: &Message) -> Result<String> {
        let Some(body) = self.stored_body(message)? else {
            return Ok(String::new());
        };
        self.render_body_html(&body)
    }

    fn render_body_html(&self, body: &str) -> Result<String> {
        let resolver = self.resolver();
        let ctx = resolver.render_context(self.request_host.clone());
        campfire_richtext::Content::load(body, &ctx)
            .and_then(|content| content.to_rendered_html_with_layout(&ctx))
            .map_err(|error| campfire_db::Error::Other(error.to_string()))
    }

    /// Compatibility entry point used by the bot and agent readers.
    pub fn editable_body(&self, message: &Message) -> Result<String> {
        self.rendered_body_html(message)
    }

    /// Message#editable_markdown_source passes rendered Content, not Content#to_html.
    pub fn editable_markdown_source(&self, message: &Message) -> Result<String> {
        if let Some(source) = &message.markdown_source {
            return Ok(source.clone());
        }
        let body = self.rendered_body_html(message)?;
        let resolver = self.resolver();
        campfire_richtext::legacy_markdown::render(
            &body,
            &resolver.render_context(self.request_host.clone()),
        )
        .map_err(|error| campfire_db::Error::Other(error.to_string()))
    }

    /// `messages/boosts/_boost.json.jbuilder` (`json.cache! boost`).
    pub fn boost_json(
        &self,
        boost: &Boost,
        message: &Message,
        base_url: &str,
    ) -> Result<BoostJson> {
        let key = || {
            jbuilder_key(
                "messages/boosts/_boost",
                &cache_key_with_version("boosts", boost.id, boost.updated_at.jiff()),
                base_url,
            )
        };
        fragment_cache::try_fetch_value(key, || self.render_boost_json(boost, message, base_url))
    }

    fn render_boost_json(
        &self,
        boost: &Boost,
        message: &Message,
        base_url: &str,
    ) -> Result<BoostJson> {
        Ok(BoostJson {
            id: boost.id,
            content: boost.content.clone(),
            created_at: json_time(boost.created_at.jiff()),
            booster: cached_user_json(self.secrets, base_url, &self.user(boost.booster_id)?),
            message: BoostMessageJson {
                id: boost.message_id,
                url: format!(
                    "{base_url}{}",
                    campfire_routes::room_message(message.room_id, message.id)
                ),
            },
        })
    }

}

/// A `User` row as the users views see it.
pub fn user_summary(secrets: &Secrets, user: &User) -> campfire_presentation::users::UserSummary {
    user_summary_in_zone(secrets, user, &campfire_presentation::time::Zone::utc())
}

pub fn user_summary_in_zone(secrets: &Secrets, user: &User, zone: &campfire_presentation::time::Zone) -> campfire_presentation::users::UserSummary {
    use campfire_presentation::users::{Role, Status};
    campfire_presentation::users::UserSummary {
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
        avatar_path: avatar_path_in_zone(secrets, user, zone),
        two_factor_enabled: false,
        google_identity_email: None,
        email_self_changed: false,
        google_email_link_allowed: false,
    }
}

/// The account list additionally offers administrator recovery for enrolled humans.
pub fn account_user_summary(
    conn: &Connection,
    secrets: &Secrets,
    user: &User,
) -> campfire_db::Result<campfire_presentation::users::UserSummary> {
    let (google_identity_email,email_self_changed,google_email_link_allowed) = conn.query_row(
        "SELECT google_identities.email,users.email_self_changed_at IS NOT NULL,users.google_email_link_allowed FROM users LEFT JOIN google_identities ON google_identities.user_id=users.id WHERE users.id=?",[user.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    )?;
    Ok(campfire_presentation::users::UserSummary {
        two_factor_enabled: user.two_factor_enabled(conn)?,
        google_identity_email,
        email_self_changed,
        google_email_link_allowed,
        ..user_summary(secrets, user)
    })
}

/// `to_fs(:epoch)` as a string (milliseconds).
pub fn epoch_string(time: jiff::Timestamp) -> String {
    campfire_presentation::messages::support::epoch_ms(time).to_string()
}

/// `attachment.metadata[:width]`: an Integer for images, a Float for videos.
pub fn dimension(blob: &campfire_storage::Blob, name: &str) -> Option<RubyNumber> {
    match blob.metadata.get(name)? {
        campfire_storage::Json::Int(value) => Some(RubyNumber::Int(*value)),
        campfire_storage::Json::Float(value) => Some(RubyNumber::Float(*value)),
        _ => None,
    }
}

pub fn storage_error(error: campfire_storage::Error) -> campfire_db::Error {
    campfire_db::Error::Other(error.to_string())
}

// Read-only owner adapter from WS13 498aa4e6; no icon mutation or rendering policy.
/// `Icons.client_icon_names`: canonical brands/aliases followed by ordered workspace icons.
pub fn client_icon_names(conn: &Connection) -> campfire_db::Result<Vec<String>> {
    crate::rich_text::client_icon_names(conn)
}


pub mod link_embeds;

/// Refreshes formerly discovered while rendering a detached message. Publication must retain
/// these requests even when no browser is subscribed.
pub fn broadcast_refreshes(conn: &Connection, app: &AppState, message: &Message) -> Result<RenderRefreshes> {
    let presenter = Presenter::new(conn, app, None);
    presenter.remember_message_refreshes(message)?;
    Ok(presenter.take_render_refreshes())
}

pub mod fizzy_cards;

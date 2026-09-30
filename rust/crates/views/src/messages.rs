//! Views for `reference/app/views/messages`, plus `MessagesHelper`,
//! `Messages::AttachmentPresentation` and the boost partials.

pub mod json;
pub mod parts;
pub mod presentation;
pub mod reactions;
pub mod support;

use askama::Template;
use campfire_routes as routes;
use jiff::Timestamp;
use serde::Deserialize;

use crate::ViewContext;
use crate::fragment_cache;
use crate::helpers as h;
use support::{RubyNumber, epoch_ms, iso8601};

/// What the message views show of a user: `avatar_tag` and the author heading.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UserView {
    pub id: i64,
    pub name: String,
    /// `User#title`: name and bio joined by " – ".
    pub title: String,
    /// `fresh_user_avatar_path(user)`.
    pub avatar_url: String,
    #[serde(default)]
    pub icon: Option<h::AvatarIcon>,
}

impl UserView {
    pub fn path(&self) -> String {
        routes::user(self.id)
    }

    pub fn avatar(&self, ctx: &ViewContext, options: h::Attrs) -> h::Html {
        h::avatar_tag_with_icon(
            ctx,
            h::AvatarUser {
                id: self.id,
                title: self.title.clone(),
                avatar_path: self.avatar_url.clone(),
            },
            self.icon.as_ref(),
            options,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    Open,
    Closed,
    Direct,
}

impl RoomKind {
    /// `Rooms::Open.model_name.param_key`, the stem of `dom_id(room)`.
    pub fn param_key(self) -> &'static str {
        match self {
            RoomKind::Open => "rooms_open",
            RoomKind::Closed => "rooms_closed",
            RoomKind::Direct => "rooms_direct",
        }
    }

    pub fn is_direct(self) -> bool {
        self == RoomKind::Direct
    }
}

/// `dom_id(room)` / `dom_id(room, prefix)`.
pub fn room_dom_id(kind: RoomKind, id: i64, prefix: &str) -> String {
    if prefix.is_empty() {
        format!("{}_{id}", kind.param_key())
    } else {
        format!("{prefix}_{}_{id}", kind.param_key())
    }
}

/// A message as `messages/_message` renders it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MessageView {
    pub id: i64,
    /// `Message#to_key`, so every `dom_id(message)` uses it.
    pub client_message_id: String,
    pub room_id: i64,
    /// `room_display_name(message.room, for_user: nil)`; see [`crate::rooms::room_display_name`].
    pub room_name: String,
    pub creator: UserView,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// `message.plain_text_body.all_emoji?` (`reference/lib/rails_ext/string.rb`).
    pub all_emoji: bool,
    pub content: MessageContent,
    /// `message.boosts.ordered`.
    #[serde(default)]
    pub boosts: Vec<BoostView>,
    #[serde(default)]
    pub details: MessageDetails,
    #[serde(default)]
    pub components: MessageComponents,
}

/// Session-independent facts read by our message partials. No viewer capability or token belongs here.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct MessageDetails {
    pub thread_id: Option<i64>,
    pub system_note: bool,
    pub action: bool,
    pub edited_at: Option<Timestamp>,
    pub streaming: bool,
    pub markdown: bool,
    pub pinned: bool,
    pub reply: Option<ReplyPreview>,
    pub forwarded: bool,
    pub forward_note: Option<String>,
    pub drive_urls: Vec<String>,
    pub reply_count: u64,
    pub room_icon: Option<h::AvatarIcon>,
    pub agent_steps: Vec<parts::AgentStep>,
    pub poll: Option<parts::Poll>,
}

/// Already rendered, session-independent child partials, supplied by the domain owners.
/// These are server-produced HTML, never unsanitized database or request text. The message
/// composition keeps all replacement targets, including empty card containers, in Rails order.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct MessageComponents {
    pub github_cards: Vec<String>,
    pub twitter_cards: Vec<String>,
    pub event_cards: Vec<String>,
    pub message_link_cards: Vec<String>,
    pub fizzy_cards: Vec<String>,
    pub linkedin_cards: Vec<String>,
    pub link_embed_cards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ReplyPreview {
    pub source: Option<ReplySource>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ReplySource {
    pub id: i64,
    pub author: String,
    pub plain_text: String,
    pub url: String,
}

impl ReplyPreview {
    pub fn id_string(&self) -> String {
        self.source
            .as_ref()
            .map(|source| source.id.to_string())
            .unwrap_or_default()
    }
}

impl ReplySource {
    pub fn link(&self, ctx: &ViewContext) -> h::Html {
        let text = if self.plain_text.chars().count() > 180 {
            self.plain_text.chars().take(177).collect::<String>() + "..."
        } else {
            self.plain_text.clone()
        };
        h::link_to(
            &if self.url.starts_with('/') {
                format!("{}{}", ctx.base_url, self.url)
            } else {
                self.url.clone()
            },
            h::attrs()
                .class("message__reply-preview-link")
                .data("reply_target_id", self.id),
            &format!(
                "\n        <span class=\"overflow-ellipsis\">{}</span>\n        <span class=\"for-screen-reader\">View original message</span>\n",
                h::escape(&text)
            ),
        )
    }
}

/// `Message#content_type` with what each presentation needs.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    /// The presentation filters' output after `auto_link`, from the richtext crate.
    Text {
        html: String,
    },
    Sound(SoundView),
    Attachment(AttachmentView),
    /// Rendering raised past `message_presentation`'s own rescue (or `plain_text_body` raised):
    /// `message_tag` rescues and renders `messages/_unrenderable` in place of the whole message.
    Unrenderable,
}

/// A `/play <name>` message's `Sound`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundView {
    /// `asset_path(sound.asset_path)`, the digested mp3.
    pub url: String,
    pub image: Option<SoundImage>,
    pub text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundImage {
    /// `image_path(image.asset_path)`.
    pub src: String,
    pub width: u32,
    pub height: u32,
}

/// The message's Active Storage attachment.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AttachmentView {
    /// `attachment.filename.to_s`.
    pub filename: String,
    /// `attachment.filename.base.to_s`, derived from the stored unsanitized filename.
    pub filename_base: String,
    /// `rails_blob_path(attachment)`.
    pub blob_path: String,
    /// `rails_blob_path(attachment, disposition: "attachment")`.
    pub download_path: String,
    pub preview: AttachmentPreview,
    /// `attachment.metadata[:width]`: an Integer for images, a Float for videos.
    pub width: Option<RubyNumber>,
    pub height: Option<RubyNumber>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AttachmentPreview {
    /// `attachment.video?`: `url_for(attachment.preview(format: :webp, resize_to_limit: ...))`.
    Video { poster_url: String },
    /// Otherwise previewable or variable: `polymorphic_url(attachment.representation(:thumb), only_path: true)`.
    Image { thumb_url: String },
    /// Neither previewable nor variable: a download link.
    File,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BoostView {
    pub id: i64,
    /// For the fragment cache key (`boost.cache_key_with_version`).
    #[serde(default)]
    pub updated_at: Timestamp,
    pub message_id: i64,
    pub content: String,
    /// `boost.content.all_emoji?`.
    pub all_emoji: bool,
    pub booster: UserView,
    /// `Boost.reaction?` and `BoostsHelper#reaction_title`, resolved by the domain's icon registry.
    #[serde(default)]
    pub reaction: Option<ReactionContent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ReactionContent {
    pub title: String,
    pub icon: Option<h::AvatarIcon>,
    #[serde(default)]
    pub icon_alt: Option<String>,
}

pub struct ReactionGroup<'a> {
    pub content: &'a str,
    pub reaction: &'a ReactionContent,
    pub reactors: Vec<&'a UserView>,
}

/// A message on its way into `messages/_message`: the fragment itself when the cache already
/// holds this message version, else the view to render it from. `cache [ message,
/// "presentation-v3" ]` wraps the whole partial, so on a hit Rails evaluates none of it (no rich
/// text, attachment, avatar or boosts); [`cached_message_fragment`] lets the presenter look first
/// and build a [`MessageView`] only on a miss.
#[derive(Clone, Debug, PartialEq)]
pub enum MessageItem {
    Fragment {
        client_message_id: String,
        room_id: i64,
        html: fragment_cache::Fragment,
    },
    View(Box<MessageView>),
}

impl MessageItem {
    /// `dom_id(message)` / `dom_id(message, prefix)`.
    pub fn dom_id(&self, prefix: &str) -> String {
        match self {
            MessageItem::Fragment {
                client_message_id, ..
            } if prefix.is_empty() => format!("message_{client_message_id}"),
            MessageItem::Fragment {
                client_message_id, ..
            } => format!("{prefix}_message_{client_message_id}"),
            MessageItem::View(message) => message.dom_id(prefix),
        }
    }

    pub fn room_id(&self) -> i64 {
        match self {
            MessageItem::Fragment { room_id, .. } => *room_id,
            MessageItem::View(message) => message.room_id,
        }
    }

    /// The cached fragments of a rendered page's `items` that are in the page as they are, in
    /// order, including those this render just stored in `cache`. Call it after rendering, so the
    /// page's parts (and its ETag) don't depend on which messages happened to be cached before.
    /// A fragment with forms isn't: the page has this render's tokens where it has slots.
    pub fn cached_fragments(
        cache: &fragment_cache::FragmentCache,
        items: &[MessageItem],
        base_url: &str,
    ) -> Vec<fragment_cache::Fragment> {
        items
            .iter()
            .filter_map(|item| match item {
                MessageItem::Fragment { html, .. } => Some(html.clone()),
                MessageItem::View(message) => cache.get(&message_fragment_key(
                    message.id,
                    message.updated_at,
                    base_url,
                )),
            })
            .filter(|html| !crate::helpers::request_forgery::has_token_slots(html))
            .collect()
    }
}

impl From<MessageView> for MessageItem {
    fn from(message: MessageView) -> Self {
        MessageItem::View(Box::new(message))
    }
}

/// Deserializes a [`MessageView`] (fixtures describe views, never cached fragments).
impl<'de> Deserialize<'de> for MessageItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        MessageView::deserialize(deserializer).map(|message| MessageItem::View(Box::new(message)))
    }
}

/// `EmojiHelper::REACTIONS` (see [`crate::helpers::emoji`]).
pub use crate::helpers::emoji::REACTIONS;

impl MessageView {
    /// `dom_id(message)` / `dom_id(message, prefix)`.
    pub fn dom_id(&self, prefix: &str) -> String {
        if prefix.is_empty() {
            format!("message_{}", self.client_message_id)
        } else {
            format!("{prefix}_message_{}", self.client_message_id)
        }
    }

    pub fn is_unrenderable(&self) -> bool {
        matches!(self.content, MessageContent::Unrenderable)
    }

    pub fn attachment(&self) -> Option<&AttachmentView> {
        match &self.content {
            MessageContent::Attachment(attachment) => Some(attachment),
            _ => None,
        }
    }

    pub fn created_at_iso(&self) -> String {
        iso8601(self.created_at)
    }

    pub fn created_at_epoch(&self) -> i64 {
        epoch_ms(self.created_at)
    }

    pub fn updated_at_epoch(&self) -> i64 {
        epoch_ms(self.updated_at)
    }

    pub fn at_path(&self) -> String {
        if let Some(thread) = self.details.thread_id {
            routes::ROOM.path_with(
                &[&self.room_id],
                None,
                &[
                    ("thread", Some(&thread.to_string())),
                    ("message_id", Some(&self.id.to_string())),
                ],
            )
        } else {
            routes::room_at_message(self.room_id, self.id)
        }
    }

    pub fn path(&self) -> String {
        if let Some(thread) = self.details.thread_id {
            routes::room_thread_message(self.room_id, thread, self.id)
        } else {
            routes::room_message(self.room_id, self.id)
        }
    }

    pub fn edit_path(&self) -> String {
        routes::edit_room_message(self.room_id, self.id)
    }

    pub fn link_url(&self, ctx: &ViewContext) -> String {
        format!("{}{}", ctx.base_url, self.at_path())
    }
    pub fn action_url(&self, ctx: &ViewContext) -> String {
        format!("{}{}", ctx.base_url, self.path())
    }

    pub fn timestamp(&self, style: &str, class: bool) -> h::Html {
        crate::time::local_datetime_tag(
            &crate::time::Zone::utc(),
            self.created_at,
            style,
            if class {
                h::attrs().class("message__timestamp")
            } else {
                h::attrs()
            },
            "",
        )
    }

    pub fn author_button(&self) -> h::Html {
        h::button_tag(
            h::attrs()
                .type_("button")
                .class("profile-card-name")
                .merge(h::profile_card_trigger(self.creator.id, false)),
            &format!(
                "\n        <strong data-reply-target=\"author\">{}</strong>\n",
                h::escape(&self.creator.name)
            ),
        )
    }

    pub fn edited_iso(&self) -> String {
        self.details.edited_at.map(iso8601).unwrap_or_default()
    }
    pub fn edited_label(&self) -> String {
        self.details
            .edited_at
            .map(|time| crate::time::Zone::utc().to_fs(time, "long"))
            .unwrap_or_default()
    }

    pub fn forward_note(&self) -> Option<&str> {
        self.details
            .forward_note
            .as_deref()
            .filter(|note| !h::is_blank(note))
    }

    pub fn reply_count_label(&self) -> String {
        format!(
            "{} {}",
            self.details.reply_count,
            if self.details.reply_count == 1 {
                "reply"
            } else {
                "replies"
            }
        )
    }

    pub fn legacy_boosts(&self) -> Vec<&BoostView> {
        self.boosts
            .iter()
            .filter(|boost| boost.reaction.is_none())
            .collect()
    }

    pub fn reaction_groups(&self) -> Vec<ReactionGroup<'_>> {
        let mut groups: Vec<ReactionGroup<'_>> = Vec::new();
        for boost in &self.boosts {
            let Some(reaction) = &boost.reaction else {
                continue;
            };
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.content == boost.content)
            {
                if !group
                    .reactors
                    .iter()
                    .any(|reactor| reactor.id == boost.booster.id)
                {
                    group.reactors.push(&boost.booster);
                }
            } else {
                groups.push(ReactionGroup {
                    content: &boost.content,
                    reaction,
                    reactors: vec![&boost.booster],
                });
            }
        }
        groups
    }

    pub fn boosts_path(&self) -> String {
        routes::message_boosts(self.id)
    }

    pub fn new_boost_path(&self) -> String {
        routes::new_message_boost(self.id)
    }
}

impl BoostView {
    pub fn dom_id(&self) -> String {
        format!("boost_{}", self.id)
    }

    pub fn path(&self) -> String {
        routes::message_boost(self.message_id, self.id)
    }
    pub fn avatar_label(&self) -> String {
        format!("{} boosted {}", self.booster.name, self.content)
    }
    pub fn content_html(&self, ctx: &ViewContext) -> h::Html {
        reaction_body(
            ctx,
            &self.content,
            self.reaction
                .as_ref()
                .and_then(|reaction| reaction.icon.as_ref()),
            self.reaction
                .as_ref()
                .and_then(|reaction| reaction.icon_alt.as_deref()),
            true,
        )
    }
}

/// `messages/_message`.
#[derive(Template)]
#[template(path = "messages/_message.html")]
pub struct MessagePartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// `render message`: `messages/_message`, whose body is `cache [ message, "presentation-v3" ]`
/// (and whose collection renders are `cached: true`), so a message version renders once.
pub fn message(ctx: &ViewContext, message: &MessageView) -> String {
    fragment_cache::fetch(
        || message_fragment_key(message.id, message.updated_at, &ctx.base_url),
        || {
            MessagePartial { ctx, message }
                .render()
                .expect("messages/_message renders")
        },
    )
}

/// [`message`] where a template renders the partial.
pub fn cached_message(ctx: &ViewContext, message: &MessageView) -> crate::helpers::Html {
    askama::filters::Safe(self::message(ctx, message))
}

/// WS8bm's complete-message callback renderer. A child step changes without
/// touching message.updated_at, so callbacks must bypass the warmed fragment.
pub fn uncached_message(ctx: &ViewContext, message: &MessageView) -> String {
    MessagePartial { ctx, message }
        .render()
        .expect("messages/_message renders")
}

/// [`cached_message`] for a [`MessageItem`]: a fragment found up front goes out as it is, with
/// this render's tokens in its slots.
pub fn cached_message_item<'a>(
    ctx: &ViewContext,
    item: &'a MessageItem,
) -> askama::filters::Safe<std::borrow::Cow<'a, str>> {
    askama::filters::Safe(match item {
        MessageItem::Fragment { html, .. } => {
            crate::helpers::request_forgery::fill_token_slots(html)
        }
        MessageItem::View(message) => std::borrow::Cow::Owned(self::message(ctx, message)),
    })
}

/// `messages/_message`'s fragment for this message version, if the current store holds it. The
/// key includes the message version and URL origin, so forged hosts cannot poison other origins.
pub fn cached_message_fragment(
    id: i64,
    updated_at: Timestamp,
    base_url: &str,
) -> Option<fragment_cache::Fragment> {
    fragment_cache::read(&message_fragment_key(id, updated_at, base_url))
}

fn message_fragment_key(id: i64, updated_at: Timestamp, base_url: &str) -> String {
    format!(
        "views/messages/_message:{}/{}/presentation-v{}/{base_url}",
        message_digest(),
        fragment_cache::cache_key_with_version("messages", id, updated_at),
        fragment_cache::keys::PRESENTATION_CACHE_VERSION,
    )
}

/// `messages/boosts/_boost`, whose body is `cache boost`.
pub fn boost(ctx: &ViewContext, boost: &BoostView) -> String {
    fragment_cache::fetch(
        || {
            format!(
                "views/messages/boosts/_boost:{}/{}",
                boost_digest(),
                fragment_cache::cache_key_with_version("boosts", boost.id, boost.updated_at)
            )
        },
        || {
            BoostPartial { ctx, boost }
                .render()
                .expect("messages/boosts/_boost renders")
        },
    )
}

/// [`boost`] where a template renders the partial.
pub fn cached_boost(ctx: &ViewContext, boost: &BoostView) -> crate::helpers::Html {
    askama::filters::Safe(self::boost(ctx, boost))
}

/// The template digest in `messages/_message`'s fragment keys: the partial and what it renders.
fn message_digest() -> &'static str {
    static DIGEST: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        fragment_cache::digest(&[
            include_str!("../templates/messages/_message.html"),
            include_str!("../templates/messages/_presentation.html"),
            include_str!("../templates/messages/_toolbar.html"),
            include_str!("../templates/messages/_pin_badge.html"),
            include_str!("../templates/messages/_meta.html"),
            include_str!("../templates/messages/_streaming_indicator.html"),
            include_str!("../templates/messages/_context.html"),
            include_str!("../templates/messages/_system_note.html"),
            include_str!("../templates/messages/_drive_attachments.html"),
            include_str!("../templates/messages/_thread_indicator.html"),
            include_str!("../templates/messages/boosts/_reactions.html"),
            include_str!("../templates/messages/boosts/_reaction.html"),
            include_str!("../templates/agent_steps/_steps.html"),
            include_str!("../templates/polls/_poll.html"),
            include_str!("../templates/messages/_unrenderable.html"),
            include_str!("../templates/messages/boosts/_boosts.html"),
            include_str!("../templates/messages/boosts/_boost.html"),
        ])
    });
    &DIGEST
}

fn boost_digest() -> &'static str {
    static DIGEST: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        fragment_cache::digest(&[include_str!("../templates/messages/boosts/_boost.html")])
    });
    &DIGEST
}

/// `messages/index`: the page of messages the client fetches while scrolling (no layout).
#[derive(Template)]
#[template(path = "messages/index.html")]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub messages: &'a [MessageItem],
}

/// `messages/show`: the message partial, inside the application layout.
#[derive(Template)]
#[template(path = "messages/show.html")]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// `messages/_presentation`, which `MessagesController#update` also broadcasts.
#[derive(Template)]
#[template(path = "messages/_presentation.html")]
pub struct PresentationPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// `messages/_unrenderable`.
#[derive(Template)]
#[template(path = "messages/_unrenderable.html")]
pub struct Unrenderable;

/// `messages/room_not_found`.
#[derive(Template)]
#[template(path = "messages/room_not_found.html")]
pub struct RoomNotFound;

/// What `messages/edit` needs besides the message.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EditView {
    pub message: MessageView,
    /// The editor's `value`: `editable_body(message)` as HTML, from the richtext crate.
    pub editable_body_html: String,
}

/// `messages/edit`.
#[derive(Template)]
#[template(path = "messages/edit.html")]
pub struct Edit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub edit: &'a EditView,
}

/// `messages/create.turbo_stream`: appends the new message to its room's list. Also what
/// `Message#broadcast_create` sends.
#[derive(Template)]
#[template(path = "messages/create.turbo_stream.html")]
pub struct CreateStream<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageItem,
    pub room_kind: RoomKind,
}

/// `messages/destroy.turbo_stream`, also what `Message#broadcast_remove` sends.
#[derive(Template)]
#[template(path = "messages/destroy.turbo_stream.html")]
pub struct DestroyStream<'a> {
    pub message: &'a MessageView,
}

/// `messages/boosts/_boosts`.
#[derive(Template)]
#[template(path = "messages/boosts/_boosts.html")]
pub struct BoostsPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// `messages/boosts/index`.
#[derive(Template)]
#[template(path = "messages/boosts/index.html")]
pub struct BoostsIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// `messages/boosts/_boost`, which the boosts controller also broadcasts on its own.
#[derive(Template)]
#[template(path = "messages/boosts/_boost.html")]
pub struct BoostPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub boost: &'a BoostView,
}

/// `messages/boosts/new`.
#[derive(Template)]
#[template(path = "messages/boosts/new.html")]
pub struct NewBoost<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
    /// `Current.user`.
    pub user: &'a UserView,
}

/// `MessagesHelper#message_tag`, in the Ruby hash's attribute order.
pub fn message_open(ctx: &ViewContext, message: &MessageView) -> h::Html {
    let details = &message.details;
    let mut class = "message".to_string();
    if !details.system_note && message.all_emoji {
        class.push_str(" message--emoji");
    }
    if details.system_note {
        class.push_str(" message--system-note");
    }
    if !details.system_note && details.action {
        class.push_str(" message--action");
    }
    let mut attributes = h::attrs()
        .id(message.dom_id(""))
        .tabindex(-1)
        .class(class)
        .attr_opt("role", details.system_note.then_some("note"))
        .data("controller", "reply")
        .data("message_id", message.id)
        .data("room_id", message.room_id)
        .attr_opt("data-thread-id", details.thread_id)
        .data("message_timestamp", message.created_at_epoch())
        .data("message_updated_at", message.updated_at_epoch())
        .data("sort_value", message.created_at_epoch())
        .data("messages_target", "message")
        .data("message_format_target", "message")
        .data("search_results_target", "message")
        .attr_opt(
            "data-refresh-room-target",
            details.thread_id.is_none().then_some("message"),
        )
        .data(
            "reply_composer_outlet",
            details.thread_id.map_or("#composer".into(), |id| {
                format!("#composer_channel_thread_{id}")
            }),
        );
    if !details.system_note {
        attributes = attributes
            .data("user_id", message.creator.id)
            .data(
                "actions_url",
                format!("{}/actions", message.action_url(ctx)),
            )
            .data("message_url", message.action_url(ctx))
            .data(
                "boost_url",
                format!("{}{}", ctx.base_url, message.boosts_path()),
            );
    }
    let tag = h::content_tag("div", attributes, "").0;
    h::raw(tag.strip_suffix("</div>").unwrap())
}

/// Card owners supply their rendered ERB loop bodies, including branch-specific whitespace.
pub fn cards(
    message: &MessageView,
    prefix: &str,
    class: &str,
    indent: usize,
    bodies: &[String],
) -> h::Html {
    h::raw(format!(
        "{}<div id=\"{}\" class=\"{class}\">{}</div>\n",
        " ".repeat(indent),
        message.dom_id(prefix),
        bodies.concat()
    ))
}

pub fn event_cards(message: &MessageView) -> h::Html {
    if message.components.event_cards.is_empty() {
        return h::raw("");
    }
    h::raw(format!(
        "  <div id=\"{}\" class=\"event-cards\">\n{}  </div>\n",
        message.dom_id("event_cards"),
        message.components.event_cards.concat()
    ))
}

pub fn drive_attachment(url: &str) -> h::Html {
    h::link_to(
        url,
        h::attrs()
            .class("drive-attachment")
            .target("_blank")
            .attr("rel", "noopener"),
        "\n        <span class=\"drive-attachment__icon\" aria-hidden=\"true\"><svg viewBox=\"0 0 16 16\" width=\"20\" height=\"20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\"><path d=\"M4 1.5h5.5L13 5v9.5H4z\"/><path d=\"M9.5 1.5V5H13\"/></svg></span>\n        <span class=\"drive-attachment__text\">\n          <span class=\"drive-attachment__name\">Google Drive file</span>\n          <span class=\"drive-attachment__meta\">Open in Drive</span>\n        </span>\n",
    )
}

pub fn reaction_body(
    ctx: &ViewContext,
    content: &str,
    icon: Option<&h::AvatarIcon>,
    icon_alt: Option<&str>,
    legacy: bool,
) -> h::Html {
    match icon {
        Some(h::AvatarIcon::Image { title, url, brand }) => {
            let attributes = h::attrs()
                .class(if *brand {
                    "icon icon--brand"
                } else {
                    "icon icon--custom"
                })
                .alt(icon_alt.unwrap_or(content));
            h::image_tag(
                ctx,
                url,
                if legacy {
                    attributes.title(title.as_str())
                } else {
                    attributes
                }
                .attr("draggable", "false"),
            )
        }
        Some(h::AvatarIcon::Emoji { character, .. }) if !legacy => h::text(character),
        _ => h::text(content),
    }
}

/// The counted chip is deliberately tokenless (#148), including on detached broadcasts.
pub fn reaction(
    ctx: &ViewContext,
    message: &MessageView,
    group: &ReactionGroup<'_>,
    index: impl std::borrow::Borrow<usize>,
) -> h::Html {
    let partial = ReactionPartial {
        ctx,
        message,
        group,
        index: *index.borrow(),
    };
    h::raw(partial.render().expect("messages/boosts/_reaction renders"))
}

#[derive(Template)]
#[template(path = "messages/boosts/_reaction.html")]
struct ReactionPartial<'a> {
    ctx: &'a ViewContext<'a>,
    message: &'a MessageView,
    group: &'a ReactionGroup<'a>,
    index: usize,
}

impl ReactionPartial<'_> {
    fn tooltip_id(&self) -> String {
        self.message.dom_id(&format!("reactors_{}", self.index))
    }
    fn open(&self) -> h::Html {
        h::form_with(self.message.boosts_path())
            .class("reaction-chip__form")
            .authenticity_token(false)
            .data("turbo_frame", self.message.dom_id("boosting"))
            .open()
    }
    fn button_open(&self) -> h::Html {
        let ids = self
            .group
            .reactors
            .iter()
            .map(|reactor| reactor.id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let attrs = h::attrs()
            .type_("submit")
            .class("reaction-chip")
            .title(self.group.reaction.title.as_str())
            .aria(
                "label",
                format!(
                    "{}: {}",
                    self.group.reaction.title,
                    self.group.reactors.len()
                ),
            )
            .aria("pressed", "false")
            .attr_opt(
                "aria-describedby",
                (!self.group.reactors.is_empty()).then(|| self.tooltip_id()),
            )
            .data("controller", "reaction-chip")
            .data("reaction_chip_booster_ids_value", ids)
            .data("reaction", self.group.content);
        let tag = h::button_tag(attrs, "").0;
        h::raw(tag.strip_suffix("</button>").unwrap())
    }
    fn body(&self) -> h::Html {
        reaction_body(
            self.ctx,
            self.group.content,
            self.group.reaction.icon.as_ref(),
            self.group.reaction.icon_alt.as_deref(),
            false,
        )
    }
}

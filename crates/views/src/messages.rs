//! Views for `reference/app/views/messages`, plus `MessagesHelper`,
//! `Messages::AttachmentPresentation` and the boost partials.

pub mod json;
// WS8b-r composer seam: published reusable facts and partial.
pub mod parts;
pub mod presentation;
pub mod reactions;
pub mod support;
pub mod composer;

use askama::Template;
use crate::helpers::filters;
use jiff::Timestamp;
use serde::Deserialize;

use crate::ViewContext;
use crate::fragment_cache;
use crate::helpers as h;
pub trait UserViewRendering {
    fn avatar(&self, ctx: &ViewContext, options: h::Attrs) -> h::Html;
}
impl UserViewRendering for UserView {

    fn avatar(&self, ctx: &ViewContext, options: h::Attrs) -> h::Html {
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

pub trait ReplySourceRendering {
    fn link(&self, ctx: &ViewContext) -> h::Html;
}
impl ReplySourceRendering for ReplySource {
    fn link(&self, ctx: &ViewContext) -> h::Html {
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

/// A message on its way into `messages/_message`: the fragment itself when the cache already
/// holds the full collection presentation key, else the view to render individually. The
/// presenter skips rich text, attachments, avatars and boosts on collection cache hits.
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
                    &message.components.github_cards_stamp,
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
pub trait MessageViewRendering {
    fn link_url(&self, ctx: &ViewContext) -> String;
    fn action_url(&self, ctx: &ViewContext) -> String;
    fn timestamp(&self, ctx: &ViewContext, style: &str, class: bool) -> h::Html;
    fn author_button(&self) -> h::Html;
    fn edited_iso(&self, ctx: &ViewContext) -> String;
}
impl MessageViewRendering for MessageView {

    fn link_url(&self, ctx: &ViewContext) -> String {
        format!("{}{}", ctx.base_url, self.at_path())
    }
    fn action_url(&self, ctx: &ViewContext) -> String {
        format!("{}{}", ctx.base_url, self.path())
    }

    fn timestamp(&self, ctx: &ViewContext, style: &str, class: bool) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
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

    fn author_button(&self) -> h::Html {
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

    fn edited_iso(&self, ctx: &ViewContext) -> String {
        self.details.edited_at.map(|at| ctx.time_zone.iso8601(at)).unwrap_or_default()
    }
}

pub trait BoostViewRendering {
    fn content_html(&self, ctx: &ViewContext) -> h::Html;
}
impl BoostViewRendering for BoostView {
    fn content_html(&self, ctx: &ViewContext) -> h::Html {
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

/// The original record-version cache API, retained for existing view consumers. HTTP
/// presenters use [`collection_fragment_key`]; individual pages/broadcasts use
/// [`uncached_message`] because Rails only caches collection rendering.
pub fn message(ctx: &ViewContext, message: &MessageView) -> String {
    fragment_cache::fetch(
        || {
            let key = message_fragment_key_in_zone(message.id, message.updated_at, &ctx.base_url, &message.components.github_cards_stamp, &ctx.time_zone);
            if message.components.event_views.is_empty() { key } else {
                use sha2::{Digest, Sha256};
                let facts = serde_json::to_vec(&message.components.event_views).expect("event facts serialize");
                format!("{key}/events/{}/{:x}", ctx.time_zone.name(), Sha256::digest(facts))
            }
        },
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

/// Rails renders individual messages without collection caching. Use this for standalone
/// pages and broadcasts, whose streaming state can change without touching the message row.
pub fn uncached_message(ctx: &ViewContext, message: &MessageView) -> String {
    MessagePartial { ctx, message }.render().expect("messages/_message renders")
}

pub fn uncached_message_html(ctx: &ViewContext, message: &MessageView) -> crate::helpers::Html {
    askama::filters::Safe(uncached_message(ctx, message))
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
        MessageItem::View(message) => std::borrow::Cow::Owned(uncached_message(ctx, message)),
    })
}

/// `messages/_message`'s fragment for this message version, if the current store holds it. The
/// key includes the message version and URL origin, so forged hosts cannot poison other origins.
pub fn cached_message_fragment(
    id: i64,
    updated_at: Timestamp,
    base_url: &str,
) -> Option<fragment_cache::Fragment> {
    cached_message_fragment_with_cards(id, updated_at, base_url, "")
}

pub fn cached_message_fragment_with_cards(id: i64, updated_at: Timestamp, base_url: &str, stamp: &str) -> Option<fragment_cache::Fragment> {
    cached_message_fragment_with_cards_in_zone(id, updated_at, base_url, stamp, &crate::time::Zone::utc())
}

/// The record-version API's cache lookup in the same zone as its renderer.
/// Existing callers without a zone retain the default UTC renderer contract.
pub fn cached_message_fragment_with_cards_in_zone(id: i64, updated_at: Timestamp, base_url: &str, stamp: &str, zone: &crate::time::Zone) -> Option<fragment_cache::Fragment> {
    fragment_cache::read(&message_fragment_key_in_zone(id, updated_at, base_url, stamp, zone))
}

fn message_fragment_key_in_zone(id: i64, updated_at: Timestamp, base_url: &str, stamp: &str, zone: &crate::time::Zone) -> String {
    format!("{}/zone/{}", message_fragment_key(id, updated_at, base_url, stamp), zone.name())
}

fn message_fragment_key(id: i64, updated_at: Timestamp, base_url: &str, stamp: &str) -> String {
    let key = format!(
        "views/messages/_message:{}/{}/presentation-v{}/{base_url}",
        message_digest(),
        fragment_cache::cache_key_with_version("messages", id, updated_at),
        fragment_cache::keys::PRESENTATION_CACHE_VERSION,
    );
    if stamp.is_empty() { key } else { format!("{key}/{stamp}") }
}

fn composite_fragment_key(key: &str, base_url: &str) -> String {
    format!("views/messages/_message:{}/{key}/{base_url}", message_digest())
}
pub fn cached_composite_fragment(key: &str, base_url: &str) -> Option<fragment_cache::Fragment> {
    fragment_cache::read(&composite_fragment_key(key, base_url))
}

/// Retained record-version API for existing view consumers. Rails' HTML partial
/// has no inner `cache boost`; the mounted message tree uses [`uncached_boost`].
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

/// Rails renders HTML boosts without a nested cache (only its JSON Jbuilder caches).
pub fn uncached_boost(ctx: &ViewContext, boost: &BoostView) -> crate::helpers::Html {
    askama::filters::Safe(BoostPartial { ctx, boost }.render().expect("messages/boosts/_boost renders"))
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
            include_str!("../templates/twitter/posts/_card.html"),
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
/// WS8b-r's room-shell entry point stays `Index { ctx, messages: &[MessageItem] }`.
/// Build items with `Presenter::messages`; the cache-aware `cached_message_item` renders each.
#[derive(Template)]
#[template(path = "messages/index.html")]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub messages: &'a [MessageItem],
}

/// The list slot in rooms/show, including its exact indentation. The divider is per viewer
/// and deliberately sits outside every cached message fragment.
#[derive(Template)]
#[template(path = "messages/room_index.html")]
pub struct RoomIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub messages: &'a [MessageItem],
    pub unread_index: Option<usize>,
    pub unread_count: i64,
}

impl RoomIndex<'_> {
    fn portion(&self, start: usize, end: usize) -> h::Html {
        h::raw(self.messages[start..end].iter().map(|message| cached_message_item(self.ctx, message).to_string()).collect::<String>())
    }
    fn divider(&self) -> h::Html {
        h::raw(UnreadDivider { unread_count: self.unread_count }.render().expect("unread divider renders"))
    }
}

#[derive(Template)]
#[template(path = "messages/_unread_divider.html")]
pub struct UnreadDivider {
    pub unread_count: i64,
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

/// Session-independent message parts replaced by the human edit endpoints.
#[derive(Template)]
#[template(path = "messages/_meta.html")]
pub struct MetaPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

#[derive(Template)]
#[template(path = "messages/_drive_attachments.html")]
pub struct DriveAttachmentsPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

#[derive(Template)]
#[template(path = "messages/_thread_indicator.html")]
pub struct ThreadIndicatorPartial<'a> {
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

/// The shared message actions menu rendered once per page by the layout.
#[derive(Template)]
#[template(path = "messages/_actions.html")]
pub struct ActionsMenu<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `messages/edit`.
#[derive(Template)]
#[template(path = "messages/edit.html")]
pub struct Edit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub edit: &'a EditView,
}







/// `messages/boosts/_boosts`.
#[derive(Template)]
#[template(path = "messages/boosts/_boosts.html")]
pub struct BoostsPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a MessageView,
}

/// The grouped replacement fragment shared by human, bot and MCP reaction actions and broadcasts.
#[derive(Template)]
#[template(path = "messages/boosts/_reactions.html")]
pub struct ReactionsPartial<'a> {
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
    cards_for_client_id(&message.client_message_id, prefix, class, indent, bodies)
}

/// Provider callbacks need only the message key, not the full message presentation.
pub fn cards_for_client_id(
    client_message_id: &str,
    prefix: &str,
    class: &str,
    indent: usize,
    bodies: &[String],
) -> h::Html {
    h::raw(format!(
        "{}<div id=\"{}\" class=\"{class}\">{}</div>\n",
        " ".repeat(indent),
        h::escape(&if prefix.is_empty() {
            format!("message_{client_message_id}")
        } else {
            format!("{prefix}_message_{client_message_id}")
        }),
        bodies.concat()
    ))
}

pub fn event_cards(ctx: &ViewContext, message: &MessageView) -> h::Html {
    if message.components.event_cards.is_empty() && message.components.event_views.is_empty() {
        return h::raw("");
    }
    let bodies = format!("{}{}", message.components.event_cards.concat(), crate::events::card_entries(&message.components.event_views, &message.id.to_string(), &ctx.time_zone).concat());
    h::raw(format!(
        "  <div id=\"{}\" class=\"event-cards\">\n{}  </div>\n",
        h::escape(&message.dom_id("event_cards")),
        bodies
    ))
}

/// The metadata-free, viewer-independent Drive chips, also used by the message composition.
#[derive(Template)]
#[template(path = "messages/_drive_attachments.html")]
pub struct DriveAttachments<'a> { pub message: &'a MessageView }

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
pub use campfire_presentation::messages::*;

/// Collection fragments carry the domain's full presentation key. The Index/MessageItem API
/// stays unchanged; presenters return its existing Fragment variant on both misses and hits.
pub fn collection_fragment_key(presentation_key: &str, base_url: &str) -> String {
    format!("views/messages/_message:{}/{presentation_key}/{base_url}", include_str!("messages/rails-template-digest.txt").trim_end())
}

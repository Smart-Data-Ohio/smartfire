pub mod composer;
pub mod support;
pub mod reactions;
pub mod parts;
pub mod json;

use campfire_routes as routes;
use jiff::Timestamp;
use serde::Deserialize;
use crate::helpers as h;
use support::{RubyNumber, epoch_ms, iso8601};

/// `EmojiHelper::REACTIONS` (see [`crate::helpers::emoji`]).
pub use crate::helpers::emoji::REACTIONS;

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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    Open,
    Closed,
    Direct,
    Voice,
    Stage,
    Board,
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
    /// Expanded Rails composite facts, built from preloads in the request's zone.
    pub cache_key: Option<String>,
    pub provider_github: Option<Vec<crate::message_providers::GithubEntry>>,
    pub provider_embeds: Option<Vec<crate::message_providers::EmbedEntry>>,
    pub provider_events: Option<Vec<crate::events::CardView>>,
    pub quote_references: Option<Vec<crate::message_links::Reference>>,
    pub github_cards: Vec<String>,
    pub github_cards_html: Option<String>,
    pub github_cards_stamp: String,
    pub twitter_cards: Vec<String>,
    pub twitter_posts: Vec<crate::twitter::Card>,
    pub event_cards: Vec<String>,
    pub event_views: Vec<crate::events::CardView>,
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
    Video { poster_url: Option<String> },
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

/// What `messages/edit` needs besides the message.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EditView {
    pub message: MessageView,
    /// Message#editable_markdown_source. The field name predates the Markdown edit form.
    pub editable_body_html: String,
}

/// `dom_id(room)` / `dom_id(room, prefix)`.
pub fn room_dom_id(kind: RoomKind, id: i64, prefix: &str) -> String {
    if prefix.is_empty() {
        format!("{}_{id}", kind.param_key())
    } else {
        format!("{prefix}_{}_{id}", kind.param_key())
    }
}
impl UserView {
    pub fn path(&self) -> String {
        routes::user(self.id)
    }
}

impl RoomKind {
    /// `Rooms::Open.model_name.param_key`, the stem of `dom_id(room)`.
    pub fn param_key(self) -> &'static str {
        match self {
            RoomKind::Open => "rooms_open",
            RoomKind::Closed => "rooms_closed",
            RoomKind::Direct => "rooms_direct",
            RoomKind::Voice => "rooms_voice",
            RoomKind::Stage => "rooms_stage",
            RoomKind::Board => "rooms_board",
        }
    }

    pub fn is_direct(self) -> bool {
        self == RoomKind::Direct
    }
}

impl ReplyPreview {
    pub fn id_string(&self) -> String {
        self.source
            .as_ref()
            .map(|source| source.id.to_string())
            .unwrap_or_default()
    }
}

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
}

impl EditView {
    /// DriveAttachment#url always uses this prefix; only ids are submitted by the edit form.
    pub fn drive_file_id<'a>(&self, url: &'a str) -> &'a str {
        url.strip_prefix("https://drive.google.com/open?id=").expect("DriveAttachment URL")
    }
}

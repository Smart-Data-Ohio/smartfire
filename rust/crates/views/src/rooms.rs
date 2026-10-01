//! Views for `reference/app/views/rooms`, plus `RoomsHelper`, `Rooms::InvolvementsHelper` and
//! the `MessagesHelper` tags the room screen uses.

mod header;
pub use header::{HeaderIdentity, header_identity};

use askama::Template;
use jiff::Timestamp;
use serde::Deserialize;

use crate::helpers as h;
use crate::layouts::Page;
use crate::messages::support::epoch_ms;
use crate::messages::{room_dom_id, MessageItem, RoomKind, UserView};
use crate::ViewContext;

/// `room_display_name(room, for_user:)`: a direct room is named after its other members
/// (`room.users.without(for_user).pluck(:name).to_sentence`), falling back to the user's own
/// name when they're alone in it.
pub fn room_display_name(name: Option<&str>, direct: bool, other_member_names: &[String], for_user_name: Option<&str>) -> String {
    if direct {
        let sentence = h::to_sentence(other_member_names, " and ");
        if sentence.trim().is_empty() { for_user_name.unwrap_or_default().to_string() } else { sentence }
    } else {
        name.unwrap_or_default().to_string()
    }
}

/// `mention_prompt_tag(room)`'s `src`: `autocompletable_users_path(room_id: room.id)`.
pub fn mention_prompt_src(room_id: i64) -> String {
    format!("{}?room_id={room_id}", campfire_routes::autocompletable_users())
}

/// A persisted room.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RoomView {
    #[serde(default = "default_involvement")]
    pub involvement: String,
    pub id: i64,
    pub kind: RoomKind,
    pub name: Option<String>,
    /// `room_display_name(room)` for `Current.user`.
    pub display_name: String,
    #[serde(default)]
    pub header: Option<HeaderIdentity>,
}

impl RoomView {
    pub fn is_stage(&self)->bool { self.header.as_ref().is_some_and(|h|h.param_key=="rooms_stage") }
    pub fn header_html(&self, ctx: &ViewContext) -> h::Html {
        let fallback;
        let header = match &self.header {
            Some(header) => header,
            None => {
                fallback = HeaderIdentity {
                    id: self.id, param_key: self.kind.param_key().into(), direct: self.is_direct(),
                    kind_label: if self.is_direct() { "Direct message" } else { "Channel" }.into(),
                    display_name: self.display_name.clone(), icon: None,
                };
                &fallback
            }
        };
        header_identity(ctx, header)
    }
    pub fn dom_id(&self, prefix: &str) -> String {
        room_dom_id(self.kind, self.id, prefix)
    }

    pub fn is_direct(&self) -> bool {
        self.kind.is_direct()
    }

    /// `edit_polymorphic_path(room)`: `/rooms/opens/1/edit` and so on.
    pub fn edit_path(&self) -> String {
        match self.kind {
            RoomKind::Open => campfire_routes::edit_rooms_open(self.id),
            RoomKind::Closed => campfire_routes::edit_rooms_closed(self.id),
            RoomKind::Direct => campfire_routes::edit_rooms_direct(self.id),
        }
    }

    /// "Ping" for direct rooms, "room" otherwise.
    pub fn noun(&self) -> &'static str {
        if self.is_direct() { "Ping" } else { "room" }
    }
}

/// What `rooms/show` shows.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShowView {
    #[serde(default)]
    pub shell: ShellComponents,
    #[serde(default)]
    pub scroll_to_unread_divider: Option<bool>,
    #[serde(default)]
    pub jump_to_unread_url: Option<String>,
    #[serde(default)]
    pub unread_divider_message_id: Option<i64>,
    #[serde(default)]
    pub unread_count:i64,
    /// Position resolved from domain message IDs before building cached message fragments.
    #[serde(default)]
    pub unread_divider_index:Option<usize>,
    pub room: RoomView,
    /// `room.updated_at`, the refresh controller's `loaded_at`.
    pub updated_at: Timestamp,
    /// `Current.user`, for the client-side message template.
    pub user: UserView,
    pub messages: Vec<MessageItem>,
    /// `@room == Room.original && !@room.messages.paged?` (`rooms/show/_invitation`).
    pub invitation: bool,
    /// `Current.account.join_code`, for the invitation's join link.
    #[serde(default)]
    pub join_code: String,
    /// `Turbo::StreamsChannel.signed_stream_name([room, :messages])`.
    pub messages_stream_name: String,
    /// Other active human DM members, including currently off members so each live stream is mounted.
    #[serde(default)]
    pub ooo_notice_members: Vec<crate::users::statuses::OooNoticeMember>,
}

/// `rooms/show`.
#[derive(Template)]
#[template(path = "rooms/show.html", blocks = ["head", "content", "nav", "member_panel", "thread_panel", "footer"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub show: &'a ShowView,
}

impl Page for Show<'_> {
    fn has_sidebar(&self)->bool { true }
    fn page_title(&self) -> Option<String> {
        Some(self.show.room.display_name.clone())
    }

    fn body_class(&self) -> Option<&str> {
        Some("sidebar room-workspace")
    }
}

impl Show<'_> {
    fn multi_select_bar(&self)->h::Html { h::raw(crate::shared::MultiSelectBar{exit_button:true}.render().expect("member selection bar renders")) }

    fn jump_to_unread(&self,url:Option<&str>)->h::Html {
        let label=h::image_tag(self.ctx,"arrow-up.svg",h::attrs().aria_hidden().size(20)).0+&h::content_tag_text("span",h::attrs(),"Jump to unread").0;
        let attrs=h::attrs().id("jump-to-unread").class("message-area__jump-to-unread btn");
        match url { Some(url)=>h::link_to(url,attrs,&label),None=>h::content_tag("button",attrs.data("action","messages#jumpToUnread").attr("hidden",true),&label) }
    }
    fn loaded_at(&self) -> i64 {
        epoch_ms(self.show.updated_at)
    }
    fn ooo_notices(&self) -> h::Html {
        h::raw(
            crate::users::statuses::OooNotices {
                direct: self.show.room.is_direct(),
                members: &self.show.ooo_notice_members,
            }
            .render()
            .expect("OOO notices"),
        )
    }
}

/// `Membership#involvement`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct InvolvementView {
    pub room_id: i64,
    pub kind: RoomKind,
    /// "mentions", "everything", "nothing" or "invisible".
    pub involvement: String,
}

/// `rooms/involvements/show`.
#[derive(Template)]
#[template(path = "rooms/involvements/show.html")]
pub struct InvolvementShow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub involvement: &'a InvolvementView,
}

impl InvolvementShow<'_> {
    fn button(&self) -> h::Html {
        let room = h::InvolvementRoom {
            id: self.involvement.room_id,
            param_key: self.involvement.kind.param_key(),
            direct: self.involvement.kind.is_direct(),
        };
        h::button_to_change_involvement(self.ctx, &room, &self.involvement.involvement)
    }
}

/// What `rooms/refreshes/show` streams: messages created and updated since the client loaded.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RefreshView {
    pub room_id: i64,
    pub room_kind: RoomKind,
    pub new_messages: Vec<MessageItem>,
    pub updated_messages: Vec<MessageItem>,
}

/// `rooms/refreshes/show.turbo_stream`.
#[derive(Template)]
#[template(path = "rooms/refreshes/show.turbo_stream.html")]
pub struct RefreshShow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub refresh: &'a RefreshView,
}

/// The room being created or edited by the open and closed room forms. `id` is `None` for a
/// new record.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct FormRoom {
    pub id: Option<i64>,
    pub name: Option<String>,
}

/// `rooms/opens/{new,edit}`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OpenFormView {
    pub room: FormRoom,
    /// `Current.user.can_administer?(room)`: administrators, the creator, or a new room.
    pub can_administer: bool,
    /// `User.active.ordered`.
    pub users: Vec<UserView>,
}

/// `rooms/closeds/{new,edit}`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ClosedFormView {
    pub room: FormRoom,
    pub can_administer: bool,
    pub current_user_id: i64,
    /// Active users with access (none for a new room).
    pub selected_users: Vec<UserView>,
    /// The other active users.
    pub unselected_users: Vec<UserView>,
}

/// `rooms/opens/new`.
#[derive(Template)]
#[template(path = "rooms/opens/new.html", blocks = ["head", "content"])]
pub struct OpensNew<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a OpenFormView,
}

/// `rooms/opens/edit`.
#[derive(Template)]
#[template(path = "rooms/opens/edit.html", blocks = ["head", "content"])]
pub struct OpensEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a OpenFormView,
    pub github: crate::github::subscriptions::Section,
}

/// `rooms/closeds/new`.
#[derive(Template)]
#[template(path = "rooms/closeds/new.html", blocks = ["head", "content"])]
pub struct ClosedsNew<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a ClosedFormView,
}

/// `rooms/closeds/edit`.
#[derive(Template)]
#[template(path = "rooms/closeds/edit.html", blocks = ["head", "content"])]
pub struct ClosedsEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a ClosedFormView,
    pub github: crate::github::subscriptions::Section,
}

impl Page for OpensNew<'_> {
    fn page_title(&self) -> Option<String> {
        Some("New chat room".into())
    }
}

impl Page for ClosedsNew<'_> {
    fn page_title(&self) -> Option<String> {
        Some("New chat room".into())
    }
}

impl Page for OpensEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Edit settings for {}", self.form.room.name.as_deref().unwrap_or_default()))
    }
}

impl Page for ClosedsEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Edit settings for {}", self.form.room.name.as_deref().unwrap_or_default()))
    }
}

impl FormRoom {
    /// `form_with model: room`'s action for an open or closed room.
    fn action(&self, kind: RoomKind) -> String {
        match (self.id, kind) {
            (Some(id), RoomKind::Open) => campfire_routes::rooms_open(id),
            (Some(id), _) => campfire_routes::rooms_closed(id),
            (None, RoomKind::Open) => campfire_routes::rooms_opens(),
            (None, _) => campfire_routes::rooms_closeds(),
        }
    }

    fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }
}

/// `rooms/directs/new`.
#[derive(Template)]
#[template(path = "rooms/directs/new.html", blocks = ["head", "content"])]
pub struct DirectsNew<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub people: Vec<crate::users::Person>,
}
impl DirectsNew<'_> {
    fn multi_select(&self) -> askama::Result<h::Html> {
        Ok(h::raw(crate::shared::MultiSelectBar { exit_button: false }.render()?))
    }
}

impl Page for DirectsNew<'_> {}

/// `rooms/directs/edit`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DirectEditView {
    pub room_id: i64,
    /// `room_display_name(@room)` for `Current.user`.
    pub display_name: String,
    /// `@room.users.many? ? @room.users.without(Current.user) : @room.users`.
    pub users: Vec<UserView>,
}

#[derive(Template)]
#[template(path = "rooms/directs/edit.html", blocks = ["head", "content"])]
pub struct DirectsEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub edit: &'a DirectEditView,
}

impl Page for DirectsEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Edit settings for {}", self.edit.display_name))
    }
}

impl OpensEdit<'_> {
    fn room_id(&self) -> i64 {
        self.form.room.id.unwrap_or_default()
    }
}

impl ClosedsEdit<'_> {
    fn room_id(&self) -> i64 {
        self.form.room.id.unwrap_or_default()
    }
}

/// `button_to_delete_room(room)`.
pub fn button_to_delete_room(ctx: &ViewContext, room_id: i64, display_name: &str) -> h::Html {
    let url = ctx.url(&campfire_routes::room(room_id));
    let content = format!(
        r#"<img aria-hidden="true" src="{}" width="20" height="20" /><span class="overflow-ellipsis">{}</span>"#,
        h::escape(&ctx.asset("trash.svg")),
        h::escape(display_name)
    );
    let options = h::attrs()
        .method("delete")
        .class("btn btn--negative max-width")
        .aria("label", format!("Delete {display_name}"))
        .data("turbo_confirm", "Are you sure you want to delete this room and all messages in it? This can’t be undone.");
    h::button_to(&url, options, &content)
}

/// `rooms/layouts/_form`, the form wrapped around the open and closed room forms' fields.
#[derive(Template)]
#[template(path = "rooms/layouts/_form.html")]
pub struct FormLayout<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room: &'a FormRoom,
    pub can_administer: bool,
    pub kind: RoomKind,
    pub content: String,
}

/// Block helpers for the room templates: A's shared ones plus `rooms/layouts/_form`.
mod filters {
    pub use crate::helpers::filters::*;

    use std::fmt::Display;

    use askama::{Template, Values};

    use super::{FormLayout, FormRoom, RoomKind};
    use crate::helpers::Html;
    use crate::ViewContext;

    /// `render layout: "rooms/layouts/form", locals: { room: } do ... end`.
    pub fn room_form(
        content: impl Display,
        _: &dyn Values,
        ctx: &ViewContext,
        room: &FormRoom,
        can_administer: &bool,
        kind: RoomKind,
    ) -> askama::Result<Html> {
        let layout = FormLayout { ctx, room, can_administer: *can_administer, kind, content: content.to_string() };
        Ok(Html::from(askama::filters::Safe(layout.render()?)))
    }
}

fn default_involvement()->String { "mentions".into() }
/// Trusted output supplied by WS8b-m (and WS13/WS17 for configured header/OOO children).
/// These fragments are page inputs, never a shared fragment cache or broadcast payload.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ShellComponents {
    pub pins_panel:String,
    pub thread_panel:String,
    pub huddle_header:String,
    pub ooo_notices:String,
    pub poll_builder:String,
    pub message_template:Option<String>,
    pub composer:Option<String>,
    pub message_list:Option<String>,
}
/// Stable WS8b-m entry point. Until its list adapter lands, an empty collection emits zero
/// bytes, exactly as Rails' `render partial: "messages/message", collection: []` does.
pub fn room_message_list(ctx:&ViewContext, show:&ShowView)->h::Html {
    if let Some(html) = &show.shell.message_list { return h::raw(html.clone()); }
    if show.messages.is_empty() { return h::raw(String::new()); }
    // The divider belongs to this viewer's list, outside all shared message-cache entries.
    let mut html=String::new();
    for (index,message) in show.messages.iter().enumerate() {
        let divider_here=show.unread_divider_index==Some(index);
        if divider_here {
            html.push_str("\n      ");
            html.push_str(&UnreadDivider{unread_count:show.unread_count}.render().expect("unread divider renders"));
        }
        html.push_str(if divider_here {"\n      "} else {"\n    "});
        html.push_str(&crate::messages::cached_message_item(ctx,message).to_string());
    }
    html.push_str("\n  ");
    h::raw(html)
}
#[derive(Template)]
#[template(path="messages/_unread_divider.html")]
struct UnreadDivider {unread_count:i64}

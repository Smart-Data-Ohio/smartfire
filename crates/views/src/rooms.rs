//! Views for `reference/app/views/rooms`, plus `RoomsHelper`, `Rooms::InvolvementsHelper` and
//! the `MessagesHelper` tags the room screen uses.

pub mod panels;
pub mod calls;
pub mod shell;
pub mod composition;
pub mod composition_page;
pub mod navigation;
pub mod edit_sections;
pub mod boards;
pub mod board_automations;

pub mod header;
pub use header::{HeaderIdentity, header_identity};

use askama::Template;
use jiff::Timestamp;
use serde::Deserialize;

use crate::ViewContext;
use crate::helpers as h;
use crate::layouts::Page;
use crate::messages::support::epoch_ms;
use crate::messages::{MessageItem, RoomKind, UserView};
pub trait RoomViewRendering {
    fn header_html(&self, ctx: &ViewContext) -> h::Html;
}
impl RoomViewRendering for RoomView {
    fn header_html(&self, ctx: &ViewContext) -> h::Html {
        let fallback;
        let header = match &self.header {
            Some(header) => header,
            None => {
                fallback = HeaderIdentity {
                    id: self.id,
                    param_key: self.kind.param_key().into(),
                    direct: self.is_direct(),
                    kind_label: if self.is_direct() {
                        "Direct message"
                    } else {
                        "Channel"
                    }
                    .into(),
                    display_name: self.display_name.clone(),
                    icon: None,
                };
                &fallback
            }
        };
        header_identity(ctx, header)
    }
}

/// `rooms/show`.
#[derive(Template)]
#[template(path = "rooms/show.html", blocks = ["head", "content", "nav", "member_panel", "thread_panel", "footer"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub show: &'a ShowView,
}

impl Page for Show<'_> {
    fn has_sidebar(&self) -> bool {
        true
    }
    fn page_title(&self) -> Option<String> {
        Some(self.show.room.display_name.clone())
    }

    fn body_class(&self) -> Option<&str> {
        Some("sidebar room-workspace")
    }
}

impl Show<'_> {
    fn member_panel(&self) -> askama::Result<h::Html> {
        Ok(h::raw(MemberPanel { ctx: self.ctx, room_id: self.show.room.id }.render()?))
    }

    fn jump_to_unread(&self, url: Option<&str>) -> h::Html {
        let label = h::image_tag(self.ctx, "arrow-up.svg", h::attrs().aria_hidden().size(20)).0
            + &h::content_tag_text("span", h::attrs(), "Jump to unread").0;
        let attrs = h::attrs()
            .id("jump-to-unread")
            .class("message-area__jump-to-unread btn");
        match url {
            Some(url) => h::link_to(url, attrs, &label),
            None => h::content_tag(
                "button",
                attrs
                    .data("action", "messages#jumpToUnread")
                    .attr("hidden", true),
                &label,
            ),
        }
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

/// The member-panel control in `rooms/show/_nav`.
#[derive(Template)]
#[template(path = "rooms/show/_member_panel_toggle.html")]
pub struct MemberPanelToggle<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// The lazy member panel. Domain data comes from Rooms::MembersController's JSON;
/// this partial holds only URLs and a request-bound selection form.
#[derive(Template)]
#[template(path = "rooms/show/_member_panel.html")]
pub struct MemberPanel<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room_id: i64,
}

impl MemberPanel<'_> {
    fn members_path(&self) -> String {
        campfire_routes::ROOM_MEMBERS.path_with(&[&self.room_id], Some("json"), &[])
    }

    fn selection_bar(&self) -> askama::Result<h::Html> {
        Ok(h::raw(crate::shared::MultiSelectBar { exit_button: true }.render()?))
    }
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

/// `rooms/refreshes/show.turbo_stream`.
#[derive(Template)]
#[template(path = "rooms/refreshes/show.turbo_stream.html")]
pub struct RefreshShow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub refresh: &'a RefreshView,
}

impl RefreshShow<'_> {
    fn pins_count(&self, list: &crate::pins::List) -> h::Html {
        h::raw(
            crate::pins::CountPartial {
                room_id: list.room_id,
                room_param_key: list.room_param_key.clone(),
                count: list.pins.len() as i64,
            }
            .render()
            .expect("owner pin count renders"),
        )
    }
    fn pins_list(&self, list: &crate::pins::List) -> h::Html {
        h::raw(
            crate::pins::ListPartial {
                ctx: self.ctx,
                list,
            }
            .render()
            .expect("owner pin list renders"),
        )
    }
}

#[derive(Template)]
#[template(path = "rooms/inbound_email_addresses/_section.html")]
pub struct InboundEmailSection<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub email: &'a InboundEmailView,
    pub can_administer: bool,
}
impl InboundEmailSection<'_> {
    fn button(&self, rotate: bool) -> h::Html {
        let options = h::attrs().class(if rotate {
            "btn btn--negative txt-small"
        } else {
            "btn txt-small"
        });
        let form_options = if rotate {
            h::attrs().data(
                "turbo_confirm",
                "Rotate this room's email address? The old address stops working.",
            )
        } else {
            h::attrs()
        };
        h::button_to_form(
            &campfire_routes::room_inbound_email_address(self.email.id),
            options,
            form_options,
            if rotate {
                "Rotate address"
            } else {
                "Create email address"
            },
        )
    }
}

#[derive(Template)]
#[template(path = "rooms/opens/_user.html")]
struct OpenFormUser<'a> {
    ctx: &'a ViewContext<'a>,
    form: &'a OpenFormView,
    user: &'a UserView,
}
#[derive(Template)]
#[template(path = "rooms/closeds/_user.html")]
struct ClosedFormUser<'a> {
    ctx: &'a ViewContext<'a>,
    form: &'a ClosedFormView,
    user: &'a UserView,
    selected: bool,
}
pub trait OpenFormViewRendering {
    fn user_list(&self, ctx: &ViewContext) -> h::Html;
}
impl OpenFormViewRendering for OpenFormView {
    /// Rails collection rendering indents the first partial only, then concatenates bytes.
    fn user_list(&self, ctx: &ViewContext) -> h::Html {
        h::raw(
            self.users
                .iter()
                .map(|user| {
                    OpenFormUser {
                        ctx,
                        form: self,
                        user,
                    }
                    .render()
                    .expect("open form user renders")
                })
                .collect::<String>(),
        )
    }
}

pub trait ClosedFormViewRendering {
    fn selected_list(&self, ctx: &ViewContext) -> h::Html;
    fn unselected_list(&self, ctx: &ViewContext) -> h::Html;
    fn user_list(&self, ctx: &ViewContext, users: &[UserView], selected: bool) -> h::Html;
}
impl ClosedFormViewRendering for ClosedFormView {
    fn selected_list(&self, ctx: &ViewContext) -> h::Html {
        self.user_list(ctx, &self.selected_users, true)
    }
    fn unselected_list(&self, ctx: &ViewContext) -> h::Html {
        self.user_list(ctx, &self.unselected_users, false)
    }
    fn user_list(&self, ctx: &ViewContext, users: &[UserView], selected: bool) -> h::Html {
        h::raw(
            users
                .iter()
                .map(|user| {
                    ClosedFormUser {
                        ctx,
                        form: self,
                        user,
                        selected,
                    }
                    .render()
                    .expect("closed form user renders")
                })
                .collect::<String>(),
        )
    }
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
        Some(format!(
            "Edit settings for {}",
            self.form.room.name.as_deref().unwrap_or_default()
        ))
    }
}

impl Page for ClosedsEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!(
            "Edit settings for {}",
            self.form.room.name.as_deref().unwrap_or_default()
        ))
    }
}
pub trait FormRoomRendering {
    fn inbound_email_html(&self, ctx: &ViewContext, can_administer: &bool) -> h::Html;
}
impl FormRoomRendering for FormRoom {
    fn inbound_email_html(&self, ctx: &ViewContext, can_administer: &bool) -> h::Html {
        self.inbound_email
            .as_ref()
            .map(|email| {
                h::raw(
                    InboundEmailSection {
                        ctx,
                        email,
                        can_administer: *can_administer,
                    }
                    .render()
                    .expect("inbound email section renders"),
                )
            })
            .unwrap_or_else(h::empty)
    }
}

/// `rooms/directs/new`.
#[derive(Template)]
#[template(path = "rooms/directs/new.html", blocks = ["head", "content"])]
pub struct DirectsNew<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub users: &'a [DirectPickerUser],
}
impl DirectsNew<'_> {
    fn multi_select_bar(&self) -> h::Html {
        h::raw(
            crate::shared::MultiSelectBar { exit_button: false }
                .render()
                .expect("picker selection bar renders"),
        )
    }
}

impl Page for DirectsNew<'_> {}

#[derive(Template)]
#[template(path = "rooms/directs/edit.html", blocks = ["head", "content"])]
pub struct DirectsEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub edit: &'a DirectEditView,
}

impl DirectsEdit<'_> {
    fn leave_label(&self) -> &str {
        if self.edit.group_capable {
            "Leave group"
        } else {
            "Leave ping"
        }
    }
    fn candidate_image(&self, user: &UserView) -> h::Html {
        match &user.icon {
            Some(icon) => h::icon_avatar_tag(self.ctx, Some(icon), 24, h::attrs()),
            None => h::image_tag(self.ctx, &user.avatar_url, h::attrs().size(24)),
        }
    }
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

impl FormLayout<'_> {
    fn errors(&self) -> String {
        h::to_sentence(&self.room.errors, " and ")
    }
    fn icon_field(&self) -> h::Html {
        let form = h::form_with(self.room.action(self.kind))
            .model(self.kind.param_key())
            .field_errors(self.room.error_attributes.clone());
        h::raw(
            crate::shared::IconField {
                ctx: self.ctx,
                form: &form,
                scope: "room",
                icon_name: self.room.icon_name.as_deref(),
                icon: self.room.icon.as_ref(),
            }
            .render()
            .expect("room icon field renders"),
        )
    }
}

/// Block helpers for the room templates: A's shared ones plus `rooms/layouts/_form`.
mod filters {
    pub use crate::helpers::filters::*;

    use std::fmt::Display;

    use askama::{Template, Values};

    use super::{FormLayout, FormRoom, RoomKind};
    use crate::ViewContext;
    use crate::helpers::Html;

    /// `render layout: "rooms/layouts/form", locals: { room: } do ... end`.
    pub fn room_form(
        content: impl Display,
        _: &dyn Values,
        ctx: &ViewContext,
        room: &FormRoom,
        can_administer: &bool,
        kind: RoomKind,
    ) -> askama::Result<Html> {
        let layout = FormLayout {
            ctx,
            room,
            can_administer: *can_administer,
            kind,
            content: content.to_string(),
        };
        Ok(Html::from(askama::filters::Safe(layout.render()?)))
    }
}
/// Stable WS8b-m entry point. Until its list adapter lands, an empty collection emits zero
/// bytes, exactly as Rails' `render partial: "messages/message", collection: []` does.
pub fn room_message_list(ctx: &ViewContext, show: &ShowView) -> h::Html {
    if let Some(html) = &show.shell.message_list {
        // The mounted list includes the empty invitation expression's whitespace.
        // A rendered invitation already supplies that boundary (rooms/show.html.erb).
        if show.invitation && !show.messages.is_empty() {
            return h::raw(html.strip_prefix("\n    ").unwrap_or(html));
        }
        return h::raw(html.clone());
    }
    if show.messages.is_empty() {
        return h::raw(String::new());
    }
    // The divider belongs to this viewer's list, outside all shared message-cache entries.
    let mut html = String::new();
    for (index, message) in show.messages.iter().enumerate() {
        let divider_here = show.unread_divider_index == Some(index);
        if divider_here {
            html.push_str("\n      ");
            html.push_str(
                &UnreadDivider {
                    unread_count: show.unread_count,
                }
                .render()
                .expect("unread divider renders"),
            );
        }
        html.push_str(if divider_here { "\n      " } else { "\n    " });
        html.push_str(&crate::messages::cached_message_item(ctx, message).to_string());
    }
    html.push_str("\n  ");
    h::raw(html)
}
#[derive(Template)]
#[template(path = "messages/_unread_divider.html")]
struct UnreadDivider {
    unread_count: i64,
}

/// `rooms/join`: alive open-room preview for a nonmember.
#[derive(Template)]
#[template(path = "rooms/join.html", blocks = ["head", "content", "nav"])]
pub struct JoinPage<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub id: i64,
    pub name: &'a str,
}
impl Page for JoinPage<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Join #{}", self.name))
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar room-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl JoinPage<'_> {
    fn join_button(&self) -> h::Html {
        h::button_to(
            &campfire_routes::join_room(self.id),
            h::attrs().class("btn btn--reversed"),
            "Join channel",
        )
    }
}
pub use campfire_presentation::rooms::*;

use crate::rendering::*;

/// What `rooms/show` shows.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShowView {
    #[serde(default)]
    pub navigation: Option<navigation::Navigation>,
    #[serde(default)]
    pub shell: ShellComponents,
    #[serde(default)]
    pub scroll_to_unread_divider: Option<bool>,
    #[serde(default)]
    pub jump_to_unread_url: Option<String>,
    #[serde(default)]
    pub unread_divider_message_id: Option<i64>,
    #[serde(default)]
    pub unread_count: i64,
    /// Position resolved from domain message IDs before building cached message fragments.
    #[serde(default)]
    pub unread_divider_index: Option<usize>,
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

/// What `rooms/refreshes/show` streams: messages created and updated since the client loaded.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RefreshView {
    pub room_id: i64,
    pub room_kind: RoomKind,
    pub new_messages: Vec<MessageItem>,
    pub updated_messages: Vec<MessageItem>,
    #[serde(default)]
    pub pins: Option<crate::pins::List>,
}

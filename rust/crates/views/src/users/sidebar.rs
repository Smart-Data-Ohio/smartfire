//! Sidebar row data from Users::SidebarHelper. Permission facts belong to the row's viewer.
use super::UserSummary;
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RoomMenu {
    pub menu_categorizable: bool,
    pub menu_favorited: bool,
    pub menu_favorite_position: Option<i64>,
    pub menu_muted: bool,
    pub menu_default_involvement: String,
    pub menu_category_id: Option<i64>,
    pub menu_can_delete: bool,
    pub menu_can_leave: bool,
    pub menu_leave_url: String,
    pub menu_open_room: bool,
    pub menu_direct_room: bool,
    pub menu_room_label: Option<String>,
}
impl RoomMenu {
    pub fn data(&self) -> h::Attrs {
        h::attrs()
            .data("menu_categorizable", self.menu_categorizable)
            .data("menu_favorited", self.menu_favorited)
            .attr_opt("data-menu-favorite-position", self.menu_favorite_position)
            .data("menu_muted", self.menu_muted)
            .data(
                "menu_default_involvement",
                self.menu_default_involvement.as_str(),
            )
            .attr_opt("data-menu-category-id", self.menu_category_id)
            .data("menu_can_delete", self.menu_can_delete)
            .data("menu_can_leave", self.menu_can_leave)
            .data("menu_leave_url", self.menu_leave_url.as_str())
            .data("menu_open_room", self.menu_open_room)
            .data("menu_direct_room", self.menu_direct_room)
            .attr_opt("data-menu-room-label", self.menu_room_label.as_deref())
    }
}

#[derive(Clone, Debug)]
pub struct SidebarDirect {
    pub room_id: i64,
    pub unread: bool,
    pub updated_at_epoch: String,
    /// Other members in preloaded association order; self if there are none.
    pub members: Vec<UserSummary>,
    pub label: String,
    pub menu: RoomMenu,
    pub viewer_administrator: bool,
    /// WS13 supplies its already rendered trusted partial and the collection-key participant IDs.
    pub huddle_participants: Option<String>,
    pub participant_ids: Option<Vec<i64>>,
    pub membership_id: i64,
    pub membership_updated_at: jiff::Timestamp,
}
impl SidebarDirect {
    fn row_attrs(&self) -> h::Attrs {
        h::attrs()
            .class(self.class_names())
            .id(h::dom_id("rooms_direct", self.room_id, Some("list")))
            .data("rooms_list_target", "room")
            .data("room_id", self.room_id)
            .data("badge_dot_target", "unread")
            .data("sorted_list_target", "item")
            .data("sorted_list_number", self.updated_at_epoch.as_str())
    }
    fn link_attrs(&self) -> h::Attrs {
        h::attrs()
            .class("direct__link")
            .data("room_id", self.room_id)
            .merge(self.menu.data())
    }
    fn class_names(&self) -> String {
        format!(
            "sidebar-item direct{}{}",
            if self.unread { " unread" } else { "" },
            if self.menu.menu_muted { " muted" } else { "" }
        )
    }
}
#[derive(Clone, Debug)]
pub enum SidebarDirectItem {
    Fragment(crate::fragment_cache::Fragment),
    View(Box<SidebarDirect>),
}
impl From<SidebarDirect> for SidebarDirectItem {
    fn from(value: SidebarDirect) -> Self {
        Self::View(Box::new(value))
    }
}
/// Single-member broadcasts do not use the page collection cache in Rails.
pub fn direct_room(ctx: &ViewContext, membership: &SidebarDirect) -> String {
    SidebarDirectPartial {
        ctx,
        membership: membership.clone(),
    }
    .render()
    .expect("direct row renders")
}
pub fn cached_direct_room<'a>(
    ctx: &ViewContext,
    item: &'a SidebarDirectItem,
) -> askama::filters::Safe<std::borrow::Cow<'a, str>> {
    askama::filters::Safe(match item {
        SidebarDirectItem::Fragment(html) => std::borrow::Cow::Borrowed(html.as_str()),
        SidebarDirectItem::View(row) => std::borrow::Cow::Owned(crate::fragment_cache::fetch(
            || direct_room_fragment_key(row),
            || direct_room(ctx, row),
        )),
    })
}
pub fn cached_direct_room_fragment(row: &SidebarDirect) -> Option<crate::fragment_cache::Fragment> {
    crate::fragment_cache::read(&direct_room_fragment_key(row))
}
fn direct_room_fragment_key(row: &SidebarDirect) -> String {
    let record = crate::fragment_cache::cache_key_with_version(
        "memberships",
        row.membership_id,
        row.membership_updated_at,
    );
    let key = crate::fragment_cache::keys::sidebar_membership(
        &record,
        row.participant_ids.as_deref(),
        row.viewer_administrator,
    );
    crate::fragment_cache::keys::fragment(
        "users/sidebars/rooms/_direct",
        direct_room_digest(),
        &key,
        &crate::time::Zone::utc(),
    )
}
fn direct_room_digest() -> &'static str {
    static DIGEST: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        crate::fragment_cache::digest(&[include_str!(
            "../../templates/users/sidebars/rooms/_direct.html"
        )])
    });
    &DIGEST
}
#[derive(Clone, Debug)]
pub struct SidebarRoom {
    pub id: i64,
    pub param_key: String,
    pub name: String,
    pub unread: bool,
    pub menu: RoomMenu,
    pub icon: Option<h::AvatarIcon>,
    /// Rendered by WS13; None means huddles are unconfigured.
    pub huddle_participants: Option<String>,
}
impl SidebarRoom {
    fn venue_children(&self) -> h::Html {
        // WS13 can replace the complete children via huddle_participants. This default is
        // Rails' empty venue mount, which exists even when Huddle.configured? is false.
        self.huddle_participants
            .as_ref()
            .map(|html| h::raw(html.clone()))
            .unwrap_or_else(|| {
                h::raw(
                    EmptyVenueChildren {
                        id: self.id,
                        param_key: &self.param_key,
                        stage: self.param_key == "rooms_stage",
                        label: if self.param_key == "rooms_stage" {
                            "on stage"
                        } else {
                            "in voice"
                        },
                    }
                    .render()
                    .expect("empty venue mount renders"),
                )
            })
    }
    fn link_attrs(&self) -> h::Attrs {
        h::attrs()
            .id(h::dom_id(&self.param_key, self.id, Some("list")))
            .data("sorted_list_name", self.name.as_str())
            .merge(self.menu.data())
            .class(self.class_names())
    }
    fn class_names(&self) -> String {
        format!(
            "sidebar-item room btn{}{}{}",
            match self.param_key.as_str() {
                "rooms_board" => " board-room",
                "rooms_voice" => " voice-room",
                "rooms_stage" => " voice-room stage-room",
                _ => "",
            },
            if self.unread { " unread" } else { "" },
            if self.menu.menu_muted { " muted" } else { "" }
        )
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/rooms/_direct.html")]
pub struct SidebarDirectPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub membership: SidebarDirect,
}
#[derive(Template)]
#[template(path = "users/sidebars/rooms/_shared.html")]
pub struct SidebarSharedPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room: SidebarRoom,
}

/// The outer sidebar owns section layout; WS12/WS13 supply configured feature children.
#[derive(Clone, Debug)]
pub enum SidebarItem {
    Direct(Box<SidebarDirect>),
    Room(Box<SidebarRoom>),
}
impl SidebarItem {
    pub fn render(&self, ctx: &ViewContext) -> h::Html {
        h::raw(match self {
            Self::Direct(row) => direct_room(ctx, row),
            Self::Room(room) => SidebarSharedPartial {
                ctx,
                room: *room.clone(),
            }
            .render()
            .expect("sidebar row renders"),
        })
    }
}
#[derive(Clone, Debug)]
pub struct SidebarCategory {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
    pub rooms: Vec<SidebarRoom>,
}
impl SidebarCategory {
    fn path(&self) -> String {
        format!("/room_categories/{}", self.id)
    }
    fn toggle_label(&self) -> &'static str {
        if self.collapsed { "Expand" } else { "Collapse" }
    }
    fn collapse_button(&self, ctx: &ViewContext) -> h::Html {
        // button_to serializes nested params after its token, sorted by field name.
        let image = h::image_tag(
            ctx,
            "disclosure.svg",
            h::attrs().size(16).aria_hidden().attr_opt(
                "class",
                self.collapsed
                    .then_some("room-category__disclosure--collapsed"),
            ),
        );
        let button = h::button_to_form(
            &self.path(),
            h::attrs()
                .method("patch")
                .class("btn sidebar-section__collapse")
                .aria("label", format!("{} {}", self.toggle_label(), self.name))
                .attr("title", self.toggle_label()),
            h::attrs().data("turbo_frame", "user_sidebar"),
            &format!("\n        {}\n", image),
        );
        let field = h::legacy_tag(
            "input",
            h::attrs()
                .type_("hidden")
                .name("room_category[collapsed]")
                .value(if self.collapsed { "false" } else { "true" }),
        );
        h::raw(
            button
                .0
                .strip_suffix("</form>")
                .expect("form closing tag")
                .to_string()
                + &field.0
                + "</form>",
        )
    }
}
#[derive(Template)]
#[template(path="users/sidebars/show.html",blocks=["head","content"])]
pub struct SidebarShow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub current_user: UserSummary,
    pub rooms_stream: String,
    pub user_rooms_stream: String,
    pub favorite_memberships: Vec<SidebarItem>,
    pub categories: Vec<SidebarCategory>,
    pub direct_memberships: Vec<SidebarDirectItem>,
    pub direct_placeholder_users: Vec<UserSummary>,
    pub other_memberships: Vec<SidebarRoom>,
    pub voice_memberships: Vec<SidebarRoom>,
    pub can_create_rooms: bool,
}
impl Page for SidebarShow<'_> {}

impl SidebarShow<'_> {
    fn workspace_initial(&self) -> String {
        self.ctx
            .account
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
    }
}

#[derive(Template)]
#[template(path = "users/sidebars/rooms/_empty_venue_children.html")]
struct EmptyVenueChildren<'a> {
    id: i64,
    param_key: &'a str,
    stage: bool,
    label: &'a str,
}

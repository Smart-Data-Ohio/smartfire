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
    fn link_attrs(&self) -> h::Attrs {
        h::attrs()
            .id(h::dom_id(&self.param_key, self.id, Some("list")))
            .data("sorted_list_name", self.name.as_str())
            .merge(self.menu.data())
            .class(self.class_names())
    }
    fn class_names(&self) -> String {
        format!(
            "sidebar-item room btn{}{}",
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

#[derive(Template)]
#[template(path="users/sidebars/show.html",blocks=["head","content"])]
pub struct SidebarShow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub current_user: UserSummary,
    pub rooms_stream: String,
    pub user_rooms_stream: String,
    pub direct_memberships: Vec<SidebarDirectItem>,
    pub direct_placeholder_users: Vec<UserSummary>,
    pub other_memberships: Vec<SidebarRoom>,
    pub can_create_rooms: bool,
}
impl Page for SidebarShow<'_> {}

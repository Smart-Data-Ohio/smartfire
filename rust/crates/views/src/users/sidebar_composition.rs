//! Complete viewer-specific sidebar composition. Inputs are ordinary render data.
use crate::layouts::Page;
use crate::{ViewContext, helpers as h};
use askama::Template;
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub avatar_path: String,
}
impl Person {
    pub fn first_name(&self) -> &str {
        self.name.split_whitespace().next().unwrap_or("")
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub raw_name: Option<String>,
    #[serde(default)]
    pub category_row: bool,
    pub epoch: String,
    /// Rails collection cache keys only the membership, participant IDs and admin flag.
    #[serde(default)]
    pub direct_cache_key: Option<String>,
    pub members: Vec<Person>,
    pub call: crate::rooms::calls::CallRow,
}
impl Row {
    fn param_key(&self) -> String {
        format!("rooms_{}", self.kind)
    }
    pub fn menu(&self) -> h::Attrs {
        h::attrs()
            .data(
                "menu_categorizable",
                self.kind == "open" || self.kind == "closed",
            )
            .data("menu_favorited", self.call.favorited)
            .attr_opt("data-menu-favorite-position", self.call.favorite_position)
            .data("menu_muted", self.call.muted)
            .data(
                "menu_default_involvement",
                if self.kind == "direct" {
                    "everything"
                } else {
                    "mentions"
                },
            )
            .attr_opt("data-menu-category-id", self.call.category_id)
            .data("menu_can_delete", self.call.can_delete)
            .data("menu_can_leave", self.call.membership)
            .data(
                "menu_leave_url",
                if self.kind == "direct" {
                    format!("/rooms/directs/{}/leave", self.id)
                } else {
                    format!("/rooms/{}/leave", self.id)
                },
            )
            .data("menu_open_room", self.kind == "open")
            .data("menu_direct_room", self.kind == "direct")
            .attr_opt(
                "data-menu-room-label",
                if self.category_row && self.raw_name.is_none() {
                    None
                } else {
                    Some(self.name.as_str())
                },
            )
    }
    fn classes(&self) -> String {
        format!(
            "sidebar-item room btn{}{}{}",
            if self.kind == "board" {
                " board-room"
            } else {
                ""
            },
            if self.call.unread { " unread" } else { "" },
            if self.call.muted { " muted" } else { "" }
        )
    }
    fn options(&self) -> h::Attrs {
        h::attrs()
            .id(h::dom_id(self.param_key().as_str(), self.id, Some("list")))
            .data("sorted_list_name", self.name.as_str())
            .merge(self.menu())
            .class(self.classes())
    }
    fn direct_options(&self) -> h::Attrs {
        h::attrs()
            .class(format!(
                "sidebar-item direct{}{}",
                if self.call.unread { " unread" } else { "" },
                if self.call.muted { " muted" } else { "" }
            ))
            .id(h::dom_id("rooms_direct", self.id, Some("list")))
            .data("rooms_list_target", "room")
            .data("room_id", self.id)
            .data("badge_dot_target", "unread")
            .data("sorted_list_target", "item")
            .data("sorted_list_number", self.epoch.as_str())
    }
    fn participants(&self) -> String {
        crate::huddle::participants(
            &format!("Rooms::{}", h::capitalize(&self.kind)),
            self.id,
            "sidebar",
            &self.call.participants,
        )
    }
    pub fn render(&self, ctx: &ViewContext, configured: bool) -> String {
        self.render_row(ctx, configured, false)
    }
    /// A standalone Rails partial has no collection separator after its root tag.
    pub fn render_fragment(&self, ctx: &ViewContext, configured: bool) -> String {
        self.render_row(ctx, configured, true)
    }
    fn render_row(&self, ctx: &ViewContext, configured: bool, collection: bool) -> String {
        if self.kind == "voice" || self.kind == "stage" {
            let html = self.call.render(ctx);
            return if collection {
                html
            } else {
                format!("{html}\n")
            };
        }
        if self.kind == "direct" {
            let render = || Direct { ctx, row: self, configured, collection }
                .render().expect("direct sidebar row renders");
            return match self.direct_cache_key.as_ref() {
                Some(key) => crate::fragment_cache::fetch(|| key.clone(), render),
                None => render(),
            };
        } else {
            Shared {
                ctx,
                row: self,
                configured,
                collection,
            }
            .render()
        }
        .expect("sidebar row renders")
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
    pub rows: Vec<Row>,
}
impl Category {
    fn action(&self) -> String {
        format!("/room_categories/{}", self.id)
    }
    fn toggle_label(&self) -> &str {
        if self.collapsed { "Expand" } else { "Collapse" }
    }
    fn render(&self, ctx: &ViewContext, configured: bool) -> String {
        CategoryView {
            ctx,
            category: self,
            configured,
        }
        .render()
        .expect("sidebar category renders")
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Sidebar {
    pub account_name: String,
    pub logo_path: Option<String>,
    pub actor: Person,
    pub configured: bool,
    pub can_create: bool,
    pub favorites: Vec<Row>,
    pub channels: Vec<Row>,
    pub boards: Vec<Row>,
    pub voice: Vec<Row>,
    pub stage: Vec<Row>,
    pub direct: Vec<Row>,
    pub placeholders: Vec<Person>,
    pub categories: Vec<Category>,
}
impl Sidebar {
    pub fn render(&self, ctx: &ViewContext) -> String {
        h::sidebar_turbo_frame_tag(
            None,
            &Shell { ctx, sidebar: self }
                .render()
                .expect("sidebar shell renders"),
        )
        .0
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_shell.html")]
struct Shell<'a> {
    ctx: &'a ViewContext<'a>,
    sidebar: &'a Sidebar,
}
impl Shell<'_> {
    fn direct_rows(&self) -> String {
        self.sidebar
            .direct
            .iter()
            .map(|row| row.render_fragment(self.ctx, self.sidebar.configured))
            .collect()
    }

    fn account_initial(&self) -> String {
        self.sidebar
            .account_name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
    }
    fn rows(&self, rows: &[Row], indent: &str) -> String {
        rows.iter()
            .map(|r| format!("{indent}{}", r.render(self.ctx, self.sidebar.configured)))
            .collect()
    }
    fn favorites(&self) -> String {
        if self.sidebar.favorites.is_empty() {
            String::new()
        } else {
            Favorites {
                ctx: self.ctx,
                sidebar: self.sidebar,
            }
            .render()
            .unwrap()
        }
    }
    fn categories(&self) -> String {
        self.sidebar
            .categories
            .iter()
            .map(|c| c.render(self.ctx, self.sidebar.configured))
            .collect()
    }
    fn placeholders(&self) -> String {
        self.sidebar
            .placeholders
            .iter()
            .map(|user| {
                Placeholder {
                    ctx: self.ctx,
                    user,
                }
                .render()
                .unwrap()
            })
            .collect()
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_favorites.html")]
struct Favorites<'a> {
    ctx: &'a ViewContext<'a>,
    sidebar: &'a Sidebar,
}
impl Favorites<'_> {
    fn rows(&self) -> String {
        self.sidebar
            .favorites
            .iter()
            .map(|r| {
                format!(
                    "                {}",
                    r.render(self.ctx, self.sidebar.configured)
                )
            })
            .collect()
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_shared.html")]
struct Shared<'a> {
    collection: bool,
    ctx: &'a ViewContext<'a>,
    row: &'a Row,
    configured: bool,
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_direct.html")]
struct Direct<'a> {
    collection: bool,
    ctx: &'a ViewContext<'a>,
    row: &'a Row,
    configured: bool,
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_placeholder.html")]
struct Placeholder<'a> {
    ctx: &'a ViewContext<'a>,
    user: &'a Person,
}
impl Placeholder<'_> {
    fn action(&self) -> String {
        h::rooms_directs_with_users(&[self.user.id])
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/composition/_category.html")]
struct CategoryView<'a> {
    ctx: &'a ViewContext<'a>,
    category: &'a Category,
    configured: bool,
}
impl CategoryView<'_> {
    fn rows(&self) -> String {
        self.category
            .rows
            .iter()
            .map(|r| {
                format!(
                    "        {}",
                    Shared {
                        ctx: self.ctx,
                        row: r,
                        configured: self.configured,
                        collection: false,
                    }
                    .render()
                    .unwrap()
                )
            })
            .collect()
    }
}
mod filters {
    pub use crate::helpers::filters::*;
}

impl Direct<'_> {
    fn avatar_button(&self, user: &Person, size: usize, group: bool) -> String {
        let content = if group {
            format!(
                "\n          {}\n",
                h::image_tag(
                    self.ctx,
                    &user.avatar_path,
                    h::attrs().size(size).aria_hidden()
                )
                .0
            )
        } else {
            format!(
                "\n      {}\n      <span class=\"avatar__presence\" data-dm-presence-user-id=\"{}\" data-presence=\"offline\" role=\"img\" aria-label=\"Offline\"></span>\n",
                h::image_tag(
                    self.ctx,
                    &user.avatar_path,
                    h::attrs().size(size).aria_hidden()
                )
                .0,
                user.id
            )
        };
        h::button_tag(
            h::attrs()
                .type_("button")
                .class("avatar profile-card-avatar")
                .aria("label", format!("View profile of {}", user.name))
                .merge(h::profile_card_trigger(user.id, false)),
            &content,
        )
        .0
    }
}

#[derive(Template)]
#[template(path = "users/sidebars/composition.html", blocks=["head","content"])]
pub struct Show<'a> {pub ctx: &'a ViewContext<'a>,pub sidebar: &'a Sidebar}
impl Page for Show<'_> {}

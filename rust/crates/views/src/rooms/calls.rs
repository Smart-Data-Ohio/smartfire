//! Voice and Stage room forms and rows; controllers supply plain render data.
use super::FormRoom;
use crate::{ViewContext, helpers as h, layouts::Page, messages::UserView};
use askama::Template;

#[derive(Clone, Debug)]
pub struct CallForm {
    pub room: FormRoom,
    pub stage: bool,
    pub can_administer: bool,
    pub current_user_id: i64,
    pub selected_users: Vec<UserView>,
    pub unselected_users: Vec<UserView>,
    pub icon_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    pub errors: Vec<String>,
}
impl CallForm {
    pub fn action(&self) -> String {
        match (self.stage, self.room.id) {
            (true, Some(id)) => campfire_routes::rooms_stage(id),
            (true, None) => campfire_routes::rooms_stages(),
            (false, Some(id)) => campfire_routes::rooms_voice(id),
            (false, None) => campfire_routes::rooms_voices(),
        }
    }
    pub fn builder(&self) -> h::FormWith {
        h::form_with(self.action())
            .model(if self.stage {
                "rooms_stage"
            } else {
                "rooms_voice"
            })
            .method(if self.room.id.is_some() {
                "patch"
            } else {
                "post"
            })
    }
    fn users_html(&self, ctx: &ViewContext, selected: bool) -> String {
        let users = if selected {
            &self.selected_users
        } else {
            &self.unselected_users
        };
        users
            .iter()
            .map(|user| {
                format!(
                    "{}\n",
                    CallUser {
                        ctx,
                        form: self,
                        user,
                        selected
                    }
                    .render()
                    .unwrap()
                )
            })
            .collect()
    }
    fn icon_field(&self, ctx: &ViewContext) -> h::Html {
        let builder = self.builder();
        let field = crate::shared::IconField {
            ctx,
            form: &builder,
            scope: "room",
            icon_name: self.icon_name.as_deref(),
            icon: self.icon.as_ref(),
        }
        .render()
        .unwrap();
        h::Safe(field)
    }
}
macro_rules! call_page {
    ($name:ident,$path:literal,$edit:literal) => {
        #[derive(Template)]
        #[template(path=$path,blocks=["head","content"])]
        pub struct $name<'a> {
            pub ctx: &'a ViewContext<'a>,
            pub form: &'a CallForm,
        }
        impl Page for $name<'_> {
            fn page_title(&self) -> Option<String> {
                Some(if $edit {
                    {
                        let _ = self.room_id();
                        format!(
                            "Edit settings for {}",
                            self.form.room.name.as_deref().unwrap_or_default()
                        )
                    }
                } else {
                    "New chat room".into()
                })
            }
        }
        impl $name<'_> {
            fn room_id(&self) -> i64 {
                self.form.room.id.unwrap_or_default()
            }
        }
    };
}
call_page!(VoicesNew, "rooms/voices/new.html", false);
call_page!(VoicesEdit, "rooms/voices/edit.html", true);
call_page!(StagesNew, "rooms/stages/new.html", false);
call_page!(StagesEdit, "rooms/stages/edit.html", true);
#[derive(Template)]
#[template(path = "rooms/voices/_form.html")]
pub struct VoiceForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a CallForm,
}
#[derive(Template)]
#[template(path = "rooms/stages/_form.html")]
pub struct StageForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a CallForm,
}

#[derive(Template)]
#[template(path = "rooms/calls/_user.html")]
struct CallUser<'a> {
    ctx: &'a ViewContext<'a>,
    form: &'a CallForm,
    user: &'a UserView,
    selected: bool,
}

mod filters {
    pub use crate::helpers::filters::*;
}

#[derive(Clone, Debug)]
pub struct CallRow {
    pub id: i64,
    pub name: String,
    pub stage: bool,
    pub icon: Option<h::AvatarIcon>,
    pub participants: Vec<crate::huddle::Participant>,
    pub live: bool,
    pub live_name: String,
    pub unread: bool,
    pub muted: bool,
    pub membership: bool,
    pub favorited: bool,
    pub favorite_position: Option<i64>,
    pub category_id: Option<i64>,
    pub can_delete: bool,
}
impl CallRow {
    fn param_key(&self) -> &str {
        if self.stage {
            "rooms_stage"
        } else {
            "rooms_voice"
        }
    }
    pub fn dom_id(&self, prefix: &str) -> String {
        h::dom_id(self.param_key(), self.id, Some(prefix))
    }
    fn options(&self) -> h::Attrs {
        h::attrs()
            .id(self.dom_id("list"))
            .data("sorted_list_name", self.name.as_str())
            .data("menu_categorizable", false)
            .data("menu_favorited", self.favorited)
            .attr_opt("data-menu-favorite-position", self.favorite_position)
            .data("menu_muted", self.muted)
            .data("menu_default_involvement", "mentions")
            .attr_opt("data-menu-category-id", self.category_id)
            .data("menu_can_delete", self.can_delete)
            .data("menu_can_leave", self.membership)
            .data("menu_leave_url", format!("/rooms/{}/leave", self.id))
            .data("menu_open_room", false)
            .data("menu_direct_room", false)
            .data("menu_room_label", self.name.as_str())
            .class(format!(
                "sidebar-item room btn voice-room{}{}{}",
                if self.stage { " stage-room" } else { "" },
                if self.unread { " unread" } else { "" },
                if self.muted { " muted" } else { "" }
            ))
    }
    fn participants_html(&self) -> String {
        crate::huddle::participants(
            if self.stage {
                "Rooms::Stage"
            } else {
                "Rooms::Voice"
            },
            self.id,
            "sidebar",
            &self.participants,
        )
    }
    fn live_dot(&self) -> String {
        // Shared dots don't read a viewer or roster.
        crate::huddle_stage::Stage {
            room_id: self.id,
            viewer_id: 0,
            members: Vec::new(),
            live: self.live.then(|| crate::huddle_stage::Live {
                id: 0,
                membership_id: 0,
                name: self.live_name.clone(),
                identity: None,
            }),
        }
        .render("live_dot")
    }
    pub fn render(&self, ctx: &ViewContext) -> String {
        if self.stage {
            StageRow { ctx, row: self }.render()
        } else {
            VoiceRow { ctx, row: self }.render()
        }
        .unwrap()
    }
    pub fn header(&self, ctx: &ViewContext) -> String {
        format!("{}\n", Header { ctx, row: self }.render().unwrap())
    }
}
#[derive(Template)]
#[template(path = "users/sidebars/rooms/_voice.html")]
struct VoiceRow<'a> {
    ctx: &'a ViewContext<'a>,
    row: &'a CallRow,
}
#[derive(Template)]
#[template(path = "users/sidebars/rooms/_stage.html")]
struct StageRow<'a> {
    ctx: &'a ViewContext<'a>,
    row: &'a CallRow,
}
#[derive(Template)]
#[template(path = "rooms/calls/_header_identity.html")]
struct Header<'a> {
    ctx: &'a ViewContext<'a>,
    row: &'a CallRow,
}

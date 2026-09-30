//! Complete room header region: app/views/rooms/show/_nav.html.erb.
use super::RoomView;
use crate::{
    ViewContext, helpers as h, huddle::Participant, huddle_stage::Stage, messages::RoomKind,
};
use askama::Template;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Navigation {
    pub room: RoomView,
    #[serde(default)]
    pub icon: Option<h::AvatarIcon>,
    pub pins_count: i64,
    pub involvement: String,
    pub participants: Vec<Participant>,
    pub stage: Option<Stage>,
}
impl Navigation {
    pub fn identity(&self, ctx: &ViewContext) -> String {
        Identity { ctx, nav: self }.render().expect("room identity")
    }
    pub fn render(&self, ctx: &ViewContext) -> String {
        Nav {
            ctx,
            nav: self,
            room: &self.room,
        }
        .render()
        .expect("room navigation")
    }
}
#[derive(Template)]
#[template(path = "rooms/show/_workspace_nav.html")]
struct Nav<'a> {
    ctx: &'a ViewContext<'a>,
    nav: &'a Navigation,
    room: &'a RoomView,
}
#[derive(Template)]
#[template(path = "rooms/show/_identity.html")]
struct Identity<'a> {
    ctx: &'a ViewContext<'a>,
    nav: &'a Navigation,
}
impl Identity<'_> {
    fn kind_label(&self) -> &str {
        match self.nav.room.kind {
            RoomKind::Stage => "Stage channel",
            RoomKind::Voice => "Voice channel",
            RoomKind::Direct => "Direct message",
            _ => "Channel",
        }
    }
}
impl Nav<'_> {
    fn identity(&self) -> String {
        Identity {
            ctx: self.ctx,
            nav: self.nav,
        }
        .render()
        .unwrap()
    }
    fn live_badge(&self) -> String {
        self.nav
            .stage
            .as_ref()
            .map(|s| s.render("live_badge"))
            .unwrap_or_default()
    }
    fn participants(&self) -> String {
        crate::huddle::participants(
            match self.room.kind {
                RoomKind::Voice => "Rooms::Voice",
                RoomKind::Stage => "Rooms::Stage",
                RoomKind::Direct => "Rooms::Direct",
                RoomKind::Open => "Rooms::Open",
                _ => "Rooms::Closed",
            },
            self.room.id,
            "header",
            &self.nav.participants,
        )
    }
    fn join_label(&self) -> &str {
        match self.room.kind {
            RoomKind::Stage => "Join stage",
            RoomKind::Voice => "Join voice",
            _ => "Join huddle",
        }
    }
    fn threads_attrs(&self) -> h::Attrs {
        h::attrs()
            .type_("button")
            .class("btn room-header__action room-header__action--threads")
            .aria("label", "Show threads")
            .aria("controls", "thread-panel")
            .aria("expanded", "false")
            .title("Show threads")
            .data("action", "thread-panel#toggle")
            .data("thread_panel_target", "browserToggle")
    }
    fn share_attrs(&self) -> h::Attrs {
        h::attrs()
            .type_("button")
            .class("btn huddle-share-indicator room-header__action")
            .attr("hidden", true)
            .aria("label", "View shared screen")
            .title("View shared screen")
            .data("controller", "huddle-share-indicator")
            .data("action", "huddle-share-indicator#view")
            .data("huddle_share_indicator_room_id_value", self.room.id)
    }
    fn launcher_attrs(&self) -> h::Attrs {
        let toggle = matches!(self.room.kind, RoomKind::Voice | RoomKind::Stage);
        h::attrs()
            .type_("button")
            .class("btn huddle-launcher room-header__action")
            .aria("pressed", "false")
            .aria("label", self.join_label())
            .data("controller", "huddle-launcher")
            .data(
                "action",
                if toggle {
                    "huddle-launcher#toggle"
                } else {
                    "huddle-launcher#join"
                },
            )
            .data("huddle_launcher_room_id_value", self.room.id)
            .data(
                "huddle_launcher_room_name_value",
                self.room.display_name.as_str(),
            )
            .data("huddle_launcher_join_label_value", self.join_label())
            .data(
                "huddle_launcher_active_label_value",
                match self.room.kind {
                    RoomKind::Stage => "Leave stage",
                    RoomKind::Voice => "Leave voice",
                    _ => "In huddle",
                },
            )
            .data("huddle_launcher_toggle_value", toggle)
            .attr_opt(
                "data-huddle-can-publish-param",
                self.nav
                    .stage
                    .as_ref()
                    .map(|s| s.viewer().role != "listener"),
            )
    }
    fn action_attrs(&self, name: &str, label: &str) -> h::Attrs {
        h::attrs()
            .class(format!(
                "btn room-header__action room-header__action--{name}"
            ))
            .aria("label", label)
            .title(label)
    }
    fn settings_attrs(&self) -> h::Attrs {
        h::attrs()
            .class("btn room-header__action room-header__action--settings")
            .style(format!("view-transition-name: edit-room-{}", self.room.id))
            .data("room_id", self.room.id)
    }
    fn events_path(&self) -> String {
        format!("/rooms/{}/events", self.room.id)
    }
    fn files_path(&self) -> String {
        format!("/rooms/{}/files", self.room.id)
    }
    fn pins_path(&self) -> String {
        format!("/rooms/{}/pins", self.room.id)
    }
    fn involvement_path(&self) -> String {
        format!("/rooms/{}/involvement", self.room.id)
    }
    fn current_level(&self, level: &str) -> bool {
        level == self.nav.involvement
    }
    fn levels(&self) -> &'static [&'static str] {
        h::involvement_levels(self.room.is_direct())
    }
    fn short_label(&self, level: &str) -> &'static str {
        h::short_involvement_label(level)
    }
    fn description(&self, level: &str) -> &'static str {
        h::humanize_involvement(level)
    }
    fn notification_icon(&self) -> h::Html {
        h::image_tag(
            self.ctx,
            format!("notification-bell-{}.svg", self.nav.involvement),
            h::attrs()
                .size(20)
                .aria_hidden()
                .data("header_overflow_target", "notificationsIcon"),
        )
    }
}
mod filters {
    pub use crate::helpers::filters::*;
}

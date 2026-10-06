//! Room composer, member/thread panels and poll builder from our Rails markup.
//! Ordinary view inputs bind all routes, assets, labels and request form tokens.
use super::RoomView;
use crate::{ViewContext, helpers as h};
use askama::Template;
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    None,
    Legacy,
    Picker,
}
macro_rules! partial {
    ($name:ident,$path:literal) => {
        #[derive(Template)]
        #[template(path=$path)]
        pub struct $name<'a> {
            pub ctx: &'a ViewContext<'a>,
            pub room: &'a RoomView,
            pub neutral_name: Option<&'a str>,
        }
        impl $name<'_> {
            pub fn form_action(&self, kind: &str) -> String {
                format!("/rooms/{}/{kind}", self.room.id)
            }
            pub fn channel_name(&self) -> &str {
                self.neutral_name.unwrap_or(&self.room.display_name)
            }
        }
    };
}
partial!(Composer, "rooms/composition/_composer_none.html");
partial!(LegacyComposer, "rooms/composition/_composer_legacy.html");
partial!(PickerComposer, "rooms/composition/_composer_picker.html");
partial!(MemberPanel, "rooms/composition/_member_panel.html");
partial!(ThreadPanel, "rooms/composition/_thread_panel.html");
partial!(PollBuilder, "rooms/composition/_poll_builder.html");
pub fn render(
    ctx: &ViewContext,
    room: &RoomView,
    neutral_name: Option<&str>,
    partial: &str,
    drive: Drive,
) -> String {
    match partial {
        "composer" => match drive {
            Drive::None => Composer {
                ctx,
                room,
                neutral_name,
            }
            .render(),
            Drive::Legacy => LegacyComposer {
                ctx,
                room,
                neutral_name,
            }
            .render(),
            Drive::Picker => PickerComposer {
                ctx,
                room,
                neutral_name,
            }
            .render(),
        },
        "member_panel" => MemberPanel {
            ctx,
            room,
            neutral_name,
        }
        .render(),
        "thread_panel" => ThreadPanel {
            ctx,
            room,
            neutral_name,
        }
        .render(),
        "poll_builder" => PollBuilder {
            ctx,
            room,
            neutral_name,
        }
        .render(),
        _ => panic!("unknown room composition partial"),
    }
    .expect("room composition renders")
}
pub fn request(
    ctx: &ViewContext,
    room: &RoomView,
    neutral_name: Option<&str>,
    partial: &str,
) -> String {
    let drive = if ctx.chrome.google_picker.is_some() && ctx.human_signed_in() {
        Drive::Picker
    } else if ctx
        .current_user
        .as_ref()
        .is_some_and(|u| u.preferences.google_drive)
    {
        Drive::Legacy
    } else {
        Drive::None
    };
    render(ctx, room, neutral_name, partial, drive)
}

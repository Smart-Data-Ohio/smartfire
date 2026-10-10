//! Viewer facts for the Google profile panels; domain policy stays outside the view layer.
use crate::helpers as h;
use askama::Template;
#[derive(Template)]
#[template(path = "users/profiles/_google_calendar.html")]
pub struct Calendar {
    pub data: CalendarData,
}
impl Calendar {
    pub(super) fn calendar_data(&self) -> CalendarData { self.data.clone() }
    pub(super) fn connect(&self, label: &str, drive: bool) -> h::Html {
        let mut form = h::button_to_form(
            "/google/connect",
            h::attrs().method("post").class("btn"),
            h::attrs().data("turbo", false),
            label,
        )
        .0;
        if drive {
            // Rails button_to(params: {features: ["drive"]}) puts sorted hidden params after the token.
            form.insert_str(
                form.len() - 7,
                "<input type=\"hidden\" name=\"features[]\" value=\"drive\" />",
            );
        }
        h::raw(form)
    }
    pub(super) fn disconnect(&self) -> h::Html {
        h::button_to_form(
            "/google/connection",
            h::attrs().method("delete").class("btn btn--negative"),
            h::attrs().data(
                "turbo_confirm",
                "Disconnect Google Calendar? Your published event entries will be removed.",
            ),
            "Disconnect",
        )
    }
}
#[derive(Template)]
#[template(path = "users/profiles/_google_sign_in.html")]
pub struct SignIn {
    pub data: SignInData,
}
impl SignIn {
    fn link(&self) -> h::Html {
        h::button_to_form(
            "/user/profile/google_sign_in_link",
            h::attrs().method("post").class("btn"),
            h::attrs().data("turbo", false),
            "Link Google sign-in",
        )
    }
}
pub use campfire_presentation::users::google::*;

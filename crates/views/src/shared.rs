//! Views for `reference/app/views/shared`.

use crate::helpers::{self as h, filters};
use askama::Template;

#[derive(Template)]
#[template(path = "shared/_multi_select_bar.html")]
pub struct MultiSelectBar {
    pub exit_button: bool,
}

#[derive(Template)]
#[template(path = "shared/_icon_field.html")]
pub struct IconField<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    pub form: &'a h::FormWith,
    pub scope: &'a str,
    pub icon_name: Option<&'a str>,
    /// Resolved by the icon owner; unknown names remain editable but render no preview.
    pub icon: Option<&'a h::AvatarIcon>,
}

impl IconField<'_> {
    fn field_id(&self) -> String {
        format!("{}_icon_name", self.scope)
    }
    fn field_name(&self) -> String {
        format!("{}[icon_name]", self.scope)
    }
    fn field_value(&self) -> Option<String> {
        h::presence(self.icon_name).map(|name| format!(":{name}:"))
    }
}

//! Block helpers as askama filter blocks. Each view module does `use crate::helpers::filters;`
//! and templates write `{% filter link_to(url, h::attrs().class("btn")) %}...{% endfilter %}`
//! where Ruby had `link_to url, class: "btn" do ... end`. The block's rendered (safe) content is
//! the first argument.

use std::fmt::Display;

use askama::Values;

pub use campfire_view_kit::helpers::filters::{
    button, button_to, button_to_form, content_tag, form_with, link_to, tag,
};

use super::html::Html;
use super::tag::{self, Attrs};

type Result = askama::Result<Html>;

/// `turbo_frame_tag(id, src:, target:, **attributes) do ... end`; `src` and `target` may be in
/// `options` and are moved after the id as turbo-rails does.
pub fn turbo_frame_tag(
    content: impl Display,
    _: &dyn Values,
    id: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    let mut options = options.borrow().clone();
    let src = options.remove("src").map(|value| attr_string(&value));
    let target = options.remove("target").map(|value| attr_string(&value));
    Ok(super::turbo::turbo_frame_tag(
        &id.to_string(),
        src.as_deref(),
        target.as_deref(),
        options,
        &content.to_string(),
    ))
}

/// `sidebar_turbo_frame_tag do ... end` (the block form never passes `src:`).
pub fn sidebar_turbo_frame_tag(content: impl Display, _: &dyn Values) -> Result {
    Ok(super::users::sidebar_turbo_frame_tag(
        None,
        &content.to_string(),
    ))
}

/// `link_to_room(room, **attributes) do ... end`.
pub fn link_to_room(
    content: impl Display,
    _: &dyn Values,
    room_id: impl std::borrow::Borrow<i64>,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(super::rooms::link_to_room(
        *room_id.borrow(),
        options.borrow().clone(),
        &content.to_string(),
    ))
}

/// `link_to_zoom_qr_code(url) do ... end`.
pub fn link_to_zoom_qr_code(content: impl Display, _: &dyn Values, url: impl Display) -> Result {
    Ok(super::application::link_to_zoom_qr_code(
        &url.to_string(),
        &content.to_string(),
    ))
}

/// `button_to_copy_to_clipboard(url) do ... end`.
pub fn button_to_copy_to_clipboard(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
) -> Result {
    Ok(super::application::button_to_copy_to_clipboard(
        &url.to_string(),
        &content.to_string(),
    ))
}

/// `web_share_session_button(url, title, text) do ... end`.
pub fn web_share_session_button(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    title: impl Display,
    text: impl Display,
) -> Result {
    Ok(super::application::web_share_session_button(
        &url.to_string(),
        &title.to_string(),
        &text.to_string(),
        &content.to_string(),
    ))
}

/// `user_filter_menu_tag do ... end`.
pub fn user_filter_menu_tag(content: impl Display, _: &dyn Values) -> Result {
    Ok(super::users::user_filter_menu_tag(&content.to_string()))
}

fn attr_string(value: &tag::Value) -> String {
    match value {
        tag::Value::Text(text) | tag::Value::Safe(text) => text.clone(),
        tag::Value::Bool(flag) => flag.to_string(),
    }
}

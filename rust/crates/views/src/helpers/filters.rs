//! Block helpers as askama filter blocks. Each view module does `use crate::helpers::filters;`
//! and templates write `{% filter link_to(url, h::attrs().class("btn")) %}...{% endfilter %}`
//! where Ruby had `link_to url, class: "btn" do ... end`. The block's rendered (safe) content is
//! the first argument.

use std::fmt::Display;

use askama::Values;

use super::forms::{self, FormWith};
use super::html::Html;
use super::tag::{self, Attrs};

type Result = askama::Result<Html>;

/// `form_with(...) do |form| ... end`.
pub fn form_with(content: impl Display, _: &dyn Values, form: impl std::borrow::Borrow<FormWith>) -> Result {
    Ok(form.borrow().wrap(&content.to_string()))
}

/// `link_to(url, options) do ... end`.
pub fn link_to(content: impl Display, _: &dyn Values, url: impl Display, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(super::links::link_to(&url.to_string(), options.borrow().clone(), &content.to_string()))
}

/// `button_to(url, options) do ... end`; `options` may include `method`.
pub fn button_to(content: impl Display, _: &dyn Values, url: impl Display, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(forms::button_to(&url.to_string(), options.borrow().clone(), &content.to_string()))
}

/// `button_to(url, options.merge(form: form_options)) do ... end`.
pub fn button_to_form(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
    form_options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(forms::button_to_form(&url.to_string(), options.borrow().clone(), form_options.borrow().clone(), &content.to_string()))
}

/// `tag.<name>(options) do ... end`, e.g. `{% filter tag("button", h::attrs().type_("button")) %}`.
pub fn tag(content: impl Display, _: &dyn Values, name: impl Display, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(tag::content_tag(&tag::dasherize(&name.to_string()), options.borrow(), &content.to_string()))
}

/// `form.button(options) do ... end`.
pub fn button(content: impl Display, _: &dyn Values, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(forms::button_tag(options.borrow().clone(), &content.to_string()))
}

/// `tag.name(options) do ... end` / `content_tag(name, options) do ... end`.
pub fn content_tag(content: impl Display, _: &dyn Values, name: impl Display, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(tag::content_tag(&name.to_string(), options.borrow(), &content.to_string()))
}

/// `turbo_frame_tag(id, src:, target:, **attributes) do ... end`; `src` and `target` may be in
/// `options` and are moved after the id as turbo-rails does.
pub fn turbo_frame_tag(content: impl Display, _: &dyn Values, id: impl Display, options: impl std::borrow::Borrow<Attrs>) -> Result {
    let mut options = options.borrow().clone();
    let src = options.remove("src").map(|value| attr_string(&value));
    let target = options.remove("target").map(|value| attr_string(&value));
    Ok(super::turbo::turbo_frame_tag(&id.to_string(), src.as_deref(), target.as_deref(), options, &content.to_string()))
}

/// `sidebar_turbo_frame_tag do ... end` (the block form never passes `src:`).
pub fn sidebar_turbo_frame_tag(content: impl Display, _: &dyn Values) -> Result {
    Ok(super::users::sidebar_turbo_frame_tag(None, &content.to_string()))
}

/// `link_to_room(room, **attributes) do ... end`.
pub fn link_to_room(content: impl Display, _: &dyn Values, room_id: impl std::borrow::Borrow<i64>, options: impl std::borrow::Borrow<Attrs>) -> Result {
    Ok(super::rooms::link_to_room(*room_id.borrow(), options.borrow().clone(), &content.to_string()))
}

/// `link_to_zoom_qr_code(url) do ... end`.
pub fn link_to_zoom_qr_code(content: impl Display, _: &dyn Values, url: impl Display) -> Result {
    Ok(super::application::link_to_zoom_qr_code(&url.to_string(), &content.to_string()))
}

/// `button_to_copy_to_clipboard(url) do ... end`.
pub fn button_to_copy_to_clipboard(content: impl Display, _: &dyn Values, url: impl Display) -> Result {
    Ok(super::application::button_to_copy_to_clipboard(&url.to_string(), &content.to_string()))
}

/// `web_share_session_button(url, title, text) do ... end`.
pub fn web_share_session_button(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    title: impl Display,
    text: impl Display,
) -> Result {
    Ok(super::application::web_share_session_button(&url.to_string(), &title.to_string(), &text.to_string(), &content.to_string()))
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

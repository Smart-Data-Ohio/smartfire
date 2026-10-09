//! Askama filter blocks shared by both shells. Classic-only blocks stay in `campfire_views`.

use std::fmt::Display;

use askama::Values;

use super::forms::{self, FormWith};
use super::html::Html;
use super::tag::{self, Attrs};

type Result = askama::Result<Html>;

/// `form_with(...) do |form| ... end`.
pub fn form_with(
    content: impl Display,
    _: &dyn Values,
    form: impl std::borrow::Borrow<FormWith>,
) -> Result {
    Ok(form.borrow().wrap(&content.to_string()))
}

/// `link_to(url, options) do ... end`.
pub fn link_to(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(super::links::link_to(
        &url.to_string(),
        options.borrow().clone(),
        &content.to_string(),
    ))
}

/// `button_to(url, options) do ... end`; `options` may include `method`.
pub fn button_to(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(forms::button_to(
        &url.to_string(),
        options.borrow().clone(),
        &content.to_string(),
    ))
}

/// `button_to(url, options.merge(form: form_options)) do ... end`.
pub fn button_to_form(
    content: impl Display,
    _: &dyn Values,
    url: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
    form_options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(forms::button_to_form(
        &url.to_string(),
        options.borrow().clone(),
        form_options.borrow().clone(),
        &content.to_string(),
    ))
}

/// `tag.<name>(options) do ... end`, e.g. `{% filter tag("button", h::attrs().type_("button")) %}`.
pub fn tag(
    content: impl Display,
    _: &dyn Values,
    name: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(tag::content_tag(
        &tag::dasherize(&name.to_string()),
        options.borrow(),
        &content.to_string(),
    ))
}

/// `form.button(options) do ... end`.
pub fn button(
    content: impl Display,
    _: &dyn Values,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(forms::button_tag(
        options.borrow().clone(),
        &content.to_string(),
    ))
}

/// `tag.name(options) do ... end` / `content_tag(name, options) do ... end`.
pub fn content_tag(
    content: impl Display,
    _: &dyn Values,
    name: impl Display,
    options: impl std::borrow::Borrow<Attrs>,
) -> Result {
    Ok(tag::content_tag(
        &name.to_string(),
        options.borrow(),
        &content.to_string(),
    ))
}

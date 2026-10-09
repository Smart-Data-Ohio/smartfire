//! Helpers the retained templates call. Shared form, tag, CSRF, CSP, route and translation
//! machinery lives in `campfire_view_kit`, which does not compile classic templates.
//! The wrappers below read [`crate::Context`] instead of a classic view context.

use campfire_view_kit::helpers::{Value, content_tag, content_tag_text};

pub use campfire_view_kit::helpers::legacy_tag;

use crate::Context;

pub use campfire_view_kit::helpers::routes;
pub use campfire_view_kit::helpers::{
    Attrs, ErbEscaper, FormWith, Html, attrs, auth_script_tag, auth_stylesheet_tag, builder_tag,
    button_tag, capitalize, csp_meta_tag, csrf_meta_tags, empty, form_with, hidden_field_tag,
    link_to, link_to_text, mail_to, page_title_tag, raw, to_sentence, translations_for,
    turbo_page_requires_reload_tag,
};
pub use campfire_view_kit::helpers::{filters, url};

/// The `id` of the auth layout's rejection message (its alert flash).
pub use campfire_view_kit::helpers::head::AUTH_ALERT_ID;

pub fn user_theme<'a>(ctx: &'a Context<'_>) -> &'a str {
    let theme = ctx
        .current_user
        .as_ref()
        .and_then(|user| user.preferences.theme.as_deref());
    match theme {
        Some(theme @ ("light" | "dark" | "system")) => theme,
        _ => "system",
    }
}

pub fn theme_color_scheme_meta_content(ctx: &Context<'_>) -> &'static str {
    match user_theme(ctx) {
        "light" => "light",
        "dark" => "dark",
        _ => "light dark",
    }
}

pub fn current_user_meta_tags(ctx: &Context<'_>) -> Html {
    match &ctx.current_user {
        Some(user) => campfire_view_kit::helpers::html::Safe(format!(
            "{}{}",
            legacy_tag(
                "meta",
                attrs().name("current-user-id").attr("content", user.id)
            )
            .0,
            legacy_tag(
                "meta",
                attrs()
                    .name("current-user-name")
                    .attr("content", user.name.as_str())
            )
            .0
        )),
        None => empty(),
    }
}

pub fn custom_styles_tag(ctx: &Context<'_>) -> Html {
    match &ctx.custom_styles {
        Some(styles) => content_tag("style", attrs().data("turbo_track", "reload"), styles),
        None => empty(),
    }
}

pub fn version_badge(ctx: &Context<'_>) -> Html {
    content_tag_text("span", attrs().class("version-badge"), &ctx.app_version)
}

fn asset_path(ctx: &Context<'_>, source: &str) -> String {
    let is_url = source.starts_with('/')
        || source.starts_with("data:")
        || source.starts_with("cid:")
        || source.split_once("://").is_some_and(|(scheme, _)| {
            !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
        });
    if is_url {
        source.to_string()
    } else {
        ctx.asset(source)
    }
}

pub fn image_tag(ctx: &Context<'_>, source: impl std::fmt::Display, options: Attrs) -> Html {
    let mut options = options;
    let size = options.remove("size");
    options.set("src", Some(asset_path(ctx, &source.to_string()).into()));
    if let Some(size) = size {
        let size = match size {
            Value::Text(text) | Value::Safe(text) => text,
            Value::Bool(flag) => flag.to_string(),
        };
        let (width, height) = match size.split_once('x') {
            Some((width, height)) => (width.to_string(), height.to_string()),
            None => (size.clone(), size),
        };
        options.set("width", Some(width.into()));
        options.set("height", Some(height.into()));
    }
    legacy_tag("img", &options)
}

pub fn rejected_field(ctx: &Context<'_>, attrs: Attrs) -> Attrs {
    if ctx.flash_alert().is_some() {
        attrs
            .attr("aria-describedby", AUTH_ALERT_ID)
            .attr("aria-invalid", "true")
    } else {
        attrs
    }
}

pub fn translation_button(ctx: &Context<'_>, key: &str) -> Html {
    let globe = image_tag(
        ctx,
        "globe.svg",
        attrs().size(20).aria_hidden().class("color-icon"),
    );
    campfire_view_kit::helpers::translations::translation_popup(&globe.0, key)
}

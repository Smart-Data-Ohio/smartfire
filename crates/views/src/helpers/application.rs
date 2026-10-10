//! `ApplicationHelper`, `CableHelper`, `VersionHelper`, `TimeHelper`, `ClipboardHelper`,
//! `DropTargetHelper` and `QrCodeHelper` (`reference/app/helpers/*.rb`).

pub use campfire_presentation::helpers::application::*;

use base64_url::urlsafe_encode64;

use super::assets::image_tag;
use super::html::{Html, Safe};
use super::links::link_to;
use super::tag::{attrs, builder_tag, content_tag, content_tag_text, legacy_tag};
use crate::ViewContext;

/// `Users::PresenceHelper#user_theme`: the user's theme if it's one of `THEMES`, else "system".
pub fn user_theme<'a>(ctx: &'a ViewContext<'_>) -> &'a str {
    let theme = ctx
        .current_user
        .as_ref()
        .and_then(|user| user.preferences.theme.as_deref());
    match theme {
        Some(theme @ ("light" | "dark" | "system")) => theme,
        _ => "system",
    }
}

/// `Users::PresenceHelper#user_text_size`: the user's text size if it's one of `TEXT_SIZES`, else
/// "default".
pub fn user_text_size<'a>(ctx: &'a ViewContext<'_>) -> &'a str {
    let text_size = ctx
        .current_user
        .as_ref()
        .and_then(|user| user.preferences.text_size.as_deref());
    match text_size {
        Some(size @ ("smaller" | "small" | "default" | "large" | "larger")) => size,
        _ => "default",
    }
}

/// `Users::PresenceHelper#theme_color_scheme_meta_content`.
pub fn theme_color_scheme_meta_content(ctx: &ViewContext) -> &'static str {
    match user_theme(ctx) {
        "light" => "light",
        "dark" => "dark",
        _ => "light dark",
    }
}

/// `current_user_time_zone_meta_content`: the saved zone, "" when "Not set" was chosen on
/// purpose, else nothing (the meta tag then has no content attribute).
pub fn current_user_time_zone_meta_content<'a>(ctx: &'a ViewContext<'_>) -> Option<&'a str> {
    let preferences = &ctx.current_user.as_ref()?.preferences;
    super::text::presence(preferences.time_zone.as_deref())
        .or(preferences.time_zone_explicit.then_some(""))
}

/// `notification_sound_meta_tags` (`app/helpers/application_helper.rb`), from what the settings'
/// owners decided ([`crate::layouts::NotificationSounds`]).
pub fn notification_sound_meta_tags(ctx: &ViewContext) -> Html {
    let Some(user) = &ctx.current_user else {
        return Safe(String::new());
    };
    let sounds = &user.preferences.notification_sounds;
    let meta = |name: &str, content: &str| {
        builder_tag("meta", attrs().name(name).attr("content", content)).0
    };
    let windows = |epochs: &[(i64, i64)]| {
        epochs
            .iter()
            .map(|(start, finish)| format!("{start}-{finish}"))
            .collect::<Vec<_>>()
            .join(",")
    };

    let mut tags = String::new();
    if sounds.muted {
        tags.push_str(&meta("notification-dnd", "muted"));
    }
    if let Some((start, finish)) = sounds.quiet_hours {
        tags.push_str(&meta("quiet-hours", &format!("{start}-{finish}")));
        // `time_zone_or_default`: the saved zone even when Rails doesn't know it, else `Time.zone`.
        let zone = super::text::presence(user.preferences.time_zone.as_deref())
            .unwrap_or(ctx.time_zone.name());
        tags.push_str(&meta("quiet-hours-zone", zone));
    }
    if !sounds.meeting_quiet.is_empty() {
        tags.push_str(&meta("meeting-quiet", &windows(&sounds.meeting_quiet)));
    }
    if !sounds.ooo_quiet.is_empty() {
        tags.push_str(&meta("ooo-quiet", &windows(&sounds.ooo_quiet)));
    }
    Safe(tags)
}

/// `current_user_meta_tags`.
pub fn current_user_meta_tags(ctx: &ViewContext) -> Html {
    match &ctx.current_user {
        Some(user) => Safe(format!(
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
        None => Safe(String::new()),
    }
}

/// `script_aware_action_cable_meta_tag`.
pub fn script_aware_action_cable_meta_tag(ctx: &ViewContext) -> Html {
    builder_tag(
        "meta",
        attrs()
            .name("action-cable-url")
            .attr("content", ctx.cable_url.as_str()),
    )
}

/// `custom_styles_tag`: the account's CSS, unescaped.
pub fn custom_styles_tag(ctx: &ViewContext) -> Html {
    match &ctx.custom_styles {
        Some(styles) => content_tag("style", attrs().data("turbo_track", "reload"), styles),
        None => Safe(String::new()),
    }
}

/// `body_classes`: `[ @body_class, admin_body_class, account_logo_body_class ].compact.join(" ")`.
pub fn body_classes(ctx: &ViewContext, body_class: Option<&str>) -> String {
    let admin = ctx.can_administer().then_some("admin");
    let logo = ctx.account.has_logo.then_some("account-has-logo");
    [body_class, admin, logo]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `link_back`: to the referrer, unless it's missing or the current page.
pub fn link_back(ctx: &ViewContext) -> Html {
    let back_url = match &ctx.referrer {
        Some(referrer) if *referrer != ctx.request_url => referrer.clone(),
        _ => campfire_routes::root(),
    };
    link_back_to(ctx, &back_url)
}

/// `link_back_to(destination)`.
pub fn link_back_to(ctx: &ViewContext, destination: impl std::fmt::Display) -> Html {
    let content = format!(
        "{}{}",
        image_tag(ctx, "arrow-left.svg", attrs().aria_hidden().size(20)).0,
        content_tag_text("span", attrs().class("for-screen-reader"), "Go Back").0
    );
    link_to(&destination.to_string(), attrs().class("btn"), &content)
}

/// `RoomsHelper#link_back_to_last_room_visited`.
pub fn link_back_to_last_room_visited(ctx: &ViewContext) -> Html {
    match ctx.last_room_visited_id {
        Some(room_id) => link_back_to(ctx, campfire_routes::room(room_id)),
        None => link_back_to(ctx, campfire_routes::root()),
    }
}

/// `version_badge`.
pub fn version_badge(ctx: &ViewContext) -> Html {
    content_tag_text("span", attrs().class("version-badge"), &ctx.app_version)
}

/// `button_to_copy_to_clipboard(url) { content }`.
pub fn button_to_copy_to_clipboard(url: &str, content: &str) -> Html {
    let options = attrs()
        .class("btn")
        .data("controller", "copy-to-clipboard")
        .data("action", "copy-to-clipboard#copy")
        .data("copy_to_clipboard_success_class", "btn--success")
        .data("copy_to_clipboard_content_value", url);
    content_tag("button", &options, content)
}

/// `link_to_zoom_qr_code(url) { content }`: the QR code route takes the URL, base64url-encoded.
pub fn link_to_zoom_qr_code(url: &str, content: &str) -> Html {
    let path = campfire_routes::qr_code(urlsafe_encode64(url));
    let options = attrs()
        .class("btn")
        .data("lightbox_target", "image")
        .data("action", "lightbox#open")
        .data("lightbox_url_value", path.as_str());
    link_to(&path, options, content)
}

/// `web_share_session_button(url, title, text) { content }` (`Users::ProfilesHelper`).
pub fn web_share_session_button(url: &str, title: &str, text: &str, content: &str) -> Html {
    let options = attrs()
        .class("btn")
        .hidden()
        .data("controller", "web-share")
        .data("action", "web-share#share")
        .data("web_share_url_value", url)
        .data("web_share_text_value", text)
        .data("web_share_title_value", title);
    content_tag("button", &options, content)
}
pub use campfire_presentation::helpers::application::base64_url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_like_rails() {
        assert_eq!(truncate("abcdef", 4, "…"), "abc…");
        assert_eq!(truncate("abcd", 4, "…"), "abcd");
    }

    #[test]
    fn encodes_urlsafe_base64() {
        assert_eq!(
            base64_url::urlsafe_encode64("http://x/?a"),
            "aHR0cDovL3gvP2E="
        );
    }
}

//! `UsersHelper`, `Users::AvatarsHelper`, `Users::FilterHelper`, `Users::ProfilesHelper`,
//! `Users::SidebarHelper` and `AccountsHelper`.

use super::assets::image_tag;
use super::forms::button_to;
use super::html::{Html, Safe};
use super::links::link_to;
use super::tag::{Attrs, attrs, builder_tag, content_tag, content_tag_text};
use super::turbo::turbo_frame_tag;
use super::url::rooms_directs_with_users;
use crate::ViewContext;

/// `Users::AvatarsHelper::AVATAR_COLORS`.
pub const AVATAR_COLORS: [&str; 18] = [
    "#AF2E1B", "#CC6324", "#3B4B59", "#BFA07A", "#ED8008", "#ED3F1C", "#BF1B1B", "#736B1E",
    "#D07B53", "#736356", "#AD1D1D", "#BF7C2A", "#C09C6F", "#698F9C", "#7C956B", "#5D618F",
    "#3B3633", "#67695E",
];

/// `avatar_background_color(user)`: `Zlib.crc32(user.to_param)` picks the color.
pub fn avatar_background_color(user_id: impl std::borrow::Borrow<i64>) -> &'static str {
    let user_id = *user_id.borrow();
    let crc = crc32(user_id.to_string().as_bytes());
    AVATAR_COLORS[(crc % AVATAR_COLORS.len() as u32) as usize]
}

/// Zlib's CRC-32 (IEEE, reflected).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// `User#initials`: `name.scan(/\b\w/).join`. Ruby's `\w` is ASCII-only while `\b` sees
/// Unicode word characters, so "Élodie" contributes nothing.
pub fn initials(name: &str) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut previous: Option<char> = None;
    let mut out = String::new();
    for c in name.chars() {
        if (c.is_ascii_alphanumeric() || c == '_') && !previous.is_some_and(is_word) {
            out.push(c);
        }
        previous = Some(c);
    }
    out
}

/// `User#title`: `[ name, bio ].compact_blank.join(" – ")`.
pub fn user_title(name: &str, bio: Option<&str>) -> String {
    [Some(name), bio]
        .into_iter()
        .flatten()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" – ")
}

/// What `avatar_tag` needs to know about a user.
#[derive(Clone, Debug, Default)]
pub struct AvatarUser {
    pub id: i64,
    /// `User#title`.
    pub title: String,
    /// `fresh_user_avatar_path(user)`.
    pub avatar_path: String,
}

/// `avatar_tag(user, **options)`: the options go to the image.
pub fn avatar_tag(
    ctx: &ViewContext,
    user: impl std::borrow::Borrow<AvatarUser>,
    options: Attrs,
) -> Html {
    avatar_tag_with_icon(ctx, user, None, options)
}

/// The owner supplies an icon only for a bot without an uploaded avatar (IconsAvatarHelper).
pub fn avatar_tag_with_icon(
    ctx: &ViewContext,
    user: impl std::borrow::Borrow<AvatarUser>,
    icon: Option<&super::icons::AvatarIcon>,
    mut options: Attrs,
) -> Html {
    let user = user.borrow();
    let size = options
        .remove("size")
        .map(|value| {
            super::tag::value_to_string(&value)
                .parse::<usize>()
                .expect("avatar size is numeric")
        })
        .unwrap_or(48);
    let image = match icon {
        Some(icon) => super::icons::icon_avatar_tag(
            ctx,
            Some(icon),
            size,
            attrs().attr_opt("class", options.get_str("class").as_deref()),
        ),
        None => image_tag(
            ctx,
            &user.avatar_path,
            attrs().aria("hidden", "true").size(size).merge(options),
        ),
    };
    link_to(
        &campfire_routes::user(user.id),
        attrs()
            .title(user.title.as_str())
            .class("btn avatar")
            .data("turbo_frame", "_top")
            .merge(profile_card_trigger(user.id, false)),
        &image.0,
    )
}

/// UsersHelper#profile_card_trigger (use tabindex/role on the triggering element for keyboard).
pub fn profile_card_trigger(user_id: i64, keyboard: bool) -> Attrs {
    attrs()
        .data(
            "action",
            if keyboard {
                "click->profile-card#open keydown->profile-card#open"
            } else {
                "click->profile-card#open"
            },
        )
        .data("profile_card_url", campfire_routes::user_card(user_id))
}

/// `button_to_direct_room_with(user)`.
pub fn button_to_direct_room_with(
    ctx: &ViewContext,
    user_id: impl std::borrow::Borrow<i64>,
    name: &str,
) -> Html {
    let user_id = *user_id.borrow();
    button_to(
        &rooms_directs_with_users(&[user_id]),
        attrs()
            .class("btn btn--primary full-width txt--large")
            .aria("label", format!("Message {name}")),
        &image_tag(ctx, "messages.svg", attrs().aria_hidden()).0,
    )
}

/// The bot curl commands in `accounts/bots/_bot`.
pub fn curl_text_line(url: impl AsRef<str>) -> String {
    format!("curl -d 'Hello!' {}", url.as_ref())
}

pub fn curl_upload_line(url: impl AsRef<str>) -> String {
    format!("curl -F \"attachment=@/path/to/file\" {}", url.as_ref())
}

/// `account_logo_tag(style:)`. A nil style leaves a trailing space in the class.
pub fn account_logo_tag(ctx: &ViewContext, style: Option<&str>) -> Html {
    let image = image_tag(
        ctx,
        &ctx.account.logo_url,
        attrs().alt("Account logo").size(300),
    );
    content_tag(
        "figure",
        attrs().class(format!("account-logo avatar {}", style.unwrap_or(""))),
        &image.0,
    )
}

/// `profile_form_submit_button`.
pub fn profile_form_submit_button(ctx: &ViewContext) -> Html {
    let content = format!(
        "{}{}",
        image_tag(ctx, "check.svg", attrs().aria_hidden().size(20)).0,
        content_tag_text("span", attrs().class("for-screen-reader"), "Save changes").0
    );
    content_tag(
        "button",
        attrs()
            .class("btn btn--reversed center txt-large")
            .type_("submit"),
        &content,
    )
}

/// `sidebar_turbo_frame_tag(src:) { content }`.
pub fn sidebar_turbo_frame_tag(src: Option<&str>, content: &str) -> Html {
    let data = attrs()
        .data("turbo_permanent", true)
        .data("controller", "rooms-list read-rooms turbo-frame")
        .data("rooms_list_unread_class", "unread")
        // html_safe in the reference so "->" isn't escaped
        .data(
            "action",
            Safe(
                "presence:present@window->rooms-list#read room:mark-unread@window->rooms-list#markUnread read-rooms:read->rooms-list#read turbo:frame-load->rooms-list#loaded refresh-room:visible@window->turbo-frame#reload"
                    .to_string(),
            ),
        );
    turbo_frame_tag("user_sidebar", src, Some("_top"), data, content)
}

/// `user_filter_menu_tag { content }`.
pub fn user_filter_menu_tag(content: &str) -> Html {
    let options = attrs()
        .class("flex flex-column gap margin-none pad overflow-y constrain-height")
        .data("controller", "filter")
        .data("filter_active_class", "filter--active")
        .data("filter_selected_class", "selected");
    content_tag("menu", &options, content)
}

/// `user_filter_search_tag`.
pub fn user_filter_search_tag() -> Html {
    builder_tag(
        "input",
        attrs()
            .type_("search")
            .id("search")
            .attr("autocorrect", "off")
            .autocomplete("off")
            .attr("data-1p-ignore", "true")
            .class("input input--transparent full-width")
            .placeholder("Filter…")
            .data("action", "input->filter#filter"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_initials_like_ruby() {
        assert_eq!(initials("Élodie Ünal-Smith o'Brien 3po _x ñ"), "SoB3_");
        assert_eq!(initials("David Heinemeier Hansson"), "DHH");
    }

    #[test]
    fn crc32_matches_zlib() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}

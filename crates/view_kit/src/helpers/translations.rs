//! `TranslationsHelper`: the language popups beside form fields.

use super::html::{Html, Safe};
use super::tag::{attrs, content_tag, content_tag_text};
use super::translations_table::TRANSLATIONS;

/// `translations_for(key)`.
pub fn translations_for(key: &str) -> Html {
    let entries = TRANSLATIONS
        .iter()
        .find(|(name, _)| *name == key)
        .unwrap_or_else(|| panic!("unknown translation key {key}"))
        .1;
    let items: String = entries
        .iter()
        .map(|(language, translation)| {
            format!(
                "{}{}",
                content_tag_text("dt", attrs(), language).0,
                content_tag_text("dd", attrs().class("margin-none"), translation).0
            )
        })
        .collect();
    content_tag("dl", attrs().class("language-list"), &items)
}

/// The `<details>` popup around an already-rendered globe image.
///
/// Each shell passes its own `image_tag` output. The markup matches the classic button,
/// including the Stimulus popup actions the unsupported-browser page still prints.
pub fn translation_popup(globe: &str, key: &str) -> Html {
    let summary = content_tag(
        "summary",
        attrs().class("btn").tabindex(-1),
        &format!(
            "{globe}{}",
            content_tag_text("span", attrs().class("for-screen-reader"), "Translate").0
        ),
    );
    let menu = content_tag(
        "div",
        attrs()
            .class("language-list-menu shadow")
            .data("popup_target", "menu"),
        &translations_for(key).0,
    );
    let details = attrs()
        .class("position-relative")
        .data("controller", "popup")
        .data(
            "action",
            "keydown.esc->popup#close toggle->popup#toggle click@document->popup#closeOnClickOutside",
        )
        .data("popup_orientation_top_class", "popup-orientation-top");
    Safe(content_tag("details", &details, &format!("{}{}", summary.0, menu.0)).0)
}

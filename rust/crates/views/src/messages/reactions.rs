//! `Boost.reaction?` and `BoostsHelper`: pinned static registry facts plus a domain icon source.
use super::{REACTIONS, ReactionContent};
use crate::helpers::{AvatarIcon, IconSource};
use regex::Regex;
use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Deserialize)]
struct Registry {
    emoji_names: HashMap<String, String>,
    shortcodes: HashMap<String, ReactionContent>,
}
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../data/message-icons.json"))
        .expect("Rails message registry")
});
static SHORTCODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A:([a-z0-9_]+):\z").unwrap());

pub fn static_icon(name: &str) -> Option<AvatarIcon> {
    REGISTRY
        .shortcodes
        .get(name)
        .and_then(|row| row.icon.clone())
}
pub fn static_icon_name(name: &str) -> Option<String> {
    REGISTRY
        .shortcodes
        .get(name)?
        .icon_alt
        .as_ref()
        .map(|alt| alt.trim_matches(':').to_string())
}

pub fn shortcode_name(content: &str) -> Option<&str> {
    SHORTCODE
        .is_match(content)
        .then(|| &content[1..content.len() - 1])
}

pub fn reaction_title(content: &str) -> String {
    REACTIONS
        .iter()
        .find(|(emoji, _)| *emoji == content)
        .map(|(_, title)| title.to_string())
        .or_else(|| {
            static SKIN_TONE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"[\x{1F3FB}-\x{1F3FF}]").unwrap());
            REGISTRY
                .emoji_names
                .get(content)
                .or_else(|| {
                    REGISTRY
                        .emoji_names
                        .get(SKIN_TONE.replace(content, "").as_ref())
                })
                .cloned()
        })
        .unwrap_or_else(|| content.to_string())
}

/// Ruby checks one grapheme, including flag pairs and keycaps, before Unicode emoji properties.
pub fn single_emoji(content: &str) -> bool {
    if content.graphemes(true).count() != 1 {
        return false;
    }
    static FLAG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A\p{Regional_Indicator}{2}\z").unwrap());
    static KEYCAP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A[0-9#*]\x{FE0F}?\x{20E3}\z").unwrap());
    static PRESENTATION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[\p{Emoji_Presentation}\p{Extended_Pictographic}]").unwrap());
    static EMOJI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{Emoji}").unwrap());
    static MODIFIER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{Emoji_Modifier}").unwrap());
    if FLAG.is_match(content) || KEYCAP.is_match(content) || PRESENTATION.is_match(content) {
        return true;
    }
    if !EMOJI.is_match(content) {
        return false;
    }
    if content.contains('\u{200D}') {
        return true;
    }
    if content.contains('\u{FE0F}') {
        let base = content.replace('\u{FE0F}', "");
        return EMOJI.is_match(&base)
            && !matches!(
                base.as_str(),
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "#" | "*"
            );
    }
    MODIFIER.is_match(content) && EMOJI.is_match(&MODIFIER.replace_all(content, ""))
}

pub fn resolve(content: &str, source: &dyn IconSource) -> Option<ReactionContent> {
    let trimmed = content.trim();
    if !single_emoji(trimmed)
        && !shortcode_name(trimmed).is_some_and(|name| source.resolve_avatar_icon(name).is_some())
    {
        return None;
    }
    let icon = shortcode_name(content).and_then(|name| source.resolve_avatar_icon(name));
    let title = match &icon {
        Some(AvatarIcon::Emoji { title, .. } | AvatarIcon::Image { title, .. }) => title.clone(),
        None => reaction_title(content),
    };
    let icon_alt = shortcode_name(content).and_then(|name| match &icon {
        Some(AvatarIcon::Image { brand: true, .. }) => REGISTRY
            .shortcodes
            .get(name)
            .and_then(|row| row.icon_alt.clone()),
        Some(AvatarIcon::Image { brand: false, .. }) => Some(format!(":{name}:")),
        _ => None,
    });
    Some(ReactionContent {
        title,
        icon,
        icon_alt,
    })
}

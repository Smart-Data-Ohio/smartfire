//! Unicode emoji validation shared by reactions and the voice soundboard.

use std::sync::LazyLock;

use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

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

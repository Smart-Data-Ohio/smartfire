

/// `Users::AvatarsHelper::AVATAR_COLORS`.
pub const AVATAR_COLORS: [&str; 18] = [
    "#AF2E1B", "#CC6324", "#3B4B59", "#BFA07A", "#ED8008", "#ED3F1C", "#BF1B1B", "#736B1E",
    "#D07B53", "#736356", "#AD1D1D", "#BF7C2A", "#C09C6F", "#698F9C", "#7C956B", "#5D618F",
    "#3B3633", "#67695E",
];

/// What `avatar_tag` needs to know about a user.
#[derive(Clone, Debug, Default)]
pub struct AvatarUser {
    pub id: i64,
    /// `User#title`.
    pub title: String,
    /// `fresh_user_avatar_path(user)`.
    pub avatar_path: String,
}

/// `avatar_background_color(user)`: `Zlib.crc32(user.to_param)` picks the color.
pub fn avatar_background_color(user_id: impl std::borrow::Borrow<i64>) -> &'static str {
    let user_id = *user_id.borrow();
    let crc = crc32(user_id.to_string().as_bytes());
    AVATAR_COLORS[(crc % AVATAR_COLORS.len() as u32) as usize]
}

/// Zlib's CRC-32 (IEEE, reflected).
pub fn crc32(bytes: &[u8]) -> u32 {
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
    static WORD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\A[\p{Alphabetic}\p{Mark}\p{Number}\p{Connector_Punctuation}\p{Join_Control}]\z").unwrap()
    });
    let mut initials = String::new();
    let mut previous_word = false;
    for c in name.chars() {
        if (c.is_ascii_alphanumeric() || c == '_') && !previous_word {
            initials.push(c);
        }
        previous_word = WORD.is_match(c.encode_utf8(&mut [0; 4]));
    }
    initials
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

/// The bot curl commands in `accounts/bots/_bot`.
pub fn curl_text_line(url: impl AsRef<str>) -> String {
    format!("curl -d 'Hello!' {}", url.as_ref())
}

pub fn curl_upload_line(url: impl AsRef<str>) -> String {
    format!("curl -F \"attachment=@/path/to/file\" {}", url.as_ref())
}



/// `truncate(text, length:, omission:)` with Rails' default of no separator: the result,
/// omission included, is at most `length` characters. Returns plain text (escape on output).
pub fn truncate(text: &str, length: usize, omission: &str) -> String {
    if text.chars().count() <= length {
        return text.to_string();
    }
    let keep = length.saturating_sub(omission.chars().count());
    let mut out: String = text.chars().take(keep).collect();
    out.push_str(omission);
    out
}

pub mod base64_url {
    /// `Base64.urlsafe_encode64` (padded).
    pub fn urlsafe_encode64(input: &str) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let bytes = input.as_bytes();
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }
}

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

//! Active Support's string predicates and cleanups the views lean on.

/// `String#blank?`: empty or only whitespace (`/\A[[:space:]]*\z/`, Unicode's White_Space).
pub fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

/// `String#presence`: the string unless it's blank.
pub fn presence(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !is_blank(text))
}

/// `String#squish`: runs of whitespace (`[[:space:]]+`) become one space, then `strip`, which also
/// drops the NULs Ruby strips from the ends.
pub fn squish(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            in_space = true;
        } else {
            if in_space {
                out.push(' ');
                in_space = false;
            }
            out.push(ch);
        }
    }
    if in_space {
        out.push(' ');
    }
    out.trim_matches(|ch: char| matches!(ch, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '))
        .to_string()
}

/// `String#capitalize`: first character upcased, the rest downcased.
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// `Array#to_sentence` with the default English connectors, or a custom `two_words_connector`.
pub fn to_sentence(items: &[String], two_words_connector: &str) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one}{two_words_connector}{two}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_and_squish_follow_active_support() {
        assert!(is_blank(""));
        assert!(is_blank(" \t\u{3000}\u{a0}"));
        assert!(!is_blank(" a "));
        assert_eq!(presence(Some("  ")), None);
        assert_eq!(presence(Some(" x")), Some(" x"));
        assert_eq!(squish(" pizza  night "), "pizza night");
        assert_eq!(squish("\u{3000}a\u{a0}\u{2003}b\n"), "a b");
        assert_eq!(squish("   "), "");
    }

    #[test]
    fn builds_sentences() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(to_sentence(&names(&["A", "B"]), "+"), "A+B");
        assert_eq!(to_sentence(&names(&["A", "B", "C"]), "+"), "A, B, and C");
        assert_eq!(to_sentence(&names(&["A"]), " and "), "A");
        assert_eq!(to_sentence(&names(&["A", "B"]), " and "), "A and B");
    }
}

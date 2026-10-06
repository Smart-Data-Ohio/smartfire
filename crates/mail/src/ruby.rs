//! The reference uses String#strip / #split(/\s/) (ASCII) and ActiveSupport#blank?
//! (Unicode POSIX space) in different places. Do not substitute Unicode trim for strip.
pub fn regex_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}
pub fn strip(value: &str) -> &str {
    value.trim_matches(|c| c == '\0' || regex_space(c))
}
pub fn blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

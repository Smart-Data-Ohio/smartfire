//! Record cache keys and the message presentation version.

/// Rails PR #148 (`fix/cached-fragment-csrf`), bumped when message HTML changes so message-page
/// ETags from before the change stop matching. 4: mentions became inline chips without the
/// classic profile-card controls.
pub const PRESENTATION_CACHE_VERSION: i64 = 4;

/// `Time#to_fs(:usec)` of a record's `updated_at`: its `cache_version`.
pub fn cache_version(updated_at: jiff::Timestamp) -> String {
    format!(
        "{}{:06}",
        updated_at.strftime("%Y%m%d%H%M%S"),
        updated_at.subsec_microsecond()
    )
}

/// `record.cache_key_with_version`: `"messages/1-20240601120000000000"`.
pub fn cache_key_with_version(table: &str, id: i64, updated_at: jiff::Timestamp) -> String {
    format!("{table}/{id}-{}", cache_version(updated_at))
}

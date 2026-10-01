//! The reference's escaped literal keyword pattern, using its pinned Ruby regex engine.
use std::ffi::{c_int, c_void};
use std::sync::{Mutex, OnceLock};

static ENGINE: Mutex<()> = Mutex::new(());
static INITIALIZED: OnceLock<c_int> = OnceLock::new();
unsafe extern "C" {
    fn campfire_keyword_regex_init() -> c_int;
    fn campfire_keyword_regex_new(
        pattern: *const u8,
        length: usize,
        error: *mut c_int,
    ) -> *mut c_void;
    fn campfire_keyword_regex_match(regex: *mut c_void, text: *const u8, length: usize) -> c_int;
    fn campfire_keyword_regex_free(regex: *mut c_void);
}

/// `Notifications::KeywordMatcher.compile(phrase).match?(text)` after strip/downcase.
/// Only literal phrases are exposed; callers cannot supply regex syntax.
pub fn is_match(phrase: &str, text: &str) -> Result<bool, String> {
    let mut body = String::new();
    for c in phrase.chars() {
        match c {
            ' ' => body.push_str("\\s+"),
            '\\' | '.' | '+' | '*' | '?' | '[' | ']' | '^' | '$' | '(' | ')' | '{' | '}' | '|'
            | '-' => {
                body.push('\\');
                body.push(c);
            }
            c => body.push(c),
        }
    }
    let pattern = format!("(?<![\\p{{L}}\\p{{N}}_]){body}(?![\\p{{L}}\\p{{N}}_])");
    let _guard = ENGINE.lock().map_err(|e| e.to_string())?;
    // SAFETY: all engine calls are serialized, its initialization runs once, all
    // buffers remain alive for the explicit lengths, and the regex is freed before
    // leaving this scope. The engine never retains a Rust buffer after these calls.
    unsafe {
        let init = *INITIALIZED.get_or_init(|| campfire_keyword_regex_init());
        if init != 0 {
            return Err(format!("Ruby regex initialization: {init}"));
        }
        let mut error = 0;
        let regex = campfire_keyword_regex_new(pattern.as_ptr(), pattern.len(), &mut error);
        if error != 0 {
            return Err(format!("Ruby keyword regex compilation: {error}"));
        }
        if regex.is_null() {
            return Err("Ruby keyword regex allocation failed".into());
        }
        let result = campfire_keyword_regex_match(regex, text.as_ptr(), text.len());
        campfire_keyword_regex_free(regex);
        match result {
            1 => Ok(true),
            -1 => Ok(false),
            other => Err(format!("Ruby keyword regex search: {other}")),
        }
    }
}

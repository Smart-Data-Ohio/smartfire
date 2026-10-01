//! Pure `Slack::MarkdownConverter` from `app/models/slack/markdown_converter.rb`.
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SOURCE_LIMIT: usize = 50_000;
const OMISSION: &str = "… (import truncated)";
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Converted {
    pub markdown: String,
    pub truncated: bool,
    pub files_linked: usize,
}
struct Patterns {
    code: Regex,
    mail_label: Regex,
    mail: Regex,
    link_label: Regex,
    link: Regex,
    user: Regex,
    channel_label: Regex,
    channel: Regex,
    broadcast: Regex,
    subteam_label: Regex,
    subteam: Regex,
    date: Regex,
    shield_link: Regex,
    shield_url: Regex,
    bullet: Regex,
    skin: Regex,
}
static PATTERNS: LazyLock<Patterns> = LazyLock::new(|| {
    let re = |pattern| Regex::new(pattern).expect("constant Slack regexp");
    Patterns {
        code: re(r"(?s)```.*?```|`[^`\n]*`"),
        mail_label: re(r"<mailto:([^>|]+)\|([^>]+)>"),
        mail: re(r"<mailto:([^>]+)>"),
        link_label: re(r"<(https?://[^|>]+)\|([^>]+)>"),
        link: re(r"<(https?://[^>]+)>"),
        user: re(r"<@([A-Z0-9]+)(?:\|([^>]+))?>"),
        channel_label: re(r"<#[A-Z0-9]+\|([^>]+)>"),
        channel: re(r"<#([A-Z0-9]+)>"),
        broadcast: re(r"<!(here|channel|everyone)(?:\|[^>]+)?>"),
        subteam_label: re(r"<!subteam\^[A-Z0-9]+\|@?([^>]+)>"),
        subteam: re(r"<!subteam\^[A-Z0-9]+>"),
        date: re(r"<!date\^[^\s|>]+(?:\|([^>]*))?>"),
        shield_link: re(r"\[[^\]\n]*\]\([^)\n]*\)"),
        shield_url: re(r"https?://[^\s<>\]]+"),
        bullet: re(r"(?m)^([\t\n\r\x0b\x0c ]*)• "),
        skin: re(r"::skin-tone-[0-9]+"),
    }
});
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
fn unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
fn present(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => false,
        Value::String(s) => !s.chars().all(char::is_whitespace),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
        _ => true,
    }
}
// Ruby String#strip removes ASCII space/control characters, rather than Unicode whitespace.
fn strip(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '))
}
fn array(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.iter().collect(),
        _ => vec![value],
    }
}
fn blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

pub fn convert(message: &Value, users: &HashMap<String, String>) -> Converted {
    let mut source = unescape(&text(&message["text"]));
    let attachments: Vec<_> = array(&message["attachments"])
        .into_iter()
        .filter_map(|attachment| {
            let mut parts = Vec::new();
            for key in ["pretext", "text", "fallback"] {
                let part = strip(&unescape(&text(&attachment[key]))).to_owned();
                if !blank(&part) && !parts.contains(&part) {
                    parts.push(part);
                }
            }
            (!parts.is_empty()).then(|| parts.join("\n"))
        })
        .collect();
    if !attachments.is_empty()
        && (blank(strip(&source))
            || message["subtype"] == "bot_message"
            || present(&message["bot_id"]))
    {
        let mut parts = Vec::new();
        if !blank(strip(&source)) {
            parts.push(strip(&source).to_owned());
        }
        for quoted in attachments {
            parts.push(
                quoted
                    .split_inclusive('\n')
                    .map(|line| {
                        let line = strip(line);
                        if blank(line) {
                            ">".to_owned()
                        } else {
                            format!("> {line}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        source = parts.join("\n\n");
    }
    let mut markdown = String::new();
    let mut end = 0;
    for segment in PATTERNS.code.find_iter(&source) {
        markdown.push_str(&prose(&source[end..segment.start()], users));
        let code = segment.as_str();
        if let Some(body) = code.strip_prefix("```") {
            let body = body.strip_suffix("```").expect("matched closing fence");
            markdown.push_str("```");
            if !body.starts_with('\n') {
                markdown.push('\n');
            }
            markdown.push_str(body);
            if !body.is_empty() && !body.ends_with('\n') {
                markdown.push('\n');
            }
            markdown.push_str("```");
        } else {
            markdown.push_str(code);
        }
        end = segment.end();
    }
    markdown.push_str(&prose(&source[end..], users));
    if message["subtype"] == "me_message" && !blank(strip(&markdown)) {
        markdown = markdown
            .split_inclusive('\n')
            .map(|line| {
                if blank(strip(line)) {
                    line.to_owned()
                } else {
                    format!("*{}*", strip(line))
                }
            })
            .collect();
    }
    let files: Vec<_> = array(&message["files"])
        .into_iter()
        .filter_map(|file| {
            let name = ["name", "title"]
                .into_iter()
                .find_map(|key| present(&file[key]).then(|| text(&file[key])))
                .unwrap_or_else(|| "file".into());
            let url = ["permalink", "permalink_public", "url_private"]
                .into_iter()
                .find_map(|key| present(&file[key]).then(|| text(&file[key])));
            url.map(|url| format!("📎 [{name}]({url})"))
        })
        .collect();
    let files_linked = files.len();
    if files_linked > 0 {
        let mut parts = Vec::new();
        if !blank(strip(&markdown)) {
            parts.push(strip(&markdown).to_owned());
        }
        parts.extend(files);
        markdown = parts.join("\n\n");
    }
    let truncated = markdown.chars().count() > SOURCE_LIMIT;
    if truncated {
        markdown = markdown
            .chars()
            .take(SOURCE_LIMIT - OMISSION.chars().count())
            .chain(OMISSION.chars())
            .collect();
    }
    Converted {
        markdown,
        truncated,
        files_linked,
    }
}
fn prose(s: &str, users: &HashMap<String, String>) -> String {
    let p = &*PATTERNS;
    let s = p.mail_label.replace_all(s, "[$2](mailto:$1)");
    let s = p.mail.replace_all(&s, "$1");
    let s = p.link_label.replace_all(&s, "[$2]($1)");
    let s = p.link.replace_all(&s, "$1");
    let s = p.user.replace_all(&s, |c: &Captures<'_>| {
        if let Some(name) = users
            .get(&c[1])
            .filter(|name| !blank(name) && !name.contains(['[', ']', '\r', '\n']))
        {
            format!("@[{name}]")
        } else {
            format!("@{}", c.get(2).map_or(&c[1], |m| m.as_str()))
        }
    });
    let s = p.channel_label.replace_all(&s, "#$1");
    let s = p.channel.replace_all(&s, "#$1");
    let s = p.broadcast.replace_all(&s, "@$1");
    let s = p.subteam_label.replace_all(&s, "@$1");
    let s = p.subteam.replace_all(&s, "@group");
    let s = p.date.replace_all(&s, "$1");
    let mut shields = Vec::new();
    let mut shield = |c: &Captures<'_>| {
        let key = format!("SMARTFIRESLACKLINK{}END", shields.len());
        shields.push((key.clone(), c[0].to_owned()));
        key
    };
    let s = p.shield_link.replace_all(&s, &mut shield);
    let s = p.shield_url.replace_all(&s, &mut shield);
    let s = emphasis(&s, b'*');
    let s = emphasis(&s, b'_');
    let mut s = emphasis(&s, b'~');
    for (key, link) in shields {
        s = s.replace(&key, &link);
    }
    let s = p.bullet.replace_all(&s, "$1- ");
    p.skin.replace_all(&s, "").into_owned()
}
// Linear scan with Ruby's lookaround boundaries. A rejected candidate resumes after its
// opening delimiter, so a later candidate sharing that closing delimiter is still considered.
fn emphasis(s: &str, delimiter: u8) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut emitted = 0;
    let mut i = 0;
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    while i < bytes.len() {
        if bytes[i] != delimiter
            || (i > 0
                && match delimiter {
                    b'*' => bytes[i - 1] == delimiter,
                    b'_' => word(bytes[i - 1]),
                    _ => false,
                })
        {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != delimiter && bytes[j] != b'\n' {
            j += 1;
        }
        if j > i + 1
            && j < bytes.len()
            && bytes[j] == delimiter
            && !(j + 1 < bytes.len()
                && match delimiter {
                    b'*' => bytes[j + 1] == delimiter,
                    b'_' => word(bytes[j + 1]),
                    _ => false,
                })
        {
            out.push_str(&s[emitted..i]);
            let marker = match delimiter {
                b'*' => "**",
                b'_' => "*",
                _ => "~~",
            };
            out.push_str(marker);
            out.push_str(&s[i + 1..j]);
            out.push_str(marker);
            i = j + 1;
            emitted = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[emitted..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slack_markdown_matches_rails_corpus_and_generated_vectors() {
        let corpus: Value =
            serde_json::from_str(include_str!("../../../../../vectors/slack/markdown.json"))
                .unwrap();
        let users = serde_json::from_value(corpus["users"].clone()).unwrap();
        for (index, entry) in corpus["cases"].as_array().unwrap().iter().enumerate() {
            let mut message = entry["message"].clone();
            if let Some(repeat) = entry.get("repeat") {
                message["text"] = Value::String(
                    repeat["unit"]
                        .as_str()
                        .unwrap()
                        .repeat(repeat["count"].as_u64().unwrap() as usize),
                );
            }
            let expected: Converted = serde_json::from_value(entry["expected"].clone()).unwrap();
            assert_eq!(
                convert(&message, &users),
                expected,
                "Rails vector {index}, message {message}"
            );
        }
    }
    #[test]
    fn slack_markdown_crafted_date_has_linear_runtime() {
        let start = std::time::Instant::now();
        let converted = convert(
            &serde_json::json!({"text": format!("<!date^{}", "!^".repeat(50_000))}),
            &HashMap::new(),
        );
        assert!(start.elapsed().as_secs_f64() < 0.5);
        assert!(converted.markdown.starts_with("<!date^!^"));
    }
}

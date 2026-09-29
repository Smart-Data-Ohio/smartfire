//! What the models need from Action Text. The real pipeline lives in `campfire_richtext`;
//! the app plugs it in through [`RichText`]. [`BasicRichText`] is a stand-in good enough
//! for this crate's tests.

use base64::Engine;
use rusqlite::Connection;

/// Looks up a user's name by id, for rendering mentions as `@Name`.
pub type UserNames<'a> = &'a dyn Fn(i64) -> Option<String>;

/// Both methods get the connection the caller is already on (the writer's transaction, or the
/// reader it holds) for any record lookups: they must never check out another connection, which
/// deadlocks once every pooled reader waits on the writer.
pub trait RichText: Send + Sync {
    /// `ActionText::Content#to_plain_text` of a stored body. Mention attachments render as
    /// `attachable_plain_text_representation`, i.e. `"@#{name}"`
    /// (`reference/app/models/user/mentionable.rb`), hence the name lookup.
    fn to_plain_text(&self, conn: &Connection, html: &str, user_names: UserNames<'_>) -> String;

    /// `body.attachables.grep(User).uniq`: user ids from mention attachments, in document
    /// order, deduplicated (`reference/app/models/message/mentionee.rb`).
    fn mentioned_user_ids(&self, conn: &Connection, html: &str) -> Vec<i64>;

    /// `Message::Markdown.render(source, room:)`: the Action Text body a Markdown source renders
    /// to, with `@[Name]` tokens resolved against the room's members. The app plugs in
    /// `campfire_richtext::markdown::render` (WS5); the default is [`BasicRichText`]'s stand-in,
    /// which escapes the source into paragraphs and resolves mentions the same way.
    fn render_markdown(&self, conn: &Connection, source: &str, room_id: i64) -> Result<String, String> {
        Ok(basic_markdown(conn, source, room_id))
    }

    /// `Message::Markdown.plain_text(body)`: the plain text of a Markdown-rendered body. The
    /// default reads it like any other body.
    fn markdown_plain_text(&self, conn: &Connection, html: &str, user_names: UserNames<'_>) -> String {
        self.to_plain_text(conn, html, user_names)
    }
}

/// `Message::Markdown::MENTION_TOKEN_PATTERN` (`/(?<!\\)@\[(?<name>[^\[\]\r\n]+)\]/`): the
/// names of the `@[Name]` tokens in `source`, in order, repeats included.
pub fn mention_token_names(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut names = Vec::new();
    let mut i = 0;
    while let Some(offset) = source[i..].find("@[") {
        let start = i + offset;
        i = start + 2;
        if start > 0 && bytes[start - 1] == b'\\' {
            continue;
        }
        let rest = &source[start + 2..];
        let Some(end) = rest.find(['[', ']', '\r', '\n']) else { break };
        if end > 0 && rest.as_bytes()[end] == b']' {
            names.push(rest[..end].to_string());
            i = start + 2 + end + 1;
        }
    }
    names
}

/// The stand-in Markdown renderer: each paragraph escaped into a `<p>`, with `@[Name]` naming
/// exactly one member of the room turned into a mention attachment.
fn basic_markdown(conn: &Connection, source: &str, room_id: i64) -> String {
    let member_id = |name: &str| -> Option<i64> {
        let mut stmt = conn
            .prepare_cached(r#"SELECT "users"."id" FROM "users" INNER JOIN "memberships" ON "memberships"."user_id" = "users"."id" WHERE "memberships"."room_id" = ? AND "users"."name" = ? AND "users"."status" = 0"#)
            .ok()?;
        let ids: Vec<i64> = stmt.query_map(rusqlite::params![room_id, name], |r| r.get(0)).ok()?.filter_map(|r| r.ok()).collect();
        (ids.len() == 1).then(|| ids[0])
    };
    source
        .split("\n\n")
        .filter(|paragraph| !paragraph.trim().is_empty())
        .map(|paragraph| {
            let mut html = escape(paragraph);
            for name in mention_token_names(paragraph) {
                if let Some(id) = member_id(&name) {
                    html = html.replacen(&format!("@[{}]", escape(&name)), &mention_attachment_for(id), 1);
                }
            }
            format!("<p>{html}</p>")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Tag stripping plus unverified SGID decoding. Campfire accepts unverified user SGIDs
/// anyway (`reference/lib/rails_ext/action_text_attachables.rb`); signature checks and the
/// exact plain-text rules belong to `campfire_richtext`.
#[derive(Debug, Default, Clone, Copy)]
pub struct BasicRichText;

impl RichText for BasicRichText {
    fn to_plain_text(&self, _conn: &Connection, html: &str, user_names: UserNames<'_>) -> String {
        let mut out = String::new();
        let mut rest = html;
        while let Some(start) = rest.find('<') {
            out.push_str(&rest[..start]);
            let Some(end) = rest[start..].find('>') else {
                rest = "";
                break;
            };
            let tag = &rest[start + 1..start + end];
            let closing = tag.starts_with('/');
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if name == "action-text-attachment" && !closing {
                let mention = attribute_values(tag, "sgid")
                    .first()
                    .and_then(|sgid| user_id_from_sgid(sgid))
                    .and_then(user_names);
                if let Some(name) = mention {
                    out.push('@');
                    out.push_str(&name);
                }
                // Skip the attachment's inner content.
                if let Some(close) = rest[start..].find("</action-text-attachment>") {
                    rest = &rest[start + close + "</action-text-attachment>".len()..];
                    continue;
                }
            }
            if matches!(
                name.as_str(),
                "br" | "p" | "div" | "li" | "h1" | "blockquote" | "pre"
            ) && !out.is_empty()
                && !closing
            {
                out.push('\n');
            }
            rest = &rest[start + end + 1..];
        }
        out.push_str(rest);
        decode_entities(out.trim())
    }

    fn mentioned_user_ids(&self, _conn: &Connection, html: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for sgid in attribute_values(html, "sgid") {
            if let Some(id) = user_id_from_sgid(&sgid)
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
        ids
    }
}

fn attribute_values(html: &str, name: &str) -> Vec<String> {
    let needle = format!("{name}=\"");
    let mut values = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(&needle) {
        let after = &rest[start + needle.len()..];
        let Some(end) = after.find('"') else { break };
        values.push(after[..end].to_string());
        rest = &after[end..];
    }
    values
}

/// Reads `gid://campfire/User/<id>` out of an SGID's message without verifying it.
pub fn user_id_from_sgid(sgid: &str) -> Option<i64> {
    let message = sgid.split("--").next()?;
    let message = message
        .replace("%3D", "=")
        .replace("%2B", "+")
        .replace("%2F", "/");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(message.as_bytes())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(message.as_bytes()))
        .ok()?;
    let text = String::from_utf8_lossy(&decoded);
    let marker = "gid://campfire/User/";
    let start = text.find(marker)? + marker.len();
    let digits: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// A mention attachment for tests, shaped like `MentionTestHelper#mention_attachment_for`
/// but with an unsigned SGID.
pub fn mention_attachment_for(user_id: i64) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(format!(
        r#"{{"_rails":{{"data":"gid://campfire/User/{user_id}","pur":"attachable"}}}}"#
    ));
    format!(
        r#"<action-text-attachment sgid="{payload}--unsigned" content-type="application/vnd.campfire.mention"></action-text-attachment>"#
    )
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn no_users(_: i64) -> Option<String> {
        None
    }

    #[test]
    fn plain_text_strips_tags() {
        assert_eq!(
            BasicRichText.to_plain_text(&memory(), "<span>My hovercraft is full of eels</span>", &no_users),
            "My hovercraft is full of eels"
        );
        assert_eq!(
            BasicRichText.to_plain_text(&memory(), "Hello <b>there</b>", &no_users),
            "Hello there"
        );
    }

    #[test]
    fn plain_text_renders_mentions() {
        let html = format!("Hey {}", mention_attachment_for(7));
        assert_eq!(
            BasicRichText.to_plain_text(&memory(), &html, &|id| (id == 7).then(|| "Kevin".to_string())),
            "Hey @Kevin"
        );
    }

    #[test]
    fn mention_tokens_skip_escaped_and_bracketed_names() {
        assert_eq!(mention_token_names("hi @[Jason Fried] and \\@[Nope] and @[David]"), vec!["Jason Fried", "David"]);
        assert_eq!(mention_token_names("@[a[b] @[]"), Vec::<String>::new());
        assert_eq!(mention_token_names("@[Line\nbreak] @[Kevin]"), vec!["Kevin"]);
    }

    #[test]
    fn mentioned_user_ids_from_sgids() {
        let html = format!(
            "<div>Hey {} {}</div>",
            mention_attachment_for(127326141),
            mention_attachment_for(127326141)
        );
        assert_eq!(BasicRichText.mentioned_user_ids(&memory(), &html), vec![127326141]);
    }
}

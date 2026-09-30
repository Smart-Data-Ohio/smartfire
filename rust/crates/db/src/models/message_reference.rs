//! Our `MessageReference`, `Message::ReferenceSync` and quote-card refresh job.
use crate::broadcasts::{Broadcast, Partial, conversation_messages, message_dom_id};
use crate::sql::{exists, query_all};
use crate::{CachedStatements, Connection, Errors, Event, Message, Result, Tx};
use campfire_richtext::dom::{Dom, NodeId};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

pub const MAX_PER_MESSAGE: usize = 10;
pub const REFRESH_MAXIMUM: usize = 200;
pub const REFRESH_BATCH: usize = 100;
static PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"/rooms/[0-9]+/@([0-9]+)\b").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteCardsRefreshJob {
    pub source_message_id: i64,
}
impl crate::Job for QuoteCardsRefreshJob {
    const CLASS: &'static str = "Message::QuoteCardsRefreshJob";
}

pub fn extract_message_ids(text: &str) -> Vec<i64> {
    // Count arbitrary-size Ruby integer ids toward the cap even when SQLite cannot store them.
    let mut unique = Vec::new();
    for capture in PATTERN.captures_iter(text) {
        let digits = capture[1].trim_start_matches('0');
        let digits = if digits.is_empty() { "0" } else { digits };
        if !unique.iter().any(|s| s == digits) {
            unique.push(digits.to_string());
            if unique.len() == MAX_PER_MESSAGE {
                break;
            }
        }
    }
    unique.iter().filter_map(|id| id.parse().ok()).collect()
}

pub fn non_code_text(html: &str) -> Result<String> {
    const BLOCKS: &[&str] = &[
        "address",
        "article",
        "aside",
        "blockquote",
        "dd",
        "dialog",
        "div",
        "dl",
        "dt",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "header",
        "hgroup",
        "hr",
        "li",
        "main",
        "nav",
        "ol",
        "p",
        "section",
        "table",
        "td",
        "th",
        "tr",
        "ul",
    ];
    fn walk(dom: &Dom, node: NodeId, text: &mut String, hrefs: &mut Vec<String>) {
        let name = dom.local_name(node).unwrap_or("");
        if matches!(name, "code" | "pre") {
            return;
        }
        if let Some(value) = dom.text(node) {
            text.push_str(value);
        }
        if name == "a"
            && let Some(href) = dom.attr(node, "href")
        {
            hrefs.push(href.to_owned());
        }
        if name == "br" {
            text.push('\n');
        }
        for child in dom.children(node) {
            walk(dom, *child, text, hrefs);
        }
        if BLOCKS.contains(&name) {
            text.push('\n');
        }
    }
    let mut dom = Dom::new();
    let root = dom
        .parse_fragment(html)
        .map_err(|e| crate::Error::Other(e.to_string()))?;
    let mut text = String::new();
    let mut hrefs = Vec::new();
    walk(&dom, root, &mut text, &mut hrefs);
    for href in hrefs {
        text.push('\n');
        text.push_str(&href);
    }
    Ok(text)
}

pub fn create(tx: &Tx<'_>, message_id: i64, referenced_message_id: i64) -> Result<i64> {
    let mut errors = Errors::default();
    if Message::find_by_id(tx.conn(), message_id)?.is_none() {
        errors.add("message", "must exist");
    }
    if Message::find_by_id(tx.conn(), referenced_message_id)?.is_none() {
        errors.add("referenced_message", "must exist");
    }
    if message_id == referenced_message_id {
        errors.add("referenced_message", "can't quote its own message");
    }
    if exists(
        tx.conn(),
        "SELECT 1 FROM message_references WHERE message_id=? AND referenced_message_id=?",
        params![message_id, referenced_message_id],
    )? {
        errors.add("referenced_message_id", "has already been taken");
    }
    errors.into_result()?;
    Ok(tx.conn().query_row_cached("INSERT INTO message_references (message_id,referenced_message_id,created_at,updated_at) VALUES (?,?,?,?) RETURNING id",params![message_id,referenced_message_id,tx.now(),tx.now()],|r|r.get(0))?)
}

pub fn sync(tx: &Tx<'_>, message: &Message) -> Result<()> {
    let mut text = non_code_text(&message.body_html(tx.conn())?.unwrap_or_default())?;
    if let Some(note) = &message.forward_note {
        text.push('\n');
        text.push_str(note);
    }
    let mut sources = Vec::new();
    for id in extract_message_ids(&text) {
        if id != message.id && Message::find_by_id(tx.conn(), id)?.is_some_and(|m| !m.system_note) {
            sources.push(id);
        }
    }
    for id in referenced_ids(tx.conn(), message.id)? {
        if !sources.contains(&id) {
            tx.conn().execute_cached(
                "DELETE FROM message_references WHERE message_id=? AND referenced_message_id=?",
                params![message.id, id],
            )?;
        }
    }
    for id in sources {
        if !exists(
            tx.conn(),
            "SELECT 1 FROM message_references WHERE message_id=? AND referenced_message_id=?",
            params![message.id, id],
        )? {
            create(tx, message.id, id)?;
        }
    }
    Ok(())
}

pub fn referenced_ids(conn: &Connection, message_id: i64) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT referenced_message_id FROM message_references WHERE message_id=? ORDER BY referenced_message_id",
        [message_id],
        |r| r.get(0),
    )
}
pub fn incoming_ids(conn: &Connection, source: i64) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT message_id FROM message_references WHERE referenced_message_id=? ORDER BY id",
        [source],
        |r| r.get(0),
    )
}
pub fn broadcast_cards(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    tx.emit_after_commit(Event::broadcast(&Broadcast::replace_keeping_scroll(
        conversation_messages(tx.conn(), message)?,
        message_dom_id(message, Some("message_link_cards")),
        Partial::QuoteCards {
            message_id: message.id,
        },
    )));
    Ok(())
}
pub fn refresh_quote_cards(
    tx: &mut Tx<'_>,
    source: i64,
    maximum: usize,
    batch: usize,
) -> Result<usize> {
    if batch == 0 {
        return Err(crate::Error::Other("batch size must be positive".into()));
    }
    let mut ids: Vec<i64> = query_all(
        tx.conn(),
        "SELECT message_id FROM message_references WHERE referenced_message_id=? ORDER BY id LIMIT ?",
        params![source, maximum as i64],
        |r| r.get(0),
    )?;
    ids.sort();
    ids.dedup();
    let mut count = 0;
    for chunk in ids.chunks(batch) {
        for id in chunk {
            if let Some(message) = Message::find_by_id(tx.conn(), *id)? {
                broadcast_cards(tx, &message)?;
                count += 1;
            }
        }
    }
    Ok(count)
}
pub fn removed_source(tx: &mut Tx<'_>, mut ids: Vec<i64>) -> Result<()> {
    ids.sort();
    ids.dedup();
    for id in ids {
        tx.conn().execute_cached(
            "UPDATE messages SET updated_at=? WHERE id=?",
            params![tx.now(), id],
        )?;
        if let Some(message) = Message::find_by_id(tx.conn(), id)? {
            broadcast_cards(tx, &message)?;
        }
    }
    Ok(())
}

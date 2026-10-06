//! `Fizzy::{Card, CardCache, CardReferenceSync}`. Shared references, private viewer payloads.
use super::{accounts::Account, client, urls};
use campfire_db::{Connection, Errors, Event, Message, Result, Timestamp, Tx, User};
use jiff::SignedDuration;
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const STALE_AFTER: SignedDuration = SignedDuration::from_mins(5);
pub const NOT_FOUND: &str = "not_found";

#[derive(Debug, Clone)]
pub struct Card {
    pub id: i64,
    pub account_id: String,
    pub number: i64,
}
impl Card {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            account_id: row.get("account_id")?,
            number: row.get("number")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row("SELECT * FROM fizzy_cards WHERE id=?", [id], Self::from_row)
            .optional()?
            .ok_or(campfire_db::Error::RecordNotFound("Fizzy::Card"))
    }
    pub fn for_reference(tx: &Tx<'_>, account: &str, number: i64) -> Result<Self> {
        let mut errors = Errors::default();
        if account.chars().all(char::is_whitespace) {
            errors.add("account_id", "can't be blank");
        }
        if number <= 0 {
            errors.add("number", "must be greater than 0");
        }
        errors.into_result()?;
        tx.conn().execute("INSERT INTO fizzy_cards(account_id,number,created_at,updated_at) VALUES (?1,?2,?3,?3) ON CONFLICT(account_id,number) DO NOTHING", params![account, number, tx.now()])?;
        Ok(tx.conn().query_row(
            "SELECT * FROM fizzy_cards WHERE account_id=?1 AND number=?2",
            params![account, number],
            Self::from_row,
        )?)
    }
    pub fn for_message(conn: &Connection, message: i64) -> Result<Vec<Self>> {
        let mut query = conn.prepare("SELECT c.* FROM fizzy_cards c JOIN fizzy_card_references r ON r.fizzy_card_id=c.id WHERE r.message_id=? ORDER BY c.account_id,c.number")?;
        Ok(query
            .query_map([message], Self::from_row)?
            .collect::<std::result::Result<_, _>>()?)
    }
    /// Read-only page preload seam; preserve the single-message card ordering.
    pub fn for_messages(conn: &Connection, ids: &[i64]) -> Result<std::collections::HashMap<i64, Vec<Self>>> {
        let mut result: std::collections::HashMap<i64, Vec<Self>> = ids.iter().map(|id| (*id, Vec::new())).collect();
        if ids.is_empty() { return Ok(result); }
        let mut query = conn.prepare("SELECT c.*, r.message_id AS referenced_message_id FROM fizzy_cards c JOIN fizzy_card_references r ON r.fizzy_card_id=c.id WHERE r.message_id IN (SELECT value FROM json_each(?)) ORDER BY r.message_id,c.account_id,c.number")?;
        for row in query.query_map([serde_json::json!(ids).to_string()], |row| Ok((row.get::<_, i64>("referenced_message_id")?, Self::from_row(row)?)))? {
            let (id, card) = row?; result.entry(id).or_default().push(card);
        }
        Ok(result)
    }
    pub fn web_url(&self) -> String {
        format!(
            "{}/{}/cards/{}",
            client::api_base_url(),
            self.account_id,
            self.number
        )
    }
    pub fn broadcast_updates(&self, tx: &mut Tx<'_>) {
        tx.emit_after_commit(Event::broadcast(&CardUpdate { card_id: self.id }));
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub id: i64,
    pub card_id: i64,
    pub user_id: i64,
    pub payload: Option<Value>,
    pub fetched_at: Option<Timestamp>,
    pub fetch_error: Option<String>,
}
impl Cache {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let payload: Option<String> = row.get("payload")?;
        Ok(Self {
            id: row.get("id")?,
            card_id: row.get("fizzy_card_id")?,
            user_id: row.get("user_id")?,
            payload: payload
                .and_then(|p| serde_json::from_str::<Value>(&p).ok())
                .filter(|p| !p.is_null()),
            fetched_at: row.get("fetched_at")?,
            fetch_error: row.get("fetch_error")?,
        })
    }
    pub fn find(conn: &Connection, card: i64, user: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT * FROM fizzy_card_caches WHERE fizzy_card_id=?1 AND user_id=?2",
                params![card, user],
                Self::from_row,
            )
            .optional()?)
    }
    pub fn for_viewer(tx: &Tx<'_>, card: &Card, user: i64) -> Result<Self> {
        Card::find(tx.conn(), card.id)?;
        User::find(tx.conn(), user)?;
        tx.conn().execute("INSERT INTO fizzy_card_caches(fizzy_card_id,user_id,created_at,updated_at) VALUES (?1,?2,?3,?3) ON CONFLICT(fizzy_card_id,user_id) DO NOTHING", params![card.id,user,tx.now()])?;
        Ok(Self::find(tx.conn(), card.id, user)?.expect("saved viewer cache"))
    }
    pub fn stale(&self, now: Timestamp) -> bool {
        self.fetched_at
            .is_none_or(|fetched| fetched < now.ago(STALE_AFTER))
    }
    /// Conditional SQL handles old snapshots and concurrent callers. Claims skip callbacks/touch.
    pub fn request_fetch(&self, tx: &mut Tx<'_>) -> Result<bool> {
        self.request_fetch_at(tx, tx.now())
    }
    pub fn request_fetch_at(&self, tx: &mut Tx<'_>, now: Timestamp) -> Result<bool> {
        if !self.stale(now) {
            return Ok(false);
        }
        let claimed = tx.conn().execute("UPDATE fizzy_card_caches SET fetch_requested_at=?2 WHERE id=?1 AND (fetch_requested_at IS NULL OR fetch_requested_at<?3)", params![self.id,now,now.ago(STALE_AFTER)])? == 1;
        if claimed {
            tx.emit_after_commit(Event::job(&super::fetch::FetchJob {
                card_id: self.card_id,
                user_id: self.user_id,
            }));
        }
        Ok(claimed)
    }
    /// Rails' explicit release API; durable enqueue errors already roll back the claim here.
    #[allow(
        dead_code,
        reason = "Rails model API; durable enqueue rejection rolls back its entire write"
    )]
    pub fn release_fetch_request(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute(
            "UPDATE fizzy_card_caches SET fetch_requested_at=NULL WHERE id=?",
            [self.id],
        )?;
        Ok(())
    }
    pub fn save(
        &self,
        tx: &Tx<'_>,
        payload: Option<&Value>,
        fetched: Option<Timestamp>,
        error: Option<&str>,
    ) -> Result<()> {
        tx.conn().execute("UPDATE fizzy_card_caches SET payload=?2,fetched_at=?3,fetch_error=?4,updated_at=?5 WHERE id=?1",params![self.id,payload.filter(|p| !p.is_null()).map(Value::to_string),fetched,error,tx.now()])?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardUpdate {
    pub card_id: i64,
}
impl campfire_db::Broadcast for CardUpdate {
    const KIND: &'static str = "Fizzy::Card#broadcast_card_updates";
}

/// WS8 invokes this inside create/edit/scheduled-send writes. Quiet imports still make refs.
pub fn sync_message(
    tx: &mut Tx<'_>,
    message: &Message,
    enqueue: bool,
    crypto: Option<&ArEncryption>,
) -> Result<()> {
    let html = message.body_html(tx.conn())?.unwrap_or_default();
    let body = super::super::linkedin::non_code_text(&html)
        .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    let text = format!(
        "{}\n{}",
        body,
        message.forward_note.as_deref().unwrap_or("")
    );
    let pairs = urls::extract(&text, &client::api_base_url())
        .map_err(|e| campfire_db::Error::Other(e.into()))?;
    let cards = pairs
        .iter()
        .map(|r| Card::for_reference(tx, &r.account_id, r.number))
        .collect::<Result<Vec<_>>>()?;
    tx.conn().execute(
        "DELETE FROM fizzy_card_references WHERE message_id=? AND fizzy_card_id NOT IN (SELECT value FROM json_each(?))",
        params![message.id, serde_json::json!(cards.iter().map(|card| card.id).collect::<Vec<_>>()).to_string()],
    )?;
    for card in cards {
        let inserted = tx.conn().execute("INSERT INTO fizzy_card_references(message_id,fizzy_card_id,created_at,updated_at) VALUES (?1,?2,?3,?3) ON CONFLICT(message_id,fizzy_card_id) DO NOTHING",params![message.id,card.id,tx.now()])? == 1;
        if enqueue
            && (inserted
                || Cache::find(tx.conn(), card.id, message.creator_id)?
                    .is_none_or(|cache| cache.stale(tx.now())))
            && let Some(crypto) = crypto
            && let Some(account) = Account::for_user(tx.conn(), message.creator_id)?
            && account.usable_token(tx, crypto)?.is_some()
        {
            Cache::for_viewer(tx, &card, message.creator_id)?.request_fetch(tx)?;
        }
    }
    Ok(())
}


use crate::sql::{query_all, query_one};
use crate::{Attachment, Connection, Errors, Event, Result, Timestamp, Tx, User};
use rand::RngCore;
use rusqlite::{Row, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct ChannelWebhook {
    pub id: i64,
    pub room_id: i64,
    pub creator_id: i64,
    pub author_id: i64,
    pub name: String,
    pub token_digest: String,
    pub last_used_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Passed through the normal message save; the token is rechecked under its writer lock.
#[derive(Debug, Clone)]
pub struct WebhookMessage {
    pub webhook_id: i64,
    pub token_digest: Option<String>,
    pub username: Option<String>,
    pub embeds: Value,
}

#[derive(Debug, Clone)]
pub struct WebhookMessagePayload {
    pub name: String,
    pub username: Option<String>,
    pub embeds: Value,
}

impl ChannelWebhook {
    fn row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            creator_id: row.get("creator_id")?,
            author_id: row.get("author_id")?,
            name: row.get("name")?,
            token_digest: row.get("token_digest")?,
            last_used_at: row.get("last_used_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn digest(token: &str) -> String {
        hex::encode(Sha256::digest(token.as_bytes()))
    }

    fn token() -> String {
        let mut bytes = [0; 32];
        rand::rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM channel_webhooks WHERE id=?",
            [id],
            Self::row,
        )
    }

    pub fn find_by_token_digest(conn: &Connection, digest: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM channel_webhooks WHERE token_digest=?",
            [digest],
            Self::row,
        )
    }

    pub fn list_for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM channel_webhooks WHERE room_id=? ORDER BY id",
            [room_id],
            Self::row,
        )
    }

    fn name(value: &str) -> Result<&str> {
        let value = value.trim();
        let mut errors = Errors::default();
        if value.is_empty() || value.chars().count() > 80 {
            errors.add("name", "must have 1 to 80 characters");
        }
        errors.into_result()?;
        Ok(value)
    }

    fn required(conn: &Connection, id: i64) -> Result<Self> {
        Self::find(conn, id)?.ok_or(crate::Error::RecordNotFound("ChannelWebhook"))
    }

    pub fn create(
        tx: &mut Tx<'_>,
        room_id: i64,
        creator_id: i64,
        name: &str,
    ) -> Result<(Self, String)> {
        let name = Self::name(name)?;
        tx.savepoint(|tx| {
            if crate::Room::find(tx.conn(), room_id)?.deleted() {
                return Err(crate::Error::RecordNotFound("Room"));
            }
            let author = User::create_webhook_author(tx, name)?;

            let token = Self::token();
            tx.conn().execute(
                "INSERT INTO channel_webhooks(room_id,creator_id,author_id,name,token_digest,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",
                params![room_id,creator_id,author.id,name,Self::digest(&token),tx.now(),tx.now()],
            )?;
            Ok((Self::required(tx.conn(), tx.conn().last_insert_rowid())?, token))
        })
    }

    pub fn rename(tx: &Tx<'_>, id: i64, name: &str) -> Result<Self> {
        let name = Self::name(name)?;
        let webhook = Self::required(tx.conn(), id)?;
        tx.conn().execute(
            "UPDATE channel_webhooks SET name=?,updated_at=? WHERE id=?",
            params![name, tx.now(), id],
        )?;
        tx.conn().execute(
            "UPDATE users SET name=?,updated_at=? WHERE id=?",
            params![name, tx.now(), webhook.author_id],
        )?;
        Self::required(tx.conn(), id)
    }

    pub fn avatar_blob_id(&self, conn: &Connection) -> Result<Option<i64>> {
        Ok(Attachment::find_for(conn, "ChannelWebhook", self.id, "avatar")?.map(|a| a.blob_id))
    }

    pub fn set_avatar(tx: &mut Tx<'_>, id: i64, blob_id: Option<i64>) -> Result<()> {
        Self::required(tx.conn(), id)?;
        if let Some(old) = Attachment::find_for(tx.conn(), "ChannelWebhook", id, "avatar")? {
            if Some(old.blob_id) == blob_id {
                return Ok(());
            }
            old.delete(tx)?;
            tx.emit_after_commit(Event::PurgeBlob {
                blob_id: old.blob_id,
            });
        }
        if let Some(blob_id) = blob_id {
            Attachment::create(tx, "ChannelWebhook", id, "avatar", blob_id)?;
        }
        tx.conn().execute(
            "UPDATE channel_webhooks SET updated_at=? WHERE id=?",
            params![tx.now(), id],
        )?;
        Ok(())
    }

    pub fn regenerate(tx: &Tx<'_>, id: i64) -> Result<(Self, String)> {
        Self::required(tx.conn(), id)?;
        let token = Self::token();
        tx.conn().execute(
            "UPDATE channel_webhooks SET token_digest=?,updated_at=? WHERE id=?",
            params![Self::digest(&token), tx.now(), id],
        )?;
        Ok((Self::required(tx.conn(), id)?, token))
    }

    pub fn delete(tx: &mut Tx<'_>, id: i64) -> Result<()> {
        Self::set_avatar(tx, id, None)?;
        tx.conn()
            .execute("DELETE FROM channel_webhooks WHERE id=?", [id])?;
        Ok(())
    }

    pub fn record_message(
        tx: &Tx<'_>,
        message: &crate::Message,
        input: &WebhookMessage,
    ) -> Result<()> {
        let webhook = Self::required(tx.conn(), input.webhook_id)?;
        let room = crate::Room::find(tx.conn(), webhook.room_id)?;
        if room.deleted()
            || webhook.room_id != message.room_id
            || webhook.author_id != message.creator_id
            || input
                .token_digest
                .as_ref()
                .is_some_and(|digest| digest != &webhook.token_digest)
        {
            return Err(crate::Error::RecordNotFound("ChannelWebhook"));
        }
        tx.conn().execute(
            "INSERT INTO channel_webhook_messages(message_id,webhook_id,name,username,embeds_json) VALUES(?,?,?,?,?)",
            params![message.id,webhook.id,webhook.name,input.username,input.embeds.to_string()],
        )?;
        if let Some(blob_id) = webhook.avatar_blob_id(tx.conn())? {
            Attachment::create(tx, "Message", message.id, "webhook_avatar", blob_id)?;
        }
        tx.conn().execute(
            "UPDATE channel_webhooks SET last_used_at=?,updated_at=? WHERE id=?",
            params![tx.now(), tx.now(), webhook.id],
        )?;
        Ok(())
    }

    pub fn message_payloads(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<(i64, WebhookMessagePayload)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = query_all(
            conn,
            "SELECT message_id,name,username,embeds_json FROM channel_webhook_messages WHERE message_id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(ids).to_string()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        rows.into_iter()
            .map(|(id, name, username, embeds)| {
                Ok((
                    id,
                    WebhookMessagePayload {
                        name,
                        username,
                        embeds: serde_json::from_str(&embeds)
                            .map_err(|error| crate::Error::Other(error.to_string()))?,
                    },
                ))
            })
            .collect()
    }

    pub(crate) fn copy_message(tx: &Tx<'_>, source_id: i64, message_id: i64) -> Result<()> {
        tx.conn().execute(
            "INSERT INTO channel_webhook_messages(message_id,name,username,embeds_json) SELECT ?,name,username,embeds_json FROM channel_webhook_messages WHERE message_id=?",
            params![message_id, source_id],
        )?;
        if let Some(avatar) = Attachment::find_for(tx.conn(), "Message", source_id, "webhook_avatar")? {
            Attachment::create(tx, "Message", message_id, "webhook_avatar", avatar.blob_id)?;
        }
        Ok(())
    }

    pub fn message_payload(
        conn: &Connection,
        message_id: i64,
    ) -> Result<Option<WebhookMessagePayload>> {
        let row: Option<(String, Option<String>, String)> = query_one(
            conn,
            "SELECT name,username,embeds_json FROM channel_webhook_messages WHERE message_id=?",
            [message_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        row.map(|(name, username, embeds)| {
            Ok(WebhookMessagePayload {
                name,
                username,
                embeds: serde_json::from_str(&embeds)
                    .map_err(|error| crate::Error::Other(error.to_string()))?,
            })
        })
        .transpose()
    }
}

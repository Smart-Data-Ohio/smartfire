//! `reference/app/models/webhook.rb`: the row and the payload. Delivery (HTTP, replies)
//! lives in the app.

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Error, Result};
use rails_compat::ar_encryption::ArEncryption;
use rand::RngCore;
use crate::models::{Message, Room, User};
use crate::rich_text::RichText;
use crate::sql::{CachedStatements, query_one};
use crate::time::Timestamp;

/// `Webhook::ENDPOINT_TIMEOUT`
pub const ENDPOINT_TIMEOUT_SECONDS: u64 = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct Webhook {
    pub id: i64,
    pub user_id: i64,
    pub url: Option<String>,
    /// The AR-encrypted column. Decrypt only when a delivery or admin view needs it.
    pub encrypted_signing_secret: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Webhook {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            url: row.get("url")?,
            encrypted_signing_secret: row.get("signing_secret")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find_by_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT "webhooks".* FROM "webhooks" WHERE "webhooks"."user_id" = ? LIMIT 1"#,
            [user_id],
            Self::from_row,
        )
    }

    /// `create_webhook!(url:)`
    pub fn create(tx: &Tx<'_>, user_id: i64, url: Option<&str>) -> Result<Self> {
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "webhooks" ("created_at", "updated_at", "url", "user_id") VALUES (?, ?, ?, ?) RETURNING "id""#,
            params![now, now, url, user_id],
            |r| r.get(0),
        )?;
        Ok(Self {
            id,
            user_id,
            url: url.map(Into::into),
            encrypted_signing_secret: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// `webhook.update!(url:)`
    pub fn update_url(&mut self, tx: &Tx<'_>, url: &str) -> Result<()> {
        if self.url.as_deref() == Some(url) {
            return Ok(());
        }
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "webhooks" SET "updated_at" = ?, "url" = ? WHERE "webhooks"."id" = ?"#,
            params![now, url, self.id],
        )?;
        self.url = Some(url.into());
        self.updated_at = now;
        Ok(())
    }

    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "webhooks" WHERE "webhooks"."id" = ?"#,
            [self.id],
        )?;
        Ok(())
    }

    /// `signing_secret`: missing means an unsigned legacy delivery, without generating one.
    pub fn signing_secret(&self, encryption: &ArEncryption) -> Result<Option<String>> {
        self.encrypted_signing_secret.as_deref().map(|value| encryption.decrypt(value).map_err(|error| Error::Other(error.to_string()))).transpose()
    }

    /// `ensure_signing_secret!`: a loaded present value returns immediately. Blank
    /// instances reload under the writer transaction and adopt a first-generation winner's secret. Encryption and the row update commit together.
    pub fn ensure_signing_secret(&mut self, tx: &Tx<'_>, encryption: &ArEncryption) -> Result<String> {
        if let Some(secret) = self.signing_secret(encryption)?.filter(|secret| !campfire_richtext::ruby::is_blank(secret)) {
            return Ok(secret);
        }
        *self = query_one(tx.conn(), "SELECT webhooks.* FROM webhooks WHERE id = ? LIMIT 1", [self.id], Self::from_row)?.ok_or(Error::RecordNotFound("Webhook"))?;
        if let Some(secret) = self.signing_secret(encryption)?.filter(|secret| !campfire_richtext::ruby::is_blank(secret)) {
            return Ok(secret);
        }
        self.reset_signing_secret(tx, encryption)
    }

    pub fn reset_signing_secret(&mut self, tx: &Tx<'_>, encryption: &ArEncryption) -> Result<String> {
        let (secret, encrypted) = new_signing_secret(encryption);
        tx.conn().execute_cached("UPDATE webhooks SET signing_secret = ?, updated_at = ? WHERE id = ?", params![encrypted, tx.now(), self.id])?;
        self.encrypted_signing_secret = Some(encrypted);
        self.updated_at = tx.now();
        Ok(secret)
    }

    /// The JSON body `deliver` posts. `room_bot_messages_path` and `message_path` come from
    /// the route helpers (`room_bot_messages_path(room, bot_key)`, `room_at_message_path(room, message)`).
    pub fn payload(
        &self,
        conn: &Connection,
        rich_text: &dyn RichText,
        message: &Message,
        room_bot_messages_path: &str,
        message_path: &str,
    ) -> Result<String> {
        let creator = message.creator(conn)?;
        let room = Room::find(conn, message.room_id)?;
        let recipient = User::find(conn, self.user_id)?;
        let html = message.body_html(conn)?;
        let plain =
            without_recipient_mentions(&message.plain_text_body(conn, rich_text)?, &recipient);

        // Hash order as written in `Webhook#payload`, encoded like `ActiveSupport::JSON`.
        let body = format!(
            r#"{{"user":{{"id":{},"name":{}}},"room":{{"id":{},"name":{},"path":{}}},"message":{{"id":{},"body":{{"html":{},"plain":{}}},"path":{}}}}}"#,
            creator.id,
            json_string(&creator.name),
            room.id,
            room.name
                .as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".into()),
            json_string(room_bot_messages_path),
            message.id,
            html.as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".into()),
            json_string(&plain),
            json_string(message_path),
        );
        if crate::sql::exists(conn, "SELECT 1 FROM agents WHERE user_id = ?", [self.user_id])? {
            Ok(body)
        } else {
            Ok(format!("{},\"reply_url\":{}}}", &body[..body.len() - 1], json_string(room_bot_messages_path)))
        }
    }

    /// Agent message payload. The caller supplies the GitHub domain's pull-request payload.
    pub fn payload_for_agent(
        &self, conn: &Connection, rich_text: &dyn RichText, message: &Message,
        agent_id: i64, delivery_id: i64, pull_request: &serde_json::Value,
    ) -> Result<String> {
        let room_path = format!("/rooms/{}", message.room_id);
        let message_path = format!("{room_path}/@{}", message.id);
        let body = self.payload(conn, rich_text, message, &room_path, &message_path)?;
        let (name, owner): (String, Option<String>) = conn.query_row_cached(
            "SELECT users.name, owners.name FROM agents JOIN users ON users.id = agents.user_id LEFT JOIN users AS owners ON owners.id = agents.owner_id WHERE agents.id = ? AND agents.user_id = ?",
            params![agent_id, self.user_id], |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let files = crate::sql::query_all(conn, "SELECT file_id FROM drive_attachments WHERE message_id = ? ORDER BY id", [message.id], |row| row.get::<_, String>(0))?;
        let attachments = files.into_iter().map(|file_id| format!(r#"{{"file_id":{},"url":{}}}"#, json_string(&file_id), json_string(&format!("https://drive.google.com/open?id={file_id}")))).collect::<Vec<_>>().join(",");
        // The last two braces close `message` and the outer hash; Drive data belongs to message.
        let prefix = &body[..body.len() - 2];
        let pr = pull_request.to_string().replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026");
        Ok(format!(r#"{prefix},"drive_attachments":[{attachments}]}},"agent":{{"id":{agent_id},"name":{},"owner":{},"delivery_id":{delivery_id}}},"pull_request":{pr}}}"#, json_string(&name), owner.as_deref().map(json_string).unwrap_or_else(|| "null".into())))
    }

}

/// SecureRandom.hex(32), whose Ruby encoding is US-ASCII.
pub(crate) fn new_signing_secret(encryption: &ArEncryption) -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let secret = hex::encode(bytes);
    let encrypted = encryption.encrypt_with_encoding(secret.as_bytes(), "US-ASCII");
    (secret, encrypted)
}

/// Removes `@Recipient` mentions and leading/trailing (Unicode) whitespace.
fn without_recipient_mentions(body: &str, recipient: &User) -> String {
    body.replace(&recipient.attachable_plain_text_representation(), "")
        .trim_matches(char::is_whitespace)
        .to_string()
}

/// `ActiveSupport::JSON.encode` of a string: JSON with `<`, `>` and `&` escaped as `\uXXXX`.
/// U+2028 and U+2029 stay raw (`load_defaults 8.2` turns `escape_js_separators_in_json` off;
/// probed in the reference image).
pub fn json_string(s: &str) -> String {
    let encoded = serde_json::to_string(s).expect("strings encode");
    encoded.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026")
}

#[cfg(test)]
mod tests {
    use super::json_string;

    /// `{ a: "\u2028<>&" }.to_json` in the reference image.
    #[test]
    fn json_strings_escape_html_but_not_line_separators() {
        assert_eq!(json_string("\u{2028}<>&\u{2029}"), "\"\u{2028}\\u003c\\u003e\\u0026\u{2029}\"");
    }
}

//! Rows of `active_storage_blobs` and `active_storage_attachments`, for the models that
//! have attachments. Uploading, analysis and variants belong to `campfire_storage`.

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{OptionalExt, Result};
use crate::sql::{CachedStatements, query_one};
use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq)]
pub struct Blob {
    pub id: i64,
    pub key: String,
    pub filename: String,
    pub content_type: Option<String>,
    /// JSON text, e.g. `{"identified":true,"width":800,"height":600,"analyzed":true}`.
    pub metadata: Option<String>,
    pub service_name: String,
    pub byte_size: i64,
    pub checksum: Option<String>,
    pub created_at: Timestamp,
}

impl Blob {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            key: row.get("key")?,
            filename: row.get("filename")?,
            content_type: row.get("content_type")?,
            metadata: row.get("metadata")?,
            service_name: row.get("service_name")?,
            byte_size: row.get("byte_size")?,
            checksum: row.get("checksum")?,
            created_at: row.get("created_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT * FROM "active_storage_blobs" WHERE "active_storage_blobs"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("ActiveStorage::Blob")
    }

    /// Catalog media stays in its catalog; signed identities cannot restore revoked private media.
    pub fn reusable_by(
        conn: &Connection,
        blob_id: i64,
        user_id: i64,
        record_type: &str,
    ) -> Result<bool> {
        let roots: Vec<i64> = crate::sql::query_all(
            conn,
            "WITH RECURSIVE roots(id) AS (VALUES (?1) UNION SELECT v.blob_id FROM active_storage_attachments a JOIN active_storage_variant_records v ON v.id=a.record_id JOIN roots r ON r.id=a.blob_id WHERE a.record_type='ActiveStorage::VariantRecord' AND a.name='image' UNION SELECT a.record_id FROM active_storage_attachments a JOIN roots r ON r.id=a.blob_id WHERE a.record_type='ActiveStorage::Blob' AND a.name='preview_image') SELECT b.id FROM roots r JOIN active_storage_blobs b ON b.id=r.id",
            [blob_id],
            |row| row.get(0),
        )?;
        if !roots.contains(&blob_id) {
            return Err(crate::Error::RecordNotFound("ActiveStorage::Blob"));
        }
        let ids = serde_json::json!(roots).to_string();
        if record_type != "Sticker" {
            // Reattaching catalog media would let message privacy and purging affect the sticker.
            let catalog_image: bool = conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM active_storage_attachments WHERE record_type='Sticker' AND name='image' AND blob_id IN (SELECT value FROM json_each(?)))",
                [&ids], |row| row.get(0))?;
            if catalog_image {
                return Ok(false);
            }
        }
        let restricted: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM active_storage_blobs b WHERE b.id IN (SELECT value FROM json_each(?)) AND COALESCE(json_extract(b.metadata,'$.private_thread_media'),0)=1) OR EXISTS (SELECT 1 FROM active_storage_attachments a LEFT JOIN action_text_rich_texts rt ON a.record_type='ActionText::RichText' AND rt.id=a.record_id LEFT JOIN poll_options po ON a.record_type='PollOption' AND po.id=a.record_id LEFT JOIN polls p ON p.id=po.poll_id JOIN messages m ON m.id=CASE WHEN a.record_type='Message' THEN a.record_id WHEN rt.record_type='Message' THEN rt.record_id WHEN a.record_type='PollOption' THEN p.message_id END JOIN channel_threads t ON t.id=m.thread_id WHERE a.blob_id IN (SELECT value FROM json_each(?)) AND t.visibility='private')",
            [&ids, &ids], |row| row.get(0))?;
        let personal = super::agent_access::is_personal_user(conn, user_id)?;
        if personal
            && crate::sql::exists(
                conn,
                "SELECT 1 FROM active_storage_blobs b WHERE b.id IN (SELECT value FROM json_each(?1)) AND (json_extract(b.metadata,'$.scheduled_by') IS NOT NULL OR (json_extract(b.metadata,'$.uploader_id') IS NOT NULL AND json_extract(b.metadata,'$.uploader_id')!=?2 AND NOT EXISTS (SELECT 1 FROM active_storage_attachments a JOIN messages m ON m.id=a.record_id WHERE a.blob_id=b.id AND a.record_type='Message' AND a.name IN ('attachment','attachments')))) UNION ALL SELECT 1 FROM active_storage_attachments a WHERE a.blob_id IN (SELECT value FROM json_each(?1)) AND a.record_type='ScheduledMessage'",
                params![&ids, user_id],
            )?
        {
            return Ok(false);
        }
        let poll_media = personal
            && crate::sql::exists(
                conn,
                "SELECT 1 FROM active_storage_blobs b WHERE b.id IN (SELECT value FROM json_each(?1)) AND COALESCE(json_extract(b.metadata,'$.poll_media'),0)=1 UNION ALL SELECT 1 FROM active_storage_attachments a WHERE a.blob_id IN (SELECT value FROM json_each(?1)) AND a.record_type='PollOption'",
                [&ids],
            )?;
        if !restricted && !poll_media {
            return Ok(true);
        }
        let messages: Vec<i64> = crate::sql::query_all(
            conn,
            "SELECT m.id FROM active_storage_attachments a LEFT JOIN action_text_rich_texts rt ON a.record_type='ActionText::RichText' AND rt.id=a.record_id LEFT JOIN poll_options po ON a.record_type='PollOption' AND po.id=a.record_id LEFT JOIN polls p ON p.id=po.poll_id JOIN messages m ON m.id=CASE WHEN a.record_type='Message' THEN a.record_id WHEN rt.record_type='Message' THEN rt.record_id WHEN a.record_type='PollOption' THEN p.message_id END WHERE a.blob_id IN (SELECT value FROM json_each(?))",
            [&ids],
            |row| row.get(0),
        )?;
        for message_id in messages {
            match crate::Message::find_reachable(conn, user_id, message_id) {
                Ok(source) => {
                    let private = source
                        .thread_id
                        .map(|id| crate::ChannelThread::find(conn, id))
                        .transpose()?
                        .is_some_and(|thread| {
                            thread.visibility == super::channel_thread::ThreadVisibility::Private
                        });
                    if !personal
                        || super::agent_access::capability_for_user(
                            conn,
                            user_id,
                            if private {
                                "read_messages"
                            } else {
                                "post_messages"
                            },
                            source.room_id,
                        )? == Some(true)
                    {
                        return Ok(true);
                    }
                }
                Err(crate::Error::RecordNotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(false)
    }

    /// Inserts a blob row; `self.id` and `created_at` are ignored and assigned.
    pub fn create(tx: &Tx<'_>, blob: &Blob) -> Result<Self> {
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "active_storage_blobs" ("byte_size", "checksum", "content_type", "created_at", "filename", "key", "metadata", "service_name") VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![blob.byte_size, blob.checksum, blob.content_type, now, blob.filename, blob.key, blob.metadata, blob.service_name],
            |r| r.get(0),
        )?;
        Ok(Self {
            id,
            created_at: now,
            ..blob.clone()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    pub spoiler: bool,
    pub id: i64,
    pub name: String,
    pub record_type: String,
    pub record_id: i64,
    pub blob_id: i64,
    pub created_at: Timestamp,
}

impl Attachment {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            spoiler: row.get("spoiler")?,
            id: row.get("id")?,
            name: row.get("name")?,
            record_type: row.get("record_type")?,
            record_id: row.get("record_id")?,
            blob_id: row.get("blob_id")?,
            created_at: row.get("created_at")?,
        })
    }

    /// `has_one_attached`'s lookup.
    pub fn find_for(
        conn: &Connection,
        record_type: &str,
        record_id: i64,
        name: &str,
    ) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "active_storage_attachments" WHERE "active_storage_attachments"."record_id" = ? AND "active_storage_attachments"."record_type" = ? AND "active_storage_attachments"."name" = ? LIMIT 1"#,
            params![record_id, record_type, name],
            Self::from_row,
        )
    }

    pub fn create(
        tx: &Tx<'_>,
        record_type: &str,
        record_id: i64,
        name: &str,
        blob_id: i64,
    ) -> Result<Self> {
        Self::create_with_spoiler(tx, record_type, record_id, name, blob_id, false)
    }

    pub fn create_with_spoiler(
        tx: &Tx<'_>,
        record_type: &str,
        record_id: i64,
        name: &str,
        blob_id: i64,
        spoiler: bool,
    ) -> Result<Self> {
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "active_storage_attachments" ("blob_id", "created_at", "name", "record_id", "record_type", "spoiler") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![blob_id, now, name, record_id, record_type, spoiler],
            |r| r.get(0),
        )?;
        let private: bool = tx.conn().query_row(
            "SELECT EXISTS (SELECT 1 FROM messages m JOIN channel_threads t ON t.id=m.thread_id WHERE t.visibility='private' AND ((?1='Message' AND m.id=?2) OR (?1='ActionText::RichText' AND m.id IN (SELECT record_id FROM action_text_rich_texts WHERE id=?2 AND record_type='Message')) OR (?1='PollOption' AND m.id IN (SELECT p.message_id FROM polls p JOIN poll_options po ON po.poll_id=p.id WHERE po.id=?2))))",
            params![record_type, record_id], |row| row.get(0),
        )?;
        if private {
            tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(COALESCE(NULLIF(metadata,''),'{}'),'$.private_thread_media',json('true')) WHERE id=?", [blob_id])?;
        }
        Ok(Self {
            spoiler,
            id,
            name: name.into(),
            record_type: record_type.into(),
            record_id,
            blob_id,
            created_at: now,
        })
    }

    pub fn delete(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "active_storage_attachments" WHERE "active_storage_attachments"."id" = ?"#, [self.id])?;
        Ok(())
    }

    pub fn blob(&self, conn: &Connection) -> Result<Blob> {
        Blob::find(conn, self.blob_id)
    }
}

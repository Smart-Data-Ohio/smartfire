//! Rails DriveAttachment: metadata-free IDs, scoped uniqueness and message touch.
use crate::{Connection, Errors, Message, Result, Tx};
use rusqlite::params;
#[derive(Clone, Debug)]
pub struct DriveAttachment {
    pub id: Option<i64>,
    pub message_id: Option<i64>,
    pub file_id: String,
}
impl DriveAttachment {
    pub fn new(message_id: Option<i64>, file_id: impl Into<String>) -> Self {
        Self {
            id: None,
            message_id,
            file_id: file_id.into(),
        }
    }
    pub fn url(&self) -> String {
        format!("https://drive.google.com/open?id={}", self.file_id)
    }
    pub fn validate(&self, conn: &Connection) -> Result<Errors> {
        let mut errors = Errors::default();
        if self.message_id.is_none()
            || !crate::sql::exists(conn, "SELECT 1 FROM messages WHERE id=?", [self.message_id])?
        {
            errors.add("message", "must exist");
        }
        if campfire_richtext::ruby::is_blank(&self.file_id) {
            errors.add("file_id", "can't be blank");
        }
        if !super::message::valid_drive_file_id(&self.file_id) {
            errors.add("file_id", "is invalid");
        }
        if crate::sql::exists(
            conn,
            "SELECT 1 FROM drive_attachments WHERE message_id=? AND file_id=? AND id IS NOT ?",
            params![self.message_id, self.file_id, self.id],
        )? {
            errors.add("file_id", "has already been taken");
        }
        Ok(errors)
    }
    pub fn create(tx: &mut Tx<'_>, message_id: i64, file_id: impl Into<String>) -> Result<Self> {
        let attachment = Self::new(Some(message_id), file_id);
        tx.savepoint(move |tx| {
            attachment.validate(tx.conn())?.into_result()?;
            let id = tx.conn().query_row(
                "INSERT INTO drive_attachments(message_id,file_id,created_at) VALUES(?,?,?) RETURNING id",
                params![message_id, attachment.file_id, tx.now()],
                |r| r.get(0),
            )?;
            Message::find(tx.conn(), message_id)?.touch(tx)?;
            Ok(Self { id: Some(id), ..attachment })
        })
    }
    pub fn for_message(conn: &Connection, message_id: i64) -> Result<Vec<Self>> {
        crate::sql::query_all(
            conn,
            "SELECT id,message_id,file_id FROM drive_attachments WHERE message_id=? ORDER BY id",
            [message_id],
            |r| {
                Ok(Self {
                    id: Some(r.get(0)?),
                    message_id: Some(r.get(1)?),
                    file_id: r.get(2)?,
                })
            },
        )
    }
}

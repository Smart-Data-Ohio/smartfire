//! The workspace's sticker catalog. Images use the Sticker/image attachment slot.

use rusqlite::{Row, params};

use crate::{CachedStatements, Connection, Error, Errors, Message, Result, Timestamp, Tx};

pub const CAPACITY: i64 = 60;
pub const MAX_IMAGE_BYTES: i64 = campfire_storage::sticker::MAX_BYTES;

#[derive(Clone, Debug)]
pub struct Sticker {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tags: String,
    pub uploader_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug)]
pub struct StickerValues {
    pub name: String,
    pub description: String,
    pub tags: String,
}

impl StickerValues {
    fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_owned();
        self.description = self.description.trim().to_owned();
        self.tags = self.tags.trim().to_owned();
        self
    }

    fn errors(&self, conn: &Connection, id: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if !(2..=30).contains(&self.name.chars().count()) {
            errors.add("name", "must be between 2 and 30 characters");
        }
        if conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM stickers WHERE name_key=? AND (? IS NULL OR id!=?))",
            params![caseless::default_case_fold_str(&self.name), id, id],
            |row| row.get::<_, bool>(0),
        )? {
            errors.add("name", "has already been taken");
        }
        Ok(errors)
    }
}

impl Sticker {
    fn row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            tags: row.get("tags")?,
            uploader_id: row.get("uploader_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn list(conn: &Connection) -> Result<Vec<Self>> {
        let mut statement = conn.prepare_cached("SELECT * FROM stickers ORDER BY name_key,id")?;
        Ok(statement
            .query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row("SELECT * FROM stickers WHERE id=?", [id], Self::row)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Error::RecordNotFound("Sticker"),
                error => error.into(),
            })
    }

    pub fn create(tx: &Tx<'_>, uploader_id: i64, values: StickerValues) -> Result<Self> {
        let values = values.normalized();
        values.errors(tx.conn(), None)?.into_result()?;
        let count: i64 = tx
            .conn()
            .query_row("SELECT COUNT(*) FROM stickers", [], |row| row.get(0))?;
        if count >= CAPACITY {
            return Err(Error::StickerLimitReached);
        }
        tx.conn().execute_cached(
            "INSERT INTO stickers(name,name_key,description,tags,uploader_id,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",
            params![values.name, caseless::default_case_fold_str(&values.name), values.description, values.tags, uploader_id, tx.now(), tx.now()],
        )?;
        Self::find(tx.conn(), tx.conn().last_insert_rowid())
    }

    pub fn update(&self, tx: &mut Tx<'_>, values: StickerValues) -> Result<Self> {
        let values = values.normalized();
        values.errors(tx.conn(), Some(self.id))?.into_result()?;
        tx.conn().execute_cached(
            "UPDATE stickers SET name=?,name_key=?,description=?,tags=?,updated_at=? WHERE id=?",
            params![
                values.name,
                caseless::default_case_fold_str(&values.name),
                values.description,
                values.tags,
                tx.revision_after(self.updated_at),
                self.id
            ],
        )?;
        tx.conn().execute_cached(
            "UPDATE messages SET sticker_name=? WHERE sticker_id=?",
            params![values.name, self.id],
        )?;
        self.refresh_messages(tx)?;
        Self::find(tx.conn(), self.id)
    }

    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        let messages = self.messages(tx.conn())?;
        tx.conn()
            .execute_cached("DELETE FROM stickers WHERE id=?", [self.id])?;
        for message in messages {
            Self::refresh_message(tx, message.id)?;
        }
        Ok(())
    }

    fn messages(&self, conn: &Connection) -> Result<Vec<Message>> {
        let mut statement = conn.prepare_cached("SELECT * FROM messages WHERE sticker_id=?")?;
        Ok(statement
            .query_map([self.id], Message::from_row)?
            .collect::<rusqlite::Result<_>>()?)
    }

    fn refresh_messages(&self, tx: &mut Tx<'_>) -> Result<()> {
        for message in self.messages(tx.conn())? {
            Self::refresh_message(tx, message.id)?;
        }
        Ok(())
    }

    fn refresh_message(tx: &mut Tx<'_>, message_id: i64) -> Result<()> {
        let mut message = Message::find(tx.conn(), message_id)?;
        let updated_at = tx.revision_after(message.updated_at);
        message.touch(tx)?;
        tx.conn().execute_cached(
            "UPDATE messages SET updated_at=? WHERE id=?",
            params![updated_at, message_id],
        )?;
        tx.emit_after_commit(crate::Event::broadcast(
            &crate::broadcasts::Broadcast::MessageUpdated { message_id },
        ));
        Ok(())
    }
}

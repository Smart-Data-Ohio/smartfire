//! The instance's voice soundboard catalog, independent of the classic chat sounds.

use rusqlite::{Row, params};

use crate::{CachedStatements, Connection, Error, Errors, Result, Timestamp, Tx};

pub const CAPACITY: i64 = 48;
pub const MAX_AUDIO_BYTES: i64 = 512 * 1024;

#[derive(Clone, Debug)]
pub struct SoundboardSound {
    pub id: i64,
    pub name: String,
    pub emoji: Option<String>,
    pub volume: f64,
    pub uploader_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug)]
pub struct SoundboardValues {
    pub name: String,
    pub emoji: Option<String>,
    pub volume: f64,
}

#[derive(Clone, Debug)]
pub struct AudioFacts {
    pub content_type: String,
    pub byte_size: i64,
}

impl AudioFacts {
    pub fn errors(&self) -> Errors {
        let mut errors = Errors::default();
        if !["audio/mpeg", "audio/ogg", "audio/wav", "audio/webm"]
            .contains(&self.content_type.as_str())
        {
            errors.add("audio", "must be an MP3, Ogg, WAV or WebM audio file");
        }
        if !(1..=MAX_AUDIO_BYTES).contains(&self.byte_size) {
            errors.add("audio", "must be between 1 byte and 512 KB");
        }
        errors
    }
}

impl SoundboardValues {
    pub fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_owned();
        self.emoji = self.emoji.filter(|emoji| !emoji.is_empty());
        self
    }

    pub fn errors(&self, conn: &Connection, id: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if !(1..=32).contains(&self.name.chars().count()) {
            errors.add("name", "must be between 1 and 32 characters");
        }
        if conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM soundboard_sounds WHERE name_key=? AND (? IS NULL OR id!=?))",
            params![caseless::default_case_fold_str(&self.name), id, id],
            |row| row.get::<_, bool>(0),
        )? {
            errors.add("name", "has already been taken");
        }
        if self
            .emoji
            .as_ref()
            .is_some_and(|emoji| !campfire_richtext::emoji::single_emoji(emoji))
        {
            errors.add("emoji", "must be a single Unicode emoji");
        }
        if !self.volume.is_finite() || !(0.0..=1.0).contains(&self.volume) {
            errors.add("volume", "must be between 0.0 and 1.0");
        }
        Ok(errors)
    }
}

impl SoundboardSound {
    fn row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            emoji: row.get("emoji")?,
            volume: row.get("volume")?,
            uploader_id: row.get("uploader_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn list(conn: &Connection) -> Result<Vec<Self>> {
        let mut statement =
            conn.prepare_cached("SELECT * FROM soundboard_sounds ORDER BY name_key,id")?;
        Ok(statement
            .query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row(
            "SELECT * FROM soundboard_sounds WHERE id=?",
            [id],
            Self::row,
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Error::RecordNotFound("SoundboardSound"),
            error => error.into(),
        })
    }

    pub fn create(
        tx: &Tx<'_>,
        uploader_id: i64,
        values: SoundboardValues,
        audio: &AudioFacts,
    ) -> Result<Self> {
        let values = values.normalized();
        let mut errors = values.errors(tx.conn(), None)?;
        errors.0.extend(audio.errors().0);
        let count: i64 =
            tx.conn()
                .query_row("SELECT COUNT(*) FROM soundboard_sounds", [], |row| {
                    row.get(0)
                })?;
        if count >= CAPACITY {
            errors.add("base", "soundboard capacity reached (limit: 48 sounds)");
        }
        errors.into_result()?;
        tx.conn().execute_cached(
            "INSERT INTO soundboard_sounds(name,name_key,emoji,volume,uploader_id,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",
            params![values.name, caseless::default_case_fold_str(&values.name), values.emoji, values.volume, uploader_id, tx.now(), tx.now()],
        )?;
        Self::find(tx.conn(), tx.conn().last_insert_rowid())
    }

    pub fn update(&self, tx: &Tx<'_>, values: SoundboardValues) -> Result<Self> {
        let values = values.normalized();
        values.errors(tx.conn(), Some(self.id))?.into_result()?;
        tx.conn().execute_cached(
            "UPDATE soundboard_sounds SET name=?,name_key=?,emoji=?,volume=?,updated_at=? WHERE id=?",
            params![values.name, caseless::default_case_fold_str(&values.name), values.emoji, values.volume, tx.now(), self.id],
        )?;
        Self::find(tx.conn(), self.id)
    }

    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn()
            .execute_cached("DELETE FROM soundboard_sounds WHERE id=?", [self.id])?;
        Ok(())
    }
}

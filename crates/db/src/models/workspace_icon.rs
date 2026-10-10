//! `WorkspaceIcon`: normalized identity, ordered queries and persistence.
use crate::{Connection, Errors, Result, Timestamp, Tx};
use campfire_richtext::ruby::{is_blank, strip};
use rusqlite::{Row, params};
#[derive(Clone, Debug)]
pub struct WorkspaceIcon {
    pub id: i64,
    pub name: String,
    pub title: String,
    pub creator_id: i64,
    pub creator_name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Clone, Debug, Default)]
pub struct NewIcon {
    pub name: Option<String>,
    pub title: Option<String>,
    pub creator_id: i64,
}
#[derive(Clone, Debug)]
pub struct ImageFacts {
    pub content_type: String,
    pub byte_size: i64,
    pub content_error: Option<String>,
    pub animated: bool,
}
impl NewIcon {
    pub fn normalized(mut self) -> Self {
        self.name = self.name.map(|s| strip(&s).to_lowercase());
        self
    }
    pub fn errors(
        &self,
        conn: &Connection,
        brand: bool,
        image: Option<&ImageFacts>,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        let name = self.name.as_deref().unwrap_or("");
        let title = self.title.as_deref().unwrap_or("");
        if is_blank(name) {
            errors.add("name", "can't be blank");
        }
        if !(2..=32).contains(&name.len())
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            errors.add("name", "is invalid");
        }
        if conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM workspace_icons WHERE LOWER(name)=LOWER(?))",
            [name],
            |r| r.get::<_, bool>(0),
        )? {
            errors.add("name", "has already been taken");
        }
        if is_blank(title) {
            errors.add("title", "can't be blank");
        }
        let length = title.chars().count();
        if length < 1 {
            errors.add("title", "is too short (minimum is 1 character)");
        }
        if length > 60 {
            errors.add("title", "is too long (maximum is 60 characters)");
        }
        if brand {
            errors.add("name", "is already taken by a built-in icon");
        }
        match image {
            None => errors.add("image", "must be attached"),
            Some(image) => {
                if !["image/png", "image/svg+xml", "image/gif", "image/webp"]
                    .contains(&image.content_type.as_str())
                {
                    errors.add("image", "must be an SVG, PNG, GIF or WebP image");
                }
                if image.byte_size > campfire_storage::workspace_icon::MAX_BYTES {
                    errors.add("image", "must be smaller than 256 KB");
                }
                if let Some(error) = &image.content_error {
                    errors.add("image", error);
                }
                if image.animated {
                    let limit = crate::models::account::Account::first(conn)?
                        .map(|account| account.settings().animated_emoji_limit())
                        .unwrap_or(crate::models::account::DEFAULT_ANIMATED_EMOJI_LIMIT);
                    if WorkspaceIcon::animated_usage(conn)? >= limit {
                        errors.add(
                            "image",
                            format!("animated emoji capacity reached (limit: {limit})"),
                        );
                    }
                }
            }
        }
        Ok(errors)
    }
    pub fn save(
        &self,
        tx: &Tx<'_>,
        brand: bool,
        image: Option<&ImageFacts>,
    ) -> Result<WorkspaceIcon> {
        self.errors(tx.conn(), brand, image)?.into_result()?;
        let result=tx.conn().execute("INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES(?,?,?,?,?)",params![self.name,self.title,self.creator_id,tx.now(),tx.now()]);
        if let Err(error) = result {
            let error = crate::Error::from(error);
            if error.is_record_not_unique() {
                let mut errors = Errors::default();
                errors.add("name", "has already been taken");
                return Err(crate::Error::RecordInvalid(errors));
            }
            return Err(error);
        }
        WorkspaceIcon::find(tx.conn(), tx.conn().last_insert_rowid())
    }
}
impl WorkspaceIcon {
    pub fn animated_usage(conn: &Connection) -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM workspace_icons i JOIN active_storage_attachments a ON a.record_type = 'WorkspaceIcon' AND a.record_id = i.id AND a.name = 'image' JOIN active_storage_blobs b ON b.id = a.blob_id WHERE json_extract(b.metadata, '$.emoji_animated') = 1",
            [], |row| row.get(0),
        )?)
    }

    pub fn animated_by_name(conn: &Connection, name: &str) -> Result<bool> {
        let Some(icon) = Self::find_by_name(conn, name)? else {
            return Ok(false);
        };
        let blob = campfire_storage::Blob::attached(conn, "WorkspaceIcon", icon.id, "image")
            .map_err(|error| crate::Error::Other(error.to_string()))?;
        Ok(blob
            .as_ref()
            .is_some_and(campfire_storage::workspace_icon::animated))
    }

    fn row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            name: r.get("name")?,
            title: r.get("title")?,
            creator_id: r.get("creator_id")?,
            creator_name: crate::User::from_prefixed_row(r, "creator_")?.display_name().to_owned(),
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn ordered(conn: &Connection) -> Result<Vec<Self>> {
        let mut s=conn.prepare(&format!("SELECT workspace_icons.*,{} FROM workspace_icons JOIN users ON users.id=creator_id ORDER BY workspace_icons.name", crate::User::projection("users", "creator_")))?;
        Ok(s.query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row(&format!("SELECT workspace_icons.*,{} FROM workspace_icons JOIN users ON users.id=creator_id WHERE workspace_icons.id=?", crate::User::projection("users", "creator_")),[id],Self::row).map_err(|e|if e==rusqlite::Error::QueryReturnedNoRows {crate::Error::RecordNotFound("WorkspaceIcon")} else {e.into()})
    }
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Self>> {
        use rusqlite::OptionalExtension;
        Ok(conn.query_row(&format!("SELECT workspace_icons.*,{} FROM workspace_icons JOIN users ON users.id=creator_id WHERE workspace_icons.name=?", crate::User::projection("users", "creator_")),[name],Self::row).optional()?)
    }
    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM workspace_icons WHERE id=?", [self.id])?;
        Ok(())
    }
}

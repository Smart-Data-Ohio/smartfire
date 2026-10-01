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
                if !["image/png", "image/svg+xml"].contains(&image.content_type.as_str()) {
                    errors.add("image", "must be an SVG or PNG");
                }
                if image.byte_size > 256 * 1024 {
                    errors.add("image", "must be smaller than 256 KB");
                }
                if let Some(error) = &image.content_error {
                    errors.add("image", error);
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
    fn row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            name: r.get("name")?,
            title: r.get("title")?,
            creator_id: r.get("creator_id")?,
            creator_name: r.get("creator_name")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn ordered(conn: &Connection) -> Result<Vec<Self>> {
        let mut s=conn.prepare("SELECT workspace_icons.*,users.name AS creator_name FROM workspace_icons JOIN users ON users.id=creator_id ORDER BY workspace_icons.name")?;
        Ok(s.query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row("SELECT workspace_icons.*,users.name AS creator_name FROM workspace_icons JOIN users ON users.id=creator_id WHERE workspace_icons.id=?",[id],Self::row).map_err(|e|if e==rusqlite::Error::QueryReturnedNoRows {crate::Error::RecordNotFound("WorkspaceIcon")} else {e.into()})
    }
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Self>> {
        use rusqlite::OptionalExtension;
        Ok(conn.query_row("SELECT workspace_icons.*,users.name AS creator_name FROM workspace_icons JOIN users ON users.id=creator_id WHERE workspace_icons.name=?",[name],Self::row).optional()?)
    }
    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM workspace_icons WHERE id=?", [self.id])?;
        Ok(())
    }
}

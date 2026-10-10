//! Shared workspace organization, independent of memberships' personal categories and favourites.
use crate::error::OptionalExt;
use crate::sql::{CachedStatements, query_all, query_one};
use crate::{Errors, Event, Result, Room, Timestamp, Tx};
use rusqlite::{Connection, Row, params};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceOrganized;

impl crate::events::Broadcast for WorkspaceOrganized {
    const KIND: &'static str = "WorkspaceCategory#sync_organized";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCategory {
    pub id: i64,
    pub name: String,
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoomPosition {
    pub room_id: i64,
    pub workspace_category_id: Option<i64>,
    pub position: Option<i64>,
}

impl WorkspaceCategory {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            position: row.get("position")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            "SELECT * FROM workspace_categories WHERE id = ?",
            [id],
            Self::from_row,
        )?
        .or_not_found("WorkspaceCategory")
    }

    pub fn ordered(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM workspace_categories ORDER BY position, id",
            [],
            Self::from_row,
        )
    }

    fn validate(name: &str) -> Result<()> {
        let mut errors = Errors::default();
        if name.trim().is_empty() {
            errors.add("name", "can't be blank");
        }
        if name.chars().count() > crate::models::room_category::NAME_LIMIT {
            errors.add("name", "is too long (maximum is 50 characters)");
        }
        errors.into_result()
    }

    pub fn create(tx: &mut Tx<'_>, name: &str) -> Result<Self> {
        Self::validate(name)?;
        let id = tx.conn().query_row_cached("INSERT INTO workspace_categories (name, position, created_at, updated_at) VALUES (?, (SELECT COALESCE(MAX(position), -1) + 1 FROM workspace_categories), ?, ?) RETURNING id", params![name, tx.now(), tx.now()], |row| row.get(0))?;
        tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        Self::find(tx.conn(), id)
    }

    pub fn rename(&self, tx: &mut Tx<'_>, name: &str) -> Result<Self> {
        Self::validate(name)?;
        let current = Self::find(tx.conn(), self.id)?;
        if current.name != name {
            tx.conn().execute_cached(
                "UPDATE workspace_categories SET name = ?, updated_at = ? WHERE id = ?",
                params![name, tx.now(), self.id],
            )?;
            tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        }
        Self::find(tx.conn(), self.id)
    }

    /// Every category exactly once, numbered from zero; a stale or duplicate list writes nothing.
    pub fn reorder(tx: &mut Tx<'_>, ids: &[i64]) -> Result<Option<Vec<Self>>> {
        let current = Self::ordered(tx.conn())?;
        let mut wanted = ids.to_vec();
        wanted.sort_unstable();
        let mut have = current.iter().map(|row| row.id).collect::<Vec<_>>();
        have.sort_unstable();
        if wanted != have {
            return Ok(None);
        }
        let mut changed = false;
        for (position, id) in ids.iter().enumerate() {
            if current
                .iter()
                .any(|row| row.id == *id && row.position != position as i64)
            {
                tx.conn().execute_cached(
                    "UPDATE workspace_categories SET position = ?, updated_at = ? WHERE id = ?",
                    params![position as i64, tx.now(), id],
                )?;
                changed = true;
            }
        }
        if changed {
            tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        }
        Self::ordered(tx.conn()).map(Some)
    }

    pub fn rooms(
        conn: &Connection,
        category_id: Option<i64>,
    ) -> Result<Vec<WorkspaceRoomPosition>> {
        query_all(
            conn,
            "SELECT id, workspace_category_id, workspace_position FROM rooms WHERE workspace_category_id IS ? AND type != 'Rooms::Direct' AND deleted_at IS NULL ORDER BY workspace_position IS NULL, workspace_position, LOWER(name), id",
            [category_id],
            room_position,
        )
    }

    /// Appends this category's rooms to uncategorized and compacts both room and category positions.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        Self::find(tx.conn(), self.id)?;
        let mut ids = Self::rooms(tx.conn(), None)?
            .iter()
            .map(|room| room.room_id)
            .collect::<Vec<_>>();
        ids.extend(
            Self::rooms(tx.conn(), Some(self.id))?
                .iter()
                .map(|room| room.room_id),
        );
        tx.conn()
            .execute_cached("DELETE FROM workspace_categories WHERE id = ?", [self.id])?;
        write_rooms(tx, None, &ids)?;
        let categories = Self::ordered(tx.conn())?
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        Self::reorder(tx, &categories)?;
        tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        Ok(())
    }

    /// Moves and reorders in the caller's transaction, densely numbering the source and target.
    pub fn move_room(
        tx: &mut Tx<'_>,
        room_id: i64,
        category_id: Option<i64>,
        position: i64,
    ) -> Result<()> {
        let room = Room::find(tx.conn(), room_id)?;
        if room.deleted_at.is_some() {
            return Err(crate::Error::RecordNotFound("Room"));
        }
        if room.direct() {
            let mut errors = Errors::default();
            errors.add("workspace_category_id", "cannot categorize direct messages");
            return errors.into_result();
        }
        if let Some(id) = category_id {
            Self::find(tx.conn(), id)?;
        }
        let source: Option<i64> = tx.conn().query_row_cached(
            "SELECT workspace_category_id FROM rooms WHERE id = ?",
            [room_id],
            |row| row.get(0),
        )?;
        let mut target = Self::rooms(tx.conn(), category_id)?
            .iter()
            .map(|room| room.room_id)
            .filter(|id| *id != room_id)
            .collect::<Vec<_>>();
        let position = position.clamp(0, target.len() as i64) as usize;
        target.insert(position, room_id);
        if source != category_id {
            let source_ids = Self::rooms(tx.conn(), source)?
                .iter()
                .map(|room| room.room_id)
                .filter(|id| *id != room_id)
                .collect::<Vec<_>>();
            write_rooms(tx, source, &source_ids)?;
        }
        write_rooms(tx, category_id, &target)?;
        tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        Ok(())
    }

    /// The shared placements the member's sidebar may expose, excluding hidden and deleted rooms.
    pub fn visible_rooms(conn: &Connection, user_id: i64) -> Result<Vec<WorkspaceRoomPosition>> {
        query_all(
            conn,
            "SELECT rooms.id, rooms.workspace_category_id, rooms.workspace_position FROM rooms JOIN memberships ON memberships.room_id = rooms.id WHERE memberships.user_id = ? AND memberships.involvement != 'invisible' AND rooms.deleted_at IS NULL AND rooms.type != 'Rooms::Direct' ORDER BY rooms.workspace_category_id, rooms.workspace_position IS NULL, rooms.workspace_position, LOWER(rooms.name), rooms.id",
            [user_id],
            room_position,
        )
    }

    /// Compacts the room's group in the deletion transaction before publishing its new layout.
    pub(crate) fn remove_room_from_layout(
        tx: &mut Tx<'_>,
        room_id: i64,
        importing: bool,
    ) -> Result<()> {
        let Some(room) = query_one(
            tx.conn(),
            "SELECT id, workspace_category_id, workspace_position FROM rooms WHERE id = ? AND type != 'Rooms::Direct'",
            [room_id],
            room_position,
        )?
        else {
            return Ok(());
        };
        if room.workspace_category_id.is_none() && room.position.is_none() {
            return Ok(());
        }
        if room.position.is_some() {
            let ids = Self::rooms(tx.conn(), room.workspace_category_id)?
                .iter()
                .map(|row| row.room_id)
                .filter(|id| *id != room_id)
                .collect::<Vec<_>>();
            write_rooms(tx, room.workspace_category_id, &ids)?;
        }
        if !importing {
            tx.emit_after_commit(Event::broadcast(&WorkspaceOrganized));
        }
        Ok(())
    }
}

fn room_position(row: &Row<'_>) -> rusqlite::Result<WorkspaceRoomPosition> {
    Ok(WorkspaceRoomPosition {
        room_id: row.get("id")?,
        workspace_category_id: row.get("workspace_category_id")?,
        position: row.get("workspace_position")?,
    })
}

fn write_rooms(tx: &Tx<'_>, category_id: Option<i64>, ids: &[i64]) -> Result<()> {
    for (position, id) in ids.iter().enumerate() {
        tx.conn().execute_cached(
            "UPDATE rooms SET workspace_category_id = ?, workspace_position = ? WHERE id = ?",
            params![category_id, position as i64, id],
        )?;
    }
    Ok(())
}

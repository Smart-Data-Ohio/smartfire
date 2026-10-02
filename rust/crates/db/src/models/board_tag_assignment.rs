//! `app/models/board_tag_assignment.rb`. Configuration authorization belongs to callers;
//! both validation and firing a rule recheck its assignee's current membership and grants.
use crate::sql::{query_all, query_one};
use crate::{Agent, Errors, Membership, Result, Room, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};

#[derive(Debug, Clone, PartialEq)]
pub struct BoardTagAssignment {
    pub id: i64,
    pub room_id: i64,
    pub tag: String,
    pub assignee_id: i64,
    pub created_by_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct NewBoardTagAssignment {
    pub room_id: i64,
    pub tag: String,
    pub assignee_id: i64,
    pub created_by_id: i64,
}

impl BoardTagAssignment {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            tag: row.get("tag")?,
            assignee_id: row.get("assignee_id")?,
            created_by_id: row.get("created_by_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM board_tag_assignments WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM board_tag_assignments WHERE room_id=? ORDER BY tag",
            [room_id],
            Self::from_row,
        )
    }
    pub fn create(tx: &mut Tx<'_>, attributes: NewBoardTagAssignment) -> Result<Self> {
        let mut rule = Self {
            id: 0,
            room_id: attributes.room_id,
            tag: attributes.tag,
            assignee_id: attributes.assignee_id,
            created_by_id: attributes.created_by_id,
            created_at: tx.now(),
            updated_at: tx.now(),
        };
        rule.normalize();
        rule.validate_excluding(tx.conn(), None)?.into_result()?;
        rule.id = tx.conn().query_row("INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES (?,?,?,?,?,?) RETURNING id",
            params![rule.room_id,rule.tag,rule.assignee_id,rule.created_by_id,rule.created_at,rule.updated_at], |r|r.get(0))?;
        Ok(rule)
    }
    pub fn update(&mut self, tx: &mut Tx<'_>, tag: &str, assignee_id: i64) -> Result<()> {
        let before = self.clone();
        self.tag = tag.into();
        self.assignee_id = assignee_id;
        self.normalize();
        self.validate(tx.conn())?.into_result()?;
        if *self != before {
            self.updated_at = tx.now();
            match (
                self.tag != before.tag,
                self.assignee_id != before.assignee_id,
            ) {
                (true, true) => tx.conn().execute(
                    "UPDATE board_tag_assignments SET tag=?,assignee_id=?,updated_at=? WHERE id=?",
                    params![self.tag, self.assignee_id, self.updated_at, self.id],
                )?,
                (true, false) => tx.conn().execute(
                    "UPDATE board_tag_assignments SET tag=?,updated_at=? WHERE id=?",
                    params![self.tag, self.updated_at, self.id],
                )?,
                (false, true) => tx.conn().execute(
                    "UPDATE board_tag_assignments SET assignee_id=?,updated_at=? WHERE id=?",
                    params![self.assignee_id, self.updated_at, self.id],
                )?,
                (false, false) => 0,
            };
        }
        Ok(())
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM board_tag_assignments WHERE id=?", [self.id])?;
        Ok(())
    }
    fn normalize(&mut self) {
        self.tag = rails_compat::unicode::downcase(campfire_richtext::ruby::strip(&self.tag));
    }
    pub fn validate(&self, conn: &Connection) -> Result<Errors> {
        self.validate_excluding(conn, Some(self.id))
    }
    fn validate_excluding(&self, conn: &Connection, existing_id: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        let room = Room::find_by_id(conn, self.room_id)?;
        let assignee = User::find_by_id(conn, self.assignee_id)?;
        if room.is_none() {
            errors.add("room", "must exist");
        }
        if assignee.is_none() {
            errors.add("assignee", "must exist");
        }
        if User::find_by_id(conn, self.created_by_id)?.is_none() {
            errors.add("created_by", "must exist");
        }
        if campfire_richtext::ruby::is_blank(&self.tag) {
            errors.add("tag", "can't be blank");
        }
        if self.tag.chars().count() > super::thread_tag::TAG_NAME_LIMIT {
            errors.add("tag", "is too long (maximum is 30 characters)");
        }
        if !super::thread_tag::valid_tag_name(&self.tag) {
            errors.add("tag", "is invalid");
        }
        if crate::sql::exists(
            conn,
            "SELECT 1 FROM board_tag_assignments WHERE room_id=? AND LOWER(tag)=LOWER(?) AND (? IS NULL OR id != ?)",
            params![self.room_id, self.tag, existing_id, existing_id],
        )? {
            errors.add("tag", "has already been taken");
        }
        if let Some(room) = &room {
            if !room.board() {
                errors.add("room", "must be a board");
            }
            if let Some(user) = assignee
                && !eligible(conn, room.id, &user)?
            {
                errors.add(
                    "assignee",
                    "must be an active board member able to own posts",
                );
            }
        }
        Ok(errors)
    }
}

pub(crate) fn eligible(conn: &Connection, room_id: i64, user: &User) -> Result<bool> {
    if !user.is_active() || Membership::find_by_room_and_user(conn, room_id, user.id)?.is_none() {
        return Ok(false);
    }
    if !user.is_bot() {
        return Ok(true);
    }
    let Some(agent) = Agent::for_user(conn, user.id)? else {
        return Ok(false);
    };
    Ok(agent.active(conn)?
        && agent.can(conn, "post_messages", Some(room_id))?
        && agent.can(conn, "read_messages", Some(room_id))?)
}

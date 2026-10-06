//! `reference/app/models/message_pin.rb`: a room's pinned messages, capped per room, with a quiet
//! system note per pin and the badge, count and panel broadcasts after every change.

use jiff::SignedDuration;
use rusqlite::{Connection, Row, params};

use crate::broadcasts::{Broadcast, Partial, message_dom_id, room_dom_id, room_messages};
use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::{Message, NewMessage, Room, User};
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::time::Timestamp;

/// `MessagePin::MAX_PER_ROOM`
pub const MAX_PER_ROOM: i64 = 50;
/// `MessagePin::PIN_NOTE_WINDOW`: one pin note per message per window.
pub const PIN_NOTE_WINDOW: SignedDuration = SignedDuration::from_mins(10);

#[derive(Debug, Clone, PartialEq)]
pub struct MessagePin {
    pub id: i64,
    pub message_id: i64,
    pub room_id: i64,
    pub pinner_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `MessagePin::CapReachedError`: refused before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapReached(pub String);

impl std::fmt::Display for CapReached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The message's permalink: `room_at_message_path(room, message)` (`/rooms/1/@2`), or for a
/// thread message `room_path(room, thread:, message_id:)` (`to_query` sorts the keys).
pub fn message_path(message: &Message) -> String {
    match message.thread_id {
        Some(thread_id) => format!("/rooms/{}?message_id={}&thread={thread_id}", message.room_id, message.id),
        None => format!("/rooms/{}/@{}", message.room_id, message.id),
    }
}

impl MessagePin {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            message_id: row.get("message_id")?,
            room_id: row.get("room_id")?,
            pinner_id: row.get("pinner_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT "message_pins".* FROM "message_pins" WHERE "message_pins"."id" = ? LIMIT 1"#, [id], Self::from_row)?
            .or_not_found("MessagePin")
    }

    pub fn find_by_message(conn: &Connection, message_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT "message_pins".* FROM "message_pins" WHERE "message_pins"."message_id" = ? LIMIT 1"#,
            [message_id],
            Self::from_row,
        )
    }

    /// `MessagePin.pinned?(message)`
    pub fn pinned(conn: &Connection, message_id: i64) -> Result<bool> {
        sql::exists(conn, r#"SELECT 1 AS one FROM "message_pins" WHERE "message_pins"."message_id" = ? LIMIT 1"#, [message_id])
    }

    /// `room.message_pins.ordered`: newest first.
    pub fn ordered_for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "message_pins".* FROM "message_pins" WHERE "message_pins"."room_id" = ? ORDER BY "message_pins"."created_at" DESC, "message_pins"."id" DESC"#,
            [room_id],
            Self::from_row,
        )
    }

    /// `room.message_pins.count`
    pub fn count_for_room(conn: &Connection, room_id: i64) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "message_pins" WHERE "message_pins"."room_id" = ?"#, [room_id])
    }

    /// `MessagePin.pin!(message:, pinner:)`: the existing pin when the message is already pinned
    /// (checked before the cap, so re-pinning in a full room succeeds); otherwise
    /// [`CapReached`] at [`MAX_PER_ROOM`], or the new pin and its note. The count runs inside the
    /// write transaction, which SQLite serializes, so two pins can't both pass the cap. The note's
    /// append goes out after commit, like `note&.broadcast_create`.
    pub fn pin(tx: &mut Tx<'_>, message: &Message, pinner_id: i64) -> Result<std::result::Result<Self, CapReached>> {
        if let Some(pin) = Self::find_by_message(tx.conn(), message.id)? {
            return Ok(Ok(pin));
        }
        if Self::count_for_room(tx.conn(), message.room_id)? >= MAX_PER_ROOM {
            return Ok(Err(CapReached(format!("This channel already has {MAX_PER_ROOM} pinned messages"))));
        }
        let pin = Self::create(tx, message, message.room_id, pinner_id)?;
        if let Some(note) = pin.post_pin_note(tx, message)? {
            let room = Room::find(tx.conn(), note.room_id)?;
            tx.emit_after_commit(Event::broadcast(&Broadcast::append(
                room_messages(&room),
                room_dom_id(&room, Some("messages")),
                Partial::Message { message_id: note.id },
            )));
        }
        Ok(Ok(pin))
    }

    /// `MessagePin.create!(message:, room:, pinner:)`
    pub fn create(tx: &mut Tx<'_>, message: &Message, room_id: i64, pinner_id: i64) -> Result<Self> {
        Self::validate(tx.conn(), message.id, room_id, pinner_id)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "message_pins" ("created_at", "message_id", "pinner_id", "room_id", "updated_at") VALUES (?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, message.id, pinner_id, room_id, now],
            |r| r.get(0),
        )?;
        let pin = Self::find(tx.conn(), id)?;
        pin.after_change_commit(tx, message)?;
        Ok(pin)
    }

    /// `belongs_to` presence, `validates :message_id, uniqueness: true` and
    /// `message_must_belong_to_room`.
    pub fn validate(conn: &Connection, message_id: i64, room_id: i64, pinner_id: i64) -> Result<Errors> {
        let mut errors = Errors::default();
        let message = Message::find_by_id(conn, message_id)?;
        if message.is_none() {
            errors.add("message", "must exist");
        }
        if Room::find_by_id(conn, room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if User::find_by_id(conn, pinner_id)?.is_none() {
            errors.add("pinner", "must exist");
        }
        if Self::find_by_message(conn, message_id)?.is_some() {
            errors.add("message_id", "has already been taken");
        }
        if let Some(message) = message
            && message.room_id != room_id
        {
            errors.add("room", "must be the message's room");
        }
        Ok(errors)
    }

    /// `unpin!`
    pub fn unpin(&self, tx: &mut Tx<'_>) -> Result<()> {
        let message = Message::find(tx.conn(), self.message_id)?;
        self.destroy(tx, &message)
    }

    /// `destroy`, with the message it pins (which may be on its way out).
    fn destroy(&self, tx: &mut Tx<'_>, message: &Message) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "message_pins" WHERE "message_pins"."id" = ?"#, [self.id])?;
        self.after_change_commit(tx, message)
    }

    /// `has_many :message_pins, dependent: :destroy` on the message.
    pub(crate) fn destroy_for_message(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
        if let Some(pin) = Self::find_by_message(tx.conn(), message.id)? {
            pin.destroy(tx, message)?;
        }
        Ok(())
    }

    /// `after_commit on: [create, destroy]`, in declaration order: `broadcast_pin_change`, then
    /// `stamp_room_pins_changed`.
    fn after_change_commit(&self, tx: &mut Tx<'_>, message: &Message) -> Result<()> {
        let room = Room::find(tx.conn(), self.room_id)?;
        let streamables = room_messages(&room);
        for (target, partial) in [
            (message_dom_id(message, Some("pin_badge")), Partial::PinBadge { message_id: message.id }),
            (room_dom_id(&room, Some("pins_count")), Partial::PinsCount { room_id: room.id }),
            (room_dom_id(&room, Some("pins_list")), Partial::PinsList { room_id: room.id }),
        ] {
            tx.emit_after_commit(Event::broadcast(&Broadcast::replace_keeping_scroll(streamables.clone(), target, partial)));
        }
        let (room_id, message_id) = (self.room_id, self.message_id);
        tx.after_commit(move |tx| Self::stamp_room_pins_changed(tx, room_id, message_id));
        Ok(())
    }

    /// `stamp_room_pins_changed`: `update_columns` on the room (its `updated_at` stays, so the
    /// sidebar doesn't reorder) and `update_all` on the message, neither running callbacks.
    fn stamp_room_pins_changed(tx: &mut Tx<'_>, room_id: i64, message_id: i64) -> Result<()> {
        let now = tx.now();
        tx.conn().execute_cached(r#"UPDATE "rooms" SET "pins_changed_at" = ? WHERE "rooms"."id" = ?"#, params![now, room_id])?;
        tx.conn().execute_cached(r#"UPDATE "messages" SET "updated_at" = ? WHERE "messages"."id" = ?"#, params![now, message_id])?;
        Ok(())
    }

    /// `post_pin_note!`: a quiet root system note by the pinner, unless the room already holds the
    /// same note from the last [`PIN_NOTE_WINDOW`].
    fn post_pin_note(&self, tx: &mut Tx<'_>, message: &Message) -> Result<Option<Message>> {
        let source = format!("pinned a message: [jump to message]({})", message_path(message));
        let since = tx.now().ago(PIN_NOTE_WINDOW);
        let recent = sql::exists(
            tx.conn(),
            r#"SELECT 1 AS one FROM "messages" WHERE "messages"."room_id" = ? AND "messages"."thread_id" IS NULL AND "messages"."system_note" = 1 AND "messages"."markdown_source" = ? AND "messages"."created_at" >= ? LIMIT 1"#,
            params![self.room_id, source, since],
        )?;
        if recent {
            return Ok(None);
        }
        let note = Message::create(
            tx,
            NewMessage {
                room_id: self.room_id,
                creator_id: self.pinner_id,
                markdown_source: Some(source),
                system_note: true,
                ..Default::default()
            },
        )?;
        Ok(Some(note))
    }
}

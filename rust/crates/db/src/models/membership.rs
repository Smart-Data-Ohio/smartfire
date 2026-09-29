//! `reference/app/models/membership.rb` and `membership/connectable.rb`.

use jiff::SignedDuration;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Row, params};

use serde::{Deserialize, Serialize};

use crate::database::Tx;
use crate::error::{OptionalExt, Result};
use crate::events::{Broadcast, Event};
use crate::models::{Room, User};
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::time::Timestamp;

/// `enum :involvement, %w[ invisible nothing muted mentions everything ].index_by(&:itself)`
/// (`app/models/membership.rb`). A muted room goes unread only when the member is mentioned
/// (`Room#unread_memberships`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Involvement {
    Invisible,
    Nothing,
    Muted,
    Mentions,
    Everything,
}

impl Involvement {
    pub fn name(self) -> &'static str {
        match self {
            Involvement::Invisible => "invisible",
            Involvement::Nothing => "nothing",
            Involvement::Muted => "muted",
            Involvement::Mentions => "mentions",
            Involvement::Everything => "everything",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "invisible" => Some(Involvement::Invisible),
            "nothing" => Some(Involvement::Nothing),
            "muted" => Some(Involvement::Muted),
            "mentions" => Some(Involvement::Mentions),
            "everything" => Some(Involvement::Everything),
            _ => None,
        }
    }
}

impl ToSql for Involvement {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.name()))
    }
}

impl FromSql for Involvement {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let name = value.as_str()?;
        Involvement::from_name(name)
            .ok_or_else(|| FromSqlError::Other(format!("unknown involvement {name:?}").into()))
    }
}

/// `enum :stage_role, %w[ listener speaker host ].index_by(&:itself)`: set on stage-room
/// memberships only, nil everywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageRole {
    Listener,
    Speaker,
    Host,
}

impl StageRole {
    pub fn name(self) -> &'static str {
        match self {
            StageRole::Listener => "listener",
            StageRole::Speaker => "speaker",
            StageRole::Host => "host",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "listener" => Some(StageRole::Listener),
            "speaker" => Some(StageRole::Speaker),
            "host" => Some(StageRole::Host),
            _ => None,
        }
    }
}

impl ToSql for StageRole {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.name()))
    }
}

impl FromSql for StageRole {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let name = value.as_str()?;
        StageRole::from_name(name)
            .ok_or_else(|| FromSqlError::Other(format!("unknown stage role {name:?}").into()))
    }
}

/// `Membership::Connectable::CONNECTION_TTL`
pub const CONNECTION_TTL: SignedDuration = SignedDuration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub struct Membership {
    pub id: i64,
    pub room_id: i64,
    pub user_id: i64,
    /// Nullable in the schema (default "mentions").
    pub involvement: Option<Involvement>,
    pub unread_at: Option<Timestamp>,
    pub connected_at: Option<Timestamp>,
    pub connections: i64,
    /// The newest root message the member has seen; the unread divider starts after it.
    pub last_read_message_id: Option<i64>,
    pub stage_role: Option<StageRole>,
    pub hand_raised_at: Option<Timestamp>,
    pub server_muted_at: Option<Timestamp>,
    pub last_huddle_join_push_at: Option<Timestamp>,
    pub room_category_id: Option<i64>,
    pub favorite_position: Option<i64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Membership {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            user_id: row.get("user_id")?,
            involvement: row.get("involvement")?,
            unread_at: row.get("unread_at")?,
            connected_at: row.get("connected_at")?,
            connections: row.get("connections")?,
            last_read_message_id: row.get("last_read_message_id")?,
            stage_role: row.get("stage_role")?,
            hand_raised_at: row.get("hand_raised_at")?,
            server_muted_at: row.get("server_muted_at")?,
            last_huddle_join_push_at: row.get("last_huddle_join_push_at")?,
            room_category_id: row.get("room_category_id")?,
            favorite_position: row.get("favorite_position")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT * FROM "memberships" WHERE "memberships"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("Membership")
    }

    pub fn count(conn: &Connection) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "memberships""#, [])
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "memberships" WHERE "memberships"."user_id" = ?"#,
            [user_id],
            Self::from_row,
        )
    }

    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "memberships" WHERE "memberships"."room_id" = ?"#,
            [room_id],
            Self::from_row,
        )
    }

    /// `room.memberships.find_by(user:)` / `user.memberships.find_by(room_id:)`
    pub fn find_by_room_and_user(
        conn: &Connection,
        room_id: i64,
        user_id: i64,
    ) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "memberships" WHERE "memberships"."room_id" = ? AND "memberships"."user_id" = ? LIMIT 1"#,
            [room_id, user_id],
            Self::from_row,
        )
    }

    /// `user.memberships.visible.with_ordered_room`, each with its room.
    pub fn visible_with_ordered_room(conn: &Connection, user_id: i64) -> Result<Vec<(Self, Room)>> {
        query_all(
            conn,
            &format!(r#"SELECT "memberships".*, {} FROM "memberships" INNER JOIN "rooms" ON "rooms"."id" = "memberships"."room_id" WHERE "memberships"."user_id" = ? AND "memberships"."involvement" != 'invisible' ORDER BY LOWER(rooms.name)"#, Room::PREFIXED_COLUMNS),
            [user_id],
            |row| Ok((Self::from_row(row)?, Room::from_prefixed_row(row)?)),
        )
    }

    /// `user.memberships.with_ordered_room` (invisible included).
    pub fn with_ordered_room(conn: &Connection, user_id: i64) -> Result<Vec<(Self, Room)>> {
        query_all(
            conn,
            &format!(r#"SELECT "memberships".*, {} FROM "memberships" INNER JOIN "rooms" ON "rooms"."id" = "memberships"."room_id" WHERE "memberships"."user_id" = ? ORDER BY LOWER(rooms.name)"#, Room::PREFIXED_COLUMNS),
            [user_id],
            |row| Ok((Self::from_row(row)?, Room::from_prefixed_row(row)?)),
        )
    }

    /// `user.memberships.without_direct_rooms.count`
    pub fn count_without_direct_rooms(conn: &Connection, user_id: i64) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "memberships" INNER JOIN "rooms" "room" ON "room"."id" = "memberships"."room_id" WHERE "memberships"."user_id" = ? AND "room"."type" != 'Rooms::Direct'"#,
            [user_id],
        )
    }

    /// `user.memberships.unread.count`: the push badge.
    pub fn unread_count(conn: &Connection, user_id: i64) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "memberships" WHERE "memberships"."user_id" = ? AND "memberships"."unread_at" IS NOT NULL"#,
            [user_id],
        )
    }

    /// `Membership.connected.exists?(id)`
    pub fn connected_exists(conn: &Connection, id: i64, now: Timestamp) -> Result<bool> {
        sql::exists(
            conn,
            r#"SELECT 1 FROM "memberships" WHERE "memberships"."connected_at" >= ? AND "memberships"."id" = ? LIMIT 1"#,
            params![Self::connection_cutoff(now), id],
        )
    }

    /// `Membership.disconnected.exists?(id)`
    pub fn disconnected_exists(conn: &Connection, id: i64, now: Timestamp) -> Result<bool> {
        sql::exists(
            conn,
            r#"SELECT 1 FROM "memberships" WHERE ("memberships"."connected_at" IS NULL OR "memberships"."connected_at" < ?) AND "memberships"."id" = ? LIMIT 1"#,
            params![Self::connection_cutoff(now), id],
        )
    }

    /// `CONNECTION_TTL.ago`
    pub fn connection_cutoff(now: Timestamp) -> Timestamp {
        now.ago(CONNECTION_TTL)
    }

    pub fn room(&self, conn: &Connection) -> Result<Room> {
        Room::find(conn, self.room_id)
    }

    pub fn user(&self, conn: &Connection) -> Result<User> {
        User::find(conn, self.user_id)
    }

    // Involvement and read state

    pub fn involved_in(&self, involvement: Involvement) -> bool {
        self.involvement == Some(involvement)
    }

    /// `update!(involvement:)`
    pub fn update_involvement(&mut self, tx: &mut Tx<'_>, involvement: impl Into<Option<Involvement>>) -> Result<()> {
        let involvement = involvement.into();
        if self.involvement == involvement {
            return Ok(());
        }
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "involvement" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
            params![involvement, now, self.id],
        )?;
        self.involvement = involvement;
        self.updated_at = now;
        Ok(())
    }

    /// `read`: `update!(unread_at: nil, last_read_message_id: latest_root_message_id)`, which
    /// writes nothing when neither changes.
    pub fn read(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let last_read_message_id = self.latest_root_message_id(tx.conn())?;
        if self.unread_at.is_none() && self.last_read_message_id == last_read_message_id {
            return Ok(());
        }
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "unread_at" = ?, "last_read_message_id" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
            params![None::<Timestamp>, last_read_message_id, now, self.id],
        )?;
        self.unread_at = None;
        self.last_read_message_id = last_read_message_id;
        self.updated_at = now;
        Ok(())
    }

    /// `latest_root_message_id`: the room's newest root (non-thread) message.
    pub fn latest_root_message_id(&self, conn: &Connection) -> Result<Option<i64>> {
        latest_root_message_id(conn, self.room_id)
    }

    pub fn unread(&self) -> bool {
        self.unread_at.is_some()
    }

    /// `destroy`: after commit the room leaves the member's sidebar
    /// (`broadcast_room_removal_to_user`), then the user's sockets reconnect, so their
    /// subscriptions to this room are dropped, and a direct room recomputes its member key
    /// (`after_destroy_commit :refresh_direct_member_key`), in the order the callbacks are
    /// declared. Not yet ported, for the workstreams that own them: the huddle, agent and stream
    /// revocations (`before_destroy`), the last stage host's successor, thread memberships and
    /// calendar syncs.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "memberships" WHERE "memberships"."id" = ?"#,
            [self.id],
        )?;
        let (user_id, room_id) = (self.user_id, self.room_id);
        tx.after_commit(move |tx| {
            // `dom_id(room, :list)` raises for a room that's gone, which the callback rescues.
            if let Some(room) = Room::find_by_id(tx.conn(), room_id)? {
                let broadcast = RoomRemovalBroadcast { user_id, room_id, room_class: room.room_type.class_name().to_string() };
                tx.emit_after_commit(Event::broadcast(&broadcast));
            }
            User::find(tx.conn(), user_id)?.reset_remote_connections(tx);
            if let Some(room) = Room::find_by_id(tx.conn(), room_id)?.filter(Room::direct) {
                room.refresh_direct_member_key(tx)?;
            }
            Ok(())
        });
        Ok(())
    }

    // Membership::Connectable

    /// `Membership.disconnect_all`
    pub fn disconnect_all(tx: &mut Tx<'_>) -> Result<usize> {
        let now = tx.now();
        Ok(tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "connected_at" = ?, "connections" = ?, "updated_at" = ? WHERE "memberships"."connected_at" >= ?"#,
            params![None::<Timestamp>, 0, now, Self::connection_cutoff(now)],
        )?)
    }

    /// `Membership.connect(membership, connections)`: no `updated_at`. The member now sees the
    /// room, so the read pointer moves to its newest root message.
    pub fn connect(tx: &mut Tx<'_>, membership: &Membership, connections: i64) -> Result<()> {
        let last_read_message_id = membership.latest_root_message_id(tx.conn())?;
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "connections" = ?, "connected_at" = ?, "unread_at" = ?, "last_read_message_id" = ? WHERE "memberships"."id" = ?"#,
            params![connections, tx.now(), None::<Timestamp>, last_read_message_id, membership.id],
        )?;
        Ok(())
    }

    /// `connected?`
    pub fn is_connected(&self, now: Timestamp) -> bool {
        self.connected_at
            .is_some_and(|at| at >= Self::connection_cutoff(now))
    }

    /// `present`
    pub fn present(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let connections = if self.is_connected(tx.now()) {
            self.connections + 1
        } else {
            1
        };
        Self::connect(tx, self, connections)
    }

    /// `connected`
    pub fn connected(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.increment_connections(tx)?;
        self.touch_connected_at(tx)
    }

    /// `disconnected`
    pub fn disconnected(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.decrement_connections(tx)?;
        if self.connections < 1 && self.connected_at.is_some() {
            let now = tx.now();
            tx.conn().execute_cached(
                r#"UPDATE "memberships" SET "connected_at" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
                params![None::<Timestamp>, now, self.id],
            )?;
            self.connected_at = None;
            self.updated_at = now;
        }
        Ok(())
    }

    /// `refresh_connection`
    pub fn refresh_connection(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if !self.is_connected(tx.now()) {
            self.increment_connections(tx)?;
        }
        self.touch_connected_at(tx)
    }

    fn increment_connections(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self.is_connected(tx.now()) {
            self.update_counter(tx, 1)
        } else {
            self.update_connections(tx, 1)
        }
    }

    fn decrement_connections(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self.is_connected(tx.now()) {
            self.update_counter(tx, -1)
        } else {
            self.update_connections(tx, 0)
        }
    }

    /// `increment!(:connections, touch: true)` / `decrement!`
    fn update_counter(&mut self, tx: &mut Tx<'_>, by: i64) -> Result<()> {
        let now = tx.now();
        let sql = if by >= 0 {
            r#"UPDATE "memberships" SET "connections" = COALESCE("memberships"."connections", 0) + ?, "updated_at" = ? WHERE "memberships"."id" = ?"#
        } else {
            r#"UPDATE "memberships" SET "connections" = COALESCE("memberships"."connections", 0) - ?, "updated_at" = ? WHERE "memberships"."id" = ?"#
        };
        tx.conn().execute_cached(sql, params![by.abs(), now, self.id])?;
        self.connections += by;
        self.updated_at = now;
        Ok(())
    }

    /// `update!(connections:)`, a no-op when unchanged.
    fn update_connections(&mut self, tx: &mut Tx<'_>, connections: i64) -> Result<()> {
        if self.connections == connections {
            return Ok(());
        }
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "connections" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
            params![connections, now, self.id],
        )?;
        self.connections = connections;
        self.updated_at = now;
        Ok(())
    }

    /// `touch :connected_at`
    fn touch_connected_at(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "updated_at" = ?, "connected_at" = ? WHERE "memberships"."id" = ?"#,
            params![now, now, self.id],
        )?;
        self.connected_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
}

/// `Message.where(room_id:, thread_id: nil).order(created_at: :desc, id: :desc).pick(:id)`
pub(crate) fn latest_root_message_id(conn: &Connection, room_id: i64) -> Result<Option<i64>> {
    query_one(
        conn,
        r#"SELECT "messages"."id" FROM "messages" WHERE "messages"."room_id" = ? AND "messages"."thread_id" IS NULL ORDER BY "messages"."created_at" DESC, "messages"."id" DESC LIMIT 1"#,
        [room_id],
        |r| r.get(0),
    )
}

/// `Membership#broadcast_room_removal_to_user` (`after_destroy_commit`): the room leaves the
/// former member's sidebar, `broadcast_remove_to user, :rooms, target: [ room, :list ]`, preceded
/// by `[ room, :header_voice_participants ]` when huddles are configured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomRemovalBroadcast {
    pub user_id: i64,
    pub room_id: i64,
    /// The room's STI class (`Rooms::Open`), which its `dom_id` names.
    pub room_class: String,
}

impl Broadcast for RoomRemovalBroadcast {
    const KIND: &'static str = "Membership#broadcast_room_removal_to_user";
}

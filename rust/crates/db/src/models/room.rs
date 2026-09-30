//! `reference/app/models/room.rb`, `rooms/*.rb` and `room/message_pusher.rb`'s queries.
//!
//! Soft-deleted rooms (`deleted_at` set, see `Room#begin_destroy!`) grant nothing: `user.rooms`
//! and everything read through it (`reachable_messages`, the open-room grants) are `alive`.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Row, params};
use sha2::{Digest, Sha256};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::{Membership, Message, StageRole, User};
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::{SQLITE_NOW, Timestamp};

/// The STI `type` column: `app/models/rooms/{open,closed,direct,voice,stage,board}.rb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomType {
    Open,
    Closed,
    Direct,
    /// A standing voice huddle; membership works like a closed room.
    Voice,
    /// A standing stage huddle with per-member stage roles; the creator is the first host.
    Stage,
    /// Work threads ("posts") at the top level; membership works like a closed room.
    Board,
}

impl RoomType {
    pub fn class_name(self) -> &'static str {
        match self {
            RoomType::Open => "Rooms::Open",
            RoomType::Closed => "Rooms::Closed",
            RoomType::Direct => "Rooms::Direct",
            RoomType::Voice => "Rooms::Voice",
            RoomType::Stage => "Rooms::Stage",
            RoomType::Board => "Rooms::Board",
        }
    }

    pub fn from_class_name(name: &str) -> Option<Self> {
        match name {
            "Rooms::Open" => Some(RoomType::Open),
            "Rooms::Closed" => Some(RoomType::Closed),
            "Rooms::Direct" => Some(RoomType::Direct),
            "Rooms::Voice" => Some(RoomType::Voice),
            "Rooms::Stage" => Some(RoomType::Stage),
            "Rooms::Board" => Some(RoomType::Board),
            _ => None,
        }
    }

    /// `default_involvement`: "everything" in direct rooms (`Rooms::Direct`), "mentions" in
    /// every other type (`Room`).
    pub fn default_involvement(self) -> &'static str {
        match self {
            RoomType::Direct => "everything",
            _ => "mentions",
        }
    }
}

impl ToSql for RoomType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.class_name()))
    }
}

impl FromSql for RoomType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let name = value.as_str()?;
        RoomType::from_class_name(name)
            .ok_or_else(|| FromSqlError::Other(format!("unknown room type {name:?}").into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    pub id: i64,
    pub name: Option<String>,
    pub room_type: RoomType,
    pub creator_id: i64,
    /// Set by `begin_destroy!`; the room is gone for everyone from then on.
    pub deleted_at: Option<Timestamp>,
    pub destroy_enqueued_at: Option<Timestamp>,
    /// Direct rooms only: the hash of the exact member set (`Rooms::Direct.member_key_for`),
    /// unique among alive rooms.
    pub direct_member_key: Option<String>,
    pub icon_name: Option<String>,
    pub inbound_email_token: Option<String>,
    pub pins_changed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `user.rooms`: `has_many :rooms, -> { alive }, through: :memberships` (`app/models/user.rb`).
const SELECT_FOR_USER: &str = r#"SELECT "rooms".* FROM "rooms" INNER JOIN "memberships" ON "rooms"."id" = "memberships"."room_id" WHERE "rooms"."deleted_at" IS NULL AND "memberships"."user_id" = ?"#;

/// `Rooms::Direct::MAX_MEMBERS`: ad hoc group DMs hold at most this many members.
pub const DIRECT_MAX_MEMBERS: usize = 10;

impl Room {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            room_type: row.get("type")?,
            creator_id: row.get("creator_id")?,
            deleted_at: row.get("deleted_at")?,
            destroy_enqueued_at: row.get("destroy_enqueued_at")?,
            direct_member_key: row.get("direct_member_key")?,
            icon_name: row.get("icon_name")?,
            inbound_email_token: row.get("inbound_email_token")?,
            pins_changed_at: row.get("pins_changed_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// The room's columns as `r_<column>`, for a join that reads its rows with
    /// [`Room::from_prefixed_row`].
    pub(crate) const PREFIXED_COLUMNS: &str = r#""rooms"."id" AS r_id, "rooms"."created_at" AS r_created_at, "rooms"."creator_id" AS r_creator_id, "rooms"."deleted_at" AS r_deleted_at, "rooms"."destroy_enqueued_at" AS r_destroy_enqueued_at, "rooms"."direct_member_key" AS r_direct_member_key, "rooms"."icon_name" AS r_icon_name, "rooms"."inbound_email_token" AS r_inbound_email_token, "rooms"."name" AS r_name, "rooms"."pins_changed_at" AS r_pins_changed_at, "rooms"."type" AS r_type, "rooms"."updated_at" AS r_updated_at"#;

    pub(crate) fn from_prefixed_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("r_id")?,
            name: row.get("r_name")?,
            room_type: row.get("r_type")?,
            creator_id: row.get("r_creator_id")?,
            deleted_at: row.get("r_deleted_at")?,
            destroy_enqueued_at: row.get("r_destroy_enqueued_at")?,
            direct_member_key: row.get("r_direct_member_key")?,
            icon_name: row.get("r_icon_name")?,
            inbound_email_token: row.get("r_inbound_email_token")?,
            pins_changed_at: row.get("r_pins_changed_at")?,
            created_at: row.get("r_created_at")?,
            updated_at: row.get("r_updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("Room")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "rooms" WHERE "rooms"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )
    }

    pub fn all(conn: &Connection) -> Result<Vec<Self>> {
        query_all(conn, r#"SELECT * FROM "rooms""#, [], Self::from_row)
    }

    /// `Room.opens` / `closeds` / `directs` / `voices` / `boards` (and `where(type:)` for stages)
    pub fn of_type(conn: &Connection, room_type: RoomType) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "rooms" WHERE "rooms"."type" = ?"#,
            [room_type],
            Self::from_row,
        )
    }

    pub fn count_of_type(conn: &Connection, room_type: RoomType) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "rooms" WHERE "rooms"."type" = ?"#,
            [room_type],
        )
    }

    /// `Room.original`: the oldest room.
    pub fn original(conn: &Connection) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "rooms" ORDER BY "rooms"."created_at" ASC LIMIT 1"#,
            [],
            Self::from_row,
        )
    }

    // `Current.user.rooms` and its scopes

    /// `user.rooms`
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(conn, SELECT_FOR_USER, [user_id], Self::from_row)
    }

    /// `user.rooms.find_by(id:)`
    pub fn find_for_user(conn: &Connection, user_id: i64, room_id: i64) -> Result<Option<Self>> {
        let sql = format!(r#"{SELECT_FOR_USER} AND "rooms"."id" = ? LIMIT 1"#);
        query_one(conn, &sql, [user_id, room_id], Self::from_row)
    }

    /// `user.rooms.directs` / `.opens` / `.closeds`
    pub fn for_user_of_type(
        conn: &Connection,
        user_id: i64,
        room_type: RoomType,
    ) -> Result<Vec<Self>> {
        let sql = format!(r#"{SELECT_FOR_USER} AND "rooms"."type" = ?"#);
        query_all(conn, &sql, params![user_id, room_type], Self::from_row)
    }

    /// `user.rooms.without_directs`
    pub fn for_user_without_directs(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        let sql = format!(r#"{SELECT_FOR_USER} AND "rooms"."type" != ?"#);
        query_all(
            conn,
            &sql,
            params![user_id, RoomType::Direct],
            Self::from_row,
        )
    }

    /// `user.rooms.original`
    pub fn original_for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        let sql = format!(r#"{SELECT_FOR_USER} ORDER BY "rooms"."created_at" ASC LIMIT 1"#);
        query_one(conn, &sql, [user_id], Self::from_row)
    }

    /// `user.rooms.last`
    pub fn last_for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        let sql = format!(r#"{SELECT_FOR_USER} ORDER BY "rooms"."id" DESC LIMIT 1"#);
        query_one(conn, &sql, [user_id], Self::from_row)
    }

    // Creating

    /// `Rooms::<Type>.create!(name:, creator:)`. An open room grants itself to every active
    /// user after commit (`Rooms::Open#grant_access_to_all_users`).
    pub fn create(
        tx: &mut Tx<'_>,
        room_type: RoomType,
        name: Option<&str>,
        creator_id: i64,
    ) -> Result<Self> {
        Self::insert(tx, room_type, name, creator_id, None)
    }

    fn insert(
        tx: &mut Tx<'_>,
        room_type: RoomType,
        name: Option<&str>,
        creator_id: i64,
        direct_member_key: Option<&str>,
    ) -> Result<Self> {
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "rooms" ("created_at", "creator_id", "direct_member_key", "name", "type", "updated_at") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, creator_id, direct_member_key, name, room_type, now],
            |r| r.get(0),
        )?;
        if room_type == RoomType::Open {
            tx.after_commit(move |tx| grant_to_active_users(tx, id));
        }
        Self::find(tx.conn(), id)
    }

    /// `Room.create_for(attributes, users:)`, and its overrides: a direct room is created with
    /// its member key (`Rooms::Direct.create_for`), and a stage makes every member a listener
    /// and the creator its host (`Rooms::Stage.create_for`).
    pub fn create_for(
        tx: &mut Tx<'_>,
        room_type: RoomType,
        name: Option<&str>,
        creator_id: i64,
        user_ids: &[i64],
    ) -> Result<Self> {
        let key = (room_type == RoomType::Direct).then(|| Self::direct_member_key_for(user_ids));
        let mut room = Self::insert(tx, room_type, name, creator_id, key.as_deref())?;
        room.grant_to(tx, user_ids)?;
        if room_type == RoomType::Stage {
            room.make_creator_the_host(tx)?;
        }
        room.reload(tx.conn())?;
        Ok(room)
    }

    /// The rest of `Rooms::Stage.create_for`: `memberships.where(stage_role: nil)
    /// .update_all(stage_role: :listener)`, then the creator's membership (created if the
    /// creator wasn't among the users) is updated to host.
    fn make_creator_the_host(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "stage_role" = ? WHERE "memberships"."room_id" = ? AND "memberships"."stage_role" IS NULL"#,
            params![StageRole::Listener, self.id],
        )?;
        let now = tx.now();
        match Membership::find_by_room_and_user(tx.conn(), self.id, self.creator_id)? {
            Some(membership) if membership.stage_role == Some(StageRole::Host) => {}
            Some(membership) => {
                tx.conn().execute_cached(
                    r#"UPDATE "memberships" SET "stage_role" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
                    params![StageRole::Host, now, membership.id],
                )?;
            }
            // `find_or_create_by!` (a listener by `default_stage_role`), then `update!` to host.
            None => {
                let id: i64 = tx.conn().query_row_cached(
                    r#"INSERT INTO "memberships" ("created_at", "room_id", "stage_role", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?) RETURNING "id""#,
                    params![now, self.id, StageRole::Listener, now, self.creator_id],
                    |r| r.get(0),
                )?;
                tx.conn().execute_cached(
                    r#"UPDATE "memberships" SET "stage_role" = ?, "updated_at" = ? WHERE "memberships"."id" = ?"#,
                    params![StageRole::Host, tx.now(), id],
                )?;
            }
        }
        Ok(())
    }

    /// `Rooms::Direct.find_or_create_for(users)`: the direct room whose members are exactly
    /// `user_ids`, created (by `creator_id`, i.e. `Current.user`) if there isn't one. A create
    /// that loses the member key to a concurrent one (`RecordNotUnique`) reuses that room.
    pub fn find_or_create_direct_for(
        tx: &mut Tx<'_>,
        user_ids: &[i64],
        creator_id: i64,
    ) -> Result<Self> {
        if let Some(room) = Self::find_direct_for(tx.conn(), user_ids)? {
            return Ok(room);
        }
        match Self::create_for(tx, RoomType::Direct, None, creator_id, user_ids) {
            Err(e) if e.is_record_not_unique() => match Self::find_direct_for(tx.conn(), user_ids)? {
                Some(room) => Ok(room),
                None => Err(e),
            },
            result => result,
        }
    }

    /// `Rooms::Direct.find_for(users)`: the alive direct room keyed by the exact member set, else
    /// (`find_unkeyed_for`) an alive direct room with no key whose members are exactly those.
    pub fn find_direct_for(conn: &Connection, user_ids: &[i64]) -> Result<Option<Self>> {
        let keyed = query_one(
            conn,
            r#"SELECT "rooms".* FROM "rooms" WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type" = ? AND "rooms"."direct_member_key" = ? LIMIT 1"#,
            params![RoomType::Direct, Self::direct_member_key_for(user_ids)],
            Self::from_row,
        )?;
        if keyed.is_some() {
            return Ok(keyed);
        }
        Self::find_unkeyed_direct_for(conn, user_ids)
    }

    /// `Rooms::Direct.find_unkeyed_for(ids)`: rooms created before the member key, in id order.
    fn find_unkeyed_direct_for(conn: &Connection, user_ids: &[i64]) -> Result<Option<Self>> {
        let mut wanted = user_ids.to_vec();
        wanted.sort_unstable();
        let candidates = query_all(
            conn,
            r#"SELECT "rooms".* FROM "rooms" WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type" = ? AND "rooms"."direct_member_key" IS NULL"#,
            [RoomType::Direct],
            Self::from_row,
        )?;
        for room in candidates {
            let mut members = room.user_ids(conn)?;
            members.sort_unstable();
            if members == wanted {
                return Ok(Some(room));
            }
        }
        Ok(None)
    }

    /// `Rooms::Direct.member_key_for(user_ids)`: `"dm:"` and the SHA-256 hex of the sorted ids
    /// joined by commas. The migration that backfilled keys computes the same in SQL.
    pub fn direct_member_key_for(user_ids: &[i64]) -> String {
        let mut ids = user_ids.to_vec();
        ids.sort_unstable();
        let joined = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        format!("dm:{}", hex::encode(Sha256::digest(joined.as_bytes())))
    }

    /// `refresh_direct_member_key!` (`app/models/rooms/direct.rb`), after members change. A room
    /// that's been deleted keeps no key. A named room that shrank to a pair (or fewer) takes a
    /// room-suffixed key, so the pair's "Message" opens a fresh one-to-one room; so does a room
    /// whose new member set collides with another alive room's key: membership changes never
    /// merge two rooms. Written with `update_column`, so `updated_at` stays.
    pub fn refresh_direct_member_key(&self, tx: &mut Tx<'_>) -> Result<()> {
        let Some(room) = Self::find_by_id(tx.conn(), self.id)? else { return Ok(()) };
        if room.deleted_at.is_some() {
            return Ok(());
        }
        let user_ids: Vec<i64> = query_all(
            tx.conn(),
            r#"SELECT "memberships"."user_id" FROM "memberships" WHERE "memberships"."room_id" = ?"#,
            [room.id],
            |r| r.get(0),
        )?;
        let mut candidate = Self::direct_member_key_for(&user_ids);
        let squats_pair_key = user_ids.len() <= 2 && room.name.as_deref().is_some_and(|n| !n.trim().is_empty());
        let taken = sql::exists(
            tx.conn(),
            r#"SELECT 1 AS one FROM "rooms" WHERE "rooms"."deleted_at" IS NULL AND "rooms"."type" = ? AND "rooms"."id" != ? AND "rooms"."direct_member_key" = ? LIMIT 1"#,
            params![RoomType::Direct, room.id, candidate],
        )?;
        if squats_pair_key || taken {
            candidate = format!("{candidate}#{}", room.id);
        }
        if room.direct_member_key.as_deref() == Some(candidate.as_str()) {
            return Ok(());
        }
        match set_direct_member_key(tx, room.id, &candidate) {
            Err(e) if e.is_record_not_unique() => {
                set_direct_member_key(tx, room.id, &format!("{candidate}#{}", room.id))
            }
            result => result,
        }
    }

    // Updating

    /// `room.update!(name:, type:)`. A direct room can't change type
    /// (`direct_rooms_keep_their_type`). Becoming open grants every active user after commit.
    pub fn update(
        &mut self,
        tx: &mut Tx<'_>,
        name: Option<Option<&str>>,
        room_type: Option<RoomType>,
    ) -> Result<()> {
        let name = name
            .map(|n| n.map(str::to_string))
            .filter(|n| *n != self.name);
        let room_type = room_type.filter(|t| *t != self.room_type);

        if room_type.is_some() && self.room_type == RoomType::Direct {
            let mut errors = Errors::default();
            errors.add("type", "can't be changed for a direct room");
            return errors.into_result();
        }
        if name.is_none() && room_type.is_none() {
            return Ok(());
        }

        let now = tx.now();
        if let Some(name) = &name {
            self.name = name.clone();
        }
        if let Some(room_type) = room_type {
            self.room_type = room_type;
        }
        self.updated_at = now;
        tx.conn().execute_cached(
            r#"UPDATE "rooms" SET "name" = ?, "type" = ?, "updated_at" = ? WHERE "rooms"."id" = ?"#,
            params![self.name, self.room_type, now, self.id],
        )?;
        if room_type == Some(RoomType::Open) && self.deleted_at.is_none() {
            let id = self.id;
            tx.after_commit(move |tx| grant_to_active_users(tx, id));
        }
        Ok(())
    }

    /// `touch`: `belongs_to :room, touch: true` on messages.
    pub fn touch(tx: &Tx<'_>, room_id: i64) -> Result<()> {
        tx.conn().execute_cached(
            r#"UPDATE "rooms" SET "updated_at" = ? WHERE "rooms"."id" = ?"#,
            params![tx.now(), room_id],
        )?;
        Ok(())
    }

    /// `room.destroy`: memberships are deleted without callbacks, messages are destroyed. Upstream's
    /// shape: our rooms are soft-deleted first (`begin_destroy!`) and destroyed in batches by
    /// `Room::DestroyJob`, which clears many more dependents; neither is ported yet.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "memberships" WHERE "memberships"."room_id" = ?"#,
            [self.id],
        )?;
        for message in Message::for_room(tx.conn(), self.id)? {
            message.destroy(tx)?;
        }
        tx.conn()
            .execute_cached(r#"DELETE FROM "rooms" WHERE "rooms"."id" = ?"#, [self.id])?;
        Ok(())
    }

    // Memberships

    pub fn memberships(&self, conn: &Connection) -> Result<Vec<Membership>> {
        Membership::for_room(conn, self.id)
    }

    /// `memberships.grant_to(users)`: `Membership.insert_all` with the room's default
    /// involvement, skipping existing members. `insert_all` skips the membership callbacks, so a
    /// direct room refreshes its member key here.
    pub fn grant_to(&self, tx: &mut Tx<'_>, user_ids: &[i64]) -> Result<()> {
        insert_memberships(tx, self.id, self.room_type.default_involvement(), user_ids)?;
        if self.direct() {
            self.refresh_direct_member_key(tx)?;
        }
        Ok(())
    }

    /// `memberships.revoke_from(users)`: each removed membership is destroyed, so its member is
    /// disconnected (with reconnect) after commit. Stage hosts go last.
    pub fn revoke_from(&self, tx: &mut Tx<'_>, user_ids: &[i64]) -> Result<()> {
        let sql = format!(
            r#"SELECT "memberships".* FROM "memberships" WHERE "memberships"."room_id" = ? AND "memberships"."user_id" IN ({})"#,
            placeholders(user_ids.len())
        );
        let values: Vec<i64> = std::iter::once(self.id)
            .chain(user_ids.iter().copied())
            .collect();
        let mut memberships = query_all(
            tx.conn(),
            &sql,
            rusqlite::params_from_iter(values),
            Membership::from_row,
        )?;
        memberships.sort_by_key(|m| m.stage_role == Some(StageRole::Host));
        for membership in memberships {
            membership.destroy(tx)?;
        }
        Ok(())
    }

    /// `memberships.revise(granted:, revoked:)`
    pub fn revise(&self, tx: &mut Tx<'_>, granted: &[i64], revoked: &[i64]) -> Result<()> {
        if !granted.is_empty() {
            self.grant_to(tx, granted)?;
        }
        if !revoked.is_empty() {
            self.revoke_from(tx, revoked)?;
        }
        Ok(())
    }

    /// `room.users`
    pub fn users(&self, conn: &Connection) -> Result<Vec<User>> {
        query_all(
            conn,
            r#"SELECT "users".* FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ?"#,
            [self.id],
            User::from_row,
        )
    }

    /// `room.user_ids`
    pub fn user_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        query_all(
            conn,
            r#"SELECT "users"."id" FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ?"#,
            [self.id],
            |r| r.get(0),
        )
    }

    /// `room.users.active_bots`
    pub fn active_bots(&self, conn: &Connection) -> Result<Vec<User>> {
        query_all(
            conn,
            r#"SELECT "users".* FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ? AND "users"."status" = 0 AND "users"."role" = 2"#,
            [self.id],
            User::from_row,
        )
    }

    /// `room.receive(message)`, from the message's `after_create_commit`: marks members
    /// unread. A system note doesn't. Its `push_later` is [`Room::push_later`], called in the
    /// message's transaction.
    pub(crate) fn receive(tx: &mut Tx<'_>, room_id: i64, message: &Message) -> Result<()> {
        if message.system_note {
            return Ok(());
        }
        Self::unread_memberships(tx, room_id, message)
    }

    /// `push_later(message)`, the rest of `room.receive(message)`: `Room::PushMessageJob`. Rails
    /// enqueues it after commit; here its row is written in the message's transaction, so the
    /// message and its push commit (or roll back) together, and the runner is woken after
    /// commit. A system note isn't pushed.
    pub(crate) fn push_later(tx: &mut Tx<'_>, room_id: i64, message: &Message) {
        if !message.system_note {
            tx.emit_after_commit(Event::PushMessage {
                room_id,
                message_id: message.id,
            });
        }
    }

    /// `unread_memberships(message)` (`app/models/room.rb`): visible, disconnected members other
    /// than the author go unread, muted ones only when mentioned. Members watching live, and the
    /// author, have their read pointer advanced to the message unless they're already unread.
    fn unread_memberships(tx: &Tx<'_>, room_id: i64, message: &Message) -> Result<()> {
        let now = tx.now();
        let cutoff = Membership::connection_cutoff(now);
        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "unread_at" = ?, "updated_at" = ? WHERE "memberships"."room_id" = ? AND "memberships"."involvement" != 'invisible' AND ("memberships"."connected_at" IS NULL OR "memberships"."connected_at" < ?) AND "memberships"."user_id" != ? AND "memberships"."involvement" != 'muted'"#,
            params![message.created_at, now, room_id, cutoff, message.creator_id],
        )?;

        // Only a room with muted recipients needs the message's mentions.
        let muted_recipients = sql::exists(
            tx.conn(),
            r#"SELECT 1 AS one FROM "memberships" WHERE "memberships"."room_id" = ? AND "memberships"."involvement" != 'invisible' AND ("memberships"."connected_at" IS NULL OR "memberships"."connected_at" < ?) AND "memberships"."user_id" != ? AND "memberships"."involvement" = 'muted' LIMIT 1"#,
            params![room_id, cutoff, message.creator_id],
        )?;
        let mentionee_ids: Vec<i64> = if muted_recipients {
            message.mentionees(tx.conn(), tx.rich_text())?.into_iter().map(|user| user.id).collect()
        } else {
            Vec::new()
        };
        if !mentionee_ids.is_empty() {
            let sql = format!(
                r#"UPDATE "memberships" SET "unread_at" = ?, "updated_at" = ? WHERE "memberships"."room_id" = ? AND "memberships"."involvement" != 'invisible' AND ("memberships"."connected_at" IS NULL OR "memberships"."connected_at" < ?) AND "memberships"."user_id" != ? AND "memberships"."involvement" = 'muted' AND "memberships"."user_id" IN ({})"#,
                placeholders(mentionee_ids.len())
            );
            let mut values: Vec<rusqlite::types::Value> = vec![
                message.created_at.to_db().into(),
                now.to_db().into(),
                room_id.into(),
                cutoff.to_db().into(),
                message.creator_id.into(),
            ];
            values.extend(mentionee_ids.into_iter().map(rusqlite::types::Value::from));
            tx.conn().execute(&sql, rusqlite::params_from_iter(values))?;
        }

        tx.conn().execute_cached(
            r#"UPDATE "memberships" SET "last_read_message_id" = ?, "updated_at" = ? WHERE "memberships"."room_id" = ? AND "memberships"."involvement" != 'invisible' AND "memberships"."unread_at" IS NULL AND (memberships.connected_at >= ? OR memberships.user_id = ?)"#,
            params![message.id, now, room_id, cutoff, message.creator_id],
        )?;
        Ok(())
    }

    /// app/models/room.rb: token rotation is independent of whether inbound email is configured.
    pub fn regenerate_inbound_email_token(&mut self, tx: &Tx<'_>) -> Result<String> {
        loop {
            let token = hex::encode(rand::random::<[u8; 16]>());
            match tx.conn().execute_cached(
                "UPDATE rooms SET inbound_email_token = ?, updated_at = ? WHERE id = ?",
                params![token, tx.now(), self.id],
            ) {
                Ok(_) => { self.inbound_email_token = Some(token.clone()); self.updated_at = tx.now(); return Ok(token); }
                Err(rusqlite::Error::SqliteFailure(error, _)) if error.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub fn emailable(&self) -> bool { !self.direct() && !self.board() }

    pub fn open(&self) -> bool {
        self.room_type == RoomType::Open
    }

    pub fn closed(&self) -> bool {
        self.room_type == RoomType::Closed
    }

    pub fn direct(&self) -> bool {
        self.room_type == RoomType::Direct
    }

    pub fn voice(&self) -> bool {
        self.room_type == RoomType::Voice
    }

    pub fn stage(&self) -> bool {
        self.room_type == RoomType::Stage
    }

    pub fn board(&self) -> bool {
        self.room_type == RoomType::Board
    }

    /// `deleted?`
    pub fn deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn default_involvement(&self) -> &'static str {
        self.room_type.default_involvement()
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
}

/// `Membership.insert_all(... { room_id:, user_id:, involvement: })`
fn insert_memberships(
    tx: &Tx<'_>,
    room_id: i64,
    involvement: &str,
    user_ids: &[i64],
) -> Result<()> {
    // In batches: SQLite binds at most 32,766 variables per statement, 3 per row here.
    for user_ids in user_ids.chunks(MEMBERSHIP_INSERT_BATCH) {
        let rows: Vec<String> = user_ids
            .iter()
            .map(|_| format!("({SQLITE_NOW}, ?, ?, {SQLITE_NOW}, ?)"))
            .collect();
        let sql = format!(
            r#"INSERT INTO "memberships" ("created_at","involvement","room_id","updated_at","user_id") VALUES {} ON CONFLICT  DO NOTHING RETURNING "id""#,
            rows.join(", ")
        );
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        for user_id in user_ids {
            values.push(involvement.to_string().into());
            values.push(room_id.into());
            values.push((*user_id).into());
        }
        let mut stmt = tx.conn().prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(values))?;
        while rows.next()?.is_some() {}
    }
    Ok(())
}

/// Rows per `INSERT` of memberships, well under SQLite's bound-variable limit.
pub(crate) const MEMBERSHIP_INSERT_BATCH: usize = 1_000;

/// `update_column(:direct_member_key, key)`: no `updated_at`.
fn set_direct_member_key(tx: &Tx<'_>, room_id: i64, key: &str) -> Result<()> {
    tx.conn().execute_cached(
        r#"UPDATE "rooms" SET "direct_member_key" = ? WHERE "rooms"."id" = ?"#,
        params![key, room_id],
    )?;
    Ok(())
}

/// `memberships.grant_to(User.active)`, from `Rooms::Open`'s `after_save_commit`, unless the
/// room was deleted meanwhile.
fn grant_to_active_users(tx: &mut Tx<'_>, room_id: i64) -> Result<()> {
    let user_ids: Vec<i64> = query_all(
        tx.conn(),
        r#"SELECT "users"."id" FROM "users" WHERE "users"."status" = ?"#,
        [0],
        |r| r.get(0),
    )?;
    let room = Room::find(tx.conn(), room_id)?;
    if room.deleted() {
        return Ok(());
    }
    insert_memberships(tx, room_id, room.default_involvement(), &user_ids)
}

//! `reference/app/models/user.rb` and `user/*.rb` (Role, Bot, Bannable, Mentionable; Avatar
//! and Transferable are signed ids, which live in `rails_compat`).

pub mod removal;
pub mod icon;
pub mod lifecycle;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Row, params};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::database::Tx;
use crate::error::{OptionalExt, Result};
use crate::events::Event;
use crate::models::{Ban, Membership, Message, Session, Webhook};
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::{SQLITE_NOW, Timestamp};

pub mod presentation;
pub mod profile_settings;

/// `enum :role, %i[ member administrator bot ]`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Role {
    #[default]
    Member = 0,
    Administrator = 1,
    Bot = 2,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Member => "member",
            Role::Administrator => "administrator",
            Role::Bot => "bot",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "member" => Some(Role::Member),
            "administrator" => Some(Role::Administrator),
            "bot" => Some(Role::Bot),
            _ => None,
        }
    }
}

/// `enum :status, %i[ active deactivated banned ], default: :active`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status {
    #[default]
    Active = 0,
    Deactivated = 1,
    Banned = 2,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Deactivated => "deactivated",
            Status::Banned => "banned",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "active" => Some(Status::Active),
            "deactivated" => Some(Status::Deactivated),
            "banned" => Some(Status::Banned),
            _ => None,
        }
    }
}

macro_rules! integer_enum_sql {
    ($ty:ty, $($variant:path = $value:literal),+) => {
        impl ToSql for $ty {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(*self as i64))
            }
        }

        impl FromSql for $ty {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                match value.as_i64()? {
                    $($value => Ok($variant),)+
                    other => Err(FromSqlError::OutOfRange(other)),
                }
            }
        }
    };
}

integer_enum_sql!(
    Role,
    Role::Member = 0,
    Role::Administrator = 1,
    Role::Bot = 2
);
integer_enum_sql!(
    Status,
    Status::Active = 0,
    Status::Deactivated = 1,
    Status::Banned = 2
);

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email_address: Option<String>,
    pub password_digest: Option<String>,
    pub role: Role,
    pub status: Status,
    pub bio: Option<String>,
    pub icon_name: Option<String>,
    /// SHA-256 hex of the bot token (`User::Bot`). The plaintext `bot_token` column is retired:
    /// never written except to clear it, never read.
    pub bot_token_digest: Option<String>,
    /// `plain_bot_token`: the token, known only on the value that just created or reset it.
    /// Never persisted.
    pub plain_bot_token: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Attributes for `User.create!`.
#[derive(Debug, Clone, Default)]
pub struct NewUser {
    pub name: String,
    pub email_address: Option<String>,
    /// `has_secure_password`'s `password=`, already hashed.
    pub password_digest: Option<PasswordDigest>,
    pub role: Role,
    pub bio: Option<String>,
    pub icon_name: Option<String>,
    pub bot_token_digest: Option<String>,
}

/// Attributes for `user.update`. `None` leaves an attribute alone.
#[derive(Debug, Clone, Default)]
pub struct UserChanges {
    pub name: Option<String>,
    pub email_address: Option<Option<String>>,
    pub password_digest: Option<PasswordDigest>,
    pub role: Option<Role>,
    pub status: Option<Status>,
    pub bio: Option<Option<String>>,
    pub icon_name: Option<Option<String>>,
    pub time_zone: Option<Option<String>>,
    /// A submitted zone key is an explicit choice even when its value is filtered or nil.
    pub time_zone_explicit: Option<bool>,
    /// `Users::ProfilesController`: blocks Google email auto-linking after a self-change.
    pub email_self_changed_at: Option<Timestamp>,
}

const INSERT: &str = r#"INSERT INTO "users" ("bio", "bot_token_digest", "created_at", "email_address", "icon_name", "name", "password_digest", "role", "status", "updated_at") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#;

impl User {
    /// A `SELECT "users".*` row.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            email_address: row.get("email_address")?,
            password_digest: row.get("password_digest")?,
            role: row.get("role")?,
            status: row.get("status")?,
            bio: row.get("bio")?,
            icon_name: row.get("icon_name")?,
            bot_token_digest: row.get("bot_token_digest")?,
            plain_bot_token: None,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    // Finders and scopes

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("User")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )
    }

    /// `User.active.find(id)`
    pub fn find_active(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("User")
    }

    pub fn find_by_email_address(conn: &Connection, email_address: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."email_address" = ? LIMIT 1"#,
            [email_address],
            Self::from_row,
        )
    }

    pub fn all(conn: &Connection) -> Result<Vec<Self>> {
        query_all(conn, r#"SELECT * FROM "users""#, [], Self::from_row)
    }

    pub fn count(conn: &Connection) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "users""#, [])
    }

    /// `User.where(id: ids)`
    pub fn where_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        let sql = format!(
            r#"SELECT * FROM "users" WHERE "users"."id" IN ({})"#,
            placeholders(ids.len())
        );
        query_all(conn, &sql, rusqlite::params_from_iter(ids), Self::from_row)
    }

    /// `User.active.ordered`
    pub fn active_ordered(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 ORDER BY LOWER(name)"#,
            [],
            Self::from_row,
        )
    }

    /// `User.active`
    pub fn active(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0"#,
            [],
            Self::from_row,
        )
    }

    /// `User.active.filtered_by(query).ordered`
    pub fn active_filtered_by_ordered(conn: &Connection, query: &str) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND (name like ?) ORDER BY LOWER(name)"#,
            [format!("%{query}%")],
            Self::from_row,
        )
    }

    /// `User.active.ordered.without_bots`
    pub fn active_ordered_without_bots(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."role" != 2 ORDER BY LOWER(name)"#,
            [],
            Self::from_row,
        )
    }

    /// `User.active_bots.ordered`
    pub fn active_bots_ordered(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."role" = 2 ORDER BY LOWER(name)"#,
            [],
            Self::from_row,
        )
    }

    /// `User.active_bots.find(id)`
    pub fn find_active_bot(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."role" = 2 AND "users"."id" = ? LIMIT 1"#, [id], Self::from_row)?
            .or_not_found("User")
    }

    /// `User.active.find_by(email_address:)`: the lookup half of `authenticate_by`.
    pub fn find_active_by_email_address(conn: &Connection, email_address: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."email_address" = ? LIMIT 1"#,
            [email_address],
            Self::from_row,
        )
    }

    /// The password half of `User.active.authenticate_by(email_address:, password:)`, given
    /// what [`User::find_active_by_email_address`] found. Blocking (bcrypt), so it runs with no
    /// connection held. A blank password returns nil before the lookup in Rails; callers skip
    /// the lookup for one too.
    pub fn authenticated(candidate: Option<Self>, password: &str) -> Option<Self> {
        if password.is_empty() {
            return None;
        }
        match candidate {
            Some(user) => user.authenticate(password).then_some(user),
            None => {
                // authenticate_by hashes anyway so a missing account takes as long as a wrong password.
                let _ = bcrypt::verify(password, DUMMY_DIGEST);
                None
            }
        }
    }

    /// `User.authenticate_bot(bot_key)` (`app/models/user/bot.rb`): the key is `"<id>-<token>"`,
    /// and the token's SHA-256 digest is compared in constant time with `bot_token_digest`. The
    /// plaintext `bot_token` column is never consulted, so a bot without a digest can't
    /// authenticate until its key is reset.
    pub fn authenticate_bot(conn: &Connection, bot_key: &str) -> Result<Option<Self>> {
        // `bot_key.to_s.split("-", 2)`: the token keeps any further dashes.
        let (id, token) = bot_key.split_once('-').unwrap_or((bot_key, ""));
        if is_blank(id) || is_blank(token) || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        // `find_by(id:)` casts "007" to 7; an id past the integer range finds nothing.
        let Ok(id) = id.parse::<i64>() else { return Ok(None) };
        let Some(bot) = query_one(
            conn,
            r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."role" = 2 AND "users"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        else {
            return Ok(None);
        };
        let Some(digest) = bot.bot_token_digest.as_deref().filter(|d| !is_blank(d)) else {
            return Ok(None);
        };
        Ok(secure_compare(digest.as_bytes(), digest_bot_token(token).as_bytes()).then_some(bot))
    }

    // Creating

    /// `User.create!`: inserts, then grants memberships to every open room after commit.
    pub fn create(tx: &mut Tx<'_>, attributes: NewUser) -> Result<Self> {
        Self::create_with_open_room_grant(tx, attributes, true)
    }

    /// User.create_bot!(skip_open_room_grant: true), used by RoomMailbox.
    pub fn create_email_bot(tx: &mut Tx<'_>) -> Result<Self> {
        Self::create_integration_bot(tx, "Email")
    }

    /// `User.create_bot!(name:, skip_open_room_grant: true)` for integration bots.
    pub fn create_integration_bot(tx: &mut Tx<'_>, name: &str) -> Result<Self> {
        let token = generate_bot_token();
        Self::create_with_open_room_grant(tx, NewUser {
            name: name.into(), role: Role::Bot,
            bot_token_digest: Some(digest_bot_token(&token)), ..Default::default()
        }, false)
    }

    fn create_with_open_room_grant(tx: &mut Tx<'_>, mut attributes: NewUser, grant: bool) -> Result<Self> {
        attributes.icon_name = icon::normalize_name(attributes.icon_name.as_deref());
        icon::validate(tx.conn(), attributes.icon_name.as_deref())?.into_result()?;
        let now = tx.now();
        let password_digest = attributes.password_digest.map(PasswordDigest::into_string);
        let id: i64 = tx.conn().query_row_cached(
            INSERT,
            params![
                attributes.bio,
                attributes.bot_token_digest,
                now,
                attributes.email_address,
                attributes.icon_name,
                attributes.name,
                password_digest,
                attributes.role,
                Status::Active,
                now
            ],
            |r| r.get(0),
        )?;
        if grant { tx.after_commit(move |tx| grant_membership_to_open_rooms(tx, id)); }
        Self::find(tx.conn(), id)
    }

    /// `User.create_bot!`: stores the token's digest alone; the returned bot knows its key
    /// (`plain_bot_token`) until it's dropped.
    pub fn create_bot(tx: &mut Tx<'_>, name: &str, webhook_url: Option<&str>) -> Result<Self> {
        Self::create_bot_with_attributes(tx, NewUser { name: name.into(), ..Default::default() }, webhook_url)
    }

    /// Bot creation with the same normalized/validated fields as ordinary User creation.
    pub fn create_bot_with_attributes(tx: &mut Tx<'_>, mut attributes: NewUser, webhook_url: Option<&str>) -> Result<Self> {
        let token = generate_bot_token();
        attributes.role = Role::Bot;
        attributes.bot_token_digest = Some(digest_bot_token(&token));
        let mut user = Self::create(tx, attributes)?;
        user.plain_bot_token = Some(token);
        if let Some(url) = webhook_url {
            Webhook::create(tx, user.id, Some(url))?;
        }
        Ok(user)
    }

    // Updating

    /// `Users::TimeZonesController`: auto-detection only fills an unset, non-explicit choice.
    /// Use the same User validations as the other settings writes, including persisted fields.
    pub fn detect_browser_time_zone(tx: &Tx<'_>, user_id: i64, zone: &str) -> Result<Option<String>> {
        let (saved, explicit): (Option<String>, bool) = tx.conn().query_row(
            "SELECT time_zone,time_zone_explicit FROM users WHERE id=?", [user_id], |row| Ok((row.get(0)?,row.get(1)?)),
        )?;
        if crate::slash_commands::time_parser::known_zone(zone).is_some()
            && !explicit && saved.as_deref().is_none_or(|zone| zone.chars().all(char::is_whitespace)) {
            crate::slash_commands::user_settings::update(tx, user_id, serde_json::json!({"time_zone":zone}))?;
        }
        Self::saved_time_zone(tx.conn(), user_id)
    }

    pub fn saved_time_zone(conn: &Connection, user_id: i64) -> Result<Option<String>> {
        Ok(conn.query_row("SELECT time_zone FROM users WHERE id=?", [user_id], |row| row.get(0))?)
    }

    /// `Current.user.touch(:tour_completed_at)`: refresh both stamps, skipping validations.
    pub fn complete_tour(tx: &Tx<'_>, user_id: i64) -> Result<()> {
        tx.conn().execute("UPDATE users SET tour_completed_at=?,updated_at=? WHERE id=?", params![tx.now(),tx.now(),user_id])?;
        Ok(())
    }

    /// `user.update(attributes)`: writes only what changed, and nothing at all (not even
    /// `updated_at`) when nothing did.
    pub fn update(&mut self, tx: &mut Tx<'_>, changes: UserChanges) -> Result<()> {
        // `User::StatusSettings`: blank Not set normalizes to nil; unknown zones fail save.
        let zone = changes.time_zone
            .map(|zone| zone.filter(|value| !campfire_richtext::ruby::is_blank(value)));
        let current_zone: Option<String> = tx.conn().query_row(
            "SELECT time_zone FROM users WHERE id=?", [self.id], |r| r.get(0),
        )?;
        // Rails validates the effective zone on every save, including an unchanged
        // persisted value; no other field or security marker may bypass that validation.
        if zone.as_ref().unwrap_or(&current_zone).as_deref()
            .filter(|name| !campfire_richtext::ruby::is_blank(name))
            .is_some_and(|name| crate::slash_commands::time_parser::known_zone(name).is_none())
        {
            let mut errors = crate::Errors::default();
            errors.add("time_zone", "is not a valid time zone");
            return Err(crate::Error::RecordInvalid(errors));
        }
        let icon = changes.icon_name.map(|name| icon::normalize_name(name.as_deref()));
        if let Some(name) = &icon && *name != self.icon_name {
            icon::validate(tx.conn(), name.as_deref())?.into_result()?;
        }
        let mut sets: Vec<(&str, Box<dyn rusqlite::ToSql>)> = Vec::new();
        if let Some(name) = icon.filter(|name| *name != self.icon_name) {
            self.icon_name = name.clone();
            sets.push(("icon_name", Box::new(name)));
        }
        if let Some(name) = changes.name.filter(|n| *n != self.name) {
            self.name = name.clone();
            sets.push(("name", Box::new(name)));
        }
        if let Some(email) = changes.email_address.filter(|e| *e != self.email_address) {
            self.email_address = email.clone();
            sets.push(("email_address", Box::new(email)));
        }
        if let Some(digest) = changes.password_digest.map(PasswordDigest::into_string) {
            self.password_digest = Some(digest.clone());
            sets.push(("password_digest", Box::new(digest)));
        }
        if let Some(role) = changes.role.filter(|r| *r != self.role) {
            self.role = role;
            sets.push(("role", Box::new(role)));
        }
        if let Some(status) = changes.status.filter(|s| *s != self.status) {
            if status != Status::Active {
                crate::models::huddle_grant::HuddleGrant::revoke_for_user(tx, self.id, &crate::models::room_delete::HuddleConfig::from_env())?;
                crate::models::AgentGrant::revoke_for_user(tx, self.id)?;
            }
            self.status = status;
            sets.push(("status", Box::new(status)));
        }
        if let Some(bio) = changes.bio.filter(|b| *b != self.bio) {
            self.bio = bio.clone();
            sets.push(("bio", Box::new(bio)));
        }
        if let Some(at) = changes.email_self_changed_at {
            sets.push(("email_self_changed_at", Box::new(at)));
        }
        // These profile preferences are not part of the compact User projection. Compare
        // stored values so an unchanged assignment doesn't touch updated_at (Rails dirty tracking).
        if let Some(zone) = zone
            && zone != current_zone
        {
            sets.push(("time_zone", Box::new(zone)));
        }
        if let Some(explicit) = changes.time_zone_explicit {
            let current: Option<bool> = tx.conn().query_row(
                "SELECT time_zone_explicit FROM users WHERE id=?",
                [self.id],
                |r| r.get(0),
            )?;
            if current != Some(explicit) {
                sets.push(("time_zone_explicit", Box::new(explicit)));
            }
        }
        if sets.is_empty() {
            return Ok(());
        }
        let now = tx.now();
        self.updated_at = now;
        sets.push(("updated_at", Box::new(now)));
        let assignments: Vec<String> = sets.iter().map(|(c, _)| format!(r#""{c}" = ?"#)).collect();
        let sql = format!(
            r#"UPDATE "users" SET {} WHERE "users"."id" = ?"#,
            assignments.join(", ")
        );
        let mut values: Vec<&dyn rusqlite::ToSql> = sets.iter().map(|(_, v)| v.as_ref()).collect();
        values.push(&self.id);
        tx.conn().execute_cached(&sql, values.as_slice())?;
        Ok(())
    }

    pub fn set_icon_name(&mut self, tx: &mut Tx<'_>, name: Option<&str>) -> Result<()> {
        self.update(tx, UserChanges { icon_name: Some(name.map(str::to_owned)), ..Default::default() })
    }

    /// `update_bot!`: the webhook first, then the user, in one transaction.
    pub fn update_bot(
        &mut self,
        tx: &mut Tx<'_>,
        changes: UserChanges,
        webhook_url: Option<&str>,
    ) -> Result<()> {
        let webhook = Webhook::find_by_user(tx.conn(), self.id)?;
        match (webhook_url.filter(|u| !u.trim().is_empty()), webhook) {
            (Some(url), Some(mut webhook)) => webhook.update_url(tx, url)?,
            (Some(url), None) => {
                Webhook::create(tx, self.id, Some(url))?;
            }
            (None, Some(webhook)) => webhook.destroy(tx)?,
            (None, None) => {}
        }
        self.update(tx, changes)
    }

    /// `reset_bot_key`: a new token, invalidating the old key. `update! bot_token: nil,
    /// bot_token_digest:` stores the digest alone and clears any retired plaintext; the new key
    /// is known on `self` afterwards.
    pub fn reset_bot_key(&mut self, tx: &mut Tx<'_>) -> Result<String> {
        let token = generate_bot_token();
        let digest = digest_bot_token(&token);
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "users" SET "bot_token" = NULL, "bot_token_digest" = ?, "updated_at" = ? WHERE "users"."id" = ?"#,
            params![digest, now, self.id],
        )?;
        self.bot_token_digest = Some(digest);
        self.updated_at = now;
        self.plain_bot_token = Some(token);
        Ok(self.bot_key())
    }

    /// `deactivate`: disconnects sockets first (mid-transaction, as Rails does), then removes
    /// non-direct memberships, push subscriptions, searches and sessions, and scrambles the
    /// email address.
    pub fn deactivate(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.deactivate_with_audit(tx, &super::audit_log::Context::default())
    }
    pub fn deactivate_with_audit(&mut self, tx: &mut Tx<'_>, audit: &super::audit_log::Context) -> Result<()> {
        self.close_remote_connections(tx, false);
        let hosted_stages = super::stage::hosted_room_ids(tx,self.id)?;
        super::stream::Stream::end_for_user(tx,self.id)?;
        let conn = tx.conn();
        conn.execute_cached(
            r#"DELETE FROM "memberships" WHERE ("memberships"."id") IN (SELECT "memberships"."id" FROM "memberships" INNER JOIN "rooms" AS "room" ON "room"."id" = "memberships"."room_id" WHERE "memberships"."user_id" = ? AND "room"."type" != ?)"#,
            params![self.id, "Rooms::Direct"],
        )?;
        for room_id in hosted_stages {
            super::stage::host_departed(tx,room_id,self.id,&super::room_delete::HuddleConfig::from_env())?;
        }
        let conn = tx.conn();
        conn.execute_cached(
            r#"DELETE FROM "push_subscriptions" WHERE "push_subscriptions"."user_id" = ?"#,
            [self.id],
        )?;
        conn.execute_cached(
            r#"DELETE FROM "searches" WHERE "searches"."user_id" = ?"#,
            [self.id],
        )?;
        conn.execute_cached("DELETE FROM two_factor_setup_secrets WHERE session_id IN (SELECT id FROM sessions WHERE user_id=?)", [self.id])?;
        conn.execute_cached(
            r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ?"#,
            [self.id],
        )?;
        for disconnect in tx.env().user_deactivation_hooks.clone() {
            disconnect(tx, self)?;
        }
        conn.execute_cached("DELETE FROM user_devices WHERE user_id = ?", [self.id])?;
        lifecycle::deactivate(tx, self.id, audit)?;
        let email = self.deactivated_email_address();
        // app/models/user.rb: manual OOO cannot survive account deactivation.
        conn.execute_cached("UPDATE users SET ooo_until=NULL, ooo_note=NULL, ooo_broadcast=NULL WHERE id=?", [self.id])?;
        self.update(
            tx,
            UserChanges {
                status: Some(Status::Deactivated),
                email_address: Some(email),
                ..Default::default()
            },
        )
    }

    fn deactivated_email_address(&self) -> Option<String> {
        let uuid = sql::uuid();
        self.email_address
            .as_ref()
            .map(|e| e.replace('@', &format!("-deactivated-{uuid}@")))
    }

    /// `User::Bannable#ban`
    pub fn ban(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.ban_with_audit(tx, &super::audit_log::Context::default())
    }
    pub fn ban_with_audit(&mut self, tx: &mut Tx<'_>, audit: &super::audit_log::Context) -> Result<()> {
        // create_bans_from_sessions: `sessions.pluck(:ip_address).compact_blank.uniq`
        let ips: Vec<Option<String>> = query_all(
            tx.conn(),
            r#"SELECT "sessions"."ip_address" FROM "sessions" WHERE "sessions"."user_id" = ?"#,
            [self.id],
            |r| r.get(0),
        )?;
        let mut seen = Vec::new();
        for ip in ips.into_iter().flatten().filter(|ip| !ip.trim().is_empty()) {
            if !seen.contains(&ip) {
                Ban::create(tx, self.id, &ip)?;
                seen.push(ip);
            }
        }
        // apply_ban
        self.close_remote_connections(tx, false);
        tx.conn().execute_cached(
            r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ?"#,
            [self.id],
        )?;
        tx.emit_after_commit(Event::RemoveBannedContent { user_id: self.id });
        super::agent_lifecycle::suspend_owned(tx, self.id, audit)?;
        self.update(
            tx,
            UserChanges {
                status: Some(Status::Banned),
                ..Default::default()
            },
        )
    }

    pub fn unban(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "bans" WHERE "bans"."user_id" = ?"#,
            [self.id],
        )?;
        self.update(
            tx,
            UserChanges {
                status: Some(Status::Active),
                ..Default::default()
            },
        )
    }

    /// `remove_banned_content`: destroys every message the user wrote. Returns them so the
    /// caller can `broadcast_remove` each one.
    pub fn remove_banned_content(&self, tx: &mut Tx<'_>) -> Result<Vec<Message>> {
        let messages = Message::by_creator(tx.conn(), self.id)?;
        for message in &messages {
            message.destroy(tx)?;
        }
        Ok(messages)
    }

    /// `reset_remote_connections`: disconnect this user's sockets and let them reconnect.
    pub fn reset_remote_connections(&self, tx: &mut Tx<'_>) {
        self.close_remote_connections(tx, true);
    }

    /// After commit: a connection authenticating meanwhile then either finds the sessions gone or
    /// is already listening for this (see `campfire_cable`'s connection setup).
    fn close_remote_connections(&self, tx: &mut Tx<'_>, reconnect: bool) {
        tx.emit_after_commit(Event::DisconnectUser {
            user_id: self.id,
            reconnect,
        });
    }

    /// `deliver_webhook_later(message)`: only bots with a webhook.
    pub fn deliver_webhook_later(&self, tx: &mut Tx<'_>, message_id: i64) -> Result<()> {
        if Webhook::find_by_user(tx.conn(), self.id)?.is_some() {
            tx.emit_after_commit(Event::DeliverWebhook {
                bot_id: self.id,
                message_id,
            });
        }
        Ok(())
    }

    // Associations

    pub fn memberships(&self, conn: &Connection) -> Result<Vec<Membership>> {
        Membership::for_user(conn, self.id)
    }

    pub fn sessions(&self, conn: &Connection) -> Result<Vec<Session>> {
        Session::for_user(conn, self.id)
    }

    pub fn webhook(&self, conn: &Connection) -> Result<Option<Webhook>> {
        Webhook::find_by_user(conn, self.id)
    }

    pub fn webhook_url(&self, conn: &Connection) -> Result<Option<String>> {
        Ok(self.webhook(conn)?.and_then(|w| w.url))
    }

    // Attributes

    /// `name.scan(/\b\w/).join`. Ruby's `\w` is ASCII-only, but `\b` treats any Unicode
    /// letter or digit as a word character, so "Émile" contributes nothing.
    pub fn initials(&self) -> String {
        static WORD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"\A[\p{Alphabetic}\p{Mark}\p{Number}\p{Connector_Punctuation}\p{Join_Control}]\z").unwrap()
        });
        let mut initials = String::new();
        let mut previous_word = false;
        for c in self.name.chars() {
            if (c.is_ascii_alphanumeric() || c == '_') && !previous_word {
                initials.push(c);
            }
            previous_word = WORD.is_match(c.encode_utf8(&mut [0; 4]));
        }
        initials
    }

    /// `[ name, bio ].compact_blank.join(" – ")`
    pub fn title(&self) -> String {
        [Some(self.name.as_str()), self.bio.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" – ")
    }

    /// `plain_bot_key`: the full key while it's still known (just created or reset).
    pub fn plain_bot_key(&self) -> Option<String> {
        self.plain_bot_token
            .as_deref()
            .filter(|token| !is_blank(token))
            .map(|token| format!("{}-{token}", self.id))
    }

    /// `bot_key`: the full key while it's still known, otherwise [`BOT_KEY_PLACEHOLDER`]. Stored
    /// bots never reveal their key.
    pub fn bot_key(&self) -> String {
        self.plain_bot_key()
            .unwrap_or_else(|| BOT_KEY_PLACEHOLDER.to_string())
    }

    /// `can_administer?(record)`: administrators, the record's creator, or a new record.
    pub fn can_administer(&self, record_creator_id: Option<i64>, record_is_new: bool) -> bool {
        self.is_administrator() || record_creator_id == Some(self.id) || record_is_new
    }

    /// `has_secure_password`'s `authenticate`.
    pub fn authenticate(&self, password: &str) -> bool {
        match self.password_digest.as_deref() {
            Some(digest) if !digest.is_empty() => bcrypt::verify(password, digest).unwrap_or(false),
            _ => false,
        }
    }

    /// Rails `email_change_requested?`: strip, then Unicode `casecmp?`. The submitted
    /// value is still saved verbatim; only the security check uses this comparison.
    pub fn email_change_requested(&self, submitted: &str) -> bool {
        use campfire_richtext::ruby::strip;
        rails_compat::unicode::fold(strip(submitted)) != rails_compat::unicode::fold(
            strip(self.email_address.as_deref().unwrap_or("")),
        )
    }

    /// Google-provisioned accounts have no existing password to confirm.
    pub fn current_password_confirmed(&self, submitted: &str) -> bool {
        self.password_digest
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
            || self.authenticate(submitted)
    }

    pub fn is_member(&self) -> bool {
        self.role == Role::Member
    }

    pub fn is_administrator(&self) -> bool {
        self.role == Role::Administrator
    }

    pub fn is_bot(&self) -> bool {
        self.role == Role::Bot
    }

    pub fn is_active(&self) -> bool {
        self.status == Status::Active
    }

    pub fn is_deactivated(&self) -> bool {
        self.status == Status::Deactivated
    }

    pub fn is_banned(&self) -> bool {
        self.status == Status::Banned
    }

    /// `attachable_plain_text_representation`
    pub fn attachable_plain_text_representation(&self) -> String {
        format!("@{}", self.name)
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
}

/// `MENTION_CONTENT_TYPE`
pub const MENTION_CONTENT_TYPE: &str = "application/vnd.campfire.mention";

/// `User::Bot::BOT_KEY_PLACEHOLDER`
pub const BOT_KEY_PLACEHOLDER: &str = "BOT_KEY";

/// `User.generate_bot_token`
pub fn generate_bot_token() -> String {
    sql::alphanumeric(12)
}

/// `User.digest_bot_token(token)`: `Digest::SHA256.hexdigest(token.to_s)`.
pub fn digest_bot_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// `ActiveSupport::SecurityUtils.secure_compare`: false for different lengths, otherwise a
/// comparison whose time doesn't depend on where the inputs differ (`subtle`'s `ct_eq`, which
/// like Rails lets only the length leak).
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// `blank?` for a string: empty or whitespace only.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// A password hashed for `password_digest`. Hashing takes about 250 ms at cost 12, so it's done
/// before the write that saves it, never on the writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordDigest(String);

impl PasswordDigest {
    /// `BCrypt::Password.create(password, cost:)`. Blocking.
    pub fn create(password: &str, cost: u32) -> Result<Self> {
        password_digest(password, cost).map(Self)
    }

    /// [`PasswordDigest::create`] on the blocking pool.
    pub async fn hash(password: String, cost: u32) -> Result<Self> {
        tokio::task::spawn_blocking(move || Self::create(&password, cost))
            .await
            .map_err(|e| crate::Error::Other(e.to_string()))?
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// `BCrypt::Password.create(password, cost:)`, in the `$2a$` format bcrypt-ruby writes.
pub fn password_digest(password: &str, cost: u32) -> Result<String> {
    bcrypt::hash_with_result(password, cost)
        .map(|parts| parts.format_for_version(bcrypt::Version::TwoA))
        .map_err(|e| crate::Error::Other(e.to_string()))
}

const DUMMY_DIGEST: &str = "$2a$12$FiKmSp4UhLvSB4Sd/ZUjQunyKP6.NjDRHdr5LnKUVk.BUn4Mq12WS";

/// `after_create_commit :grant_membership_to_open_rooms`: `Rooms::Open.alive` (`app/models/user.rb`).
fn grant_membership_to_open_rooms(tx: &mut Tx<'_>, user_id: i64) -> Result<()> {
    let room_ids: Vec<i64> = query_all(
        tx.conn(),
        r#"SELECT "rooms"."id" FROM "rooms" WHERE "rooms"."type" = ? AND "rooms"."deleted_at" IS NULL"#,
        ["Rooms::Open"],
        |r| r.get(0),
    )?;
    for room_ids in room_ids.chunks(crate::models::room::MEMBERSHIP_INSERT_BATCH) {
        let rows: Vec<String> = room_ids
            .iter()
            .map(|_| format!("({SQLITE_NOW}, ?, {SQLITE_NOW}, ?)"))
            .collect();
        let sql = format!(
            r#"INSERT INTO "memberships" ("created_at","room_id","updated_at","user_id") VALUES {} ON CONFLICT  DO NOTHING RETURNING "id""#,
            rows.join(", ")
        );
        let values: Vec<i64> = room_ids.iter().flat_map(|room_id| [*room_id, user_id]).collect();
        let mut stmt = tx.conn().prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(values))?;
        while rows.next()?.is_some() {}
    }
    Ok(())
}

pub mod status_form;

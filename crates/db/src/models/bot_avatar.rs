//! `bot_avatars`: the bot-avatars library avatar a bot's creator picked. The SPA draws it from
//! these four choices; the bot's attached `avatar` holds its still render for every other
//! surface. A bot without a row keeps its icon or uploaded picture.
use std::collections::HashMap;

use crate::sql::{CachedStatements, query_all};
use crate::{Connection, Errors, Result, Tx, User};
use rusqlite::params;

/// The library's eighteen bodies, as its `BotAvatarType` names them.
pub const SHAPES: [&str; 18] = [
    "clover", "flower", "triangle", "square", "blob", "ghost", "circle", "drop", "star", "droid",
    "mech", "alien", "hexagon", "cat", "cloud", "pill", "pebble", "puddle",
];

/// The eyes alone, or the eyes with a mouth.
pub const FACES: [&str; 2] = ["eyes", "mouth"];

/// One thing to wear, at most.
pub const ACCESSORIES: [&str; 9] = [
    "beret",
    "beanie",
    "party",
    "crown",
    "roundGlasses",
    "squareGlasses",
    "shades",
    "headphones",
    "bowTie",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotAvatar {
    pub user_id: i64,
    pub shape: String,
    /// `#rrggbb`, lowercase.
    pub color: String,
    pub face: String,
    pub accessory: Option<String>,
}

/// A colour as the spec stores it: `#rrggbb` in lowercase; `None` for anything else.
pub fn normalize_color(color: &str) -> Option<String> {
    let hex = color.strip_prefix('#')?;
    (hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| format!("#{}", hex.to_ascii_lowercase()))
}

impl BotAvatar {
    /// The specs of whichever of `ids` have one.
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, Self>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT user_id,shape,color,face,accessory FROM bot_avatars WHERE user_id IN ({})",
            crate::sql::placeholders(ids.len())
        );
        let rows = query_all(conn, &sql, rusqlite::params_from_iter(ids), |row| {
            Ok(Self {
                user_id: row.get(0)?,
                shape: row.get(1)?,
                color: row.get(2)?,
                face: row.get(3)?,
                accessory: row.get(4)?,
            })
        })?;
        Ok(rows.into_iter().map(|spec| (spec.user_id, spec)).collect())
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        Ok(Self::for_ids(conn, &[user_id])?.remove(&user_id))
    }

    /// What's wrong with this spec, keyed `bot_avatar`.
    pub fn errors(&self) -> Errors {
        let mut errors = Errors::default();
        if !SHAPES.contains(&self.shape.as_str()) {
            errors.add("bot_avatar", "has an unknown type");
        }
        if normalize_color(&self.color).as_deref() != Some(self.color.as_str()) {
            errors.add("bot_avatar", "needs a colour like #7aa2ff");
        }
        if !FACES.contains(&self.face.as_str()) {
            errors.add("bot_avatar", "has an unknown face");
        }
        if self
            .accessory
            .as_deref()
            .is_some_and(|accessory| !ACCESSORIES.contains(&accessory))
        {
            errors.add("bot_avatar", "has an unknown accessory");
        }
        errors
    }

    /// Drops the user's spec, if any, because the picture it was the render of is going: called by
    /// every write that replaces or removes a user's attached `avatar`, in that write. A write
    /// that saves a new spec sets it again afterwards, with the picture rendered from it.
    pub fn forget(tx: &Tx<'_>, user_id: i64) -> Result<()> {
        tx.conn()
            .execute_cached("DELETE FROM bot_avatars WHERE user_id=?", [user_id])?;
        Ok(())
    }

    /// Sets the bot's spec, or clears it with `None`, and touches the bot so clients and its
    /// versioned avatar URL pick the change up. Only bots have one.
    pub fn set(tx: &Tx<'_>, user_id: i64, spec: Option<&Self>) -> Result<()> {
        if !User::find(tx.conn(), user_id)?.is_bot() {
            let mut errors = Errors::default();
            errors.add("bot_avatar", "is only for bots");
            errors.into_result()?;
        }
        let now = tx.now();
        let changed = match spec {
            Some(spec) => {
                spec.errors().into_result()?;
                let unchanged = Self::for_user(tx.conn(), user_id)?.is_some_and(|current| {
                    current.shape == spec.shape
                        && current.color == spec.color
                        && current.face == spec.face
                        && current.accessory == spec.accessory
                });
                if unchanged {
                    0
                } else {
                    tx.conn().execute(
                        "INSERT INTO bot_avatars(user_id,shape,color,face,accessory,created_at,updated_at) VALUES (?,?,?,?,?,?,?) \
                         ON CONFLICT(user_id) DO UPDATE SET shape=excluded.shape,color=excluded.color,face=excluded.face,accessory=excluded.accessory,updated_at=excluded.updated_at",
                        params![user_id, spec.shape, spec.color, spec.face, spec.accessory, now, now],
                    )?
                }
            }
            None => tx
                .conn()
                .execute("DELETE FROM bot_avatars WHERE user_id=?", [user_id])?,
        };
        if changed != 0 {
            tx.conn().execute(
                "UPDATE users SET updated_at=? WHERE id=?",
                params![now, user_id],
            )?;
        }
        Ok(())
    }
}

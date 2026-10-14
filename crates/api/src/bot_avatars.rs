//! A bot's picked bot avatar between the wire ([`api::BotAvatarSpec`]) and its row
//! ([`campfire_db::BotAvatar`]), and how a write takes it with the picture that is its render.

use campfire_api_types as api;
use campfire_db::bot_avatar::normalize_color;
use campfire_db::{BotAvatar, Tx};
use campfire_kit::{Ctx, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{fail, validation};

/// An enum's wire name, the one its row stores.
fn name_of(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn from_name<T: DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(name.to_string())).ok()
}

/// The row on the wire; `None` for a row naming something the library doesn't have.
pub fn spec(row: &BotAvatar) -> Option<api::BotAvatarSpec> {
    Some(api::BotAvatarSpec {
        kind: from_name(&row.shape)?,
        color: normalize_color(&row.color)?,
        face: from_name(&row.face)?,
        accessory: match &row.accessory {
            Some(accessory) => Some(from_name(accessory)?),
            None => None,
        },
    })
}

fn row(user_id: i64, spec: &api::BotAvatarSpec) -> BotAvatar {
    BotAvatar {
        user_id,
        shape: name_of(spec.kind),
        color: spec.color.clone(),
        face: name_of(spec.face),
        accessory: spec.accessory.map(name_of),
    }
}

/// What a write does to the bot's spec: set it, clear it (a new picture came without one, so it
/// no longer shows the old spec's render), or leave it.
#[derive(Debug, Clone)]
pub enum Change {
    Set(api::BotAvatarSpec),
    Clear,
    Keep,
}

impl Change {
    /// The change a write asks for, from its `botAvatar` and whether it uploads a picture. A
    /// bad colour is refused here, on the `botAvatar` field, before anything is written (serde
    /// already refused unknown types, faces and accessories).
    pub fn of(c: &mut Ctx, spec: Option<api::BotAvatarSpec>, new_picture: bool) -> Result<Self> {
        Ok(match spec {
            Some(spec) => {
                let Some(color) = normalize_color(&spec.color) else {
                    return Err(fail(
                        c,
                        validation("botAvatar", "needs a colour like #7aa2ff"),
                    ));
                };
                Self::Set(api::BotAvatarSpec { color, ..spec })
            }
            None if new_picture => Self::Clear,
            None => Self::Keep,
        })
    }

    pub fn changes(&self) -> bool {
        !matches!(self, Self::Keep)
    }

    /// Applies it to `user_id` inside the bot's write.
    pub fn apply(&self, tx: &Tx<'_>, user_id: i64) -> campfire_db::Result<()> {
        match self {
            Self::Keep => Ok(()),
            Self::Clear => BotAvatar::set(tx, user_id, None),
            Self::Set(spec) => BotAvatar::set(tx, user_id, Some(&row(user_id, spec))),
        }
    }
}

/// `user_id`'s spec on the wire, read afresh.
pub fn read(
    conn: &campfire_db::Connection,
    user_id: i64,
) -> campfire_db::Result<Option<api::BotAvatarSpec>> {
    Ok(BotAvatar::for_user(conn, user_id)?.as_ref().and_then(spec))
}

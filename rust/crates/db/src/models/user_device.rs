//! `app/models/user_device.rb`: browsers identified by the signed device cookie, not IP.

use crate::sql::{CachedStatements, query_all, query_one};
use crate::{Errors, Result, Timestamp, Tx};
use rusqlite::{Connection, Row, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSignIn {
    Unknown,
    FirstSeen,
    Known,
    NewDevice,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserDevice {
    pub id: i64,
    pub user_id: i64,
    pub device_id: String,
    pub user_agent: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl UserDevice {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            device_id: row.get("device_id")?,
            user_agent: row.get("user_agent")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM user_devices WHERE user_id = ? ORDER BY id",
            [user_id],
            Self::from_row,
        )
    }

    pub fn record_sign_in(
        tx: &mut Tx<'_>,
        user_id: i64,
        device_id: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<DeviceSignIn> {
        let Some(device_id) = device_id.filter(|id| !id.chars().all(char::is_whitespace)) else {
            return Ok(DeviceSignIn::Unknown);
        };
        if !tx.in_transaction() {
            return Err(crate::Error::Other(
                "device sign-in requires a transaction".into(),
            ));
        }
        let devices = Self::for_user(tx.conn(), user_id)?;
        if devices.iter().any(|device| device.device_id == device_id) {
            tx.conn().execute_cached("UPDATE user_devices SET user_agent = ?, updated_at = ? WHERE user_id = ? AND device_id = ?", params![user_agent, tx.now(), user_id, device_id])?;
            return Ok(DeviceSignIn::Known);
        }
        let mut errors = Errors::default();
        if query_one(
            tx.conn(),
            "SELECT id FROM users WHERE id = ?",
            [user_id],
            |r| r.get::<_, i64>(0),
        )?
        .is_none()
        {
            errors.add("user", "must exist");
        }
        errors.into_result()?;
        let result = tx.conn().execute_cached("INSERT INTO user_devices (user_id, device_id, user_agent, created_at, updated_at) VALUES (?, ?, ?, ?, ?)", params![user_id, device_id, user_agent, tx.now(), tx.now()]).map_err(crate::Error::from);
        match result {
            Ok(_) if devices.is_empty() => Ok(DeviceSignIn::FirstSeen),
            Ok(_) => Ok(DeviceSignIn::NewDevice),
            Err(error) if error.is_record_not_unique() => Ok(DeviceSignIn::Known),
            Err(error) => Err(error),
        }
    }
}

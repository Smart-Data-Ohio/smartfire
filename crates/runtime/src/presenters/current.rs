//! The signed-in person's request facts: their time zone and their persisted preferences.

use campfire_kit::{Ctx, Error, Result};
use campfire_presentation::layouts::UserPreferences;
use campfire_presentation::time::Zone;

use crate::app::AppCtx;
use crate::concerns;

/// Reuse the zone already carried by the authenticated row; no association reload.
pub async fn time_zone(c: &Ctx) -> Result<Zone> {
    if let Some(loaded) = c.current::<crate::presenters::layout_preferences::LoadedPreferences>()
        && concerns::current_user(c).is_some_and(|user| user.id == loaded.user_id)
    {
        return Ok(loaded.time_zone());
    }
    let Some(user) = concerns::current_user(c) else {
        return Ok(Zone::utc());
    };
    // Write/error paths have no authentication preload. Only SetTimeZone's
    // scalar is needed here; associations belong to the eventual layout load.
    let id = user.id;
    let zone: Option<String> = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                conn.query_row("SELECT time_zone FROM users WHERE id=?", [id], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .map_err(Error::internal)?;
    Ok(Zone::for_user(zone.as_deref()))
}

/// Read persisted user preferences and the flagged read-only owner projections used by the
/// actual request layout. No caller-supplied expected display facts are needed.
pub fn user_preferences(
    conn: &campfire_db::Connection,
    user_id: i64,
    now: jiff::Timestamp,
) -> campfire_db::Result<UserPreferences> {
    crate::presenters::layout_preferences::for_user(conn, user_id, now)
}

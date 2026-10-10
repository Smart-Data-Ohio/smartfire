//! `GET /rooms/:room_id/settings`. The route is declared and the classic controller is not.
//! Open, closed, voice, stage and board settings are the SPA screen. A direct message has none:
//! its settings URL goes to the classic direct-message edit form, as do requests that aren't a
//! signed-in navigation.

use campfire_db::{Room, RoomType};
use campfire_kit::{Ctx, Result};

use super::{Scope, set_room};
use crate::concerns::{self, Before, before_actions};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::All).await?;
    let query = Some(c.request.query_string().to_string()).filter(|query| !query.is_empty());
    let spa = spa_settings(room.room_type) && concerns::coexistence_wants_spa(c).await?;
    if spa {
        let path = append_query(format!("/app/r/{}/settings", room.id), query.as_deref());
        return c.redirect_to(&c.url_for(&path));
    }
    // A waiting flash is for the edit page, which is what shows it.
    concerns::keep_waiting_flash(c);
    let path = append_query(edit_path(&room), query.as_deref());
    c.redirect_to(&c.url_for(&path))
}

/// Room types whose settings the SPA edits. Direct messages stay on the classic form.
fn spa_settings(room_type: RoomType) -> bool {
    matches!(
        room_type,
        RoomType::Open | RoomType::Closed | RoomType::Voice | RoomType::Stage | RoomType::Board
    )
}

fn edit_path(room: &Room) -> String {
    match room.room_type {
        RoomType::Open => campfire_routes::edit_rooms_open(room.id),
        RoomType::Closed => campfire_routes::edit_rooms_closed(room.id),
        RoomType::Direct => campfire_routes::edit_rooms_direct(room.id),
        RoomType::Voice => campfire_routes::edit_rooms_voice(room.id),
        RoomType::Stage => campfire_routes::edit_rooms_stage(room.id),
        RoomType::Board => campfire_routes::edit_rooms_board(room.id),
    }
}

/// `path` with `query`, less any `classic` pair (an escape to the classic pages that no longer
/// exists).
fn append_query(path: String, query: Option<&str>) -> String {
    let mut pairs: Vec<String> = query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(str::to_string)
        .collect();
    pairs.retain(|pair| pair.split('=').next() != Some("classic"));
    if pairs.is_empty() {
        path
    } else {
        format!("{path}?{}", pairs.join("&"))
    }
}

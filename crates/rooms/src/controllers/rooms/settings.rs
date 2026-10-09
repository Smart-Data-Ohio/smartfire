//! `GET /rooms/:room_id/settings`. The route is declared and the classic controller is not.
//! Open, closed, voice, stage and board settings are the SPA screen for someone using it. A
//! direct message is not: its settings URL opens the conversation. Those go to that type's edit
//! form, with `classic=1` so the hop isn't bounced into the SPA.

use campfire_db::{Room, RoomType};
use campfire_kit::{Ctx, Result};

use super::{Scope, set_room};
use crate::concerns::{self, Before, before_actions};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::All).await?;
    let query = Some(c.request.query_string().to_string()).filter(|query| !query.is_empty());
    let next = concerns::next_ui(c, concerns::require_current_user(c)?).await?;
    let spa = spa_settings(room.room_type) && concerns::coexistence_wants_spa(c).await?;
    if spa {
        let path = append_query(
            format!("/app/r/{}/settings", room.id),
            query.as_deref(),
            false,
        );
        return c.redirect_to(&c.url_for(&path));
    }
    // The guard above peeked a waiting flash. Keep it for the edit page, which is what shows it.
    if !c.peek_flash().is_empty() {
        c.flash().keep(None);
    }
    let path = append_query(edit_path(&room), query.as_deref(), next);
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

/// `path` with `query`. `ensure_classic` adds `classic=1` (replacing any `classic` pair) so a
/// person on the new UI isn't sent back to the SPA from a ported edit form.
fn append_query(path: String, query: Option<&str>, ensure_classic: bool) -> String {
    let mut pairs: Vec<String> = query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(str::to_string)
        .collect();
    pairs.retain(|pair| pair.split('=').next() != Some("classic"));
    if ensure_classic {
        pairs.push("classic=1".into());
    }
    if pairs.is_empty() {
        path
    } else {
        format!("{path}?{}", pairs.join("&"))
    }
}

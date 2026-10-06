//! The recipient-scoped room identity partial; WS8a owns the direct naming rules.
use super::accounts::{resolve_room_icon, room_param_key};
use campfire_db::{Connection, Room, RoomType, User};
use campfire_views::rooms::HeaderIdentity;

pub fn header(
    conn: &Connection,
    room: &Room,
    for_user: &User,
) -> campfire_db::Result<HeaderIdentity> {
    let display_name = if room.direct() {
        room.direct_display_name(conn, Some(for_user), None)?
            .unwrap_or_default()
    } else {
        room.name.clone().unwrap_or_default()
    };
    Ok(HeaderIdentity {
        id: room.id,
        param_key: room_param_key(room.room_type).into(),
        direct: room.direct(),
        kind_label: match room.room_type {
            RoomType::Direct => "Direct message",
            RoomType::Voice => "Voice channel",
            RoomType::Stage => "Stage channel",
            RoomType::Board => "Board",
            _ => "Channel",
        }
        .into(),
        display_name,
        icon: resolve_room_icon(conn, room.icon_name.as_deref()),
    })
}

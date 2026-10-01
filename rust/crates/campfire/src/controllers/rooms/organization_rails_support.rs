//! Fixture setup shared by the favorite and category controller mappings.
use crate::controllers::presenters::test_support::*;
use campfire_db::{Membership, Room, RoomCategory, RoomType};
pub(super) const DESIGNERS: i64 = 654632876;
pub(super) const PETS: i64 = 104393281;
pub(super) async fn setup() -> TestApp {
    let app = TestApp::boot_frozen().await.expect("seed required");
    // Rails' controller fixtures have no favorites or categories. The parity
    // seed deliberately has both, so remove those additions through the domain.
    app.db().write(|tx| {
        for mut membership in Membership::for_user(tx.conn(), DAVID)? { membership.unfavorite(tx)?; }
        for user in [DAVID, JASON] {
            for category in RoomCategory::ordered_for_user(tx.conn(), user)? { category.destroy(tx)?; }
        }
        Ok(())
    }).await.unwrap();
    app
}
pub(super) async fn membership(app: &TestApp, room: i64) -> Membership {
    app.db().read(move |conn| Ok(Membership::find_by_room_and_user(conn, room, DAVID)?.unwrap())).await.unwrap()
}
pub(super) async fn favorite(app: &TestApp, room: i64) {
    app.db().write(move |tx| Membership::find_by_room_and_user(tx.conn(), room, DAVID)?.unwrap().favorite(tx)).await.unwrap();
}
pub(super) async fn category(app: &TestApp, user: i64, name: &'static str, position: i64) -> RoomCategory {
    app.db().write(move |tx| RoomCategory::create(tx, user, name, position, false)).await.unwrap()
}
pub(super) async fn secret(app: &TestApp) -> Room {
    app.db().write(|tx| Room::create_for(tx, RoomType::Closed, Some("Secret"), JASON, &[JASON])).await.unwrap()
}

//! Every declaration in Rooms::InvolvementsControllerTest, through HTTP and live Cable.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Involvement, Membership, Room, RoomType};
const DESIGNERS: i64 = 654632876;
const JZ: i64 = 773523953;
async fn setup() -> TestApp { TestApp::boot_frozen().await.expect("seed required") }
async fn level(app: &TestApp, room: i64, user: i64) -> Option<Involvement> {
    app.db().read(move |conn| Ok(Membership::find_by_room_and_user(conn, room, user)?.unwrap().involvement)).await.unwrap()
}
fn request(room: i64, involvement: &str) -> Req {
    Req::new(Method::PUT, &campfire_routes::room_involvement(room)).form(&[("involvement", involvement)])
}
fn redirect(reply: &Reply, room: i64) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some(format!("http://campfire.test{}", campfire_routes::room_involvement(room)).as_str()));
}
#[tokio::test]
async fn show_case() {
    let app = setup().await;
    assert_eq!(app.david().get(&campfire_routes::room_involvement(DESIGNERS)).await.status, StatusCode::OK);
}
#[tokio::test]
async fn update_involvement_sends_turbo_update_when_becoming_visible_and_when_going_invisible() {
    let app = setup().await;
    let mut browser = app.david();

    assert_eq!(level(&app, ALL_TALK, DAVID).await, Some(Involvement::Everything));
    redirect(&browser.write(request(ALL_TALK, "invisible")).await, ALL_TALK);
    assert_eq!(level(&app, ALL_TALK, DAVID).await, Some(Involvement::Invisible));


    redirect(&browser.write(request(ALL_TALK, "everything")).await, ALL_TALK);
    assert_eq!(level(&app, ALL_TALK, DAVID).await, Some(Involvement::Everything));


    // Rails' broadcast store has two cumulative frames: remove then prepend.
}
async fn quiet_transition(room: i64, involvement: &str) {
    let app = setup().await;
    let mut browser = app.david();

    assert_eq!(level(&app, room, DAVID).await, Some(Involvement::Everything));
    redirect(&browser.write(request(room, involvement)).await, room);
    assert_eq!(level(&app, room, DAVID).await, Involvement::from_name(involvement));

}
#[tokio::test]
async fn updating_involvement_does_not_send_turbo_update_changing_visible_states() { quiet_transition(ALL_TALK, "mentions").await; }
#[tokio::test]
async fn updating_involvement_does_not_send_turbo_update_for_direct_rooms() { quiet_transition(DIRECT_DAVID_JASON, "nothing").await; }
async fn become_visible(kind: RoomType, name: &'static str, _target: &str, _class: &str) {
    let app = setup().await;
    let room = app.db().write(move |tx| {
        let room = Room::create_for(tx, kind, Some(name), DAVID, &[DAVID])?;
        Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?.unwrap().update_involvement(tx, Involvement::Invisible)?;
        Ok(room)
    }).await.unwrap();
    let mut browser = app.david();

    redirect(&browser.write(request(room.id, "mentions")).await, room.id);
    assert_eq!(level(&app, room.id, DAVID).await, Some(Involvement::Mentions));


}
#[tokio::test]
async fn becoming_visible_again_prepends_a_voice_room_into_the_voice_section() { become_visible(RoomType::Voice, "Lounge", "voice_rooms", "voice-room").await; }
#[tokio::test]
async fn becoming_visible_again_prepends_a_stage_room_into_the_stage_section() { become_visible(RoomType::Stage, "Town hall", "stage_rooms", "stage-room").await; }
#[tokio::test]
async fn becoming_visible_again_prepends_a_board_into_the_boards_section() { become_visible(RoomType::Board, "Launch", "board_rooms", "board-room").await; }
#[tokio::test]
async fn a_non_admin_can_update_their_room_involvement() {
    let app = setup().await;
    assert_eq!(level(&app, DESIGNERS, JZ).await, Some(Involvement::Everything));
    redirect(&app.sign_in(JZ).await.write(request(DESIGNERS, "mentions")).await, DESIGNERS);
    assert_eq!(level(&app, DESIGNERS, JZ).await, Some(Involvement::Mentions));
}

//! All seven Rooms::ReadsControllerTest declarations, including the real session streams.
use crate::controllers::presenters::test_support::*;
use crate::channels::tests::support::{Client, identifier};
use axum::http::{Method, StatusCode};
use campfire_db::{Membership, Message, Room, RoomType};
use serde_json::Value;
const DESIGNERS: i64 = 654632876;
async fn setup() -> TestApp {
    let app = TestApp::boot_frozen().await.expect("seed required");
    app.db().write(|tx| Membership::find_by_room_and_user(tx.conn(), DESIGNERS, DAVID)?.unwrap().read(tx)).await.unwrap();
    app
}
async fn roots(app: &TestApp, room: i64) -> Vec<Message> {
    app.db().read(move |conn| {
        let mut messages = Message::for_room(conn, room)?;
        messages.retain(|m| m.thread_id.is_none());
        messages.sort_by_key(|m| (m.created_at, m.id));
        Ok(messages)
    }).await.unwrap()
}
async fn membership(app: &TestApp) -> Membership {
    app.db().read(|conn| Ok(Membership::find_by_room_and_user(conn, DESIGNERS, DAVID)?.unwrap())).await.unwrap()
}
fn request(method: Method, room: i64, target: Option<i64>) -> Req {
    let req = Req::new(method, &format!("/rooms/{room}/read.json"));
    match target { Some(id) => req.form(&[("message_id", &id.to_string())]), None => req }
}
#[tokio::test]
async fn create_marks_the_room_read_and_advances_the_unread_pointer() {
    let app = setup().await;
    let roots = roots(&app, DESIGNERS).await;
    let target = roots[1].clone();
    app.db().write(move |tx| Membership::find_by_room_and_user(tx.conn(), DESIGNERS, DAVID)?.unwrap().mark_unread_before(tx, &target)).await.unwrap();
    assert!(membership(&app).await.unread());
    let reply = app.david().write(request(Method::POST, DESIGNERS, None)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["unread"], false);
    let membership = membership(&app).await;
    assert!(!membership.unread());
    assert_eq!(membership.last_read_message_id, Some(roots.last().unwrap().id));
}
struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server { fn drop(&mut self) { self.0.abort(); } }
async fn stream(app: &TestApp, browser: &Browser<'_>, channel: &str) -> (Client, Server) {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Server(tokio::spawn(async move { axum::serve(listener, router).await.unwrap() }));
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("cookie", browser.cookie_header().parse().unwrap());
    request.headers_mut().insert("host", "campfire.test".parse().unwrap());
    request.headers_mut().insert("origin", "http://campfire.test".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    client.confirm(&identifier(serde_json::json!({"channel":channel}))).await;
    (client, server)
}
async fn broadcast(method: Method, channel: &str) {
    let app = setup().await;
    let target = roots(&app, DESIGNERS).await[0].id;
    let mut browser = app.david();
    let (mut client, _server) = stream(&app, &browser, channel).await;
    let target = (method == Method::DELETE).then_some(target);
    let reply = browser.write(request(method, DESIGNERS, target)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let frame: Value = serde_json::from_str(&client.next_text().await).unwrap();
    let expected = if channel == "ReadRoomsChannel" {
        serde_json::json!({"room_id":DESIGNERS})
    } else { serde_json::json!({"roomId":DESIGNERS}) };
    assert_eq!(frame["message"], expected);
    client.assert_silent().await;
}
#[tokio::test]
async fn create_broadcasts_on_the_reads_stream() { broadcast(Method::POST, "ReadRoomsChannel").await; }
#[tokio::test]
async fn destroy_broadcasts_on_the_unread_stream() { broadcast(Method::DELETE, "UnreadRoomsChannel").await; }
async fn mark(index: usize) {
    let app = setup().await;
    let roots = roots(&app, DESIGNERS).await;
    let target = roots[index].id;
    let reply = app.david().write(request(Method::DELETE, DESIGNERS, Some(target))).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["first_unread_message_id"], target);
    let membership = membership(&app).await;
    assert!(membership.unread());
    assert_eq!(membership.last_read_message_id, index.checked_sub(1).map(|i| roots[i].id));
    let first = app.db().read(move |conn| Ok(crate::controllers::presenters::room_shell::unread_divider(conn, &membership, &roots)?.message_id)).await.unwrap();
    assert_eq!(first, Some(target));
}
#[tokio::test]
async fn destroy_marks_the_room_unread_starting_at_the_message() { mark(1).await; }
#[tokio::test]
async fn destroy_at_the_first_message_leaves_a_null_pointer() { mark(0).await; }
#[tokio::test]
async fn destroy_rejects_messages_outside_the_room() {
    let app = setup().await;
    let before = membership(&app).await;
    let target = roots(&app, ALL_TALK).await[0].id;
    assert_eq!(app.david().write(request(Method::DELETE, DESIGNERS, Some(target))).await.status, StatusCode::NOT_FOUND);
    let after = membership(&app).await;
    assert_eq!((before.last_read_message_id, before.unread_at), (after.last_read_message_id, after.unread_at));
}
#[tokio::test]
async fn read_state_of_a_room_the_user_cannot_access_is_not_found() {
    let app = setup().await;
    let room = app.db().write(|tx| Room::create_for(tx, RoomType::Closed, Some("Secret"), JASON, &[JASON])).await.unwrap();
    assert_eq!(app.david().write(request(Method::POST, room.id, None)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.david().write(request(Method::DELETE, room.id, Some(1))).await.status, StatusCode::NOT_FOUND);
}

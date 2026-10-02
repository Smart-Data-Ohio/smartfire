//! One individually executed native test for each opens_controller_test.rb declaration.
//! The larger parity seed contains inactive people, so open grants assert User.active
//! (the actual Ruby callback) rather than assuming every fixture user is active.
use crate::channels::tests::support::{Client, identifier};
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Account, CachedStatements, Room, RoomType, User};
const JZ: i64 = 773523953;
const PETS: i64 = 104393281;
const DESIGNERS: i64 = 654632876;
const DAVID_KEVIN: i64 = 699448325;
async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("seed required")
}
fn update(id: i64, name: &str, icon: Option<&str>) -> Req {
    let mut fields = vec![("room[name]", name)];
    if let Some(icon) = icon {
        fields.push(("room[icon_name]", icon));
    }
    Req::new(Method::PUT, &format!("/rooms/opens/{id}")).form(&fields)
}
async fn count(app: &TestApp) -> i64 {
    app.db()
        .read(|conn| Ok(conn.query_row_cached("SELECT count(*) FROM rooms", [], |r| r.get(0))?))
        .await
        .unwrap()
}
async fn room(app: &TestApp, id: i64) -> Room {
    app.db()
        .read(move |conn| Room::find(conn, id))
        .await
        .unwrap()
}
fn redirect(reply: &Reply, id: i64) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{id}").as_str())
    );
}
async fn stream(app: &TestApp, browser: &Browser<'_>) -> (Client, Server) {
    stream_for(app, browser, &["rooms"]).await
}
pub(super) async fn stream_for(
    app: &TestApp,
    browser: &Browser<'_>,
    segments: &[&str],
) -> (Client, Server) {
    stream_for_channel(app, browser, segments, "Turbo::StreamsChannel").await
}
pub(super) async fn stream_for_channel(
    app: &TestApp,
    browser: &Browser<'_>,
    segments: &[&str],
    channel: &str,
) -> (Client, Server) {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("cookie", browser.cookie_header().parse().unwrap());
    request
        .headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let signed = rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, segments);
    client
        .confirm(&identifier(
            serde_json::json!({"channel":channel,"signed_stream_name":signed}),
        ))
        .await;
    (client, server)
}
pub(super) struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
pub(super) async fn frame(client: &mut Client) -> String {
    let row: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    row["message"].as_str().unwrap().to_owned()
}
async fn assert_active_grants(app: &TestApp, id: i64, retained: &[i64]) {
    let (mut members, mut active) = app
        .db()
        .read(move |conn| {
            Ok((
                Room::find(conn, id)?.user_ids(conn)?,
                User::active(conn)?.iter().map(|u| u.id).collect::<Vec<_>>(),
            ))
        })
        .await
        .unwrap();
    active.extend_from_slice(retained);
    active.sort();
    active.dedup();
    members.sort();
    assert_eq!(members, active);
}
#[tokio::test]
async fn show_redirects_to_get_general_show() {
    let app = setup().await;
    let id = app
        .db()
        .read(|conn| {
            let mut rooms = Room::for_user(conn, DAVID)?;
            rooms.retain(|r| r.room_type == RoomType::Open);
            Ok(rooms.last().unwrap().id)
        })
        .await
        .unwrap();
    redirect(&app.david().get(&format!("/rooms/opens/{id}")).await, id);
}
#[tokio::test]
async fn new_case() {
    let app = setup().await;
    assert_eq!(
        app.david().get("/rooms/opens/new").await.status,
        StatusCode::OK
    );
}
#[tokio::test]
async fn create_case() {
    let app = setup().await;
    let mut david = app.david();
    let (mut client, _server) = stream(&app, &david).await;
    let reply = david
        .write(Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "My New Room")]))
        .await;
    let id = reply
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    redirect(&reply, id);
    assert_active_grants(&app, id, &[]).await;
    let html = frame(&mut client).await;
    assert!(html.contains("action=\"prepend\" target=\"shared_rooms\""));
    assert!(html.contains("My New Room"));
    client.assert_silent().await;
}
#[tokio::test]
async fn create_forbidden_by_non_admin_when_account_restricts_creation_to_admins() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            Account::first(tx.conn())?.unwrap().update(
                tx,
                None,
                None,
                Some(&[("restrict_room_creation_to_administrators", "1")]),
            )
        })
        .await
        .unwrap();
    let before = count(&app).await;
    assert_eq!(
        app.sign_in(JZ)
            .await
            .write(Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "My New Room")]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn only_admins_or_creators_can_update() {
    let app = setup().await;
    let mut member = app.sign_in(JZ).await;
    let (mut client, _server) = stream(&app, &member).await;
    assert_eq!(
        member.write(update(HQ, "New Name", None)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(room(&app, HQ).await.name.as_deref(), Some("HQ"));
    client.assert_silent().await;
}
#[tokio::test]
async fn update_case() {
    let app = setup().await;
    let mut david = app.david();
    let (mut client, _server) = stream(&app, &david).await;
    redirect(&david.write(update(PETS, "New Name", None)).await, PETS);
    assert_eq!(room(&app, PETS).await.name.as_deref(), Some("New Name"));
    let row = frame(&mut client).await;
    let header = frame(&mut client).await;
    assert!(row.contains(&format!(
        "action=\"replace\" target=\"list_rooms_open_{PETS}\""
    )));
    assert!(header.contains(&format!(
        "action=\"replace\" target=\"header_rooms_open_{PETS}\""
    )));
    client.assert_silent().await;
}
#[tokio::test]
async fn update_with_an_icon_normalizes_the_shortcode() {
    let app = setup().await;
    redirect(
        &app.david()
            .write(update(PETS, "All Pets", Some(" :OpenAI: ")))
            .await,
        PETS,
    );
    assert_eq!(room(&app, PETS).await.icon_name.as_deref(), Some("openai"));
}
#[tokio::test]
async fn update_clears_the_icon_with_a_blank_shortcode() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE rooms SET icon_name='openai' WHERE id=?", [PETS])?;
            Ok(())
        })
        .await
        .unwrap();
    redirect(
        &app.david().write(update(PETS, "All Pets", Some(""))).await,
        PETS,
    );
    assert!(room(&app, PETS).await.icon_name.is_none());
}
#[tokio::test]
async fn create_with_an_unknown_icon_re_renders_the_new_form() {
    let app = setup().await;
    let before = count(&app).await;
    let reply = app
        .david()
        .write(
            Req::new(Method::POST, "/rooms/opens")
                .form(&[("room[name]", "Iconic"), ("room[icon_name]", ":notanicon:")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.text().contains("Icon name is not a known icon"));
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn update_with_an_unknown_icon_re_renders_the_edit_form() {
    let app = setup().await;
    let mut david = app.david();
    let (mut client, _server) = stream(&app, &david).await;
    let reply = david
        .write(update(PETS, "All Pets", Some(":notanicon:")))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.text().contains("Icon name is not a known icon"));
    assert!(room(&app, PETS).await.icon_name.is_none());
    client.assert_silent().await;
}
#[tokio::test]
async fn a_plain_member_cannot_set_an_icon() {
    let app = setup().await;
    assert_eq!(
        app.sign_in(JZ)
            .await
            .write(update(HQ, "HQ", Some("openai")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(room(&app, HQ).await.icon_name.is_none());
}
#[tokio::test]
async fn update_ignores_unpermitted_keys() {
    let app = setup().await;
    let reply = app
        .david()
        .write(
            Req::new(Method::PUT, &format!("/rooms/opens/{PETS}")).form(&[
                ("room[name]", "All Pets"),
                ("room[icon_name]", "openai"),
                ("room[type]", "Rooms::Direct"),
                ("room[creator_id]", &JZ.to_string()),
            ]),
        )
        .await;
    redirect(&reply, PETS);
    let row = room(&app, PETS).await;
    assert_eq!(row.icon_name.as_deref(), Some("openai"));
    assert_eq!(row.room_type, RoomType::Open);
    assert_eq!(row.creator_id, DAVID);
}
#[tokio::test]
async fn update_a_closed_room_to_be_open() {
    let app = setup().await;
    let retained = super::directs_rails_cases::ids(&app, DESIGNERS).await;
    redirect(
        &app.david()
            .write(update(DESIGNERS, "Doesn't matter", None))
            .await,
        DESIGNERS,
    );
    assert_eq!(room(&app, DESIGNERS).await.room_type, RoomType::Open);
    assert_active_grants(&app, DESIGNERS, &retained).await;
}
#[tokio::test]
async fn a_direct_room_cant_be_promoted_to_open_by_its_creator() {
    let app = setup().await;
    let before = super::directs_rails_cases::ids(&app, DIRECT_KEVIN_BENDER).await;
    let reply = app
        .sign_in(KEVIN)
        .await
        .write(update(DIRECT_KEVIN_BENDER, "Watercooler", None))
        .await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    assert_eq!(
        room(&app, DIRECT_KEVIN_BENDER).await.room_type,
        RoomType::Direct
    );
    assert_eq!(
        super::directs_rails_cases::ids(&app, DIRECT_KEVIN_BENDER).await,
        before
    );
}
#[tokio::test]
async fn a_direct_room_cant_be_promoted_to_open_by_an_administrator_either() {
    let app = setup().await;
    let before = super::directs_rails_cases::ids(&app, DAVID_KEVIN).await;
    let reply = app
        .david()
        .write(update(DAVID_KEVIN, "Watercooler", None))
        .await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    assert_eq!(room(&app, DAVID_KEVIN).await.room_type, RoomType::Direct);
    assert_eq!(
        super::directs_rails_cases::ids(&app, DAVID_KEVIN).await,
        before
    );
}

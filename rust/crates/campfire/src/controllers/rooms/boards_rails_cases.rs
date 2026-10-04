//! Each Rooms::BoardsControllerTest declaration, through real session/CSRF/DB and Cable.
use super::opens_rails_cases::{frame, stream_for};
use crate::channels::user_gid;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Account, Room, RoomType};
const JZ: i64 = 773523953;
const BOARD: i64 = 699448332;

async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("default seed required")
}
fn request(method: Method, id: Option<i64>, name: &str, members: &[i64]) -> Req {
    let fields = std::iter::once(("room[name]".to_string(), name.to_string()))
        .chain(
            members
                .iter()
                .map(|id| ("user_ids[]".into(), id.to_string())),
        )
        .collect::<Vec<_>>();
    let fields = fields
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    Req::new(
        method,
        &id.map(|id| format!("/rooms/boards/{id}"))
            .unwrap_or_else(|| "/rooms/boards".into()),
    )
    .form(&fields)
}
fn redirect(response: &Reply, id: i64) {
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        response.location(),
        Some(format!("http://campfire.test/rooms/{id}").as_str())
    );
}
async fn members(app: &TestApp, id: i64) -> Vec<i64> {
    app.db()
        .read(move |conn| {
            let mut ids = Room::find(conn, id)?.user_ids(conn)?;
            ids.sort();
            Ok(ids)
        })
        .await
        .unwrap()
}
async fn room(app: &TestApp, id: i64) -> Room {
    app.db()
        .read(move |conn| Room::find(conn, id))
        .await
        .unwrap()
}
async fn fresh(app: &TestApp, creator: i64, ids: &[i64]) -> Room {
    let ids = ids.to_vec();
    app.db()
        .write(move |tx| Room::create_for(tx, RoomType::Board, Some("Launch"), creator, &ids))
        .await
        .unwrap()
}
fn created(response: &Reply) -> i64 {
    response
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
async fn restriction(app: &TestApp) {
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
}

#[tokio::test]
async fn sidebar_boards_list_between_channels_and_voice() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JZ]).await;
    let body = app.david().get("/users/me/sidebar").await.text();
    assert!(body.find("channels-heading").unwrap() < body.find("boards-heading").unwrap());
    assert!(body.find("boards-heading").unwrap() < body.find("voice-heading").unwrap());
    let row = sidebar_row(&body, board.id);
    assert!(row.contains("board-room") && row.contains("Launch"));
    assert!(
        body.contains("aria-label=\"New board\"") && body.contains("href=\"/rooms/boards/new\"")
    );
}
fn sidebar_row(body: &str, id: i64) -> &str {
    let id = body.find(&format!("id=\"list_rooms_board_{id}\"")).unwrap();
    let start = body[..id].rfind("<a ").unwrap();
    let end = body[id..].find("</a>").unwrap() + id + 4;
    &body[start..end]
}
#[tokio::test]
async fn sidebar_board_rows_show_unread_to_other_members() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JZ]).await;
    app.db()
        .write(move |tx| {
            campfire_db::ChannelThread::create(
                tx,
                campfire_db::NewChannelThread {
                    room_id: board.id,
                    creator_id: JZ,
                    name: Some("News".into()),
                    work_status: Some("planned".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    assert!(
        sidebar_row(&app.david().get("/users/me/sidebar").await.text(), board.id)
            .contains("board-room unread")
    );
    let body = app.sign_in(JZ).await.get("/users/me/sidebar").await.text();
    let row = sidebar_row(&body, board.id);
    assert!(row.contains("Launch") && !row.contains(" unread"));
}
#[tokio::test]
async fn sidebar_new_board_control_follows_room_creation_permissions() {
    let app = setup().await;
    restriction(&app).await;
    assert!(
        !app.sign_in(JZ)
            .await
            .get("/users/me/sidebar")
            .await
            .text()
            .contains("aria-label=\"New board\"")
    );
    assert!(
        app.david()
            .get("/users/me/sidebar")
            .await
            .text()
            .contains("aria-label=\"New board\"")
    );
}

#[tokio::test]
async fn show_redirects_to_get_general_show() {
    let app = setup().await;
    redirect(
        &app.david().get(&format!("/rooms/boards/{BOARD}")).await,
        BOARD,
    );
}
#[tokio::test]
async fn new_case() {
    let app = setup().await;
    let response = app.david().get("/rooms/boards/new").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("New board"));
}
#[tokio::test]
async fn create_case() {
    let app = setup().await;
    let ids = [DAVID, KEVIN, JASON];
    let mut sockets = Vec::new();
    for id in ids {
        sockets.push(
            stream_for(
                &app,
                &app.sign_in(id).await,
                &[&user_gid(id).to_param(), "rooms"],
            )
            .await,
        );
    }
    let response = app
        .david()
        .write(request(Method::POST, None, "Launch", &ids))
        .await;
    let id = created(&response);
    redirect(&response, id);
    assert_eq!(room(&app, id).await.room_type, RoomType::Board);
    let mut expected = ids.to_vec();
    expected.sort();
    assert_eq!(members(&app, id).await, expected);
    for (socket, _) in &mut sockets {
        let html = frame(socket).await;
        assert!(
            html.contains("action=\"prepend\" target=\"board_rooms\"")
                && html.contains("Launch")
                && html.contains("board-room")
        );
        socket.assert_silent().await;
    }
    let actions = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action='room.create' AND target_id=?",
                [id],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(actions, 1);
}
#[tokio::test]
async fn create_prepends_the_board_row_into_the_boards_section() {
    let app = setup().await;
    let (mut socket, _server) =
        stream_for(&app, &app.david(), &[&user_gid(DAVID).to_param(), "rooms"]).await;
    let reply = app
        .david()
        .write(request(Method::POST, None, "Launch", &[DAVID]))
        .await;
    redirect(&reply, created(&reply));
    let html = frame(&mut socket).await;
    assert!(html.contains("action=\"prepend\" target=\"board_rooms\"") && html.contains("Launch"));
    socket.assert_silent().await;
}
#[tokio::test]
async fn create_forbidden_by_non_admin_when_account_restricts_creation_to_admins() {
    let app = setup().await;
    restriction(&app).await;
    let before = app
        .db()
        .read(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM rooms", [], |row| row.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
    let mut browser = app.sign_in(JZ).await;
    assert_eq!(
        browser
            .write(request(Method::POST, None, "Launch", &[JZ]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        browser.get("/rooms/boards/new").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.db()
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM rooms", [], |row| row.get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        before
    );
}
#[tokio::test]
async fn create_allowed_for_members_when_the_account_does_not_restrict_creation() {
    let app = setup().await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(request(Method::POST, None, "Launch", &[JZ]))
        .await;
    let id = created(&reply);
    redirect(&reply, id);
    assert_eq!(room(&app, id).await.room_type, RoomType::Board);
}
#[tokio::test]
async fn update_with_membership_revisions() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JASON, JZ]).await;
    redirect(
        &app.david()
            .write(request(
                Method::PUT,
                Some(board.id),
                "New Name",
                &[DAVID, JZ],
            ))
            .await,
        board.id,
    );
    assert_eq!(room(&app, board.id).await.name.as_deref(), Some("New Name"));
    let mut expected = vec![DAVID, JZ];
    expected.sort();
    assert_eq!(members(&app, board.id).await, expected);
    let changes=app.db().read(move |conn|Ok(conn.query_row("SELECT details FROM audit_logs WHERE action='room.membership.change' AND target_id=? ORDER BY id DESC LIMIT 1",[board.id],|r|r.get::<_,String>(0))?)).await.unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&changes).unwrap(),
        serde_json::json!({"granted":[],"revoked":["Jason"]})
    );
}
#[tokio::test]
async fn update_replaces_the_board_row_and_header() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID]).await;
    let (mut socket, _server) =
        stream_for(&app, &app.david(), &[&user_gid(DAVID).to_param(), "rooms"]).await;
    redirect(
        &app.david()
            .write(request(Method::PUT, Some(board.id), "New Name", &[DAVID]))
            .await,
        board.id,
    );
    assert!(frame(&mut socket).await.contains(&format!(
        "action=\"replace\" target=\"list_rooms_board_{}\"",
        board.id
    )));
    assert!(frame(&mut socket).await.contains(&format!(
        "action=\"replace\" target=\"header_rooms_board_{}\"",
        board.id
    )));
    socket.assert_silent().await;
}
#[tokio::test]
async fn a_non_administrator_creator_can_manage_members_of_their_own_board() {
    let app = setup().await;
    let board = fresh(&app, KEVIN, &[KEVIN, JZ]).await;
    redirect(
        &app.sign_in(KEVIN)
            .await
            .write(request(Method::PUT, Some(board.id), "New Name", &[KEVIN]))
            .await,
        board.id,
    );
    assert_eq!(room(&app, board.id).await.name.as_deref(), Some("New Name"));
    assert_eq!(members(&app, board.id).await, vec![KEVIN]);
}
#[tokio::test]
async fn only_admins_or_creators_can_update() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JZ]).await;
    let mut browser = app.sign_in(JZ).await;
    let (mut socket, _server) =
        stream_for(&app, &browser, &[&user_gid(JZ).to_param(), "rooms"]).await;
    assert_eq!(
        browser
            .write(request(Method::PUT, Some(board.id), "Changed", &[JZ]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(room(&app, board.id).await.name, board.name);
    socket.assert_silent().await;
}
#[tokio::test]
async fn remove_yourself() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JASON]).await;
    let mut browser = app.david();
    redirect(
        &browser
            .write(request(Method::PUT, Some(board.id), "Launch", &[JASON]))
            .await,
        board.id,
    );
    assert_eq!(members(&app, board.id).await, vec![JASON]);
    assert_eq!(
        browser
            .get(&format!("/rooms/{}", board.id))
            .await
            .location(),
        Some("http://campfire.test/")
    );
}
#[tokio::test]
async fn non_members_cannot_see_the_board_page() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID]).await;
    assert_eq!(
        app.sign_in(JZ)
            .await
            .get(&format!("/rooms/{}", board.id))
            .await
            .location(),
        Some("http://campfire.test/")
    );
}
#[tokio::test]
async fn non_members_cannot_reach_the_board_namespace() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID]).await;
    let mut browser = app.sign_in(JZ).await;
    for path in [
        format!("/rooms/boards/{}", board.id),
        format!("/rooms/boards/{}/edit", board.id),
    ] {
        assert_eq!(
            browser.get(&path).await.location(),
            Some("http://campfire.test/")
        );
    }
}
#[tokio::test]
async fn open_and_closed_rooms_cannot_be_converted_to_boards() {
    let app = setup().await;
    for id in [104393281, 654632876] {
        let before = room(&app, id).await;
        assert_eq!(
            app.david()
                .write(request(Method::PUT, Some(id), "Launch", &[DAVID]))
                .await
                .location(),
            Some("http://campfire.test/")
        );
        assert_eq!(room(&app, id).await, before);
    }
}
#[tokio::test]
async fn boards_cannot_be_converted_through_the_open_or_closed_namespaces() {
    let app = setup().await;
    let board = fresh(&app, DAVID, &[DAVID, JASON]).await;
    for namespace in ["opens", "closeds"] {
        let response = app
            .david()
            .write(
                Req::new(Method::PUT, &format!("/rooms/{namespace}/{}", board.id))
                    .form(&[("room[name]", "Watercooler"), ("user_ids[]", "127326141")]),
            )
            .await;
        assert_eq!(response.location(), Some("http://campfire.test/"));
    }
    assert_eq!(room(&app, board.id).await.room_type, RoomType::Board);
    let mut expected = vec![DAVID, JASON];
    expected.sort();
    assert_eq!(members(&app, board.id).await, expected);
}
#[tokio::test]
async fn a_direct_room_cannot_be_converted_to_a_board_and_have_its_participants_revised() {
    let app = setup().await;
    let before = room(&app, DIRECT_DAVID_JASON).await;
    let ids = members(&app, before.id).await;
    assert_eq!(
        app.david()
            .write(request(
                Method::PUT,
                Some(before.id),
                "Launch",
                &[DAVID, KEVIN]
            ))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(room(&app, before.id).await, before);
    assert_eq!(members(&app, before.id).await, ids);
}

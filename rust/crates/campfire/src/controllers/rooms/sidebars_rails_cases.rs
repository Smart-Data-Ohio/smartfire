//! Supported declarations from Users::SidebarsControllerTest. WS13 huddle cases stay inventoried.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::{Membership, Message, NewMessage, Room, RoomType};
use campfire_richtext::Content;

async fn setup() -> TestApp {
    let app = TestApp::boot_frozen().await.expect("seed required");
    // The parity seed adds unread rows for other scenarios; restore this file's
    // Rails fixture baseline before creating its one new unread message.
    app.db().write(|tx| {
        for mut membership in Membership::for_user(tx.conn(), DAVID)? {
            membership.update_involvement(tx, campfire_db::Involvement::Everything)?;
            membership.read(tx)?;
        }
        Ok(())
    }).await.unwrap();
    app
}
async fn sidebar(app: &TestApp) -> Content {
    let reply = app.david().get(&campfire_routes::user_sidebar()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    Content::wrap(&reply.text()).unwrap()
}
fn has_class(content: &Content, node: usize, name: &str) -> bool {
    content.dom.attr(node, "class").is_some_and(|s| s.split_whitespace().any(|c| c == name))
}
async fn unread(app: &TestApp, direct: bool) -> usize {
    app.db().read(move |conn| {
        let mut count = 0;
        for membership in Membership::for_user(conn, DAVID)? {
            if membership.unread() && Room::find(conn, membership.room_id)?.direct() == direct { count += 1; }
        }
        Ok(count)
    }).await.unwrap()
}
async fn assert_unread(room: i64, direct: bool) {
    let app = setup().await;
    app.db().write(move |tx| Message::create(tx, NewMessage {
        room_id: room, creator_id: JASON, client_message_id: Some("999".into()),
        body: Some("Hello".into()), ..Default::default()
    }).map(|_| ())).await.unwrap();
    let content = sidebar(&app).await;
    let rendered = content.dom.descendants(content.root).iter().filter(|&&id| has_class(&content, id, "unread")).count();
    assert_eq!(rendered, unread(&app, direct).await);
    assert!(rendered > 0, "the newly posted message must be visible as unread");
}
#[tokio::test]
async fn show_case() {
    let app = setup().await;
    let content = sidebar(&app).await;
    let text = content.dom.text_content(content.root);
    let names = app.db().read(|conn| Ok(Room::for_user_of_type(conn, DAVID, RoomType::Open)?.into_iter().filter_map(|r| r.name).collect::<Vec<_>>())).await.unwrap();
    for name in names { assert!(text.contains(&name), "{name}: {text}"); }
}
#[tokio::test]
async fn unread_directs() { assert_unread(DIRECT_DAVID_JASON, true).await; }
#[tokio::test]
async fn unread_other() { assert_unread(ALL_TALK, false).await; }
#[tokio::test]
async fn quiet_rows_keep_the_room_name_as_their_exact_link_text() {
    let app = setup().await;
    let content = sidebar(&app).await;
    let link = content.dom.descendants(content.root).into_iter().find(|&id| content.dom.attr(id, "id") == Some("list_rooms_open_201306877")).unwrap();
    assert_eq!(content.dom.name(link), "a");
    assert_eq!(content.dom.text_content(link).trim(), "HQ");
}
#[tokio::test]
async fn direct_rows_keep_avatar_card_triggers_as_siblings_of_the_room_link() {
    let app = setup().await;
    let group = app.db().write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, JASON, KEVIN])).await.unwrap();
    let content = sidebar(&app).await;
    for (room, expected) in [(DIRECT_DAVID_JASON, 1), (group.id, 2)] {
        let row_id = format!("list_rooms_direct_{room}");
        let row = content.dom.descendants(content.root).into_iter().find(|&id| content.dom.attr(id, "id") == Some(row_id.as_str())).unwrap();
        let url = format!("/rooms/{room}");
        let link = content.dom.descendants(row).into_iter().find(|&id| content.dom.name(id) == "a" && content.dom.attr(id, "href") == Some(url.as_str())).unwrap();
        assert!(content.dom.descendants(link).iter().all(|&id| content.dom.name(id) != "button" && content.dom.attr(id, "role") != Some("button")));
        let buttons = content.dom.descendants(row).into_iter().filter(|&id| content.dom.name(id) == "button" && has_class(&content, id, "profile-card-avatar")).collect::<Vec<_>>();
        assert_eq!(buttons.len(), expected);
        for button in buttons {
            let parent = content.dom.parent(button).unwrap();
            assert!(parent == row || has_class(&content, parent, "avatar__group"));
            assert!(content.dom.attr(button, "aria-label").unwrap().starts_with("View profile of "));
            assert_ne!(content.dom.attr(button, "tabindex"), Some("-1"));
        }
    }
}

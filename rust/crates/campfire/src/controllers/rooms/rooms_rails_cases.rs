//! Individually executed RoomController cases. Owner-rendered thread/message cases and
//! the deliberately different atomic queue-failure contract remain explicitly deferred.
use super::directs_rails_cases::{group, ids, note, pending_destroy};
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership, Message, NewMessage, Room, RoomType};
const JZ: i64 = 773523953;

#[tokio::test]
async fn show_renders_the_unread_divider_above_the_first_unread_message_on_the_page() {
    let app = setup().await;
    let room = 654632876;
    app.db().write(move |tx| {
        let first = Message::create(tx, NewMessage { room_id: room, creator_id: KEVIN, body: Some("First new".into()), client_message_id: Some("show-divider-first".into()), ..Default::default() })?;
        Message::create(tx, NewMessage { room_id: room, creator_id: KEVIN, body: Some("Second new".into()), client_message_id: Some("show-divider-second".into()), ..Default::default() })?;
        Membership::find_by_room_and_user(tx.conn(), room, DAVID)?.unwrap().mark_unread_before(tx, &first)
    }).await.unwrap();
    let reply = app.david().get(&campfire_routes::room(room)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    let content = campfire_richtext::Content::wrap(&html).unwrap();
    let dom = &content.dom;
    let divider = dom.descendants(content.root).into_iter().find(|&n| dom.attr(n, "id") == Some("unread-divider")).unwrap();
    assert!(dom.text_content(divider).to_lowercase().contains("new messages"));
    assert!(html.find("unread-divider").unwrap() < html.find("First new").unwrap());
    let button = dom.descendants(content.root).into_iter().find(|&n| dom.attr(n, "id") == Some("jump-to-unread")).unwrap();
    assert_eq!(dom.name(button), "button");
    assert!(dom.text_content(button).to_lowercase().contains("jump to unread"));
}

#[tokio::test]
async fn show_keeps_the_last_page_when_the_first_unread_fell_off_it_and_links_the_pill_to_it() {
    let app = setup().await;
    let room = 654632876;
    let first = app.db().write(move |tx| {
        let first = Message::create(tx, NewMessage { room_id: room, creator_id: KEVIN, body: Some("First unread off page".into()), client_message_id: Some("show-offpage-first".into()), ..Default::default() })?;
        for index in 0..=40 {
            Message::create(tx, NewMessage { room_id: room, creator_id: KEVIN, body: Some(format!("Later {index}")), client_message_id: Some(format!("show-offpage-{index}")), ..Default::default() })?;
        }
        Membership::find_by_room_and_user(tx.conn(), room, DAVID)?.unwrap().mark_unread_before(tx, &first)?;
        Ok(first.id)
    }).await.unwrap();
    let reply = app.david().get(&campfire_routes::room(room)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(!html.contains("First unread off page"));
    assert!(!html.contains("id=\"unread-divider\""));
    for index in 1..=40 { assert!(html.contains(&format!("id=\"message_show-offpage-{index}\"")), "last-page root {index} missing"); }
    assert!(!html.contains("id=\"message_show-offpage-0\""));
    let content = campfire_richtext::Content::wrap(&html).unwrap();
    let dom = &content.dom;
    let link = dom.descendants(content.root).into_iter().find(|&n| dom.attr(n, "id") == Some("jump-to-unread")).unwrap();
    assert_eq!(dom.name(link), "a");
    assert_eq!(dom.attr(link, "href"), Some(format!("/rooms/{room}?message_id={first}").as_str()));
    assert!(dom.text_content(link).to_lowercase().contains("jump to unread"));
}
async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("seed required")
}
async fn closed(app: &TestApp) -> i64 {
    app.db()
        .write(|tx| {
            Ok(Room::create_for(
                tx,
                RoomType::Closed,
                Some("Designers"),
                DAVID,
                &[DAVID, KEVIN, JZ],
            )?
            .id)
        })
        .await
        .unwrap()
}
fn destroy(id: i64) -> Req {
    Req::new(Method::DELETE, &format!("/rooms/{id}"))
}
fn leave(id: i64) -> Req {
    Req::new(Method::DELETE, &format!("/rooms/{id}/leave"))
}
fn root(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}
async fn last_room(app: &TestApp) -> i64 {
    app.db()
        .read(|conn| Ok(Room::last_for_user(conn, DAVID)?.unwrap().id))
        .await
        .unwrap()
}
#[tokio::test]
async fn index_redirects_to_the_users_last_room() {
    let app = setup().await;
    let id = last_room(&app).await;
    let reply = app.david().get("/rooms").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{id}").as_str())
    );
}
#[tokio::test]
async fn show_case() {
    let app = setup().await;
    let id = last_room(&app).await;
    assert_eq!(
        app.david().get(&format!("/rooms/{id}")).await.status,
        StatusCode::OK
    );
}
#[tokio::test]
async fn shows_records_the_last_room_visited_in_a_cookie() {
    let app = setup().await;
    let id = last_room(&app).await;
    let reply = app.david().get(&format!("/rooms/{id}")).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply
            .headers
            .get_all("set-cookie")
            .iter()
            .any(|h| h.to_str().unwrap().starts_with(&format!("last_room={id};")))
    );
}
#[tokio::test]
async fn destroy_removes_the_room_from_everyone_and_enqueues_its_deletion() {
    use crate::channels::tests::support::{Client, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let app = setup().await;
    let id = closed(&app).await;
    let mut david = app.david();
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("cookie", david.cookie_header().parse().unwrap());
    request
        .headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let signed = rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &["rooms"]);
    client
        .confirm(&identifier(
            serde_json::json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}),
        ))
        .await;
    root(&david.write(destroy(id)).await);
    pending_destroy(&app, id).await;
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    assert_eq!(
        frame["message"],
        format!(r#"<turbo-stream action="remove" target="list_rooms_closed_{id}"></turbo-stream>"#)
    );
    client.assert_silent().await;
    server.abort();
}
#[tokio::test]
async fn destroy_stamps_the_sweep_claim() {
    let app = setup().await;
    let id = closed(&app).await;
    root(&app.david().write(destroy(id)).await);
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.destroy_enqueued_at.is_some()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn destroyed_room_is_inaccessible_while_deletion_is_pending() {
    let app = setup().await;
    let id = closed(&app).await;
    let mut david = app.david();
    root(&david.write(destroy(id)).await);
    root(&david.get(&format!("/rooms/{id}")).await);
}
#[tokio::test]
async fn destroy_finishes_through_the_enqueued_job() {
    let app = setup().await;
    let id = closed(&app).await;
    root(&app.david().write(destroy(id)).await);
    pending_destroy(&app, id).await;
    campfire_db::models::room_delete::perform_with_config(app.db(), id, Default::default())
        .await
        .unwrap();
    assert!(
        app.db()
            .read(move |conn| Room::find_by_id(conn, id))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn destroy_only_allowed_for_creators_or_those_who_can_administer() {
    let app = setup().await;
    let id = closed(&app).await;
    let mut jz = app.sign_in(JZ).await;
    assert_eq!(jz.write(destroy(id)).await.status, StatusCode::FORBIDDEN);
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE rooms SET creator_id=? WHERE id=?", (JZ, id))?;
            Ok(())
        })
        .await
        .unwrap();
    root(&jz.write(destroy(id)).await);
    pending_destroy(&app, id).await;
}
#[tokio::test]
async fn destroy_answers_the_sidebar_menu_with_json_and_no_redirect() {
    let app = setup().await;
    let id = closed(&app).await;
    let reply = app
        .david()
        .write(Req::new(Method::DELETE, &format!("/rooms/{id}.json")))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.location(), None);
    assert_eq!(
        reply.json(),
        serde_json::json!({"deleted":true,"room_id":id})
    );
    pending_destroy(&app, id).await;
}
#[tokio::test]
async fn destroy_announces_the_deleted_room() {
    let app = setup().await;
    let id = closed(&app).await;
    let reply = app.david().write(destroy(id)).await;
    root(&reply);
    assert_eq!(
        super::direct_selection_tests::next_flash(&app, &reply, &mut None)["notice"],
        "Deleted #Designers"
    );
}
#[tokio::test]
async fn destroy_of_a_group_dm_is_refused_for_non_administrators() {
    let app = setup().await;
    let id = group(&app, &[DAVID, KEVIN, JZ], JZ).await;
    assert_eq!(
        app.sign_in(JZ).await.write(destroy(id)).await.status,
        StatusCode::FORBIDDEN
    );
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    assert_eq!(ids(&app, id).await.len(), 3);
}
#[tokio::test]
async fn leave_removes_only_your_membership_and_the_room_keeps_working() {
    let app = setup().await;
    let id = closed(&app).await;
    let mut jz = app.sign_in(JZ).await;
    root(&jz.write(leave(id)).await);
    assert!(!ids(&app, id).await.contains(&JZ));
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    root(&jz.get(&format!("/rooms/{id}")).await);
    assert_eq!(
        app.david().get(&format!("/rooms/{id}")).await.status,
        StatusCode::OK
    );
}
#[tokio::test]
async fn leave_answers_the_sidebar_menu_with_json_and_no_redirect() {
    let app = setup().await;
    let id = closed(&app).await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(Req::new(Method::DELETE, &format!("/rooms/{id}/leave.json")))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.location(), None);
    assert_eq!(reply.json(), serde_json::json!({"left":true,"room_id":id}));
    assert!(!ids(&app, id).await.contains(&JZ));
}
#[tokio::test]
async fn the_last_member_out_does_not_delete_the_room() {
    let app = setup().await;
    let id = app
        .db()
        .write(|tx| Ok(Room::create_for(tx, RoomType::Closed, Some("Solo"), DAVID, &[DAVID])?.id))
        .await
        .unwrap();
    root(&app.david().write(leave(id)).await);
    assert!(ids(&app, id).await.is_empty());
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn leave_is_rejected_for_non_members() {
    let app = setup().await;
    root(&app.sign_in(JZ).await.write(leave(ALL_TALK)).await);
    assert_eq!(ids(&app, ALL_TALK).await.len(), 3);
}
#[tokio::test]
async fn leave_of_a_group_dm_through_the_room_route_keeps_direct_semantics() {
    let app = setup().await;
    let id = group(&app, &[DAVID, KEVIN, JZ], JZ).await;
    root(&app.sign_in(JZ).await.write(leave(id)).await);
    assert!(!ids(&app, id).await.contains(&JZ));
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    assert_eq!(note(&app, id).await, "left the group");
}
#[tokio::test]
async fn show_renders_the_join_page_for_a_non_member_of_an_open_room() {
    let app = setup().await;
    let reply = app.sign_in(JZ).await.get("/rooms/104393281").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("<h2>#All Pets</h2>"));
    assert!(reply.text().contains("action=\"/rooms/104393281/join\""));
    assert!(reply.text().contains("Join channel"));
}
#[tokio::test]
async fn show_still_redirects_non_members_of_private_rooms() {
    let app = setup().await;
    root(
        &app.sign_in(JZ)
            .await
            .get(&format!("/rooms/{ALL_TALK}"))
            .await,
    );
}
#[tokio::test]
async fn show_still_redirects_non_members_of_deleted_open_rooms() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at=? WHERE id=104393281",
                [tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    root(&app.sign_in(JZ).await.get("/rooms/104393281").await);
}
#[tokio::test]
async fn join_recreates_the_membership_and_returns_to_the_room() {
    let app = setup().await;
    let mut jz = app.sign_in(JZ).await;
    assert!(
        app.db()
            .read(|conn| Membership::find_by_room_and_user(conn, 104393281, JZ))
            .await
            .unwrap()
            .is_none()
    );
    let reply = jz
        .write(Req::new(Method::POST, "/rooms/104393281/join"))
        .await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/rooms/104393281")
    );
    let member = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, 104393281, JZ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.involvement.unwrap().name(), "mentions");
}
#[tokio::test]
async fn join_is_refused_for_private_rooms() {
    let app = setup().await;
    let before = ids(&app, ALL_TALK).await;
    root(
        &app.sign_in(JZ)
            .await
            .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/join")))
            .await,
    );
    assert_eq!(ids(&app, ALL_TALK).await, before);
}
#[tokio::test]
async fn join_of_a_room_you_already_belong_to_returns_to_it() {
    let app = setup().await;
    let before = ids(&app, HQ).await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(Req::new(Method::POST, &format!("/rooms/{HQ}/join")))
        .await;
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{HQ}").as_str())
    );
    assert_eq!(ids(&app, HQ).await, before);
}

async fn posted_link_preview(href:&str,url:&str,client_id:&str)->String {
    let app=setup().await;
    let body=format!("<div><action-text-attachment content-type=\"application/vnd.actiontext.opengraph-embed\" href=\"{href}\" url=\"{url}\" filename=\"Free cookies\" caption=\"Cookies here\"></action-text-attachment></div>");
    let mut browser=app.david();
    let response=browser.write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages.turbo_stream"))
        .form(&[("message[body]",&body),("message[client_message_id]",client_id)])).await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let response=browser.get(&campfire_routes::room(ALL_TALK)).await;
    assert_eq!(response.status,StatusCode::OK);
    response.text()
}
#[tokio::test]
async fn show_renders_a_link_preview_written_by_hand_without_its_off_scheme_image_and_link() {
    let html=posted_link_preview("javascript:alert(1)","data:image/svg+xml;base64,PHN2Zy8+","hand-written-preview").await;
    assert!(!html.contains("javascript:alert"));assert!(!html.contains("data:image/svg"));
    assert!(html.contains("Free cookies"));
}
#[tokio::test]
async fn show_renders_a_link_preview_written_by_hand_without_its_image_pointed_at_this_smartfire() {
    let own_url=format!("http://campfire.test/rooms/{ALL_TALK}");
    let html=posted_link_preview(&own_url,&own_url,"same-host-preview").await;
    assert!(!html.contains(&format!("<img src=\"{own_url}\"")));
    assert!(!html.contains(&format!("<a rel=\"noreferrer\" target=\"_blank\" href=\"{own_url}\"")));
    assert!(html.contains("Free cookies"));
}
#[tokio::test]
async fn show_renders_an_unfurled_link_preview() {
    let html=posted_link_preview("https://example.com/page","https://example.com/image.png","unfurled-preview").await;
    assert!(html.contains("<img src=\"/embeds/image/"));
    assert!(!html.contains("https://example.com/image.png"));
    assert!(html.contains("href=\"https://example.com/page\""));
}

#[tokio::test]
async fn show_renders_collapsed_work_thread_guidance_in_the_new_thread_panel() {
    let app = setup().await;
    let reply = app.david().get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let content = campfire_richtext::Content::wrap(&reply.text()).unwrap();
    let dom = &content.dom;
    let create = dom.descendants(content.root).into_iter().find(|&id| dom.attr(id, "data-thread-panel-target") == Some("create")).expect("new-thread panel");
    let guide = dom.descendants(create).into_iter().find(|&id| dom.name(id) == "details" && dom.attr(id, "class").is_some_and(|s| s.split_whitespace().any(|s| s == "thread-panel__guide"))).expect("collapsed work-thread guidance");
    assert!(dom.attr(guide, "open").is_none());
    let summary = dom.descendants(guide).into_iter().find(|&id| dom.name(id) == "summary").unwrap();
    assert_eq!(dom.text_content(summary), "How to start a work thread");
    let items = dom.descendants(guide).into_iter().filter(|&id| dom.name(id) == "li").map(|id| dom.text_content(id)).collect::<Vec<_>>();
    assert!(items.iter().any(|s| s.contains("Track as work")));
    assert!(items.iter().all(|s| !s.contains("Open a channel")));
}

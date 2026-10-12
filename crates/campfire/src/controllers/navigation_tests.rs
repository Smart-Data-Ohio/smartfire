use axum::http::{Method, StatusCode};

use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};

const COLLECTIONS: [&str; 6] = ["opens", "closeds", "directs", "boards", "voices", "stages"];
const ORIGIN: &str = "http://campfire.test";

#[tokio::test]
async fn room_collections_redirect_users_without_rooms() {
    let app = TestApp::boot().await.expect("seed required");
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM memberships WHERE user_id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        app.db()
            .read(|conn| campfire_db::Room::for_user(conn, DAVID))
            .await
            .unwrap()
            .is_empty()
    );

    let mut browser = app.sign_in(DAVID).await;
    let mut failures = Vec::new();
    for path in COLLECTIONS
        .map(|kind| format!("/rooms/{kind}"))
        .into_iter()
        .chain(["/rooms".into()])
    {
        for (suffix, accept, target) in [
            ("", "text/html", "/app/"),
            (".html/", "text/html", "/app/"),
            (".xhtml/", "text/html", "/app/"),
            (".json", "application/json", "/"),
            (".json/", "application/json", "/"),
            (".xml", "application/xml", "/"),
            ("", "application/json", "/"),
        ] {
            for method in [Method::GET, Method::HEAD] {
                let path = format!("{path}{suffix}");
                let reply = browser
                    .send(Req::new(method.clone(), &path).header("accept", accept))
                    .await;
                let location = format!("{ORIGIN}{target}");
                let status = if target == "/app/" { StatusCode::MOVED_PERMANENTLY } else { StatusCode::FOUND };
                if reply.status != status || reply.location() != Some(location.as_str())
                {
                    failures.push(format!(
                        "{method} {path} ({accept}): {:?} {:?}, expected {status} {location}",
                        reply.status,
                        reply.location()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn typed_room_collections_keep_machine_redirects_for_users_with_rooms() {
    let app = TestApp::boot().await.expect("seed required");
    let mut browser = app.sign_in(DAVID).await;
    for kind in COLLECTIONS {
        for (suffix, accept) in [
            (".json", "application/json"),
            (".xml/", "application/xml"),
            ("", "application/json"),
        ] {
            let path = format!("/rooms/{kind}{suffix}");
            let reply = browser
                .send(Req::new(Method::GET, &path).header("accept", accept))
                .await;
            assert_eq!(reply.status, StatusCode::FOUND, "{path}");
            assert_eq!(reply.location(), Some("http://campfire.test/"), "{path}");
        }
    }
    let last_room = app
        .db()
        .read(|conn| campfire_db::Room::last_for_user(conn, DAVID))
        .await
        .unwrap()
        .unwrap();
    let reply = browser.get("/rooms.json").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("{ORIGIN}/rooms/{}", last_room.id).as_str())
    );
}

#[tokio::test]
async fn typed_bookmarks_redirect_permanently_with_message_queries_and_keep_machine_urls() {
    use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Room, RoomType};
    let app = TestApp::boot_frozen().await.expect("restored seed required");
    let mut browser = app.sign_in(DAVID).await;
    for (namespace, kind) in [
        ("opens", RoomType::Open),
        ("closeds", RoomType::Closed),
        ("directs", RoomType::Direct),
        ("boards", RoomType::Board),
        ("voices", RoomType::Voice),
        ("stages", RoomType::Stage),
    ] {
        let (room, message, thread) = app.db().write(move |tx| {
            let room = Room::create_for(tx, kind, Some("Bookmark"), DAVID, &[DAVID])?;
            let thread = if kind == RoomType::Board {
                Some(ChannelThread::create(tx, NewChannelThread {
                    room_id: room.id,
                    creator_id: DAVID,
                    name: Some("Bookmark post".into()),
                    work_status: Some("planned".into()),
                    ..Default::default()
                })?.id)
            } else { None };
            let message = Message::create(tx, NewMessage {
                room_id: room.id,
                creator_id: DAVID,
                thread_id: thread,
                markdown_source: Some("Bookmark anchor".into()),
                ..Default::default()
            })?;
            Ok((room.id, message.id, thread))
        }).await.unwrap();
        let thread_query = thread.map(|id| format!("thread={id}&")).unwrap_or_default();
        let query = format!("{thread_query}message_id={message}&source=mail&source=push&note=a%20b&classic=1");
        let path = format!("/rooms/{namespace}/{room}");
        for method in [Method::GET, Method::HEAD] {
            let reply = browser.send(Req::new(method.clone(), &format!("{path}?{query}"))).await;
            let target = thread.map_or_else(
                || format!("{ORIGIN}/app/r/{room}/m/{message}?source=mail&source=push&note=a%20b"),
                |id| format!("{ORIGIN}/app/r/{room}/t/{id}?m={message}&source=mail&source=push&note=a%20b"),
            );
            assert_eq!((reply.status, reply.location()), (StatusCode::MOVED_PERMANENTLY, Some(target.as_str())), "{method} {path}");
        }
        for header in [("accept", "application/json"), ("x-requested-with", "XMLHttpRequest"), ("turbo-frame", "room")] {
            let reply = browser.send(Req::new(Method::GET, &path).header(header.0, header.1)).await;
            assert_eq!((reply.status, reply.location()), (StatusCode::FOUND, Some(format!("{ORIGIN}/rooms/{room}").as_str())), "{path} {header:?}");
        }
    }
}

#[tokio::test]
async fn room_collections_preserve_query_context_and_refresh_keeps_its_protocol_redirect() {
    let app = TestApp::boot_frozen().await.expect("restored seed required");
    let mut browser = app.sign_in(DAVID).await;
    for path in COLLECTIONS.map(|kind| format!("/rooms/{kind}")).into_iter().chain(["/rooms".into()]) {
        let reply = browser.get(&format!("{path}?source=mail&source=push&classic=1")).await;
        assert_eq!((reply.status, reply.location()), (StatusCode::MOVED_PERMANENTLY, Some("http://campfire.test/app/?source=mail&source=push")), "{path}");
    }
    let room = app.db().read(|conn| campfire_db::Room::last_for_user(conn, DAVID)).await.unwrap().unwrap();
    let reply = browser.get(&format!("/rooms/{}/refresh", room.id)).await;
    assert_eq!((reply.status, reply.location()), (StatusCode::FOUND, Some(format!("{ORIGIN}/rooms/{}", room.id).as_str())));
}

#[tokio::test]
async fn stored_message_links_keep_navigation_context_and_the_fragment_response() {
    let app = TestApp::boot_frozen().await.expect("restored seed required");
    let mut browser = app.sign_in(DAVID).await;
    let path = "/rooms/654632876/message_links/1?source=mail&source=push&classic=1";
    let destination = "http://campfire.test/app/r/104393281/m/935962045";
    for method in [Method::GET, Method::HEAD] {
        let reply = browser.send(Req::new(method.clone(), path)).await;
        assert_eq!((reply.status, reply.location()), (StatusCode::MOVED_PERMANENTLY, Some(format!("{destination}?source=mail&source=push").as_str())), "{method}");
    }
    let reply = browser.classic_page(path).await;
    assert_eq!((reply.status, reply.location()), (StatusCode::FOUND, Some(destination)));
}

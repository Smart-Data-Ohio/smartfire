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
                if reply.status != StatusCode::FOUND || reply.location() != Some(location.as_str())
                {
                    failures.push(format!(
                        "{method} {path} ({accept}): {:?} {:?}, expected 302 {location}",
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

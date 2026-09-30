//! Real controller gates and atomic icon validation, against pinned Rails observations.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/room_icons.json")).unwrap()
}
fn request(method: Method, path: &str, icon: &serde_json::Value) -> Req {
    Req::new(method, path).header("Content-Type", "application/json").header("Accept", "text/html")
        .body(serde_json::to_vec(&serde_json::json!({"room":{"name":"Attempted","icon_name":icon},"user_ids":[DAVID]})).unwrap())
}
async fn counts(app: &TestApp) -> (i64, i64, i64) {
    app.db()
        .read(|conn| {
            Ok((
                conn.query_row_cached("SELECT count(*) FROM rooms", [], |r| r.get(0))?,
                conn.query_row_cached("SELECT count(*) FROM memberships", [], |r| r.get(0))?,
                conn.query_row_cached("SELECT count(*) FROM audit_logs", [], |r| r.get(0))?,
            ))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn room_icon_writes_require_a_human_with_create_or_administer_permission() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let before = counts(&app).await;
    for namespace in ["opens", "closeds"] {
        let path = format!("/rooms/{namespace}/{HQ}");
        let denied = app
            .sign_in(KEVIN)
            .await
            .write(request(Method::PATCH, &path, &serde_json::json!("smile")))
            .await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN);
        let denied = app
            .anonymous()
            .send(request(
                Method::PATCH,
                &format!("{path}?bot_key={BENDER_KEY}"),
                &serde_json::json!("smile"),
            ))
            .await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN);
        let denied = app
            .anonymous()
            .send(request(
                Method::POST,
                &format!("/rooms/{namespace}"),
                &serde_json::json!("smile"),
            ))
            .await;
        assert_eq!(denied.location(), Some("http://campfire.test/session/new"));
    }
    assert_eq!(counts(&app).await, before);
}
#[tokio::test]
async fn room_icon_creation_casts_normalizes_and_validates_before_any_write() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut david = app.david();
    for row in oracle()["creations"].as_array().unwrap() {
        let before = counts(&app).await;
        let reply = david
            .write(request(
                Method::POST,
                &format!("/rooms/{}", row["namespace"].as_str().unwrap()),
                &row["input"],
            ))
            .await;
        assert_eq!(
            reply.status.as_u16() as u64,
            row["status"].as_u64().unwrap(),
            "{}: {}",
            row["input"],
            reply.text()
        );
        let after = counts(&app).await;
        assert_eq!(
            serde_json::json!({"rooms":after.0-before.0,"memberships":after.1-before.1,"audits":after.2-before.2}),
            row["delta"]
        );
        if after.0 > before.0 {
            let icon = app
                .db()
                .read(|conn| {
                    let id: i64 =
                        conn.query_row_cached("SELECT max(id) FROM rooms", [], |r| r.get(0))?;
                    Ok(Room::find(conn, id)?.icon_name)
                })
                .await
                .unwrap();
            assert_eq!(
                serde_json::json!(icon),
                row["icon_name"],
                "{}",
                row["input"]
            );
        } else {
            assert!(reply.text().contains("Icon name is not a known icon"));
            assert!(reply.text().contains("field_with_errors"));
        }
    }
}
#[tokio::test]
async fn room_icon_update_errors_preserve_type_memberships_name_timestamp_and_audits() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut david = app.david();
    for row in oracle()["updates"].as_array().unwrap() {
        app.db().write(|tx| {tx.conn().execute_cached("UPDATE rooms SET name='Baseline',type='Rooms::Open',icon_name='github' WHERE id=?",[HQ])?;Ok(())}).await.unwrap();
        let before = app
            .db()
            .read(|conn| {
                let r = Room::find(conn, HQ)?;
                Ok((r.clone(), r.user_ids(conn)?))
            })
            .await
            .unwrap();
        let count = counts(&app).await;
        let reply = david
            .write(request(
                Method::PATCH,
                &format!("/rooms/{}/{HQ}", row["namespace"].as_str().unwrap()),
                &row["input"],
            ))
            .await;
        assert_eq!(
            reply.status.as_u16() as u64,
            row["status"].as_u64().unwrap(),
            "{}",
            reply.text()
        );
        let after = app
            .db()
            .read(|conn| {
                let r = Room::find(conn, HQ)?;
                Ok((r.clone(), r.user_ids(conn)?))
            })
            .await
            .unwrap();
        assert_eq!(
            serde_json::json!(after.0.icon_name),
            row["after"]["icon_name"]
        );
        assert_eq!(serde_json::json!(after.0.name), row["after"]["name"]);
        assert_eq!(
            after.0.room_type.class_name(),
            row["after"]["type"].as_str().unwrap()
        );
        if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
            assert_eq!(after, before);
            assert_eq!(counts(&app).await, count);
            assert!(reply.text().contains("Icon name is not a known icon"));
        }
    }
    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET type='Rooms::Open',icon_name='legacy_unknown' WHERE id=?",
                [HQ],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(request(
            Method::PATCH,
            &format!("/rooms/opens/{HQ}"),
            &serde_json::json!(":legacy_unknown:"),
        ))
        .await;
    assert_eq!(
        reply.status.as_u16() as u64,
        oracle()["legacy"]["status"].as_u64().unwrap()
    );
    assert_eq!(
        app.db()
            .read(|conn| Ok(Room::find(conn, HQ)?.icon_name))
            .await
            .unwrap()
            .as_deref(),
        Some("legacy_unknown")
    );
}

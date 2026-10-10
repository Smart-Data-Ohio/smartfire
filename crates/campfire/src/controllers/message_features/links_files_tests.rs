//! Named Rails quote endpoint and file-browser ports; all rows/bytes are committed fixtures.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/links_files.json"
    ))
    .unwrap()
}
async fn app() -> TestApp {
    let app = TestApp::boot_with_test_clock(std::sync::Arc::new(
        campfire_kit::clock::FrozenClock::new(SEED_NOW.parse().unwrap()),
    ))
    .await
    .expect("WS8bm2 requires default seed");
    let rows = fixture()["rows"].clone();
    app.db()
        .write(move |tx| {
            for table in [
                "rooms",
                "memberships",
                "channel_threads",
                "messages",
                "action_text_rich_texts",
                "message_references",
                "active_storage_blobs",
                "active_storage_attachments",
                "drive_attachments",
            ] {
                for row in rows[table].as_array().unwrap() {
                    let row = row.as_object().unwrap();
                    let columns = row
                        .keys()
                        .map(|k| format!("\"{k}\""))
                        .collect::<Vec<_>>()
                        .join(",");
                    let placeholders = vec!["?"; row.len()].join(",");
                    let values = row.values().map(|v| match v {
                        Value::Null => rusqlite::types::Value::Null,
                        Value::Number(n) if n.is_i64() => {
                            rusqlite::types::Value::Integer(n.as_i64().unwrap())
                        }
                        Value::Number(n) => rusqlite::types::Value::Real(n.as_f64().unwrap()),
                        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                        _ => panic!("SQL fixture value {v}"),
                    });
                    tx.conn().execute(
                        &format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})"),
                        rusqlite::params_from_iter(values),
                    )?;
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    app
}
const FILE_ROOM: i64 = 918001;
fn path(room: i64) -> String {
    format!(
        "/rooms/{room}/message_links/{}",
        fixture()["reference_id"].as_i64().unwrap()
    )
}
#[tokio::test]
async fn non_members_get_nothing() {
    let app = app().await;
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .get(&format!("/api/v1/rooms/{FILE_ROOM}/files"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn a_non_member_of_the_quoting_room_gets_nothing() {
    let app = app().await;
    assert_eq!(
        app.sign_in(KEVIN).await.get(&path(ALL_TALK)).await.status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn a_reference_from_another_room_gets_nothing() {
    let app = app().await;
    assert_eq!(
        app.david().get(&path(ALL_TALK)).await.status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn file_sizes_match_pinned_rails_and_pages_are_bounded() {
    for case in fixture()["sizes"].as_array().unwrap() {
        assert_eq!(
            campfire_presentation::room_files::human_size(case["size"].as_i64().unwrap()),
            case["text"].as_str().unwrap()
        );
    }
    for (raw, expected) in [
        ("0", 1),
        ("-2", 1),
        ("1junk", 1),
        (" +2", 2),
        ("99999999999999999999999999999999999", 20),
        ("-99999999999999999999999999999999999", 1),
        ("20", 20),
        ("21", 20),
    ] {
        assert_eq!(campfire_db::room_files::page(raw), expected);
    }
}

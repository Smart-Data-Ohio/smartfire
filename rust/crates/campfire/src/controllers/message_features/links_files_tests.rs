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
async fn files(app: &TestApp, query: &str) -> Reply {
    app.david()
        .get(&format!("/rooms/{FILE_ROOM}/files{query}"))
        .await
}
fn count(r: &Reply) -> usize {
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    r.text().matches("class=\"room-files__name\"").count()
}
fn path(room: i64) -> String {
    format!(
        "/rooms/{room}/message_links/{}",
        fixture()["reference_id"].as_i64().unwrap()
    )
}
#[tokio::test]
async fn lists_uploads_newest_first_with_jump_links() {
    let app = app().await;
    let r = files(&app, "").await;
    assert_eq!(count(&r), 30);
    let body = r.text();
    assert!(body.find("alpha-report.pdf").unwrap() < body.find("beta-mockup.png").unwrap());
    assert!(body.contains("message_id=") && body.contains("thread="));
}
#[tokio::test]
async fn lists_drive_attachments_as_generic_picker_only_rows() {
    let app = app().await;
    let r = files(&app, "").await;
    count(&r);
    assert_eq!(
        r.text()
            .matches("class=\"drive-attachment room-files__drive-link\"")
            .count(),
        30
    );
    assert!(
        r.text().contains("Google Drive file")
            && r.text()
                .contains("https://drive.google.com/open?id=picker01234567890")
    );
}
#[tokio::test]
async fn type_filters_narrow_uploads() {
    let app = app().await;
    for (kind, n) in [
        ("images", 1),
        ("videos", 1),
        ("documents", 29),
        ("other", 1),
    ] {
        assert_eq!(count(&files(&app, &format!("?type={kind}")).await), n);
    }
}
#[tokio::test]
async fn filename_search_matches_substrings_and_escapes_wildcards() {
    let app = app().await;
    for (q, n) in [
        ("REPORT", 1),
        ("%25", 1),
        ("%5F", 1),
        ("%5C", 1),
        ("nothing", 0),
    ] {
        assert_eq!(count(&files(&app, &format!("?filename={q}")).await), n);
    }
}
#[tokio::test]
async fn uploads_page_cumulatively() {
    let app = app().await;
    assert_eq!(count(&files(&app, "").await), 30);
    assert_eq!(count(&files(&app, "?page=2").await), 33);
}
#[tokio::test]
async fn only_the_rooms_own_files_are_listed() {
    let app = app().await;
    let source = fixture()["source_id"].as_i64().unwrap();
    app.db()
        .write(move |tx| {
            let blob = campfire_storage::blob::NewBlob::unfurl(
                b"foreign",
                campfire_storage::filename::Filename::new("foreign-room-only.txt"),
                Some("text/plain"),
                "local",
                false,
            )
            .insert(tx.conn(), tx.now().jiff())
            .map_err(crate::controllers::presenters::storage_error)?;
            campfire_storage::blob::insert_attachment(
                tx.conn(),
                "attachment",
                "Message",
                source,
                blob.id,
                tx.now().jiff(),
            )
            .map_err(crate::controllers::presenters::storage_error)?;
            Ok(())
        })
        .await
        .unwrap();
    let r = files(&app, "").await;
    count(&r);
    assert!(!r.text().contains("foreign-room-only.txt"));
    let r = app.david().get(&format!("/rooms/{ALL_TALK}/files")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(!r.text().contains("alpha-report.pdf"));
}
#[tokio::test]
async fn non_members_get_nothing() {
    let app = app().await;
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .get(&format!("/rooms/{FILE_ROOM}/files"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn a_member_of_the_source_room_sees_the_quote_card() {
    let app = app().await;
    let r = app.david().get(&path(QUIET_CORNER)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.text().contains("message-quote__author")
            && r.text().contains("All Talk")
            && r.text().contains("Jump to message")
    );
}
#[tokio::test]
async fn a_quote_of_a_soft_deleted_source_room_shows_only_the_private_chip() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                (tx.now(), ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app.david().get(&path(QUIET_CORNER)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("Message in a private room"));
    assert!(!r.text().contains("private&gt;") && !r.text().contains("message-quote__author"));
}
#[tokio::test]
async fn a_non_member_of_the_source_room_sees_only_the_private_chip() {
    let app = app().await;
    let r = app.sign_in(KEVIN).await.get(&path(QUIET_CORNER)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("Message in a private room"));
    assert!(!r.text().contains("Source ") && !r.text().contains("All Talk"));
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
async fn quote_responses_match_pinned_rails_exact_bytes() {
    let app = app().await;
    for case in fixture()["quotes"].as_array().unwrap() {
        if case["deleted"] == true {
            app.db()
                .write(|tx| {
                    tx.conn().execute(
                        "UPDATE rooms SET deleted_at=? WHERE id=?",
                        (tx.now(), ALL_TALK),
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let mut browser = app.sign_in(case["viewer_id"].as_i64().unwrap()).await;
        let r = browser.get(&path(case["room_id"].as_i64().unwrap())).await;
        assert_eq!(r.status.as_u16(), case["status"].as_u64().unwrap() as u16);
        assert_eq!(r.text(), case["body"].as_str().unwrap());
    }
}
#[tokio::test]
async fn files_sections_match_pinned_rails_exact_bytes() {
    let app = app().await;
    for case in fixture()["files"].as_array().unwrap() {
        let r = app.david().get(case["path"].as_str().unwrap()).await;
        assert_eq!(r.status, StatusCode::OK);
        let body = r.text();
        let start = body.find("<section class=\"room-files\"").unwrap();
        let end = start + body[start..].find("</section>").unwrap() + 10;
        assert_eq!(
            &body[start..end],
            case["content"].as_str().unwrap(),
            "{}",
            case["path"]
        );
    }
}

#[tokio::test]
async fn rendering_costs_the_same_queries_for_4_files_as_for_16() {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    // Trace executions on every reader, including cached statements and the complete HTTP layout.
    extern "C" fn trace(
        mask: u32,
        data: *mut std::ffi::c_void,
        stmt: *mut std::ffi::c_void,
        _: *mut std::ffi::c_void,
    ) -> i32 {
        if mask == rusqlite::ffi::SQLITE_TRACE_STMT {
            unsafe {
                let sql = rusqlite::ffi::sqlite3_sql(stmt.cast());
                if !sql.is_null()
                    && std::ffi::CStr::from_ptr(sql)
                        .to_string_lossy()
                        .trim_start()
                        .to_ascii_uppercase()
                        .starts_with("SELECT")
                {
                    (&*data.cast::<AtomicUsize>()).fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        0
    }
    let app = app().await;
    let rows = fixture()["rows"].clone();
    app.db().write(|tx| {
        tx.conn().execute("DELETE FROM active_storage_attachments WHERE record_type='Message' AND record_id IN (SELECT id FROM messages WHERE room_id=?) AND id NOT IN (SELECT id FROM active_storage_attachments WHERE record_type='Message' AND record_id IN (SELECT id FROM messages WHERE room_id=?) ORDER BY id LIMIT 2)",(FILE_ROOM,FILE_ROOM))?;
        tx.conn().execute("DELETE FROM drive_attachments WHERE message_id IN (SELECT id FROM messages WHERE room_id=?) AND id NOT IN (SELECT id FROM drive_attachments WHERE message_id IN (SELECT id FROM messages WHERE room_id=?) ORDER BY id LIMIT 2)",(FILE_ROOM,FILE_ROOM))?; Ok(())
    }).await.unwrap();
    count(&files(&app, "").await);
    let counter = Arc::new(AtomicUsize::new(0));
    let n = app.booted.app.config.db_readers;
    let barrier = Arc::new(Barrier::new(n));
    let pointers = futures_util::future::join_all((0..n).map(|_| {
        let counter = counter.clone();
        let barrier = barrier.clone();
        app.db().read(move |conn| {
            let pointer = Arc::into_raw(counter) as usize;
            unsafe {
                assert_eq!(
                    rusqlite::ffi::sqlite3_trace_v2(
                        conn.handle(),
                        rusqlite::ffi::SQLITE_TRACE_STMT,
                        Some(trace),
                        pointer as *mut std::ffi::c_void
                    ),
                    rusqlite::ffi::SQLITE_OK
                );
            }
            barrier.wait();
            Ok(pointer)
        })
    }))
    .await
    .into_iter()
    .map(Result::unwrap)
    .collect::<Vec<_>>();
    counter.store(0, Ordering::SeqCst);
    assert_eq!(count(&files(&app, "").await), 2);
    let few = counter.load(Ordering::SeqCst);
    app.db()
        .write(move |tx| {
            for table in ["active_storage_attachments", "drive_attachments"] {
                for row in &rows[table].as_array().unwrap()[2..8] {
                    let row = row.as_object().unwrap();
                    let columns = row
                        .keys()
                        .map(|k| format!("\"{k}\""))
                        .collect::<Vec<_>>()
                        .join(",");
                    let placeholders = vec!["?"; row.len()].join(",");
                    let values = row.values().map(|v| match v {
                        Value::Null => rusqlite::types::Value::Null,
                        Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                        _ => panic!("fixture {v}"),
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
    counter.store(0, Ordering::SeqCst);
    assert_eq!(count(&files(&app, "").await), 8);
    let many = counter.load(Ordering::SeqCst);
    let barrier = Arc::new(Barrier::new(n));
    for result in futures_util::future::join_all((0..n).map(|_| {
        let barrier = barrier.clone();
        app.db().read(move |conn| {
            unsafe {
                rusqlite::ffi::sqlite3_trace_v2(conn.handle(), 0, None, std::ptr::null_mut());
            }
            barrier.wait();
            Ok(())
        })
    }))
    .await
    {
        result.unwrap();
    }
    for pointer in pointers {
        unsafe {
            drop(Arc::from_raw(pointer as *const AtomicUsize));
        }
    }
    assert!(few > 0);
    assert_eq!(
        few, many,
        "complete HTTP requests with four and sixteen file rows: ({few},{many})"
    );
}
#[tokio::test]
async fn file_sizes_match_pinned_rails_and_pages_are_bounded() {
    for case in fixture()["sizes"].as_array().unwrap() {
        assert_eq!(
            campfire_views::room_files::human_size(case["size"].as_i64().unwrap()),
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
#[tokio::test]
async fn quote_frames_hide_system_notes_and_keep_direct_room_names_neutral() {
    let app = app().await;
    let source = fixture()["source_id"].as_i64().unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE messages SET room_id=?,creator_id=? WHERE id=?",
                (DIRECT_DAVID_JASON, JASON, source),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app.david().get(&path(QUIET_CORNER)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("in a direct message"));
    assert!(!r.text().contains("in Jason"));
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE messages SET system_note=1 WHERE id=?", [source])?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app.david().get(&path(QUIET_CORNER)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("Message in a private room"));
    assert!(!r.text().contains("Source "));
}

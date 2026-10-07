//! Actual presenter and HTTP responses compared against Rails, including shared chrome.
use crate::controllers::presenters::{boards, page, test_support::*};
use askama::Template;

#[tokio::test]
async fn board_rows_reuse_the_loaded_room_for_aged_posts() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/boards_lifecycle_queries.json"
    ))
    .unwrap();
    let mut measured = Vec::new();
    let mut expected = Vec::new();
    for row in oracle["rows"].as_array().unwrap() {
        let count = row["posts"].as_i64().unwrap();
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let (room, posts) = app.db().write(move |tx| {
            let room = campfire_db::Room::find(tx.conn(), 699448332)?;
            let mut posts = Vec::new();
            for n in 0..count {
                tx.conn().execute("INSERT INTO channel_threads(room_id,creator_id,name,work_status,last_activity_at,created_at,updated_at) VALUES(699448332,127326141,?,'planned',?,?,?)", rusqlite::params![format!("Query probe {n}"),tx.now().ago(jiff::SignedDuration::from_hours(168)),tx.now(),tx.now()])?;
                posts.push(campfire_db::ChannelThread::find(tx.conn(), tx.conn().last_insert_rowid())?);
            }
            Ok((room, posts))
        }).await.unwrap();
        assert_eq!(
            app.booted.app.clock.now().to_string(),
            oracle["now"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap()
                .to_string()
        );
        let probe =
            super::query_probe::SqlProbe::start(app.db(), app.booted.app.config.db_readers).await;
        let state = app.booted.app.clone();
        let states = app
            .db()
            .read(move |conn| {
                let presenter = crate::controllers::presenters::Presenter::new(conn, &state, None);
                Ok(boards::rows(&presenter, &room, &posts)?
                    .into_iter()
                    .map(|row| row.lifecycle)
                    .collect::<Vec<_>>())
            })
            .await
            .unwrap();
        let queries = probe.finish().await;
        assert_eq!(
            serde_json::json!(states),
            row["states"],
            "aged board posts must remain open"
        );
        let reads = queries
            .iter()
            .filter(|q| {
                let sql = q.sql.to_ascii_uppercase().replace('"', "");
                sql.trim_start().starts_with("SELECT") && sql.contains("FROM ROOMS")
            })
            .count();
        println!(
            "Board lifecycle: {count} aged posts; {reads} Room SELECTs; Rails {}",
            row["room_reads"]
        );
        measured.push(reads);
        expected.push(row["room_reads"].as_u64().unwrap() as usize);
    }
    assert_eq!(
        measured, expected,
        "reuse the parent loaded by the board association"
    );
}

#[tokio::test]
async fn board_rows_match_both_rails_renderings() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/boards_read.json")).unwrap();
    for row in oracle["fragments"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("default seed required");
        fixture_setup(&app, row).await;
        let (id, column) = (
            row["thread_id"].as_i64().unwrap(),
            row["column"].as_bool().unwrap(),
        );
        let state = app.booted.app.clone();
        let actual = app
            .db()
            .read(move |conn| {
                let thread = campfire_db::ChannelThread::find(conn, id)?;
                let room = campfire_db::Room::find(conn, thread.room_id)?;
                let presenter = crate::controllers::presenters::Presenter::new(conn, &state, None);
                let facts = boards::rows(&presenter, &room, &[thread])?;
                page::render_detached_at(&state, None, "http://campfire.test", |ctx| {
                    campfire_views::rooms::boards::RowPartial {
                        ctx,
                        row: &facts[0],
                        column,
                    }
                    .render()
                })
                .map_err(|error| campfire_db::Error::Other(error.to_string()))
            })
            .await
            .unwrap();
        assert_eq!(
            actual,
            row["html"].as_str().unwrap(),
            "thread {id} column {column}"
        );
    }
}

#[tokio::test]
async fn board_list_and_columns_match_complete_rails_http_responses() {
    let env = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../parity/.env.reference"
    ))
    .unwrap();
    let vapid = env
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
        .collect::<Vec<_>>();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/boards_read.json")).unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .expect("default seed required");
        fixture_setup(&app, row).await;
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let method = row["method"]
            .as_str()
            .unwrap_or("get")
            .to_ascii_uppercase()
            .parse::<axum::http::Method>()
            .unwrap();
        let mut request =
            Req::new(method.clone(), row["path"].as_str().unwrap()).header("user-agent", "Mozilla");
        if let Some(headers) = row["headers"].as_object() {
            for (key, value) in headers {
                request = request.header(key, value.as_str().unwrap());
            }
        }
        let form = row["form"].as_array().map(|pairs| {
            pairs
                .iter()
                .map(|pair| (pair[0].as_str().unwrap(), pair[1].as_str().unwrap()))
                .collect::<Vec<_>>()
        });
        if let Some(form) = form {
            request = request.form(&form);
        }
        let response = if method == axum::http::Method::GET {
            with_fixed_render_secrets(browser.send(request)).await
        } else {
            with_fixed_render_secrets(browser.write(request)).await
        };
        if let Some(location) = row["location"].as_str() {
            assert_eq!(response.location(), Some(location));
        }
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16
        );
        let actual = response.text();
        let expected = row["html"].as_str().unwrap();
        if actual != expected {
            rails_mismatch(&actual, expected, row["name"].as_str().unwrap());
        }
    }
}

async fn fixture_setup(app: &TestApp, row: &serde_json::Value) {
    if let Some(sql) = row["setup"].as_array() {
        let sql = sql
            .iter()
            .map(|sql| sql.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        app.db()
            .write(move |tx| {
                for sql in sql {
                    tx.conn().execute_batch(&sql)?;
                }
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn committed_board_rows_render_on_real_cable_and_rollback_stays_silent() {
    use super::opens_rails_cases::{frame, stream_for_channel};
    use campfire_db::{ChannelThread, NewChannelThread, Room, ThreadTag};

    let app = TestApp::boot_frozen().await.expect("default seed required");
    let room = app
        .db()
        .read(|conn| Room::find(conn, 699448332))
        .await
        .unwrap();
    let browser = app.david();
    let (mut socket, _server) = stream_for_channel(
        &app,
        &browser,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
        "RoomMessagesChannel",
    )
    .await;
    let post = app
        .db()
        .write(move |tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: DAVID,
                    name: Some("Live launch".into()),
                    work_status: Some("planned".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    for target in ["board_posts", "board_column_planned"] {
        let html = frame(&mut socket).await;
        assert!(
            html.contains(&format!("action=\"prepend\" target=\"{target}\"")),
            "{html}"
        );
        assert!(
            html.contains("Live launch") && html.contains("data-board-row"),
            "{html}"
        );
        assert!(
            !html.contains("GLOBAL") && !html.contains("csrf-token"),
            "background rows must carry no request secrets"
        );
    }
    let id = post.id;
    app.db()
        .write(move |tx| ThreadTag::create(tx, id, "release"))
        .await
        .unwrap();
    for prefix in ["board_row", "board_column_row"] {
        let html = frame(&mut socket).await;
        assert!(
            html.contains(&format!(
                "action=\"replace\" target=\"{prefix}_channel_thread_{id}\""
            )) && html.contains("data-tags=\"release\""),
            "{html}"
        );
    }
    let rollback = app
        .db()
        .write(move |tx| {
            ChannelThread::find(tx.conn(), id)?.update_settings(tx, Some("Rolled back"), None)?;
            Err::<(), _>(campfire_db::Error::Other("rollback".into()))
        })
        .await;
    assert!(rollback.is_err());
    socket.assert_silent().await;
    app.db()
        .write(move |tx| ChannelThread::find(tx.conn(), id)?.destroy(tx))
        .await
        .unwrap();
    for prefix in ["board_row", "board_column_row"] {
        assert_eq!(
            frame(&mut socket).await,
            format!(
                "<turbo-stream action=\"remove\" target=\"{prefix}_channel_thread_{id}\"></turbo-stream>"
            )
        );
    }
    socket.assert_silent().await;
}

//! #201: real SQLite bind-limit boundaries and every shared user-payload caller.
use super::*;
use crate::controllers::presenters::Presenter;
use campfire_db::Message;
use std::io::Read;

#[tokio::test]
async fn pr201_unbounded_work_json_matches_all_32767_rails_rows_byte_for_byte() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let golden: Value =
        serde_json::from_str(include_str!("../../../../../vectors/human_work_http.json")).unwrap();
    setup(&app, &golden["rows"][0]).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("UPDATE channel_threads SET work_status=NULL; DELETE FROM work_thread_links; DELETE FROM work_thread_events; WITH RECURSIVE seq(x) AS (SELECT 0 UNION ALL SELECT x+1 FROM seq WHERE x+1<32767) INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) SELECT 1000+x,486777696,127326141,'Large work '||x,'planned',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM seq")?;
        let max:String=tx.conn().query_row("SELECT compile_options FROM pragma_compile_options WHERE compile_options LIKE 'MAX_VARIABLE_NUMBER=%'",[],|r|r.get(0))?;
        assert_eq!(max,"MAX_VARIABLE_NUMBER=32766");Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    browser.authenticity_token().await;
    let response = browser.get("/work.json?state=all").await;
    assert_eq!(response.status, 200);
    let mut expected = String::new();
    flate2::read::GzDecoder::new(&include_bytes!("../../../../../vectors/work_large.json.gz")[..])
        .read_to_string(&mut expected)
        .unwrap();
    let actual = response.text();
    assert!(
        actual == expected,
        "complete large JSON differs: Rust {} bytes; Rails {} bytes",
        actual.len(),
        expected.len()
    );
    assert_eq!(response.json()["threads"].as_array().unwrap().len(), 32767);
    println!(
        "WS12 PR201 large work JSON: 32767 threads; HTTP 200; {} bytes; byte-identical Rails response; 0 masks",
        actual.len()
    );
}

#[tokio::test]
async fn pr201_work_batches_respect_lowered_sqlite_limit_for_threads_rooms_users_and_agents() {
    let golden: Value =
        serde_json::from_str(include_str!("../../../../../vectors/work_read_growth.json")).unwrap();
    let mut failed = Vec::new();
    for row in golden["json"].as_array().unwrap() {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        setup(&app, row).await;
        let mut browser = app.david();
        browser.authenticity_token().await;
        let previous = app.db().set_reader_parameter_limit(64);
        assert!(previous.iter().all(|limit| *limit == 32766));
        let response = browser.get(row["path"].as_str().unwrap()).await;
        let name = format!("{} rows={}", row["kind"], row["size"]);
        println!(
            "WS12 PR201 SQLite limit=64: {name}; HTTP {}",
            response.status
        );
        if response.status != 200 {
            failed.push(name);
        } else {
            assert_eq!(
                response.text(),
                row["body"].as_str().unwrap(),
                "lowered-limit bytes: {name}"
            );
        }
    }
    assert!(
        failed.is_empty(),
        "real bind-variable limit broke batches: {failed:?}"
    );
}

#[tokio::test]
async fn pr201_shared_brand_icon_json_callers_match_complete_rails_outputs() {
    let golden: Value =
        serde_json::from_str(include_str!("../../../../../vectors/pr201_icons.json")).unwrap();
    let mut failed = Vec::new();
    for row in golden["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        setup(&app, row).await;
        let name = format!(
            "{} {}",
            row["kind"],
            row["path"].as_str().unwrap_or("agent-adapter")
        );
        if row["helper"] == "agent-adapter" {
            let copy = app.booted.app.clone();
            let actual = app
                .db()
                .read(move |conn| {
                    let mut p = Presenter::new(conn, &copy, Some("campfire.test".into()));
                    p.current_user_id = Some(DAVID);
                    p.cache_base_url = Some("http://campfire.test".into());
                    let message = Message::find(conn, 970000004)?;
                    Ok((
                        p.agent_message_payload(&message)?,
                        p.agent_message_payloads(std::slice::from_ref(&message))?,
                        crate::controllers::presenters::message_payload::user(
                            &p,
                            &p.user(DAVID)?,
                            "http://campfire.test",
                        )?,
                    ))
                })
                .await
                .unwrap();
            if actual.0 != row["payload"]
                || actual.1.as_slice() != std::slice::from_ref(&row["payload"])
                || actual.2 != row["payload"]["creator"]
            {
                failed.push(name);
            }
            continue;
        }
        let mut browser = app.david();
        browser.authenticity_token().await;
        let response = if row["method"] == "POST" {
            browser
                .write(
                    Req::new(Method::POST, row["path"].as_str().unwrap())
                        .header("content-type", "application/json")
                        .header(
                            "x-ws8bm-forward-client-ids",
                            row["client_id"].as_str().unwrap(),
                        )
                        .body(row["input"].to_string()),
                )
                .await
        } else {
            browser.get(row["path"].as_str().unwrap()).await
        };
        if response.status.as_u16() != row["status"].as_u64().unwrap() as u16
            || response.text() != row["body"].as_str().unwrap()
        {
            failed.push(name);
        }
    }
    println!(
        "WS12 PR201 icon callers: {} complete Rails responses/helper payloads; {} differences; 0 masks",
        golden["rows"].as_array().unwrap().len(),
        failed.len()
    );
    assert!(
        failed.is_empty(),
        "shared icon payload differences: {failed:?}"
    );
}

#[tokio::test]
async fn pr201_each_id_loader_obeys_the_same_sqlite_limit_at_two_sizes() {
    let golden: Value =
        serde_json::from_str(include_str!("../../../../../vectors/work_read_growth.json")).unwrap();
    let mut failures = Vec::new();
    for row in golden["json"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == "agents")
    {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        setup(&app, row).await;
        let size = row["size"].as_i64().unwrap();
        let results = app
            .db()
            .read(move |conn| {
                let rooms = (0..size).map(|i| 2_100_000_000 + i).collect::<Vec<_>>();
                let users = (0..size).map(|i| 2_000_000_000 + i).collect::<Vec<_>>();
                let threads = (0..size).map(|i| 1000 + i).collect::<Vec<_>>();
                let old =
                    conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 64)?;
                let results = vec![
                    (
                        "Room::for_ids",
                        campfire_db::Room::for_ids(conn, &rooms).map(|v| v.len()),
                        size as usize,
                    ),
                    (
                        "User::where_ids",
                        campfire_db::User::where_ids(conn, &users).map(|v| v.len()),
                        size as usize,
                    ),
                    (
                        "Agent::for_users",
                        campfire_db::Agent::for_users(conn, &users).map(|v| v.len()),
                        size as usize,
                    ),
                    (
                        "ChannelThread::for_rooms",
                        campfire_db::ChannelThread::for_rooms(conn, &rooms).map(|v| v.len()),
                        size as usize,
                    ),
                    (
                        "board_reply_counts",
                        campfire_db::ChannelThread::board_reply_counts(conn, &threads)
                            .map(|v| v.len()),
                        0,
                    ),
                    (
                        "board_link_counts",
                        campfire_db::ChannelThread::board_link_counts(conn, &threads)
                            .map(|v| v.len()),
                        0,
                    ),
                    (
                        "ThreadTag::for_threads",
                        campfire_db::ThreadTag::for_threads(conn, &threads).map(|v| v.len()),
                        0,
                    ),
                ];
                conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, old)?;
                Ok(results
                    .into_iter()
                    .map(|(name, result, expected)| {
                        (name, result.map_err(|e| e.to_string()), expected)
                    })
                    .collect::<Vec<_>>())
            })
            .await
            .unwrap();
        for (name, result, expected) in results {
            println!("WS12 PR201 loader limit=64: {name} rows={size}: {result:?}");
            if result != Ok(expected) {
                failures.push(format!("{name} rows={size}: {result:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "unbounded ID loaders: {failures:?}");
}

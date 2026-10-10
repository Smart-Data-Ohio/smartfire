//! BoardSlaNudge reader/inbox contracts, with no response masks.
use super::*;
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/board_sla_nudge.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn board_nudge_inbox_matches_rails_complete_responses_and_permissions() {
    let mut differences = Vec::new();
    for case in oracle()["http"].as_array().unwrap().iter().filter(|case| case["accept"] == "application/json") {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let sql = case["setup"].as_array().unwrap().clone();
        app.db()
            .write(move |tx| {
                for statement in sql {
                    tx.conn().execute_batch(statement.as_str().unwrap())?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let response = app
            .david()
            .send(
                Req::new(Method::GET, case["path"].as_str().unwrap())
                    .header("accept", case["accept"].as_str().unwrap())
                    .header("turbo-frame", "activity_test"),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        if response.text() != case["body"].as_str().unwrap() {
            differences.push(case["name"].clone());
        }

    }
    assert!(
        differences.is_empty(),
        "unmasked Rails response differences: {differences:?}"
    );
}

#[tokio::test]
async fn board_nudge_inbox_batches_distinct_sources_threads_and_boards() {
    let mut counts = Vec::new();
    for size in [10_i64, 100] {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM activity_items")?;
            for i in 0..size {
                let room = 2900000000+i;
                let thread = 2910000000+i;
                let source = 2920000000+i;
                tx.conn().execute("INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(?,127326141,?,'Rooms::Board',?,?)", rusqlite::params![room,format!("Nudge board {i}"),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,127326141,'everything',?,?)", rusqlite::params![room,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(?,?,127326141,?,'planned',?,?,?)", rusqlite::params![thread,room,format!("Nudge post {i}"),tx.now(),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO board_sla_nudges(id,room_id,channel_thread_id,recipient_id,work_status,stage,status_entered_at,created_at,updated_at) VALUES(?,?,?,127326141,'planned','nudge','2026-03-02 14:00:00',?,?)", rusqlite::params![source,room,thread,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO activity_items(user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(127326141,'BoardSlaNudge',?,'work_sla',?,?)", rusqlite::params![source,tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let mut browser = app.david();
        browser.authenticity_token().await;
        let log = app.db().capture_read_queries();
        let response = browser
            .send(
                Req::new(Method::GET, "/activity?type=threads")
                    .header("accept", "application/json")
                    .header("turbo-frame", "activity_test"),
            )
            .await;
        app.db().stop_capturing_read_queries();
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        assert_eq!(response.json()["activity_items"].as_array().unwrap().len(), size as usize);
        let reads = log.lock().unwrap().len();
        println!("WS12 BoardSlaNudge inbox: {size} distinct boards/sources; {reads} reader SQL");
        counts.push(reads);
    }
    assert_eq!(
        counts[0], counts[1],
        "nudge facts and associations must preload once per page"
    );
}

fn claim_input() -> campfire_db::NewBoardSlaNudge {
    campfire_db::NewBoardSlaNudge {
        room_id: 486777696,
        channel_thread_id: 970000001,
        recipient_id: 127326141,
        work_status: Some("in_progress".into()),
        stage: Some("nudge".into()),
        status_entered_at: campfire_db::Timestamp::parse_db("2026-03-02 14:00:00"),
    }
}
async fn claim_fixture() -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let sql = oracle()["setup"].as_array().unwrap().clone();
    app.db()
        .write(move |tx| {
            for statement in sql {
                tx.conn().execute_batch(statement.as_str().unwrap())?;
            }
            Ok(())
        })
        .await
        .unwrap();
    app
}
async fn assert_claim_counts(app: &TestApp, claims: i64, items: i64, jobs: i64) {
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM board_sla_nudges WHERE channel_thread_id=970000001",[],|r|r.get::<_,i64>(0))?,claims);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='BoardSlaNudge'",[],|r|r.get::<_,i64>(0))?,items);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='BoardAutomations::NudgePushJob'",[],|r|r.get::<_,i64>(0))?,jobs);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn board_nudge_claim_commits_source_inbox_and_typed_job_together() {
    let app = claim_fixture().await;
    let nudge = app
        .db()
        .write(|tx| campfire_db::BoardSlaNudge::claim_and_notify(tx, claim_input(), true))
        .await
        .unwrap();
    assert_claim_counts(&app, 1, 1, 1).await;
    app.db().read(move |conn| {
        let arguments: serde_json::Value = conn.query_row("SELECT arguments FROM background_jobs WHERE job_class='BoardAutomations::NudgePushJob'", [], |row|row.get(0))?;
        assert_eq!(arguments, serde_json::json!({"nudge_id":nudge.id}));
        Ok(())
    }).await.unwrap();
    app.db()
        .write(|tx| {
            let mut input = claim_input();
            input.stage = Some("escalation".into());
            campfire_db::BoardSlaNudge::claim_and_notify(tx, input, false)
        })
        .await
        .unwrap();
    assert_claim_counts(&app, 2, 2, 1).await;
}
#[tokio::test]
async fn board_nudge_claim_rolls_back_on_queue_source_or_inbox_failure() {
    for table in ["background_jobs", "board_sla_nudges", "activity_items"] {
        let app = claim_fixture().await;
        app.db().write(move |tx| {
            tx.conn().execute_batch(&format!("CREATE TRIGGER reject_nudge BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'rejected nudge transaction'); END"))?;
            Ok(())
        }).await.unwrap();
        assert!(
            app.db()
                .write(|tx| campfire_db::BoardSlaNudge::claim_and_notify(tx, claim_input(), true))
                .await
                .is_err(),
            "{table}"
        );
        assert_claim_counts(&app, 0, 0, 0).await;
    }
}
#[tokio::test]
async fn board_nudge_concurrent_claims_make_one_inbox_item_and_one_job() {
    let app = claim_fixture().await;
    let mut config = campfire_db::Config::new(app.db().path());
    config.prepare = false;
    config.readers = 1;
    let other = campfire_db::Database::open(config, app.db().env().clone()).unwrap();
    let (a, b) = tokio::join!(
        app.db()
            .write(|tx| campfire_db::BoardSlaNudge::claim_and_notify(tx, claim_input(), true)),
        other.write(|tx| campfire_db::BoardSlaNudge::claim_and_notify(tx, claim_input(), true))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let error = a.err().or(b.err()).unwrap();
    assert!(matches!(error, campfire_db::Error::RecordInvalid(_)) || error.is_record_not_unique());
    assert_claim_counts(&app, 1, 1, 1).await;
}

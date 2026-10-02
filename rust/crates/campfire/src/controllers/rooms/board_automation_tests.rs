//! Complete Rails automation settings responses and persisted facts, including denials.
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use serde_json::{Value, json};

#[tokio::test]
async fn board_automation_settings_match_rails_response_bytes_and_audit_facts() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automation_settings.json"
    ))
    .unwrap();
    let env = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/.env.reference"),
    )
    .unwrap();
    let vapid = env
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
        .collect::<Vec<_>>();
    let mut mismatches = Vec::new();
    let mut counts = Vec::new();
    for row in golden["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db()
            .write(move |tx| {
                for sql in setup.as_array().unwrap() {
                    tx.conn().execute_batch(sql.as_str().unwrap())?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let method: Method = row["method"]
            .as_str()
            .unwrap()
            .to_uppercase()
            .parse()
            .unwrap();
        let mut req =
            Req::new(method.clone(), row["path"].as_str().unwrap()).header("user-agent", "Mozilla");
        if method != Method::GET {
            req = req
                .header("content-type", "application/json")
                .body(row["input"].to_string());
        }
        let probe =
            super::query_probe::SqlProbe::start(app.db(), app.booted.app.config.db_readers).await;
        let response = with_fixed_render_secrets(async {
            if method == Method::GET {
                browser.send(req).await
            } else {
                browser.write(req).await
            }
        })
        .await;
        let statements = probe.finish().await;
        let reads = statements
            .iter()
            .filter(|q| {
                q.sql
                    .trim_start()
                    .to_ascii_uppercase()
                    .starts_with("SELECT")
            })
            .count();
        let name = row["name"].as_str().unwrap();
        if name.starts_with("choices-") {
            counts.push((reads, row["reads"].as_u64().unwrap() as usize));
            println!(
                "WS12 settings {name}: Rust {reads} SELECTs; Rails {}",
                row["reads"]
            );
        }
        if response.status.as_u16() != row["status"].as_u64().unwrap() as u16
            || response.text() != row["body"].as_str().unwrap()
        {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../.scratch/board-automations-2/diffs");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(format!("{name}-actual.html")), response.text()).unwrap();
            std::fs::write(
                dir.join(format!("{name}-expected.html")),
                row["body"].as_str().unwrap(),
            )
            .unwrap();
            mismatches.push(format!("{name}: {} vs {}", response.status, row["status"]));
        }
        assert_eq!(response.location(), row["location"].as_str(), "{name}");
        assert_eq!(
            response.header("content-type"),
            row["content_type"].as_str(),
            "{name}"
        );
        let facts=app.db().read(|conn| {
            let rules=query_all(conn,"SELECT work_status,nudge_after_minutes,escalate_after_minutes FROM board_sla_rules WHERE room_id=699448332 ORDER BY work_status",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?])))?;
            let tags=query_all(conn,"SELECT tag,assignee_id,created_by_id FROM board_tag_assignments WHERE room_id=699448332 ORDER BY tag",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?])))?;
            let audits=query_all(conn,"SELECT actor_id,target_type,target_id,details FROM audit_logs WHERE action='board.automation.change' ORDER BY id",[],|r| {let details:String=r.get(3)?;Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,serde_json::from_str::<Value>(&details).unwrap()]))})?;
            Ok(json!({"rules":rules,"tags":tags,"audits":audits}))
        }).await.unwrap();
        assert_eq!(facts, row["facts"], "{name}");
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(counts.len(), 2);
    assert!(
        counts[1].0.saturating_sub(counts[0].0) <= counts[1].1.saturating_sub(counts[0].1),
        "settings read growth: {counts:?}"
    );
    println!(
        "WS12 settings: {} full Rails responses and rule/tag/audit facts; 0 masks",
        golden["rows"].as_array().unwrap().len()
    );
}

fn query_all<F>(
    conn: &campfire_db::Connection,
    sql: &str,
    _params: [(); 0],
    map: F,
) -> campfire_db::Result<Vec<Value>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
{
    Ok(conn
        .prepare(sql)?
        .query_map([], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

#[test]
fn board_automation_scheduler_registers_both_rails_cadences() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automations.json"
    ))
    .unwrap();
    let periodic = crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
        reminders: std::time::Duration::from_secs(30),
        retention: std::time::Duration::from_secs(86400),
    });
    let tasks: Vec<_> = periodic
        .tasks()
        .filter(|task| task.name().starts_with("board "))
        .map(|task| json!({"name":task.name(),"seconds":task.interval().as_secs()}))
        .collect();
    assert_eq!(json!(tasks), golden["cadence"]);
}

#[tokio::test]
async fn board_automation_recurring_tasks_use_frozen_clock_for_intervals_and_sources() {
    use campfire_kit::clock::Clock;
    use std::sync::Arc;
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let test = TestApp::boot_without_periodic_with_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let setup: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automations.json"
    ))
    .unwrap();
    let setup = setup["sla"][0]["setup"].clone();
    test.db().write(move |tx| {for sql in setup.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;}tx.conn().execute("UPDATE channel_threads SET work_status_changed_at='2026-03-02 15:00:00' WHERE id=970000001",[])?;Ok(())}).await.unwrap();
    let app = test.booted.app.clone();
    let mut loop_ = campfire_jobs::periodic::Periodic::new("WS12 boundary test");
    for task in crate::jobs::periodic::board_automation_tasks() {
        loop_.task(task);
    }
    let tick = |clock: &campfire_kit::FrozenClock| campfire_db::Timestamp::from_jiff(clock.now());
    assert_eq!(
        loop_.tick(app.clone(), tick(&clock)).await,
        vec!["board sla nudges", "board stale digests"]
    );
    test.db()
        .read(|conn| {
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM board_sla_nudges", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM board_stale_digests", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
    clock.advance(jiff::SignedDuration::from_secs(299));
    assert!(loop_.tick(app.clone(), tick(&clock)).await.is_empty());
    clock.advance(jiff::SignedDuration::from_secs(1));
    assert_eq!(
        loop_.tick(app.clone(), tick(&clock)).await,
        vec!["board sla nudges"]
    );
    clock.advance(jiff::SignedDuration::from_secs(3299));
    assert_eq!(
        loop_.tick(app.clone(), tick(&clock)).await,
        vec!["board sla nudges"]
    );
    clock.advance(jiff::SignedDuration::from_secs(1));
    assert_eq!(
        loop_.tick(app.clone(), tick(&clock)).await,
        vec!["board stale digests"]
    );
    clock.set("2026-03-03T00:00:00Z".parse().unwrap());
    assert_eq!(
        loop_.tick(app, tick(&clock)).await,
        vec!["board sla nudges", "board stale digests"]
    );
    test.db().read(|conn| {assert_eq!(conn.query_row("SELECT COUNT(*) FROM board_sla_nudges",[],|r|r.get::<_,i64>(0))?,2);assert_eq!(conn.query_row("SELECT COUNT(*) FROM board_stale_digests",[],|r|r.get::<_,i64>(0))?,2);assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='BoardAutomations::NudgePushJob'",[],|r|r.get::<_,i64>(0))?,2);Ok(())}).await.unwrap();
}

#[tokio::test]
async fn board_automation_dispatcher_jobs_are_atomic_and_one_failure_does_not_stop_the_sweep() {
    for table in ["background_jobs", "activity_items", "board_sla_nudges"] {
        let test = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let golden: Value = serde_json::from_str(include_str!(
            "../../../../../vectors/board_automations.json"
        ))
        .unwrap();
        let setup = golden["sla"][0]["setup"].clone();
        test.db().write(move |tx| {
            for sql in setup.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;}
            tx.conn().execute("UPDATE channel_threads SET work_status_changed_at='2026-03-02 12:00:00' WHERE id=970000001",[])?;
            tx.conn().execute_batch("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,work_status_changed_at,created_at,updated_at,last_activity_at) SELECT 970000002,room_id,creator_id,'Successful post',work_status,work_owner_id,work_status_changed_at,created_at,updated_at,last_activity_at FROM channel_threads WHERE id=970000001")?;
            let condition=match table {"background_jobs"=>"NEW.job_class='BoardAutomations::NudgePushJob' AND (SELECT channel_thread_id FROM board_sla_nudges WHERE id=json_extract(NEW.arguments,'$.nudge_id'))=970000001","activity_items"=>"NEW.source_type='BoardSlaNudge' AND (SELECT channel_thread_id FROM board_sla_nudges WHERE id=NEW.source_id)=970000001",_=>"NEW.channel_thread_id=970000001"};
            tx.conn().execute_batch(&format!("CREATE TRIGGER reject_dispatch BEFORE INSERT ON {table} WHEN {condition} BEGIN SELECT RAISE(ABORT,'reject dispatcher transaction'); END"))?;Ok(())
        }).await.unwrap();
        let stats =
            campfire_db::models::board_automations::dispatch_sla(test.db(), test.db().env().now())
                .await
                .unwrap();
        assert_eq!(stats.failed_ids, vec![970000001], "{table}");
        assert_eq!(stats.claims, 2);
        assert_eq!(stats.pushes, 2);
        test.db().read(|conn| {
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM board_sla_nudges WHERE channel_thread_id=970000001",[],|r|r.get::<_,i64>(0))?,0);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='BoardSlaNudge'",[],|r|r.get::<_,i64>(0))?,2);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='BoardAutomations::NudgePushJob'",[],|r|r.get::<_,i64>(0))?,2);Ok(())
        }).await.unwrap();
    }
}

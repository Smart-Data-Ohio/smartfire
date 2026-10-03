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
    check_settings(golden).await;
}

#[tokio::test]
async fn review_pr206_settings() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automation_review_settings.json"
    ))
    .unwrap();
    check_settings(golden).await;
}

async fn check_settings(golden: Value) {
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
    let mut approved_differences = 0;
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
        let approved_bad_request = row["status"] == 500
            && (row["input"]["sla_rules"].is_array()
                || row["input"]["sla_rules"]
                    .as_object()
                    .is_some_and(|o| o.values().any(Value::is_array)));
        let expected_status = if approved_bad_request {
            approved_differences += 1;
            400
        } else {
            row["status"].as_u64().unwrap() as u16
        };
        let expected_body = if approved_bad_request {
            ""
        } else {
            row["body"].as_str().unwrap()
        };
        if response.status.as_u16() != expected_status || response.text() != expected_body {
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
        if response.location() != row["location"].as_str()
            || response.header("content-type")
                != (if approved_bad_request {
                    Some("text/html; charset=UTF-8")
                } else {
                    row["content_type"].as_str()
                })
        {
            mismatches.push(format!("{name}: response headers"));
        }
        let facts=app.db().read(|conn| {
            let rules=query_all(conn,"SELECT work_status,nudge_after_minutes,escalate_after_minutes FROM board_sla_rules WHERE room_id=699448332 ORDER BY work_status",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?])))?;
            let tags=query_all(conn,"SELECT tag,assignee_id,created_by_id FROM board_tag_assignments WHERE room_id=699448332 ORDER BY tag",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?])))?;
            let audits=query_all(conn,"SELECT actor_id,target_type,target_id,details FROM audit_logs WHERE action='board.automation.change' ORDER BY id",[],|r| {let details:String=r.get(3)?;Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,serde_json::from_str::<Value>(&details).unwrap()]))})?;
            Ok(json!({"rules":rules,"tags":tags,"audits":audits}))
        }).await.unwrap();
        if facts != row["facts"] {
            mismatches.push(format!(
                "{name}: persisted facts Rust={facts}; Rails={}",
                row["facts"]
            ));
        }
        println!(
            "PR206 settings {name}: Rust {}; Rails {}; facts_equal={}",
            response.status.as_u16(),
            row["status"],
            facts == row["facts"]
        );
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(counts.len(), 2);
    assert!(
        counts[1].0.saturating_sub(counts[0].0) <= counts[1].1.saturating_sub(counts[0].1),
        "settings read growth: {counts:?}"
    );
    println!(
        "WS12 settings: {} complete responses and rule/tag/audit facts; {approved_differences} explicit approved malformed-array 400 responses",
        golden["rows"].as_array().unwrap().len(),
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

#[tokio::test]
async fn review_pr206_dispatch_with_production_richtext_jobs_and_bind_limit() {
    use campfire_db::Message;
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automation_review_dispatch.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    let mut measurements = Vec::new();
    let mut runs = 0;
    for row in golden["cases"].as_array().unwrap() {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db()
            .write(move |tx| {
                for s in setup.as_array().unwrap() {
                    tx.conn().execute_batch(s.as_str().unwrap())?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let before = app.db().read(Message::count).await.unwrap();
        let kind = row["kind"].as_str().unwrap();
        let name = row["name"].as_str().unwrap();
        let limited = row["count_size"] == 100 || row["bind_limit"] == 64;
        if limited {
            let limits = app.db().set_reader_parameter_limit(64);
            assert!(!limits.is_empty());
            app.db()
                .write(|tx| {
                    tx.conn()
                        .set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 64)?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        for (index, run) in row["runs"].as_array().unwrap().iter().enumerate() {
            let extra = run["setup"].clone();
            app.db()
                .write(move |tx| {
                    for s in extra.as_array().unwrap() {
                        tx.conn().execute_batch(s.as_str().unwrap())?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
            let now = campfire_db::Timestamp::parse_db(run["now"].as_str().unwrap()).unwrap();
            let probe =
                super::query_probe::SqlProbe::start(app.db(), app.booted.app.config.db_readers)
                    .await;
            let stats = if kind == "sla" {
                campfire_db::models::board_automations::dispatch_sla(app.db(), now).await
            } else {
                campfire_db::models::board_automations::dispatch_digests(app.db(), now).await
            }
            .unwrap();
            let queries = probe.finish().await;
            let reads = queries
                .iter()
                .filter(|q| {
                    q.sql
                        .trim_start()
                        .to_ascii_uppercase()
                        .starts_with("SELECT")
                })
                .count();
            let expected_failed = if name.starts_with("committed-") && index == 0 {
                vec![971000001]
            } else {
                vec![]
            };
            assert_eq!(stats.failed_ids, expected_failed, "{name} run{index}");
            let is_sla = kind == "sla";
            let ids = row["rooms"].to_string();
            let richtext = app.db().env().rich_text.clone();
            let actual=app.db().read(move|c|{
    if is_sla {
     let claims=query_all(c,"SELECT room_id,channel_thread_id,work_status,stage,recipient_id,status_entered_at FROM board_sla_nudges ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,campfire_db::Timestamp>(5)?.jiff().strftime("%Y-%m-%d %H:%M:%S.%6f").to_string()])))?;
     let items=query_all(c,"SELECT a.user_id,a.event_type,a.source_type,n.channel_thread_id,a.read_at,a.handled_at FROM activity_items a JOIN board_sla_nudges n ON n.id=a.source_id WHERE a.source_type='BoardSlaNudge' ORDER BY a.id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?])))?;
     let jobs=query_all(c,"SELECT j.job_class,n.recipient_id,n.channel_thread_id,n.stage FROM background_jobs j JOIN board_sla_nudges n ON n.id=json_extract(j.arguments,'$.nudge_id') WHERE j.job_class='BoardAutomations::NudgePushJob' ORDER BY j.id",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?])))?;
     Ok(json!({"claims":claims,"items":items,"jobs":jobs}))
    } else {
     let claims=query_all(c,"SELECT room_id,digest_on,message_id FROM board_stale_digests ORDER BY id",[],|r|Ok(json!({"room_id":r.get::<_,i64>(0)?,"on":r.get::<_,String>(1)?,"attached":r.get::<_,Option<i64>>(2)?.is_some()})))?;
     let message_ids=c.prepare("SELECT id FROM messages WHERE system_note=1 AND room_id IN (SELECT value FROM json_each(?)) ORDER BY id")?.query_map([&ids],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;let messages=message_ids.into_iter().map(|id|Message::find(c,id)).collect::<campfire_db::Result<Vec<_>>>()?;
     let mut notes=Vec::new();for m in messages {notes.push(json!({"room_id":m.room_id,"creator_id":m.creator_id,"body":m.body_html(c)?.unwrap(),"plain":m.plain_text_body(c,richtext.as_ref())?,"system_note":m.system_note,"thread_id":m.thread_id,"streaming":m.streaming}));}
     let items:i64=c.query_row("SELECT COUNT(*) FROM activity_items",[],|r|r.get(0))?;let unread:i64=c.query_row("SELECT COUNT(*) FROM memberships WHERE unread_at IS NOT NULL AND room_id IN (SELECT value FROM json_each(?))",[&ids],|r|r.get(0))?;
     let jobs=c.prepare("SELECT job_class FROM background_jobs ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
     Ok(json!({"claims":claims,"notes":notes,"message_delta":Message::count(c)?-before,"items":items,"unread":unread,"jobs":jobs}))
    }
   }).await.unwrap();
            if !is_sla {
                assert_digest_broadcast_bytes(&app, row["rooms"].to_string()).await;
                if row["bind_limit"] == 64 {
                    let quotes = app.db().read(|conn| Ok(conn.query_row(
                        "SELECT COUNT(DISTINCT referenced_message_id) FROM message_references r JOIN messages m ON m.id=r.message_id WHERE m.system_note=1", [], |row| row.get::<_,i64>(0))?)).await.unwrap();
                    assert_eq!(
                        quotes, 64,
                        "the batch loads 32 notes plus 64 quoted sources"
                    );
                }
            }
            let mut fields = Vec::new();
            for (k, expected) in run["facts"].as_object().unwrap() {
                if &actual[k] != expected {
                    fields.push(k.to_owned());
                    differences.push(json!({"case":name,"run":index,"field":k,"actual":actual[k],"expected":expected}));
                }
            }
            if let Some(size) = row["count_size"].as_i64() {
                measurements.push((
                    kind.to_owned(),
                    size,
                    index,
                    reads,
                    run["reads"].as_u64().unwrap() as usize,
                ));
            }
            println!(
                "APP_DISPATCH {name} run={index}: fields_differ={fields:?}; Rust_reads={reads}; Rails_reads={}; bound={}",
                run["reads"],
                if limited { 64 } else { 32766 }
            );
            runs += 1;
        }
    }
    let fact_differences = differences.len();
    for kind in ["sla", "digest"] {
        for run in 0..2 {
            let a = measurements
                .iter()
                .find(|(k, s, r, _, _)| k == kind && *s == 10 && *r == run)
                .unwrap();
            let b = measurements
                .iter()
                .find(|(k, s, r, _, _)| k == kind && *s == 100 && *r == run)
                .unwrap();
            if b.3 - a.3 > b.4 - a.4 {
                differences.push(json!({"field":"read-growth","kind":kind,"run":run,"rust":b.3-a.3,"rails":b.4-a.4}));
            }
        }
    }

    println!(
        "PR206 independent full-app dispatch: {runs} runs; persisted/HTML/job differences={fact_differences}; total differences={}; 100-board sweeps with SQLite bind limit=64",
        differences.len()
    );
    assert!(
        differences.is_empty(),
        "full-app dispatch differences {differences:?}"
    );
}

async fn assert_digest_broadcast_bytes(app: &TestApp, rooms: String) {
    use crate::controllers::presenters::{Presenter, page};
    use askama::Template;
    use campfire_db::{Message, models::board_automations::DigestNotes};
    let copy = app.booted.app.clone();
    let (ids, expected) = app.db().read(move |conn| {
        let ids = conn.prepare("SELECT id FROM messages WHERE system_note=1 AND room_id IN (SELECT value FROM json_each(?)) ORDER BY id")?
            .query_map([rooms], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut expected = Vec::new();
        for id in &ids {
            let message = Message::find(conn, *id)?;
            let view = Presenter::new(conn, &copy, None).message(&message)?;
            let html = page::render_detached_at(&copy, None, page::default_renderer_base_url(&copy), |ctx| {
                campfire_views::messages::MessagePartial { ctx, message: &view }.render().unwrap()
            });
            expected.push((message.room_id, html));
        }
        Ok((ids, expected))
    }).await.unwrap();
    let actual =
        crate::channels::board_digests::render(&app.booted.app, &DigestNotes { message_ids: ids })
            .unwrap()
            .into_iter()
            .map(|(room, html)| (room.id, html))
            .collect::<Vec<_>>();
    assert_eq!(
        actual, expected,
        "batched digest broadcasts preserve shared renderer bytes and order"
    );
}

//! Exercise WS12's model services against the app's real durable EventSink.
use crate::controllers::presenters::test_support::*;
use campfire_db::models::{agent_work, audit_log};
use campfire_db::{
    Agent, AgentGrant, ChannelThread, HandoffPackage, NewAgent, NewChannelThread, NewGrant, User,
};
use serde_json::{Value, json};

const BOARD: i64 = 699448332;
const BENDER: i64 = 394959859;
const BENDER_AGENT: i64 = 773018776;

#[tokio::test]
async fn tag_auto_assignment_matches_rails_committed_http_response_bytes() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_tag_assignment_http.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen()
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let setup = row["setup"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        app.db()
            .write(move |tx| {
                for statement in setup {
                    tx.conn().execute_batch(&statement)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.sign_in(DAVID).await;
        let response = browser
            .write(
                Req::new(
                    row["method"]
                        .as_str()
                        .unwrap()
                        .to_ascii_uppercase()
                        .parse::<axum::http::Method>()
                        .unwrap(),
                    row["path"].as_str().unwrap(),
                )
                .header("content-type", "application/json")
                .header("user-agent", "Mozilla")
                .body(row["input"].to_string()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}",
            row["name"]
        );
        assert_eq!(
            response.text(),
            row["body"].as_str().unwrap(),
            "{}",
            row["name"]
        );
    }
}

async fn setup(app: &TestApp) -> (i64, i64) {
    app.db()
        .write(|tx| {
            let receiver =
                User::create_bot(tx, "Receiver", Some("https://receiver.example.test/hook"))?;
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: receiver.id,
                    owner_id: Some(DAVID),
                    kind: campfire_db::AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            campfire_db::Room::find(tx.conn(), BOARD)?.grant_to(tx, &[BENDER, receiver.id])?;
            tx.conn()
                .execute("DELETE FROM agent_grants WHERE agent_id=?", [BENDER_AGENT])?;
            for who in [BENDER_AGENT, agent.id] {
                for cap in ["read_messages", "post_messages", "manage_threads"] {
                    AgentGrant::create(
                        tx,
                        NewGrant {
                            agent_id: who,
                            room_id: Some(BOARD),
                            granted_by_id: DAVID,
                            capability: cap.into(),
                            ..Default::default()
                        },
                    )?;
                }
            }
            let thread = ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: DAVID,
                    name: Some("Handoff".into()),
                    work_status: Some("planned".into()),
                    work_owner_id: Some(BENDER),
                    ..Default::default()
                },
                None,
            )?;
            Ok((thread.id, agent.id))
        })
        .await
        .unwrap()
}
async fn counts(app: &TestApp, thread_id: i64) -> Value {
    app.db().read(move |conn| {
        let count=|table:&str,predicate:&str|conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"),[thread_id],|r|r.get::<_,i64>(0));
        Ok(json!({"owner":ChannelThread::find(conn,thread_id)?.work_owner_id,
            "handoffs":count("work_handoffs","channel_thread_id=?")?,"history":count("work_thread_events","channel_thread_id=?")?,
            "audit":count("audit_logs","action='work.handoff' AND target_id=?")?,"ledger":count("agent_events","json_extract(metadata,'$.thread_id')=?")?,
            "jobs":count("background_jobs","job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id') IN (SELECT id FROM agent_events WHERE json_extract(metadata,'$.thread_id')=?)")?}))
    }).await.unwrap()
}

#[tokio::test]
async fn handoff_package_owner_history_ledger_audit_and_webhook_jobs_are_atomic() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let (thread, receiver) = setup(&app).await;
    let before = counts(&app, thread).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_handoff_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' AND EXISTS(SELECT 1 FROM agent_events WHERE id=json_extract(NEW.arguments,'$.event_id') AND event_type='work_handed_off') BEGIN SELECT RAISE(ABORT,'rejected handoff job'); END")?;Ok(())
    }).await.unwrap();
    let failed = app
        .db()
        .write(move |tx| {
            agent_work::handoff_work(
                tx,
                &Agent::find(tx.conn(), BENDER_AGENT)?.unwrap(),
                thread,
                receiver,
                HandoffPackage {
                    summary: "Halfway".into(),
                    links: json!(["https://example.test/a"]),
                    open_questions: json!(["Why?"]),
                },
                &audit_log::Context::default(),
            )
        })
        .await;
    assert!(failed.is_err());
    assert_eq!(counts(&app, thread).await, before);
    app.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_handoff_job")?;
            Ok(())
        })
        .await
        .unwrap();
    let success = app
        .db()
        .write(move |tx| {
            agent_work::handoff_work(
                tx,
                &Agent::find(tx.conn(), BENDER_AGENT)?.unwrap(),
                thread,
                receiver,
                HandoffPackage {
                    summary: "Halfway".into(),
                    ..Default::default()
                },
                &audit_log::Context::default(),
            )
        })
        .await
        .unwrap();
    assert!(matches!(
        success,
        agent_work::Outcome::Success { status: 201, .. }
    ));
    let after = counts(&app, thread).await;
    for (key, delta) in [
        ("handoffs", 1),
        ("history", 1),
        ("audit", 1),
        ("ledger", 2),
        ("jobs", 2),
    ] {
        assert_eq!(
            after[key].as_i64().unwrap(),
            before[key].as_i64().unwrap() + delta,
            "{key}"
        );
    }
    assert_ne!(after["owner"], before["owner"]);
}

#[tokio::test]
async fn failed_auto_assignment_job_keeps_the_post_and_tags_but_rolls_back_assignment() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let (_, receiver) = setup(&app).await;
    app.db().write(move |tx| {
        let agent=Agent::find(tx.conn(),receiver)?.unwrap();
        campfire_db::BoardTagAssignment::create(tx,campfire_db::NewBoardTagAssignment {room_id:BOARD,tag:"auto".into(),assignee_id:agent.user_id,created_by_id:DAVID})?;
        tx.conn().execute_batch("CREATE TRIGGER reject_auto_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN SELECT RAISE(ABORT,'rejected auto job'); END")?;Ok(())
    }).await.unwrap();
    let post = app
        .db()
        .write(|tx| {
            ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: DAVID,
                    name: Some("Auto".into()),
                    work_status: Some("planned".into()),
                    tag_names: Some(vec!["auto".into()]),
                    ..Default::default()
                },
                None,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        counts(&app, post.id).await,
        json!({"owner":null,"handoffs":0,"history":0,"audit":0,"ledger":0,"jobs":0})
    );
    let tags = app
        .db()
        .read(move |conn| ChannelThread::find(conn, post.id)?.tag_names(conn))
        .await
        .unwrap();
    assert_eq!(tags, ["auto"]);
}

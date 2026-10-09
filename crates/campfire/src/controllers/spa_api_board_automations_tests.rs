//! The SPA board-automation API preserves the classic settings page's access, validation,
//! ordering and audit facts.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Reply, Req, TestApp,
};

const BOARD: i64 = 699448332;
const DESIGNERS: i64 = 654632876;
const AGENT: i64 = 773018776;

async fn sql(a: &TestApp, statements: impl Into<String>) {
    let statements = statements.into();
    a.db()
        .write(move |tx| {
            tx.conn().execute_batch(&statements)?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn reset(a: &TestApp) {
    sql(
        a,
        format!(
            "DELETE FROM board_tag_assignments WHERE room_id={BOARD};
             DELETE FROM board_sla_rules WHERE room_id={BOARD};
             DELETE FROM audit_logs WHERE action='board.automation.change';"
        ),
    )
    .await;
}

async fn settings(browser: &mut Browser<'_>, room_id: i64) -> api::BoardAutomations {
    let reply = browser
        .send(get(&format!("/api/v1/rooms/{room_id}/automations")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    parse(&reply)
}

fn validation(reply: &Reply, field: &str, messages: &[&str]) -> String {
    assert_eq!(
        (reply.status, tag(reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { message, fields } = parse::<api::ApiErrorResponse>(reply).error
    else {
        unreachable!()
    };
    let expected = messages
        .iter()
        .map(|message| (*message).to_string())
        .collect::<Vec<_>>();
    assert_eq!(fields.get(field), Some(&expected), "{}", reply.text());
    message
}

async fn audit_details(a: &TestApp) -> Vec<Value> {
    let rows = a
        .db()
        .read(|conn| {
            let mut statement = conn.prepare(
                "SELECT details FROM audit_logs WHERE action='board.automation.change' ORDER BY id",
            )?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .await
        .unwrap();
    rows.into_iter()
        .map(|details| serde_json::from_str(&details).unwrap())
        .collect()
}

async fn sla_rows(a: &TestApp) -> Vec<(String, i64, i64)> {
    a.db()
        .read(|conn| {
            let mut statement = conn.prepare(
                "SELECT work_status,nudge_after_minutes,escalate_after_minutes
                 FROM board_sla_rules WHERE room_id=? ORDER BY work_status",
            )?;
            let rows = statement
                .query_map([BOARD], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .await
        .unwrap()
}

fn timer(nudge: Option<i64>, escalate: Option<i64>) -> Value {
    json!({"nudgeAfterMinutes": nudge, "escalateAfterMinutes": escalate})
}

fn timers(planned: Value, in_progress: Value, blocked: Value) -> Value {
    json!({"planned": planned, "inProgress": in_progress, "blocked": blocked})
}

#[tokio::test]
async fn spa_api_board_automations_access_matches_classic_board_administration() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    sql(
        &a,
        format!(
            "UPDATE users SET role=0 WHERE id IN ({DAVID},{KEVIN});
             UPDATE users SET role=1 WHERE id={JASON};
             INSERT OR IGNORE INTO memberships(room_id,user_id,created_at,updated_at)
             VALUES({BOARD},{KEVIN},'2026-03-02 16:00:00','2026-03-02 16:00:00');"
        ),
    )
    .await;

    let mut david = a.sign_in(DAVID).await;
    assert_eq!(settings(&mut david, BOARD).await.room_id, BOARD);
    let mut jason = a.sign_in(JASON).await;
    assert_eq!(settings(&mut jason, BOARD).await.room_id, BOARD);

    let mut kevin = a.sign_in(KEVIN).await;
    let forbidden = kevin
        .send(get(&format!("/api/v1/rooms/{BOARD}/automations")))
        .await;
    assert_eq!(
        (forbidden.status, tag(&forbidden)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    for (method, suffix, body) in [
        (
            Method::POST,
            "tag_rules",
            json!({"tag":"member","assigneeId":KEVIN}),
        ),
        (Method::DELETE, "tag_rules/999999999", json!({})),
        (
            Method::PUT,
            "sla_timers",
            timers(timer(None, None), timer(None, None), timer(None, None)),
        ),
    ] {
        let reply = kevin
            .write(json_body(
                method,
                &format!("/api/v1/rooms/{BOARD}/automations/{suffix}"),
                &body,
            ))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::FORBIDDEN, "Forbidden".into()),
            "{suffix}: {}",
            reply.text()
        );
    }

    sql(
        &a,
        format!(
            "DELETE FROM memberships WHERE room_id={BOARD} AND user_id={KEVIN};
             UPDATE users SET role=1 WHERE id={KEVIN};"
        ),
    )
    .await;
    let outside = kevin
        .send(get(&format!("/api/v1/rooms/{BOARD}/automations")))
        .await;
    assert_eq!(
        (outside.status, tag(&outside)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
    let non_board = david
        .send(get(&format!("/api/v1/rooms/{DESIGNERS}/automations")))
        .await;
    assert_eq!(
        (non_board.status, tag(&non_board)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );

    crate::controllers::agent_http_tests::initialize(&a).await;
    let agent = a
        .anonymous()
        .send(get(&format!("/api/v1/rooms/{BOARD}/automations")).header(
            "authorization",
            &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET),
        ))
        .await;
    assert_eq!(
        (agent.status, tag(&agent)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    let bot_key = a
        .db()
        .write(|tx| campfire_db::User::find(tx.conn(), BENDER)?.reset_bot_key(tx))
        .await
        .unwrap();
    let bot = a
        .anonymous()
        .send(get(&format!(
            "/api/v1/rooms/{BOARD}/automations?bot_key={bot_key}"
        )))
        .await;
    assert_eq!(
        (bot.status, tag(&bot)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
}

#[tokio::test]
async fn spa_api_board_automations_show_orders_candidates_rules_timers_and_unique_users() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    sql(
        &a,
        format!(
            "UPDATE users SET name='İpek',status=0 WHERE id={DAVID};
             UPDATE users SET name='Zulu',status=0 WHERE id={JASON};
             UPDATE users SET name='alpha',status=0 WHERE id={BENDER};
             UPDATE users SET name='Former member',status=1 WHERE id={KEVIN};
             DELETE FROM memberships WHERE room_id={BOARD} AND user_id={KEVIN};
             INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at)
             VALUES
             ({BOARD},'zeta',{DAVID},{DAVID},'2026-03-02 16:00:01','2026-03-02 16:00:01'),
             ({BOARD},'legacy',{KEVIN},{DAVID},'2026-03-02 16:00:02','2026-03-02 16:00:02');
             INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at)
             VALUES
             ({BOARD},'blocked',30,60,'2026-03-02 16:00:01','2026-03-02 16:00:01'),
             ({BOARD},'planned',10,20,'2026-03-02 16:00:02','2026-03-02 16:00:02'),
             ({BOARD},'in_progress',20,40,'2026-03-02 16:00:03','2026-03-02 16:00:03');"
        ),
    )
    .await;

    let mut david = a.sign_in(DAVID).await;
    let shown = settings(&mut david, BOARD).await;
    assert_eq!(shown.room_id, BOARD);
    assert_eq!(
        shown
            .tag_rules
            .iter()
            .map(|rule| (rule.tag.as_str(), rule.assignee_id))
            .collect::<Vec<_>>(),
        [("legacy", KEVIN), ("zeta", DAVID)]
    );
    assert_eq!(shown.candidates, [BENDER, DAVID, JASON]);
    assert_eq!(
        shown
            .sla_timers
            .iter()
            .map(|rule| {
                (
                    rule.status,
                    rule.nudge_after_minutes,
                    rule.escalate_after_minutes,
                )
            })
            .collect::<Vec<_>>(),
        [
            (api::WorkStatus::Planned, 10, 20),
            (api::WorkStatus::InProgress, 20, 40),
            (api::WorkStatus::Blocked, 30, 60),
        ]
    );
    let mut user_ids = shown.users.iter().map(|user| user.id).collect::<Vec<_>>();
    user_ids.sort_unstable();
    assert_eq!(user_ids, [DAVID, JASON, BENDER, KEVIN]);
    assert_eq!(
        shown
            .users
            .iter()
            .find(|user| user.id == KEVIN)
            .map(|user| user.status),
        Some(api::UserStatus::Deactivated),
        "a departed or inactive rule assignee remains renderable"
    );
    assert!(
        shown
            .users
            .iter()
            .find(|user| user.id == BENDER)
            .unwrap()
            .agent
            .is_some(),
        "the classic picker includes agents"
    );
}

#[tokio::test]
async fn spa_api_board_automations_show_batches_candidates_users_and_bot_icons() {
    let a = app(true)
        .await
        .expect("the frozen default seed")
        .without_job_runner()
        .await;
    reset(&a).await;
    sql(
        &a,
        format!(
            "WITH RECURSIVE sequence(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM sequence WHERE n<10)
             INSERT INTO users(id,name,role,status,icon_name,created_at,updated_at)
             SELECT 990100000+n,printf('Batch user %03d',n),CASE WHEN n%2=0 THEN 2 ELSE 0 END,0,
                    CASE WHEN n%2=0 THEN 'batch-icon' ELSE NULL END,
                    '2026-03-02 16:00:00','2026-03-02 16:00:00' FROM sequence;
             WITH RECURSIVE sequence(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM sequence WHERE n<10)
             INSERT INTO memberships(room_id,user_id,created_at,updated_at)
             SELECT {BOARD},990100000+n,'2026-03-02 16:00:00','2026-03-02 16:00:00' FROM sequence;"
        ),
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/automations");

    let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
        a.db(),
        a.booted.app.config.db_readers,
    )
    .await;
    let small = david.send(get(&path)).await;
    assert_eq!(small.status, StatusCode::OK, "{}", small.text());
    assert_eq!(parse::<api::BoardAutomations>(&small).candidates.len(), 13);
    let small_reads = probe
        .finish()
        .await
        .iter()
        .filter(|query| {
            query
                .sql
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("SELECT")
        })
        .count();

    sql(
        &a,
        format!(
            "WITH RECURSIVE sequence(n) AS (SELECT 11 UNION ALL SELECT n+1 FROM sequence WHERE n<100)
             INSERT INTO users(id,name,role,status,icon_name,created_at,updated_at)
             SELECT 990100000+n,printf('Batch user %03d',n),CASE WHEN n%2=0 THEN 2 ELSE 0 END,0,
                    CASE WHEN n%2=0 THEN 'batch-icon' ELSE NULL END,
                    '2026-03-02 16:00:00','2026-03-02 16:00:00' FROM sequence;
             WITH RECURSIVE sequence(n) AS (SELECT 11 UNION ALL SELECT n+1 FROM sequence WHERE n<100)
             INSERT INTO memberships(room_id,user_id,created_at,updated_at)
             SELECT {BOARD},990100000+n,'2026-03-02 16:00:00','2026-03-02 16:00:00' FROM sequence;"
        ),
    )
    .await;
    let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
        a.db(),
        a.booted.app.config.db_readers,
    )
    .await;
    let large = david.send(get(&path)).await;
    assert_eq!(large.status, StatusCode::OK, "{}", large.text());
    assert_eq!(parse::<api::BoardAutomations>(&large).candidates.len(), 103);
    let large_reads = probe
        .finish()
        .await
        .iter()
        .filter(|query| {
            query
                .sql
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("SELECT")
        })
        .count();
    assert_eq!(
        small_reads, large_reads,
        "automation settings SELECTs must stay flat for 10/100 candidates, including icon-bearing bots"
    );
}

#[tokio::test]
async fn spa_api_board_automations_create_tag_rule_normalizes_validates_and_audits() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/automations/tag_rules");

    let created = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"tag": "  ReLeAsE  ", "assigneeId": JASON}),
        ))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let body: api::BoardAutomations = parse(&created);
    assert_eq!(body.tag_rules.len(), 1);
    assert_eq!(
        (
            body.tag_rules[0].tag.as_str(),
            body.tag_rules[0].assignee_id
        ),
        ("release", JASON)
    );
    assert_eq!(
        audit_details(&a).await,
        [json!({"tag_rule":"created","tag":"release","assignee":"Jason"})]
    );

    for (body, field, messages) in [
        (
            json!({"tag": " RELEASE ", "assigneeId": JASON}),
            "tag",
            vec!["has already been taken"],
        ),
        (
            json!({"tag": "bad_tag", "assigneeId": JASON}),
            "tag",
            vec!["is invalid"],
        ),
        (
            json!({"tag": "   ", "assigneeId": JASON}),
            "tag",
            vec!["can't be blank", "is invalid"],
        ),
        (
            json!({"tag": "a".repeat(31), "assigneeId": JASON}),
            "tag",
            vec!["is too long (maximum is 30 characters)"],
        ),
        (
            json!({"tag": "missing-owner"}),
            "assigneeId",
            vec!["must exist"],
        ),
        (
            json!({"tag": "unknown-owner", "assigneeId": 999999999}),
            "assigneeId",
            vec!["must exist"],
        ),
    ] {
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        validation(&reply, field, &messages);
    }
    let null = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"tag": "null-owner", "assigneeId": null}),
        ))
        .await;
    assert_eq!(
        validation(&null, "assigneeId", &["must exist"]),
        "Assignee must exist"
    );

    sql(
        &a,
        format!(
            "INSERT OR IGNORE INTO memberships(room_id,user_id,created_at,updated_at)
             VALUES({BOARD},{KEVIN},'2026-03-02 16:00:00','2026-03-02 16:00:00');
             UPDATE users SET status=1 WHERE id={KEVIN};"
        ),
    )
    .await;
    let inactive = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"tag": "inactive", "assigneeId": KEVIN}),
        ))
        .await;
    validation(
        &inactive,
        "assigneeId",
        &["must be an active board member able to own posts"],
    );

    sql(
        &a,
        format!(
            "UPDATE users SET status=0 WHERE id={KEVIN};
             DELETE FROM memberships WHERE room_id={BOARD} AND user_id={KEVIN};"
        ),
    )
    .await;
    let nonmember = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"tag": "nonmember", "assigneeId": KEVIN}),
        ))
        .await;
    validation(
        &nonmember,
        "assigneeId",
        &["must be an active board member able to own posts"],
    );

    sql(
        &a,
        format!(
            "DELETE FROM agent_grants WHERE agent_id={AGENT};
             INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,revoked_at,created_at,updated_at)
             VALUES
             ({AGENT},{BOARD},'post_messages',{DAVID},'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00'),
             ({AGENT},{BOARD},'read_messages',{DAVID},NULL,'2026-03-02 16:00:00','2026-03-02 16:00:00');"
        ),
    )
    .await;
    let agent = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"tag": "agent", "assigneeId": BENDER}),
        ))
        .await;
    validation(
        &agent,
        "assigneeId",
        &["must be an active board member able to own posts"],
    );

    let count = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM board_tag_assignments WHERE room_id=?",
                [BOARD],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1, "failed validations persist no rule");
    assert_eq!(audit_details(&a).await.len(), 1);
}

#[tokio::test]
async fn spa_api_board_automations_destroy_tag_rule_scopes_lookup_and_audits() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    sql(
        &a,
        format!(
            "UPDATE rooms SET type='Rooms::Board' WHERE id={DESIGNERS};
             DELETE FROM board_tag_assignments WHERE id IN (980000001,980000002);
             INSERT INTO board_tag_assignments(id,room_id,tag,assignee_id,created_by_id,created_at,updated_at)
             VALUES
             (980000001,{BOARD},'mine',{JASON},{DAVID},'2026-03-02 16:00:00','2026-03-02 16:00:00'),
             (980000002,{DESIGNERS},'other',{JASON},{DAVID},'2026-03-02 16:00:00','2026-03-02 16:00:00');"
        ),
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    let prefix = format!("/api/v1/rooms/{BOARD}/automations/tag_rules");

    for id in [980000002, 999999999] {
        let reply = david
            .write(json_body(
                Method::DELETE,
                &format!("{prefix}/{id}"),
                &json!({}),
            ))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{}",
            reply.text()
        );
        let api::ApiError::NotFound { message } = parse::<api::ApiErrorResponse>(&reply).error
        else {
            unreachable!()
        };
        assert_eq!(message, "Rule not found.");
    }

    let removed = david
        .write(json_body(
            Method::DELETE,
            &format!("{prefix}/980000001"),
            &json!({}),
        ))
        .await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    let body: api::BoardAutomations = parse(&removed);
    assert!(body.tag_rules.is_empty());
    assert_eq!(
        audit_details(&a).await,
        [json!({"tag_rule":"removed","tag":"mine","assignee":"Jason"})]
    );
    let other = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM board_tag_assignments WHERE id=980000002",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(other, 1, "a rule from another board is untouched");
}

#[tokio::test]
async fn spa_api_board_automations_sla_create_update_noop_remove_and_audit() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/automations/sla_timers");
    let initial = timers(
        timer(Some(10), Some(30)),
        timer(Some(20), Some(40)),
        timer(Some(30), Some(60)),
    );
    let created = david.write(json_body(Method::PUT, &path, &initial)).await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.text());
    let body: api::BoardAutomations = parse(&created);
    assert_eq!(
        body.sla_timers
            .iter()
            .map(|timer| timer.status)
            .collect::<Vec<_>>(),
        [
            api::WorkStatus::Planned,
            api::WorkStatus::InProgress,
            api::WorkStatus::Blocked,
        ]
    );
    assert_eq!(
        audit_details(&a).await,
        [
            json!({"sla_rule":"created","status":"planned","nudge_after_minutes":10,"escalate_after_minutes":30}),
            json!({"sla_rule":"created","status":"in_progress","nudge_after_minutes":20,"escalate_after_minutes":40}),
            json!({"sla_rule":"created","status":"blocked","nudge_after_minutes":30,"escalate_after_minutes":60}),
        ]
    );

    let unchanged = david.write(json_body(Method::PUT, &path, &initial)).await;
    assert_eq!(unchanged.status, StatusCode::OK, "{}", unchanged.text());
    assert_eq!(audit_details(&a).await.len(), 3, "a no-op has no audit");

    let changed = timers(
        timer(Some(15), Some(30)),
        timer(Some(20), Some(40)),
        timer(None, None),
    );
    let updated = david.write(json_body(Method::PUT, &path, &changed)).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.text());
    let body: api::BoardAutomations = parse(&updated);
    assert_eq!(
        body.sla_timers
            .iter()
            .map(|timer| (timer.status, timer.nudge_after_minutes))
            .collect::<Vec<_>>(),
        [
            (api::WorkStatus::Planned, 15),
            (api::WorkStatus::InProgress, 20),
        ]
    );
    assert_eq!(
        audit_details(&a).await,
        [
            json!({"sla_rule":"created","status":"planned","nudge_after_minutes":10,"escalate_after_minutes":30}),
            json!({"sla_rule":"created","status":"in_progress","nudge_after_minutes":20,"escalate_after_minutes":40}),
            json!({"sla_rule":"created","status":"blocked","nudge_after_minutes":30,"escalate_after_minutes":60}),
            json!({"sla_rule":"updated","status":"planned","nudge_after_minutes":{"before":10,"after":15},"escalate_after_minutes":{"before":30,"after":30}}),
            json!({"sla_rule":"removed","status":"blocked"}),
        ]
    );

    let off = timers(timer(None, None), timer(None, None), timer(None, None));
    let removed = david.write(json_body(Method::PUT, &path, &off)).await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    let body: api::BoardAutomations = parse(&removed);
    assert!(body.sla_timers.is_empty());
    assert_eq!(
        &audit_details(&a).await[5..],
        [
            json!({"sla_rule":"removed","status":"planned"}),
            json!({"sla_rule":"removed","status":"in_progress"}),
        ]
    );
    assert!(sla_rows(&a).await.is_empty());
}

#[tokio::test]
async fn spa_api_board_automations_sla_validates_every_row_before_any_write() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/automations/sla_timers");

    let invalid = timers(
        timer(Some(0), Some(1)),
        timer(Some(6), Some(5)),
        timer(Some(43_201), Some(43_202)),
    );
    let reply = david.write(json_body(Method::PUT, &path, &invalid)).await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { message, fields } =
        parse::<api::ApiErrorResponse>(&reply).error
    else {
        unreachable!()
    };
    assert_eq!(fields.len(), 3);
    assert_eq!(
        fields.get("planned"),
        Some(&vec![
            "Nudge after minutes must be greater than 0".to_string()
        ])
    );
    assert_eq!(
        fields.get("inProgress"),
        Some(&vec![
            "Escalate after minutes must be after the nudge threshold".to_string()
        ])
    );
    assert_eq!(
        fields.get("blocked"),
        Some(&vec![
            "Nudge after minutes must be less than or equal to 43200".to_string(),
            "Escalate after minutes must be less than or equal to 43200".to_string(),
        ])
    );
    assert_eq!(
        message,
        "Planned: Nudge after minutes must be greater than 0, In progress: Escalate after minutes must be after the nudge threshold, and Blocked: Nudge after minutes must be less than or equal to 43200 and Escalate after minutes must be less than or equal to 43200"
    );
    assert!(sla_rows(&a).await.is_empty());

    let one_null = timers(timer(Some(10), None), timer(None, None), timer(None, None));
    let reply = david.write(json_body(Method::PUT, &path, &one_null)).await;
    assert_eq!(
        validation(
            &reply,
            "planned",
            &[
                "Escalate after minutes can't be blank",
                "Escalate after minutes is not a number",
            ],
        ),
        "Planned: Escalate after minutes can't be blank and Escalate after minutes is not a number"
    );

    let partially_valid = timers(
        timer(Some(10), Some(20)),
        timer(None, None),
        timer(None, Some(30)),
    );
    let reply = david
        .write(json_body(Method::PUT, &path, &partially_valid))
        .await;
    validation(
        &reply,
        "blocked",
        &[
            "Nudge after minutes can't be blank",
            "Nudge after minutes is not a number",
        ],
    );
    assert!(
        sla_rows(&a).await.is_empty(),
        "a valid planned row is not created when blocked is invalid"
    );

    sql(
        &a,
        format!(
            "INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at)
             VALUES
             ({BOARD},'planned',8,16,'2026-03-02 16:00:00','2026-03-02 16:00:00'),
             ({BOARD},'in_progress',20,40,'2026-03-02 16:00:00','2026-03-02 16:00:00');"
        ),
    )
    .await;
    let reply = david
        .write(json_body(Method::PUT, &path, &partially_valid))
        .await;
    validation(
        &reply,
        "blocked",
        &[
            "Nudge after minutes can't be blank",
            "Nudge after minutes is not a number",
        ],
    );
    assert_eq!(
        sla_rows(&a).await,
        [
            ("in_progress".to_string(), 20, 40),
            ("planned".to_string(), 8, 16),
        ],
        "an invalid blocked row prevents the planned update and in-progress removal"
    );
    assert!(audit_details(&a).await.is_empty());
}

#[tokio::test]
async fn spa_api_board_automations_sla_leaves_rows_the_client_left_out() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/automations/sla_timers");
    let initial = timers(
        timer(Some(10), Some(30)),
        timer(Some(20), Some(40)),
        timer(Some(30), Some(60)),
    );
    let created = david.write(json_body(Method::PUT, &path, &initial)).await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.text());

    let only_blocked = json!({"blocked": timer(None, None)});
    let saved = david
        .write(json_body(Method::PUT, &path, &only_blocked))
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.text());
    assert_eq!(
        sla_rows(&a).await,
        [("in_progress".into(), 20, 40), ("planned".into(), 10, 30)],
        "only the row sent changed"
    );
    assert_eq!(
        &audit_details(&a).await[3..],
        [json!({"sla_rule":"removed","status":"blocked"})]
    );

    let nothing = david.write(json_body(Method::PUT, &path, &json!({}))).await;
    assert_eq!(nothing.status, StatusCode::OK, "{}", nothing.text());
    assert_eq!(sla_rows(&a).await.len(), 2);
    assert_eq!(
        audit_details(&a).await.len(),
        4,
        "an empty form changes nothing"
    );

    let invalid_planned = json!({"planned": timer(Some(30), Some(10))});
    let refused = david
        .write(json_body(Method::PUT, &path, &invalid_planned))
        .await;
    validation(
        &refused,
        "planned",
        &["Escalate after minutes must be after the nudge threshold"],
    );
    assert_eq!(
        sla_rows(&a).await,
        [("in_progress".into(), 20, 40), ("planned".into(), 10, 30)]
    );
}

/// Every change reaches the other open panes on the board's room topic: here an administrator's
/// and a plain member's connections (the member can't read the settings, but the signal carries
/// nothing but the room).
#[tokio::test]
async fn spa_api_board_automations_changes_signal_the_boards_other_clients() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    sql(
        &a,
        format!(
            "UPDATE users SET role=1 WHERE id={JASON};
             UPDATE users SET role=0 WHERE id={KEVIN};
             INSERT OR IGNORE INTO memberships(room_id,user_id,created_at,updated_at)
             VALUES({BOARD},{KEVIN},'2026-03-02 16:00:00','2026-03-02 16:00:00'),
                   ({BOARD},{JASON},'2026-03-02 16:00:00','2026-03-02 16:00:00');"
        ),
    )
    .await;
    let (addr, server) = serve(&a).await;
    let topic = format!("room:{BOARD}");
    let jason = a.sign_in(JASON).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut clients = [
        Sync::connect(addr, &jason.cookie_header(), std::slice::from_ref(&topic)).await,
        Sync::connect(addr, &kevin.cookie_header(), std::slice::from_ref(&topic)).await,
    ];
    for client in &mut clients {
        client.welcome().await;
    }
    let changed = |event: &api::SyncEvent| {
        event.topic == format!("room:{BOARD}")
            && event.payload
                == api::SyncPayload::BoardAutomationsChanged(api::BoardAutomationsChanged {
                    room_id: BOARD,
                })
    };

    let mut david = a.sign_in(DAVID).await;
    let rule = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/automations/tag_rules"),
            &json!({"tag": "bug", "assigneeId": DAVID}),
        ))
        .await;
    assert_eq!(rule.status, StatusCode::CREATED, "{}", rule.text());
    for client in &mut clients {
        client.until(changed, |_| false).await;
    }

    for values in [
        timer(Some(10), Some(20)),
        timer(Some(15), Some(30)),
        timer(None, None),
    ] {
        let timers = david
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/rooms/{BOARD}/automations/sla_timers"),
                &json!({"planned": values}),
            ))
            .await;
        assert_eq!(timers.status, StatusCode::OK, "{}", timers.text());
        for client in &mut clients {
            client.until(changed, |_| false).await;
        }
    }

    let rule_id = parse::<api::BoardAutomations>(&rule).tag_rules[0].id;
    let removed = david
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/rooms/{BOARD}/automations/tag_rules/{rule_id}"),
            &json!({}),
        ))
        .await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    for client in &mut clients {
        client.until(changed, |_| false).await;
    }
    server.abort();
}

#[tokio::test]
async fn classic_board_automations_changes_signal_spa_clients() {
    let a = app(true).await.expect("the frozen default seed");
    reset(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let topic = format!("room:{BOARD}");
    let mut client =
        Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
    client.welcome().await;
    let changed = |event: &api::SyncEvent| {
        event.topic == topic
            && event.payload
                == api::SyncPayload::BoardAutomationsChanged(api::BoardAutomationsChanged {
                    room_id: BOARD,
                })
    };
    let path = format!("/rooms/boards/{BOARD}/automations");

    let created = david
        .write(
            Req::new(Method::POST, &format!("{path}/tag_assignments"))
                .header("content-type", "application/json")
                .body(json!({"tag": "bug", "assignee_id": DAVID}).to_string()),
        )
        .await;
    assert_eq!(created.status, StatusCode::FOUND, "{}", created.text());
    client.until(changed, |_| false).await;

    for (nudge, escalate) in [("10", "20"), ("15", "30"), ("", "")] {
        let saved = david
            .write(
                Req::new(Method::PATCH, &format!("{path}/sla_rules"))
                    .header("content-type", "application/json")
                    .body(
                        json!({"sla_rules": {"planned": {
                            "nudge_after_minutes": nudge,
                            "escalate_after_minutes": escalate,
                        }}})
                        .to_string(),
                    ),
            )
            .await;
        assert_eq!(saved.status, StatusCode::FOUND, "{}", saved.text());
        client.until(changed, |_| false).await;
    }

    let rule_id = settings(&mut david, BOARD).await.tag_rules[0].id;
    let removed = david
        .write(Req::new(
            Method::DELETE,
            &format!("{path}/tag_assignments/{rule_id}"),
        ))
        .await;
    assert_eq!(removed.status, StatusCode::FOUND, "{}", removed.text());
    client.until(changed, |_| false).await;
    server.abort();
}

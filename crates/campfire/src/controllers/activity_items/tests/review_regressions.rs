//! Rails `preload(:source)`, `includes(:agent)` and explicit `head :not_found` contracts.
use super::*;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, KEVIN};

fn table_selects(log: &[String], table: &str) -> usize {
    let from = regex::Regex::new(r#"(?i)\bFROM\s+\"?([a-z_]+)"#).unwrap();
    log.iter()
        .filter(|sql| {
            from.captures(sql)
                .is_some_and(|capture| &capture[1] == table)
        })
        .count()
}

#[tokio::test]
async fn ws11ui_review_inbox_preloads_message_sources_once_for_every_format() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../../vectors/inbox-http.json")).unwrap();
    let paging = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "pagination ")
        .unwrap();
    let mut failures = Vec::new();
    for accept in [
        "application/json",
    ] {
        for size in [1, 100] {
            let t = TestApp::boot_frozen()
                .await
                .unwrap()
                .without_job_runner()
                .await;
            let sql: Vec<String> = corpus["setup"]
                .as_array()
                .unwrap()
                .iter()
                .chain(paging["sql"].as_array().unwrap())
                .map(|sql| sql.as_str().unwrap().into())
                .collect();
            t.db().write(move |tx| {
                for sql in sql { tx.conn().execute(&sql, [])?; }
                tx.conn().execute("DELETE FROM activity_items WHERE id NOT IN (SELECT id FROM activity_items ORDER BY id DESC LIMIT ?)", [size])?;
                Ok(())
            }).await.unwrap();
            let mut browser = t.david();
            let log = t.db().capture_read_queries();
            let response = browser
                .send(
                    Req::new(Method::GET, "/activity")
                        .header("accept", accept)
                        .header("turbo-frame", "activity_test"),
                )
                .await;
            t.db().stop_capturing_read_queries();
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
            if accept == "application/json" {
                assert_eq!(
                    response.json()["activity_items"].as_array().unwrap().len(),
                    size as usize
                );
            }
            let queries = log.lock().unwrap();
            let expected = if accept == "application/json" {
                vec![
                    ("messages", 1),
                    ("rooms", 1),
                    ("users", 1),
                    ("action_text_rich_texts", size as usize),
                ]
            } else {
                vec![("messages", 1)]
            };
            for (table, rails) in expected {
                let count = table_selects(&queries, table);
                println!("inbox {accept}: {size} items; {table}_selects={count}; Rails={rails}");
                if count != rails {
                    failures.push(format!("{accept}, {size}: {table}={count}, Rails={rails}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Rails inbox query counts: {failures:?}"
    );
}

#[tokio::test]
async fn ws11ui_review_member_polling_preloads_agents_once() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    for additional in [0, 10] {
        t.db()
            .write(move |tx| {
                for i in 0..additional {
                    let bot = campfire_db::User::create_bot(tx, &format!("Polling bot {i}"), None)?;
                    campfire_db::Agent::create(
                        tx,
                        campfire_db::NewAgent {
                            user_id: bot.id,
                            owner_id: Some(DAVID),
                            ..Default::default()
                        },
                    )?;
                    campfire_db::Membership::create_default(tx, ALL_TALK, bot.id)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let log = t.db().capture_read_queries();
        let response = browser
            .get(&format!("/rooms/{ALL_TALK}/members.json"))
            .await;
        t.db().stop_capturing_read_queries();
        assert_eq!(response.status, StatusCode::OK);
        let count = table_selects(&log.lock().unwrap(), "agents");
        println!("member polling +{additional} bots: agent_selects={count}; Rails=1");
        assert_eq!(
            count, 1,
            "Rails preloads agents once regardless of member count"
        );
    }
}

#[tokio::test]
async fn ws11ui_review_explicit_approval_denials_have_empty_bodies_and_no_writes() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let (agent_id, approval_id) = t
        .db()
        .write(move |tx| {
            let mut agent = campfire_db::Agent::for_user(tx.conn(), BENDER)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(DAVID)),
                    ..Default::default()
                },
            )?;
            let approval = campfire_db::AgentApproval::create(
                tx,
                campfire_db::NewApproval {
                    agent_id: agent.id,
                    action: "deploy".into(),
                    summary: "Denial body regression".into(),
                    ..Default::default()
                },
            )?;
            Ok((agent.id, approval.id))
        })
        .await
        .unwrap();
    let mut member = t.sign_in(KEVIN).await;
    let mut failures = Vec::new();
    for suffix in ["", ".json"] {
        for id in [agent_id, 999999999999] {
            let response = member.get(&format!("/api/v1/agents/{id}/approvals")).await;
            assert_eq!(response.status, StatusCode::NOT_FOUND);
            println!(
                "approval history {id}{suffix}: status=404 body_bytes={}",
                response.body.len()
            );
        }
        for id in [approval_id, 999999999999] {
            let response = member
                .write(
                    Req::new(Method::PATCH, &format!("/agent_approvals/{id}{suffix}"))
                        .form(&[("decision", "approved")]),
                )
                .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND);
            assert_eq!(
                response.header("content-type"),
                Some(if suffix.is_empty() {
                    "text/html"
                } else {
                    "application/json"
                })
            );
            println!(
                "approval decision {id}{suffix}: status=404 body_bytes={}",
                response.body.len()
            );
            if !response.body.is_empty() {
                failures.push(format!("decision {id}{suffix}"));
            }
        }
    }
    t.db()
        .read(move |conn| {
            assert_eq!(
                campfire_db::AgentApproval::find(conn, approval_id)?
                    .unwrap()
                    .status,
                "pending"
            );
            let audit: i64 = conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action='agent.approval.decide'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(audit, 0);
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        failures.is_empty(),
        "Rails explicit head :not_found returns no body: {failures:?}"
    );
}

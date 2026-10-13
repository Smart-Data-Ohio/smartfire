use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{ALL_TALK, BENDER, DAVID, Req};
use campfire_kit::Method;
use rusqlite::params;
use serde_json::json;

#[tokio::test]
async fn review_s1_owner_ceiling_rejects_authenticated_endpoints_after_live_owner_deactivation() {
    let app = setup().await.without_job_runner().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
            tx.conn().execute(
                "UPDATE agents SET kind='personal',owner_id=? WHERE id=?",
                params![DAVID, AGENT],
            )?;
            campfire_db::Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER, DAVID])?;
            campfire_db::AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: AGENT,
                    room_id: Some(ALL_TALK),
                    capability: "read_messages".into(),
                    granted_by_id: DAVID,
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let requests = [
        (Method::GET, "/agents/me", json!({})),
        (Method::GET, "/agents/events", json!({})),
        (
            Method::POST,
            "/agents/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ),
    ];
    for (method, path, body) in &requests {
        let reply = app
            .anonymous()
            .send(
                Req::new(method.clone(), path)
                    .header("authorization", &[concat!("Bear", "er"), SECRET].join(" "))
                    .header("accept", "application/json")
                    .json_body(body),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 200, "{path}: {}", reply.text());
    }
    app.db()
        .write(|tx| {
            // Leave the bot, credential and agent live to test the owner ceiling itself.
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [DAVID])?;
            assert!(campfire_db::User::find(tx.conn(), BENDER)?.is_active());
            assert!(
                campfire_db::Agent::find(tx.conn(), AGENT)?
                    .unwrap()
                    .suspended_at
                    .is_none()
            );
            assert!(
                campfire_db::AgentCredential::authenticate(tx.conn(), SECRET, tx.now())?.is_some()
            );
            Ok(())
        })
        .await
        .unwrap();
    for (method, path, body) in requests {
        let reply = app
            .anonymous()
            .send(
                Req::new(method, path)
                    .header("authorization", &[concat!("Bear", "er"), SECRET].join(" "))
                    .header("accept", "application/json")
                    .json_body(&body),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 401, "{path}: {}", reply.text());
    }
}

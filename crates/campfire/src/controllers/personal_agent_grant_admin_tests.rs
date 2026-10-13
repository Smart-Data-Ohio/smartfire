use super::agent_http_tests::{AGENT, setup};
use super::presenters::test_support::{ALL_TALK, BENDER, DAVID, Req, TestApp};
use campfire_db::{AgentGrant, NewGrant, Room};
use campfire_kit::Method;
use rusqlite::params;
use serde_json::{Value, json};

async fn legacy_personal() -> TestApp {
    let app = setup().await.without_job_runner().await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET kind='personal',owner_id=? WHERE id=?",
                params![DAVID, AGENT],
            )?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER, DAVID])?;
            AgentGrant::create(tx, NewGrant {
                agent_id: AGENT, room_id: Some(ALL_TALK), capability: "read_messages".into(),
                granted_by_id: DAVID, ..Default::default()
            })?;
            // Reproduce rows preserved by the additive migration from the old schema.
            tx.conn().execute_batch("DROP TRIGGER personal_agent_grant_insert")?;
            for capability in ["external_action", "read_messages"] {
                tx.conn().execute(
                    "INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?)",
                    params![AGENT, capability, DAVID, tx.now(), tx.now()],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    app
}

fn grant<'a>(list: &'a Value, capability: &str, room: &str) -> &'a Value {
    list["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["capability"] == capability && row["roomName"] == room)
        .unwrap()
}

#[tokio::test]
async fn legacy_personal_admin_grants_show_policy_status_and_remain_revocable() {
    let app = legacy_personal().await;
    let mut owner = app.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let path = format!("/api/v1/admin/bots/{BENDER}/grants");
    let reply = owner.send(Req::new(Method::GET, &path)).await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let list: Value = serde_json::from_slice(&reply.body).unwrap();
    for capability in ["external_action", "read_messages"] {
        let row = grant(&list, capability, "Workspace-wide");
        assert_eq!(row["status"], "incompatible", "{row}");
        assert_eq!(row["revoked"], false, "the audit record remains unrevoked");
    }
    let scoped = list["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["roomName"] != "Workspace-wide")
        .unwrap();
    assert_eq!(scoped["status"], "active");
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                params![DAVID, ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = owner.send(Req::new(Method::GET, &path)).await;
    let after: Value = serde_json::from_slice(&reply.body).unwrap();
    let id = scoped["id"].as_i64().unwrap();
    assert_eq!(
        after["grants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap()["status"],
        "inactive"
    );
    let id = grant(&list, "external_action", "Workspace-wide")["id"]
        .as_i64()
        .unwrap();
    let reply = owner
        .write(Req::new(Method::DELETE, &format!("{path}/{id}")))
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let after: Value = serde_json::from_slice(&reply.body).unwrap();
    let row = grant(&after, "external_action", "Workspace-wide");
    assert_eq!(row["id"], id);
    assert_eq!(row["status"], "revoked");
    assert_eq!(row["revoked"], true);
}

#[tokio::test]
async fn legacy_personal_duplicate_grant_requests_validate_before_noop() {
    let app = legacy_personal().await;
    let mut owner = app.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let before = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row(
                    "SELECT COUNT(*) FROM agent_grants WHERE agent_id=?",
                    [AGENT],
                    |r| r.get::<_, i64>(0),
                )?,
                conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    for capability in ["external_action", "read_messages"] {
        let reply = owner
            .write(
                Req::new(Method::POST, &format!("/api/v1/admin/bots/{BENDER}/grants"))
                    .json_body(&json!({"capability": capability, "roomId": null})),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 422, "{capability}: {}", reply.text());
        let reply = owner
            .write(
                Req::new(Method::POST, &format!("/account/bots/{BENDER}/grants")).form(&[
                    ("agent_grant[capability]", capability),
                    ("agent_grant[room_id]", ""),
                ]),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            422,
            "classic {capability}: {}",
            reply.text()
        );
    }
    // A compatible duplicate is still an idempotent success.
    let reply = owner
        .write(
            Req::new(Method::POST, &format!("/api/v1/admin/bots/{BENDER}/grants"))
                .json_body(&json!({"capability": "read_messages", "roomId": ALL_TALK})),
        )
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let after = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row(
                    "SELECT COUNT(*) FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
                    [AGENT],
                    |r| r.get::<_, i64>(0),
                )?,
                conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(after, before, "requests leave legacy audit rows unchanged");
}

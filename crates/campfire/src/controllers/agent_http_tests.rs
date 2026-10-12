//! Committed Rails HTTP vectors, through the complete router and live SQLite policy.
use super::presenters::test_support::{Reply, Req, TestApp};
use campfire_db::{AgentCredential, NewCredential};
use campfire_kit::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const SECRET: &str = "ws11api-fixture-credential";
pub(super) const AGENT: i64 = 773018776;

pub(super) async fn setup() -> TestApp {
    let app = TestApp::boot_with_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    )))
    .await
    .expect("WS11-api tests require the pinned default seed");
    initialize(&app).await;
    app
}

pub(super) async fn initialize(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET status='idle',status_note=NULL,status_changed_at=NULL,working_presence=NULL,working_presence_expires_at=NULL,last_seen_at=NULL WHERE id=?",[AGENT])?;
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_credentials WHERE agent_id=?", [AGENT])?;
        let digest = format!("{:x}", Sha256::digest(SECRET));
        AgentCredential::create(tx, NewCredential { agent_id: AGENT, created_by_id: 127326141, name: "HTTP contract".into(), token_last_four: digest[..4].into(), token_digest: digest, ..Default::default() })?;
        for (id, kind, metadata) in [(900_000_001, "github_action_completed", json!({"action":"github.comment","status":"done"})), (900_000_002, "approval_decided", json!({"approval_id":0}))] {
            tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,metadata,created_at) VALUES(?,?,?,'delivered',?,'2026-03-02 16:00:00')", rusqlite::params![id, AGENT, kind, metadata.to_string()])?;
        }
        Ok(())
    }).await.unwrap();
}

async fn request(app: &TestApp, case: &Value) -> Reply {
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        case["path"].as_str().unwrap(),
    )
    .header("accept", "application/json")
    .header("content-type", "application/json");
    if let Some(secret) = case["token"].as_str() {
        req = req.header("authorization", &["Bearer", secret].join(" "));
    }
    if let Some(body) = case["body"].as_str() {
        req = req.body(body);
    }
    for (name, value) in case["headers"].as_object().unwrap() {
        req = req.header(name, value.as_str().unwrap());
    }
    app.anonymous().send(req).await
}

async fn check(name: &str) {
    let app = setup().await;
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_http.json")).unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    app.db().write({let name=name.to_string(); move |tx| {
        if ["profile_invalid","profile_clear","profile_boolean_note"].contains(&name.as_str()) {
            tx.conn().execute("UPDATE agents SET status='working',status_note='Tests',status_changed_at=?,working_presence='Thinking',working_presence_expires_at=? WHERE id=?",rusqlite::params![tx.now(),tx.now().since(jiff::SignedDuration::from_mins(5)),AGENT])?;
        }
        if name == "revoked_credential" { tx.conn().execute("UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
        if name == "expired_credential" { tx.conn().execute("UPDATE agent_credentials SET expires_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
        if ["missing_grant", "slash_missing_grant"].contains(&name.as_str()) {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,'read_messages',127326141,?,?,?)", rusqlite::params![AGENT, if name == "missing_grant" {Some(tx.now())} else {None},tx.now(),tx.now()])?;
        }
        Ok(())
    }}).await.unwrap();
    if name == "event_rate_overflow" {
        let mut polling = case.clone();
        polling["path"] = json!("/agents/events");
        for _ in 0..120 {
            assert_eq!(request(&app, &polling).await.status.as_u16(), 200);
        }
    }
    if ["event_ack_replay", "slash_register_replay"].contains(&name) {
        assert_eq!(
            request(&app, case).await.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
    }
    let reply = request(&app, case).await;
    if name == "profile_update" {
        let agent = app
            .db()
            .read(|conn| campfire_db::Agent::find(conn, AGENT))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(agent.provider, None);
        assert_eq!(agent.daily_message_cap, None);
    }
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    let body = if reply.body.is_empty() {
        Value::Null
    } else {
        reply.json()
    };
    assert_eq!(body, case["response"], "{name}");
    assert_eq!(
        reply.text(),
        case["response_body"].as_str().unwrap(),
        "{name}: raw body"
    );
    for (header, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(header), expected.as_str(), "{name}: {header}");
    }
}

#[tokio::test]
async fn revoked_and_expired_credentials() {
    check("revoked_credential").await;
    check("expired_credential").await;
}
#[tokio::test]
async fn grant_denial_is_403_and_nonmembership_is_404() {
    check("missing_grant").await;
    check("slash_nonmember").await;
    check("slash_missing_grant").await;
}
#[tokio::test]
async fn credential_rate_overflow() {
    check("event_rate_overflow").await;
}
#[tokio::test]
async fn agent_token_cannot_access_human_endpoint() {
    check("agent_human_endpoint").await;
}
#[tokio::test]
async fn rest_vectors() {
    for name in [
        "event_envelope",
        "event_array",
        "event_dropped_cursor",
        "event_ack",
        "event_ack_replay",
        "event_ack_missing",
        "unknown_credential",
        "step_no_parent",
        "step_missing",
        "step_create",
        "slash_invalid",
        "slash_register",
        "slash_register_replay",
        "slash_missing",
    ] {
        check(name).await;
    }
}

#[tokio::test]
async fn profile_vectors() {
    for name in [
        "profile_get",
        "profile_update",
        "profile_invalid",
        "profile_clear",
        "profile_boolean_note",
    ] {
        check(name).await;
    }
}

#[tokio::test]
async fn step_duration_validates_before_cast() {
    for index in 0..7 {
        check(&format!("step_duration_{index}")).await;
    }
}

#[tokio::test]
async fn invalid_duration_never_persists_and_parent_policy_runs_first() {
    let app = setup().await;
    for (message_id, status) in [(935961918, 422), (0, 404)] {
        let reply = app
            .anonymous()
            .send(
                Req::new(Method::POST, "/agents/steps")
                    .header("authorization", &["Bearer", SECRET].join(" "))
                    .header("content-type", "application/json")
                    .body(
                        json!({"message_id":message_id,"name":"Inspect","duration_ms":4.5})
                            .to_string(),
                    ),
            )
            .await;
        assert_eq!(reply.status.as_u16(), status);
    }
    let count: i64 = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM agent_steps", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(count, 0);
}

async fn register_typed(app: &TestApp, room: i64, body: Value, token: Option<&str>) -> Reply {
    let req = Req::new(
        Method::POST,
        &format!("/rooms/{room}/agents/slash_commands"),
    )
    .header("accept", "application/json")
    .header("content-type", "application/json")
    .body(body.to_string());
    let req = if let Some(token) = token {
        req.header("authorization", &format!("Bearer {token}"))
    } else {
        req
    };
    app.anonymous().send(req).await
}

async fn malformed_slash_schema_app() -> TestApp {
    use super::presenters::test_support::ALL_TALK;
    let app = setup().await;
    for name in ["broken", "valid"] {
        let reply = register_typed(
            &app,
            ALL_TALK,
            json!({"name":name,"option_schema":{"options":[]}}),
            Some(SECRET),
        )
        .await;
        assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    }
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agent_slash_commands SET option_schema='{' WHERE name='broken'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    app
}

#[tokio::test]
async fn slash_malformed_schema_composer_listing_keeps_both_commands() {
    use super::presenters::test_support::{ALL_TALK, DAVID};
    use super::spa::api_tests::{get, parse};
    let app = malformed_slash_schema_app().await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david
        .send(get(&format!("/api/v1/rooms/{ALL_TALK}/slash_commands")))
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let list: campfire_api_types::SlashCommandList = parse(&reply);
    let broken = list.commands.iter().find(|c| c.name == "broken").unwrap();
    assert_eq!(broken.agent_name.as_deref(), Some("Bender Bot"));
    assert!(broken.option_schema.is_none());
    let valid = list.commands.iter().find(|c| c.name == "valid").unwrap();
    assert_eq!(json!(valid.option_schema), json!({"options":[]}));
}

#[tokio::test]
async fn slash_malformed_schema_composer_invocation_returns_validation() {
    use super::presenters::test_support::{ALL_TALK, DAVID};
    use super::spa::api_tests::{json_body, parse};
    let app = malformed_slash_schema_app().await;
    let mut david = app.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/slash_commands");
    for body in [
        json!({"text":"/broken"}),
        json!({"text":"/broken","options":{"values":{}}}),
    ] {
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert_eq!(reply.status.as_u16(), 422, "{}", reply.text());
        assert_eq!(
            parse::<Value>(&reply),
            json!({"error":{"_tag":"Validation","message":"Invalid slash command options","fields":{"options":["command option schema is unavailable"]}}})
        );
    }
    let reply = david
        .write(json_body(Method::POST, &path, &json!({"text":"/valid"})))
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(parse::<Value>(&reply)["status"], "ephemeral");
    app.db()
        .read(|conn| {
            let metadata: String = conn.query_row(
                "SELECT metadata FROM agent_events WHERE event_type='slash_command'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(
                serde_json::from_str::<Value>(&metadata).unwrap()["command"],
                "valid"
            );
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE event_type='slash_command'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(count, 1);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slash_malformed_schema_http_management_can_clear_and_replace() {
    use super::presenters::test_support::ALL_TALK;
    let app = malformed_slash_schema_app().await;
    for schema in [Value::Null, json!({"options":[]})] {
        app.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE agent_slash_commands SET option_schema='{' WHERE name='broken'",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let reply = register_typed(
            &app,
            ALL_TALK,
            json!({"name":"broken","option_schema":schema}),
            Some(SECRET),
        )
        .await;
        assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
        assert_eq!(
            serde_json::from_str::<Value>(&reply.text()).unwrap()["option_schema"],
            schema
        );
        let stored: Option<String> = app
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT option_schema FROM agent_slash_commands WHERE name='broken'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            stored
                .map(|s| serde_json::from_str::<Value>(&s).unwrap())
                .unwrap_or(Value::Null),
            schema
        );
    }
}

#[tokio::test]
async fn slash_schema_http_management_set_keep_clear_invalid_and_authorization() {
    use super::presenters::test_support::{ALL_TALK, DAVID};
    let app = setup().await;
    let schema = json!({"options":[{"name":"count","description":"Count","type":"integer","required":true,"min":1,"max":3}]});
    let body = json!({"name":"typed","option_schema":schema});
    let reply = register_typed(&app, ALL_TALK, body.clone(), Some(SECRET)).await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    assert_eq!(
        serde_json::from_str::<Value>(&reply.text()).unwrap()["option_schema"],
        schema
    );
    let reply = register_typed(
        &app,
        ALL_TALK,
        json!({"name":"typed","description":"Changed"}),
        Some(SECRET),
    )
    .await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    assert_eq!(
        serde_json::from_str::<Value>(&reply.text()).unwrap()["option_schema"],
        schema
    );
    for invalid in [
        json!({"options":[{"name":"Bad","description":"Count","type":"integer","required":true}]}),
        json!({"options":[],"subcommands":[]}),
    ] {
        let reply = register_typed(
            &app,
            ALL_TALK,
            json!({"name":"typed","option_schema":invalid}),
            Some(SECRET),
        )
        .await;
        assert_eq!(reply.status.as_u16(), 422, "{}", reply.text());
        assert!(
            serde_json::from_str::<Value>(&reply.text()).unwrap()["errors"]
                .as_object()
                .unwrap()
                .keys()
                .all(|key| key.starts_with("option_schema"))
        );
    }
    assert_eq!(
        register_typed(&app, ALL_TALK, body.clone(), None)
            .await
            .status
            .as_u16(),
        302
    );
    assert_eq!(
        register_typed(&app, ALL_TALK, body.clone(), Some("unknown-credential"))
            .await
            .status
            .as_u16(),
        401
    );
    assert_eq!(
        register_typed(&app, 201306877, body.clone(), Some(SECRET))
            .await
            .status
            .as_u16(),
        404
    );
    app.db()
        .write(|tx| {
            campfire_db::AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: AGENT,
                    capability: "read_messages".into(),
                    granted_by_id: DAVID,
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        register_typed(
            &app,
            ALL_TALK,
            json!({"name":"typed","option_schema":null}),
            Some(SECRET)
        )
        .await
        .status
        .as_u16(),
        403
    );
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_grants WHERE agent_id=?", [AGENT])?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = register_typed(
        &app,
        ALL_TALK,
        json!({"name":"typed","option_schema":null}),
        Some(SECRET),
    )
    .await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    app.db()
        .read(|conn| {
            assert!(
                campfire_db::AgentSlashCommand::find_by_name(conn, ALL_TALK, "typed")?
                    .unwrap()
                    .option_schema()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slash_schema_composer_listing_validation_visibility_and_delivery() {
    use super::presenters::test_support::{ALL_TALK, DAVID, KEVIN};
    use super::spa::api_tests::{get, json_body, parse, tag};
    let app = setup().await;
    let schema = json!({"subcommands":[{"name":"run","description":"Run","options":[
        {"name":"target","description":"Target","type":"string","required":true,"choices":[{"name":"Staging","value":"staging"}]},
        {"name":"count","description":"Count","type":"integer","required":true,"min":1,"max":3},
        {"name":"user","description":"User","type":"user","required":false},
        {"name":"room","description":"Room","type":"room","required":false}
    ]}]});
    assert_eq!(
        register_typed(
            &app,
            ALL_TALK,
            json!({"name":"typed","option_schema":schema}),
            Some(SECRET)
        )
        .await
        .status
        .as_u16(),
        201
    );
    let mut david = app.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/slash_commands");
    let list: Value = parse(&david.send(get(&path)).await);
    assert_eq!(
        list["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "typed")
            .unwrap()["optionSchema"],
        schema
    );
    let good = json!({"subcommand":"run","values":{"target":"staging","count":2,"user":DAVID,"room":ALL_TALK}});
    for (name, value) in [
        ("target", json!(false)),
        ("target", json!("production")),
        ("count", json!(4)),
        ("count", json!(2.5)),
    ] {
        let mut options = good.clone();
        options["values"][name] = value;
        let reply = david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"text":"/typed raw text","options":options}),
            ))
            .await;
        assert_eq!(reply.status.as_u16(), 422, "{}", reply.text());
        assert_eq!(tag(&reply), "Validation");
        let body: Value = parse(&reply);
        assert!(body["error"]["fields"][format!("options.values.{name}")].is_array());
    }
    let missing = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"text":"/typed","options":{"subcommand":"run","values":{}}}),
        ))
        .await;
    assert_eq!(missing.status.as_u16(), 422);
    assert_eq!(tag(&missing), "Validation");
    let absent = david
        .write(json_body(Method::POST, &path, &json!({"text":"/typed"})))
        .await;
    assert_eq!(absent.status.as_u16(), 422);
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=104393281",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for (name, id) in [
        ("user", KEVIN),
        ("user", i64::MAX),
        ("room", 104393281),
        ("room", i64::MAX),
    ] {
        let mut options = good.clone();
        options["values"][name] = json!(id);
        let reply = david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"text":"/typed","options":options}),
            ))
            .await;
        assert_eq!(reply.status.as_u16(), 404, "{}", reply.text());
        assert_eq!(tag(&reply), "NotFound");
        assert_eq!(
            parse::<Value>(&reply),
            json!({"error":{"_tag":"NotFound","message":"Not found"}})
        );
    }
    app.db()
        .read(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='slash_command'",
                [AGENT],
                |r| r.get(0),
            )?;
            assert_eq!(count, 0);
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"text":"/typed raw text","options":good}),
        ))
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(parse::<Value>(&reply)["status"], "ephemeral");
    let poll = app
        .anonymous()
        .send(
            Req::new(Method::GET, "/agents/events")
                .header("authorization", &format!("Bearer {SECRET}"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(poll.status.as_u16(), 200, "{}", poll.text());
    let body: Value = parse(&poll);
    let command = &body
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["event_type"] == "slash_command")
        .unwrap()["command"];
    assert_eq!(command["options"], good);
    assert_eq!(command["arguments"], "raw text");
    assert_eq!(
        register_typed(&app, ALL_TALK, json!({"name":"legacy"}), Some(SECRET))
            .await
            .status
            .as_u16(),
        201
    );
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"text":"/legacy old text","options":good}),
        ))
        .await;
    assert_eq!(reply.status.as_u16(), 200);
    app.db().read(|conn| {
        let metadata:String=conn.query_row("SELECT metadata FROM agent_events WHERE agent_id=? AND event_type='slash_command' ORDER BY id DESC LIMIT 1",[AGENT],|r|r.get(0))?;
        assert_eq!(serde_json::from_str::<Value>(&metadata).unwrap(),json!({"command":"legacy","arguments":"old text"}));Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn step_endpoints_recheck_removed_private_agent_parents() {
    let app = setup().await;
    let (user, thread_id, message_id) = app.db().write(|tx| {
        let user = campfire_db::Agent::find(tx.conn(), AGENT)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?.user_id;
        let room = super::presenters::test_support::ALL_TALK;
        if campfire_db::Membership::find_by_room_and_user(tx.conn(), room, user)?.is_none() {
            campfire_db::Membership::create_default(tx, room, user)?;
        }
        for capability in ["post_messages", "manage_threads"] {
            campfire_db::AgentGrant::create(tx, campfire_db::NewGrant { agent_id: AGENT, capability: capability.into(), granted_by_id: 127326141, ..Default::default() })?;
        }
        let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: room, creator_id: user,
            visibility: campfire_db::models::channel_thread::ThreadVisibility::Private,
            work_status: Some("in_progress".into()), work_owner_id: Some(user),
            ..Default::default()
        })?;
        let message = thread.post_message(tx, user, campfire_db::NewMessage { markdown_source: Some("Secret parent".into()), ..Default::default() })?;
        Ok((user, thread.id, message.id))
    }).await.unwrap();
    let bodies = [json!({"message_id":message_id,"name":"Inspect","output_summary":"Secret output"}), json!({"thread_id":thread_id,"name":"Inspect","output_summary":"Secret output"})];
    let mut ids = Vec::new();
    for body in &bodies {
        let reply = app.anonymous().send(Req::new(Method::POST, "/agents/steps")
            .header("authorization", &["Bearer", SECRET].join(" "))
            .header("content-type", "application/json").body(body.to_string())).await;
        assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
        ids.push(reply.json()["id"].as_i64().unwrap());
    }
    app.db().write(move |tx| campfire_db::ChannelThread::find(tx.conn(), thread_id)?.remove_access_member(tx, user, user)).await.unwrap();
    for (body, id) in bodies.into_iter().zip(ids) {
        let reply = app.anonymous().send(Req::new(Method::POST, "/agents/steps")
            .header("authorization", &["Bearer", SECRET].join(" "))
            .header("content-type", "application/json").body(body.to_string())).await;
        assert_eq!(reply.status.as_u16(), 404, "{}", reply.text());
        let reply = app.anonymous().send(Req::new(Method::PATCH, &format!("/agents/steps/{id}"))
            .header("authorization", &["Bearer", SECRET].join(" "))
            .header("content-type", "application/json").body(json!({"status":"done"}).to_string())).await;
        assert_eq!(reply.status.as_u16(), 403, "{}", reply.text());
        assert!(!reply.text().contains("Secret output"));
        let step = app.db().read(move |conn| campfire_db::AgentStep::find(conn, id)).await.unwrap().unwrap();
        assert_eq!(step.status, "running");
        assert_eq!(step.output_summary.as_deref(), Some("Secret output"));
    }
}

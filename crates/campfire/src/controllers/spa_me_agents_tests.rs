use super::admin_tests::{get, json_body, parse, write};
use crate::controllers::presenters::test_support::{ALL_TALK, Browser, DAVID, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

async fn app() -> Option<TestApp> {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let app = TestApp::boot_seed_with_env(
        "default",
        clock,
        &[
            ("APP_URL", "https://agents.example.test"),
            ("SMARTFIRE_MCP_OAUTH", "0"),
        ],
    )
    .await?
    .without_job_runner()
    .await;
    app.db()
        .write(|tx| {
            campfire_db::Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[KEVIN])?;
            campfire_db::Room::find(tx.conn(), 654632876)?.revoke_from(tx, &[KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    Some(app)
}
const ROOT: &str = "/api/v1/me/agents";
const DISABLED_MESSAGE: &str = "Personal agents are turned off for this workspace.";
async fn create(b: &mut Browser<'_>) -> Value {
    let r = write(b, Method::POST, ROOT, json!({"name":"Personal helper","roomIds":[ALL_TALK],"capabilities":["read_messages","post_messages"]})).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    parse(&r)
}
async fn mcp(a: &TestApp, secret: &str) -> StatusCode {
    let reply = a
        .anonymous()
        .send(
            Req::new(Method::POST, "/agents/mcp")
                .header("authorization", &["Bearer", secret].join(" "))
                .json_body(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})),
        )
        .await;
    if reply.status == StatusCode::OK {
        let body: Value = parse(&reply);
        let tools = body["result"]["tools"].as_array().expect("MCP tool list");
        assert!(tools.iter().any(|tool| tool["name"] == "read_messages"));
        assert!(!tools.iter().any(|tool| tool["name"] == "request_approval"));
    }
    reply.status
}
#[tokio::test]
async fn personal_agent_member_api_tokens_access_ownership_and_removal() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let list: Value = parse(&member.send(get(ROOT)).await);
    assert_eq!(list["enabled"], true);
    assert_eq!(list["limit"], 3);
    assert_eq!(list["oauthAvailable"], false);
    assert_eq!(list["mcpUrl"], "https://agents.example.test/agents/mcp");
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let path = format!("{ROOT}/{id}");
    let secret = created["secret"].as_str().unwrap();
    assert_eq!(secret.len(), 68);
    assert!(secret.starts_with("sfa_"));
    assert!(secret[4..].bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(mcp(&a, secret).await, StatusCode::OK);
    let credentials: Value = parse(&member.send(get(&format!("{path}/credentials"))).await);
    let credential = &credentials["credentials"][0];
    let cid = credential["id"].as_i64().unwrap();
    assert_eq!(credential["kind"], "token");
    assert!(credential["lastUsedAt"].is_string());
    assert!(credential["expiresAt"].is_string());
    assert!(!credentials.to_string().contains(secret));
    let failed = write(
        &mut member,
        Method::POST,
        &format!("{path}/credentials/{cid}/rotate"),
        json!({"name":""}),
    )
    .await;
    assert_eq!(failed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(mcp(&a, secret).await, StatusCode::OK);
    let patch = write(
        &mut member,
        Method::PATCH,
        &path,
        json!({"name":"Personal helper","description":"My assistant","dailyMessageCap":null}),
    )
    .await;
    assert_eq!(patch.status, StatusCode::OK, "{}", patch.text());
    let profile: Value = parse(&patch);
    assert!(profile["dailyMessageCap"].is_null());
    let mut other = a.sign_in(DAVID).await;
    other.grant_sudo().await;
    for (method, suffix, body) in [
        (Method::GET, "", json!({})),
        (Method::PATCH, "", json!({"name":"stolen"})),
        (Method::DELETE, "", json!({})),
        (
            Method::PUT,
            "/access",
            json!({"roomIds":[],"capabilities":[]}),
        ),
        (Method::GET, "/credentials", json!({})),
        (Method::POST, "/credentials", json!({"name":"stolen"})),
    ] {
        let r = write(&mut other, method, &format!("{path}{suffix}"), body).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{}", r.text());
    }
    for suffix in [
        format!("/credentials/{cid}/rotate"),
        format!("/credentials/{cid}"),
    ] {
        let method = if suffix.ends_with("rotate") {
            Method::POST
        } else {
            Method::DELETE
        };
        assert_eq!(
            write(&mut other, method, &format!("{path}{suffix}"), json!({}))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let other_list: Value = parse(&other.send(get(ROOT)).await);
    assert!(
        !other_list["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|agent| agent["id"] == id)
    );
    let admin: Value = parse(&other.send(get("/api/v1/admin/bots")).await);
    assert!(admin["bots"].as_array().unwrap().iter().any(|b| {
        b["name"] == "Personal helper"
            && b["ownership"]
                .as_str()
                .unwrap()
                .contains("Personal agent · Owned by Kevin")
    }));
    let r = write(
        &mut member,
        Method::POST,
        &format!("{path}/credentials/{cid}/rotate"),
        json!({"name":"replacement","expiresAt":"never"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let rotated: Value = parse(&r);
    let new_secret = rotated["secret"].as_str().unwrap();
    assert_eq!(mcp(&a, secret).await, StatusCode::UNAUTHORIZED);
    assert_eq!(mcp(&a, new_secret).await, StatusCode::OK);
    let new_cid = rotated["credential"]["id"].as_i64().unwrap();
    assert!(rotated["credential"]["expiresAt"].is_null());
    assert_eq!(
        write(
            &mut member,
            Method::DELETE,
            &format!("{path}/credentials/{new_cid}"),
            json!({})
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(mcp(&a, new_secret).await, StatusCode::UNAUTHORIZED);
    let before_removal = write(
        &mut member,
        Method::POST,
        &format!("{path}/credentials"),
        json!({"name":"Remove me"}),
    )
    .await;
    assert_eq!(
        before_removal.status,
        StatusCode::CREATED,
        "{}",
        before_removal.text()
    );
    let before_removal: Value = parse(&before_removal);
    let removal_secret = before_removal["secret"].as_str().unwrap();
    assert_eq!(mcp(&a, removal_secret).await, StatusCode::OK);
    assert_eq!(
        write(&mut member, Method::DELETE, &path, json!({}))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(member.send(get(&path)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(mcp(&a, removal_secret).await, StatusCode::UNAUTHORIZED);
    let audits = a.db().read(|conn| {
        let mut statement=conn.prepare("SELECT action,details FROM audit_logs WHERE action IN ('agent.create','agent.credential.issue','agent.credential.rotate','agent.credential.revoke','agent.access.update') ORDER BY id")?;
        Ok(statement.query_map([],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    for action in [
        "agent.create",
        "agent.access.update",
        "agent.credential.issue",
        "agent.credential.rotate",
        "agent.credential.revoke",
    ] {
        assert!(
            audits.iter().any(|(event, _)| event == action),
            "missing audit {action}"
        );
    }
    assert!(
        !audits
            .iter()
            .any(|(_, details)| details.contains(secret) || details.contains(new_secret))
    );
    let bot = created["agent"]["userId"].as_i64().unwrap();
    a.db()
        .read(move |conn| {
            assert_eq!(
                campfire_db::User::find(conn, bot)?.status,
                campfire_db::Status::Deactivated
            );
            let agent = campfire_db::Agent::find(conn, id)?.unwrap();
            assert!(!agent.active(conn)?);
            assert!(
                campfire_db::User::find(conn, bot)?
                    .bot_token_digest
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn personal_agent_member_api_rejects_grants_and_rolls_back_creation() {
    let a = app().await.expect("restored seed");
    let mut b = a.sign_in(KEVIN).await;
    b.grant_sudo().await;
    let direct = a
        .db()
        .write(|tx| {
            Ok(campfire_db::Room::find_or_create_direct_for(tx, &[DAVID, KEVIN], KEVIN)?.id)
        })
        .await
        .unwrap();
    for body in [
        json!({"name":"bad","roomIds":[654632876],"capabilities":["read_messages"]}),
        json!({"name":"bad","roomIds":[direct],"capabilities":["read_messages"]}),
        json!({"name":"bad","roomIds":[ALL_TALK],"capabilities":["dm_anyone"]}),
        json!({"name":"bad","roomIds":[ALL_TALK],"capabilities":["external_action"]}),
        json!({"name":"bad","roomIds":[],"capabilities":["fizzy"]}),
    ] {
        let r = write(&mut b, Method::POST, ROOT, body).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
    }
    let list: Value = parse(&b.send(get(ROOT)).await);
    assert_eq!(list["agents"], json!([]));
    let created = create(&mut b).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let r = write(
        &mut b,
        Method::PUT,
        &format!("{ROOT}/{id}/access"),
        json!({"roomIds":[direct],"capabilities":["react"]}),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let shown: Value = parse(&b.send(get(&format!("{ROOT}/{id}"))).await);
    assert_eq!(shown["roomIds"], json!([ALL_TALK]));
}
#[tokio::test]
async fn personal_agent_member_api_sudo_csrf_toggle_and_limit() {
    let a = app().await.expect("restored seed");
    let mut b = a.sign_in(KEVIN).await;
    let body = json!({"name":"helper","roomIds":[],"capabilities":[]});
    assert_eq!(
        b.send(json_body(Method::POST, ROOT, &body)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let r = write(&mut b, Method::POST, ROOT, body.clone()).await;
    let error: Value = parse(&r);
    assert_eq!(error["error"]["_tag"], "SudoRequired");
    b.grant_sudo().await;
    let created = create(&mut b).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let mut admin = a.sign_in(DAVID).await;
    let r = write(
        &mut admin,
        Method::PATCH,
        "/api/v1/admin/workspace",
        json!({"memberAgentsEnabled":false,"memberAgentLimit":1}),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let workspace: Value = parse(&r);
    assert_eq!(workspace["memberAgentsEnabled"], false);
    assert_eq!(
        write(&mut b, Method::POST, ROOT, body.clone()).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        write(
            &mut b,
            Method::POST,
            &format!("{ROOT}/{id}/credentials"),
            json!({"name":"new"})
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        write(
            &mut admin,
            Method::PATCH,
            "/api/v1/admin/workspace",
            json!({"memberAgentsEnabled":true})
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        write(&mut b, Method::POST, ROOT, body).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut fresh = a.sign_in(KEVIN).await;
    for suffix in [
        "/credentials".to_string(),
        format!(
            "/credentials/{}/rotate",
            created["agent"]["credentials"][0]["id"].as_i64().unwrap()
        ),
    ] {
        let r = write(
            &mut fresh,
            Method::POST,
            &format!("{ROOT}/{id}{suffix}"),
            json!({"name":"another"}),
        )
        .await;
        let error: Value = parse(&r);
        assert_eq!(error["error"]["_tag"], "SudoRequired");
    }
}

#[tokio::test]
async fn personal_agent_member_api_credential_expiry_atomic_rotation_and_csrf() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let cid = created["agent"]["credentials"][0]["id"].as_i64().unwrap();
    let path = format!("{ROOT}/{id}");
    let secret = created["secret"].as_str().unwrap();
    let expected = crate::controllers::presenters::test_support::SEED_NOW
        .parse::<jiff::Timestamp>()
        .unwrap()
        .checked_add(jiff::SignedDuration::from_hours(90 * 24))
        .unwrap();
    assert_eq!(
        created["agent"]["credentials"][0]["expiresAt"],
        campfire_db::Timestamp::from_jiff(expected).to_wire()
    );
    for (method, suffix, body) in [
        (
            Method::PATCH,
            "".to_string(),
            json!({"name":"without csrf"}),
        ),
        (Method::DELETE, "".to_string(), json!({})),
        (
            Method::PUT,
            "/access".to_string(),
            json!({"roomIds":[],"capabilities":[]}),
        ),
        (
            Method::POST,
            "/credentials".to_string(),
            json!({"name":"without csrf"}),
        ),
        (
            Method::POST,
            format!("/credentials/{cid}/rotate"),
            json!({}),
        ),
        (Method::DELETE, format!("/credentials/{cid}"), json!({})),
    ] {
        assert_eq!(
            member
                .send(json_body(method, &format!("{path}{suffix}"), &body))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let issue_path = format!("{path}/credentials");
    for expires_at in ["yesterday", "2020-01-01T00:00:00Z"] {
        assert_eq!(
            write(
                &mut member,
                Method::POST,
                &issue_path,
                json!({"name":"expired","expiresAt":expires_at})
            )
            .await
            .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let issued = write(
        &mut member,
        Method::POST,
        &issue_path,
        json!({"name":"Second"}),
    )
    .await;
    assert_eq!(issued.status, StatusCode::CREATED, "{}", issued.text());
    let issued: Value = parse(&issued);
    assert_eq!(
        mcp(&a, issued["secret"].as_str().unwrap()).await,
        StatusCode::OK
    );
    let mut second = a.sign_in(KEVIN).await;
    second.grant_sudo().await;
    let rotate_path = format!("{path}/credentials/{cid}/rotate");
    let (first, later) = tokio::join!(
        write(&mut member, Method::POST, &rotate_path, json!({})),
        write(&mut second, Method::POST, &rotate_path, json!({})),
    );
    let mut statuses = [first.status, later.status];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::NOT_FOUND]);
    assert_eq!(mcp(&a, secret).await, StatusCode::UNAUTHORIZED);
    let list: Value = parse(&member.send(get(&issue_path)).await);
    assert_eq!(
        list["credentials"].as_array().unwrap().len(),
        3,
        "only one replacement"
    );
    let replaced = write(
        &mut member,
        Method::PUT,
        &format!("{path}/access"),
        json!({"roomIds":[],"capabilities":[]}),
    )
    .await;
    assert_eq!(replaced.status, StatusCode::OK, "{}", replaced.text());
    let profile: Value = parse(&replaced);
    assert_eq!(profile["roomIds"], json!([]));
    assert_eq!(profile["capabilities"], json!([]));
}

#[tokio::test]
async fn personal_agent_review_mcp_url_is_null_without_app_url_despite_forged_hosts() {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let a = TestApp::boot_seed_with_env("default", clock, &[])
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let mut member = a.sign_in(KEVIN).await;
    let mut advertised = Vec::new();
    for request in [
        get(ROOT),
        get(ROOT).header("host", "evil.example"),
        get(ROOT).header("x-forwarded-host", "evil.example"),
        get(ROOT)
            .header("host", "internal.example")
            .header("x-forwarded-host", "evil.example"),
    ] {
        let reply = member.send(request).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: Value = parse(&reply);
        advertised.push(list["mcpUrl"].clone());
    }
    assert!(
        advertised.iter().all(Value::is_null),
        "URLs from unconfigured requests: {advertised:?}"
    );
    let configured = app().await.expect("restored seed");
    let mut member = configured.sign_in(KEVIN).await;
    let reply = member
        .send(
            get(ROOT)
                .header("host", "evil.example")
                .header("x-forwarded-host", "another-evil.example"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let list: Value = parse(&reply);
    assert_eq!(list["mcpUrl"], "https://agents.example.test/agents/mcp");
}

#[tokio::test]
async fn personal_agent_review_profile_audit_records_member_and_changed_fields_atomically() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let path = format!("{ROOT}/{id}");
    let update = json!({"name":"Renamed helper","provider":"Local","runtime":"CLI","description":"Owner configured","dailyMessageCap":null});
    assert_eq!(
        write(&mut member, Method::PATCH, &path, update.clone())
            .await
            .status,
        StatusCode::OK
    );
    let rows=a.db().read(move |conn| {
        let mut statement=conn.prepare("SELECT actor_id,actor_label,target_type,target_id,target_label,details FROM audit_logs WHERE action='agent.update' AND target_id=? ORDER BY id")?;
        Ok(statement.query_map([id],|row| Ok(json!({"actorId":row.get::<_,i64>(0)?,"actorLabel":row.get::<_,String>(1)?,"targetType":row.get::<_,String>(2)?,"targetId":row.get::<_,i64>(3)?,"targetLabel":row.get::<_,String>(4)?,"changes":serde_json::from_str::<Value>(&row.get::<_,String>(5)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    assert_eq!(rows.len(), 1, "profile and budget changes need an audit");
    assert_eq!(rows[0]["actorId"], KEVIN);
    assert!(rows[0]["actorLabel"].as_str().unwrap().contains("Kevin"));
    assert_eq!(rows[0]["targetType"], "Agent");
    assert_eq!(rows[0]["targetId"], id);
    assert_eq!(rows[0]["targetLabel"], "Renamed helper");
    assert_eq!(
        rows[0]["changes"],
        json!({
            "name":{"before":"Personal helper","after":"Renamed helper"},
            "provider":{"before":null,"after":"Local"},
            "runtime":{"before":null,"after":"CLI"},
            "description":{"before":null,"after":"Owner configured"},
            "daily_message_cap":{"before":200,"after":null},
        })
    );
    // A no-op and a validation failure must not claim a change.
    assert_eq!(
        write(&mut member, Method::PATCH, &path, update)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        write(
            &mut member,
            Method::PATCH,
            &path,
            json!({"name":"Invalid rename","dailyMessageCap":0})
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    a.db()
        .read(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action='agent.update' AND target_id=?",
                [id],
                |r| r.get(0),
            )?;
            assert_eq!(count, 1);
            Ok(())
        })
        .await
        .unwrap();
    a.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_profile_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.update' BEGIN SELECT RAISE(ABORT,'audit unavailable'); END;")?;
        Ok(())
    }).await.unwrap();
    assert_eq!(
        write(
            &mut member,
            Method::PATCH,
            &path,
            json!({"name":"Must roll back","dailyMessageCap":50})
        )
        .await
        .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let profile: Value = parse(&member.send(get(&path)).await);
    assert_eq!(profile["name"], "Renamed helper");
    assert_eq!(profile["dailyMessageCap"], Value::Null);
}

#[tokio::test]
async fn personal_agent_s3_creation_timestamp_survives_updates() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let expected = a
        .db()
        .read(move |conn| {
            Ok(campfire_db::Agent::find(conn, id)?
                .unwrap()
                .created_at
                .to_wire())
        })
        .await
        .unwrap();
    assert_eq!(created["agent"]["createdAt"], expected);
    let path = format!("{ROOT}/{id}");
    let updated = write(&mut member, Method::PATCH, &path, json!({"name":"Renamed"})).await;
    assert_eq!(updated.status, StatusCode::OK);
    assert_eq!(parse::<Value>(&updated)["createdAt"], expected);
    let list: Value = parse(&member.send(get(ROOT)).await);
    assert_eq!(list["agents"][0]["createdAt"], expected);
}

#[tokio::test]
async fn personal_agent_s3_icon_url_matches_the_spa_avatar() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    assert_eq!(created["agent"].get("iconUrl"), Some(&Value::Null));
    let id = created["agent"]["id"].as_i64().unwrap();
    let bot = created["agent"]["userId"].as_i64().unwrap();
    a.db().write(|tx| {
        tx.conn().execute("INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES('personal-test','Personal test',?,?,?)", rusqlite::params![KEVIN,tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    for (icon, image) in [
        ("claude", true),
        ("personal-test", true),
        ("rocket", false),
        ("", false),
    ] {
        let updated = write(
            &mut member,
            Method::PATCH,
            &format!("{ROOT}/{id}"),
            json!({"iconName":icon}),
        )
        .await;
        assert_eq!(updated.status, StatusCode::OK, "{}", updated.text());
        let profile: Value = parse(&member.send(get(&format!("/api/v1/people/{bot}"))).await);
        let expected = profile["user"]["avatarIcon"]["imageUrl"].clone();
        assert_eq!(expected.is_string(), image, "avatar for {icon}: {profile}");
        assert_eq!(parse::<Value>(&updated)["iconUrl"], expected);
        let shown: Value = parse(&member.send(get(&format!("{ROOT}/{id}"))).await);
        assert_eq!(shown["iconUrl"], expected);
        let list: Value = parse(&member.send(get(ROOT)).await);
        assert_eq!(list["agents"][0]["iconUrl"], expected);
    }
    let created = write(
        &mut member,
        Method::POST,
        ROOT,
        json!({"name":"Branded","iconName":"claude","roomIds":[],"capabilities":[]}),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert!(parse::<Value>(&created)["agent"]["iconUrl"].is_string());
}

#[tokio::test]
async fn personal_agent_s3_room_choices_describe_privacy() {
    let a = app().await.expect("restored seed");
    let rooms = a
        .db()
        .write(|tx| {
            let mut rooms = Vec::new();
            for kind in [
                campfire_db::RoomType::Open,
                campfire_db::RoomType::Closed,
                campfire_db::RoomType::Voice,
                campfire_db::RoomType::Stage,
                campfire_db::RoomType::Board,
            ] {
                let room =
                    campfire_db::Room::create(tx, kind, Some(&format!("Privacy {kind:?}")), KEVIN)?;
                room.grant_to(tx, &[KEVIN])?;
                rooms.push((room.id, !room.open()));
            }
            Ok(rooms)
        })
        .await
        .unwrap();
    let mut member = a.sign_in(KEVIN).await;
    let list: Value = parse(&member.send(get(ROOT)).await);
    for (id, private) in rooms {
        let choice = list["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .expect("grantable room");
        assert_eq!(choice["private"], private, "{choice}");
    }
    assert!(
        !list["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == 654632876)
    );
}

#[tokio::test]
async fn personal_agent_s3_removal_returns_bot_user_id() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let bot = created["agent"]["userId"].as_i64().unwrap();
    assert_ne!(id, bot, "distinct agent and user identifiers");
    let removed = write(
        &mut member,
        Method::DELETE,
        &format!("{ROOT}/{id}"),
        json!({}),
    )
    .await;
    assert_eq!(removed.status, StatusCode::OK);
    assert_eq!(parse::<Value>(&removed)["id"], bot);
    assert_eq!(
        member.send(get(&format!("{ROOT}/{id}"))).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn personal_agent_s3_disabled_errors_are_readable() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let cid = created["agent"]["credentials"][0]["id"].as_i64().unwrap();
    let mut admin = a.sign_in(DAVID).await;
    let toggled = write(
        &mut admin,
        Method::PATCH,
        "/api/v1/admin/workspace",
        json!({"memberAgentsEnabled":false}),
    )
    .await;
    assert_eq!(toggled.status, StatusCode::OK);
    for (method, path, body) in [
        (
            Method::POST,
            ROOT.to_string(),
            json!({"name":"Disabled","roomIds":[],"capabilities":[]}),
        ),
        (
            Method::POST,
            format!("{ROOT}/{id}/credentials"),
            json!({"name":"Disabled"}),
        ),
        (
            Method::POST,
            format!("{ROOT}/{id}/credentials/{cid}/rotate"),
            json!({}),
        ),
        (
            Method::PUT,
            format!("{ROOT}/{id}/access"),
            json!({"roomIds":[],"capabilities":[]}),
        ),
    ] {
        let reply = write(&mut member, method, &path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        assert_eq!(
            parse::<Value>(&reply)["error"],
            json!({"_tag":"Forbidden","message":DISABLED_MESSAGE})
        );
    }
}

#[tokio::test]
async fn personal_agent_s3_limit_error_is_form_level() {
    let a = app().await.expect("restored seed");
    let mut admin = a.sign_in(DAVID).await;
    let changed = write(
        &mut admin,
        Method::PATCH,
        "/api/v1/admin/workspace",
        json!({"memberAgentLimit":1}),
    )
    .await;
    assert_eq!(changed.status, StatusCode::OK);
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    create(&mut member).await;
    let reply = write(
        &mut member,
        Method::POST,
        ROOT,
        json!({"name":"Over limit","roomIds":[],"capabilities":[]}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        parse::<Value>(&reply)["error"]["fields"],
        json!({"base":["personal agent limit reached"]})
    );
}

#[tokio::test]
async fn personal_agent_s3_toggle_refuses_and_restores_tokens_without_changing_grants() {
    let a = app().await.expect("restored seed");
    let mut member = a.sign_in(KEVIN).await;
    member.grant_sudo().await;
    let created = create(&mut member).await;
    let id = created["agent"]["id"].as_i64().unwrap();
    let secret = created["secret"].as_str().unwrap();
    let mut admin = a.sign_in(DAVID).await;
    admin.grant_sudo().await;
    let issued = write(
        &mut admin,
        Method::POST,
        "/api/v1/admin/bots/394959859/credentials",
        json!({"name":"Workspace control","expiresAt":null}),
    )
    .await;
    assert_eq!(issued.status, StatusCode::OK);
    let issued: Value = parse(&issued);
    let workspace_secret = issued["secret"].as_str().unwrap();
    let state = a.db().read(move |conn| {
        Ok((
            conn.query_row("SELECT json_group_array(json_object('id',id,'digest',token_digest,'revoked',revoked_at)) FROM agent_credentials WHERE agent_id=?", [id], |r| r.get::<_,String>(0))?,
            conn.query_row("SELECT json_group_array(json_object('id',id,'cap',capability,'room',room_id,'revoked',revoked_at)) FROM agent_grants WHERE agent_id=?", [id], |r| r.get::<_,String>(0))?,
        ))
    }).await.unwrap();
    for enabled in [true, false, true] {
        let toggled = write(
            &mut admin,
            Method::PATCH,
            "/api/v1/admin/workspace",
            json!({"memberAgentsEnabled":enabled}),
        )
        .await;
        assert_eq!(toggled.status, StatusCode::OK);
        assert_eq!(
            mcp(&a, secret).await,
            if enabled {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        );
        let workspace = a
            .anonymous()
            .send(
                Req::new(Method::POST, "/agents/mcp")
                    .header("authorization", &format!("Bearer {workspace_secret}"))
                    .json_body(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})),
            )
            .await;
        assert_eq!(workspace.status, StatusCode::OK, "{}", workspace.text());
        assert!(parse::<Value>(&workspace)["result"]["tools"].is_array());
        let profile: Value = parse(&member.send(get(&format!("{ROOT}/{id}"))).await);
        assert_eq!(
            profile["capabilities"],
            if enabled {
                json!(["post_messages", "read_messages"])
            } else {
                json!([])
            }
        );
        let unchanged = a.db().read(move |conn| {
            Ok((
                conn.query_row("SELECT json_group_array(json_object('id',id,'digest',token_digest,'revoked',revoked_at)) FROM agent_credentials WHERE agent_id=?", [id], |r| r.get::<_,String>(0))?,
                conn.query_row("SELECT json_group_array(json_object('id',id,'cap',capability,'room',room_id,'revoked',revoked_at)) FROM agent_grants WHERE agent_id=?", [id], |r| r.get::<_,String>(0))?,
            ))
        }).await.unwrap();
        assert_eq!(unchanged, state);
    }
}
#[tokio::test]
async fn personal_agent_access_gives_one_identical_error_for_any_room_the_owner_cannot_see() {
    use crate::controllers::presenters::test_support::{DIRECT_DAVID_JASON, QUIET_CORNER};
    let a = app().await.expect("restored seed");
    // A private room Kevin has left, so it exists but he can't see it.
    a.db()
        .write(|tx| {
            campfire_db::Room::find(tx.conn(), QUIET_CORNER)?.revoke_from(tx, &[KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.sign_in(KEVIN).await;
    b.grant_sudo().await;
    let id = create(&mut b).await["agent"]["id"].as_i64().unwrap();
    let mut bodies = Vec::new();
    for room in [999_999_999, QUIET_CORNER, DIRECT_DAVID_JASON] {
        let r = write(
            &mut b,
            Method::PUT,
            &format!("{ROOT}/{id}/access"),
            json!({"roomIds":[room],"capabilities":["read_messages"]}),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{room}: {}",
            r.text()
        );
        bodies.push(r.text());
    }
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(bodies[0], bodies[2]);
    assert!(!bodies[0].contains("existing room"), "{}", bodies[0]);
}

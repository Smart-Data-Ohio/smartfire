use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership};

async fn stage(test: &TestApp) -> (i64, i64, i64) {
    test.db().write(|tx| {
        tx.conn().execute_cached("UPDATE users SET role=0 WHERE id=?",[JASON])?;
        tx.conn().execute_cached("UPDATE rooms SET type='Rooms::Stage' WHERE id=?",[ALL_TALK])?;
        tx.conn().execute_cached("INSERT INTO memberships (room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![ALL_TALK,KEVIN,tx.now(),tx.now()])?;
        tx.conn().execute_cached("UPDATE memberships SET stage_role=CASE WHEN user_id=? THEN 'host' WHEN user_id=? THEN 'speaker' ELSE 'listener' END WHERE room_id=?",rusqlite::params![JASON,KEVIN,ALL_TALK])?;
        Ok((Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().id,Membership::find_by_room_and_user(tx.conn(),ALL_TALK,JASON)?.unwrap().id,Membership::find_by_room_and_user(tx.conn(),ALL_TALK,KEVIN)?.unwrap().id))
    }).await.unwrap()
}

#[tokio::test]
async fn call_moderation_security_denies_rank_self_and_outsiders_before_any_write() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let (admin, host, speaker) = stage(&test).await;
    for (actor, target, method, action, status, body) in [
        (
            JASON,
            admin,
            Method::POST,
            "mute",
            StatusCode::FORBIDDEN,
            "Only administrators can moderate an administrator",
        ),
        (
            JASON,
            admin,
            Method::DELETE,
            "mute",
            StatusCode::FORBIDDEN,
            "Only administrators can moderate an administrator",
        ),
        (
            JASON,
            admin,
            Method::POST,
            "disconnect",
            StatusCode::FORBIDDEN,
            "Only administrators can moderate an administrator",
        ),
        (
            JASON,
            host,
            Method::POST,
            "mute",
            StatusCode::UNPROCESSABLE_ENTITY,
            "You cannot moderate your own call session",
        ),
        (
            JASON,
            host,
            Method::DELETE,
            "mute",
            StatusCode::UNPROCESSABLE_ENTITY,
            "You cannot moderate your own call session",
        ),
        (
            JASON,
            host,
            Method::POST,
            "disconnect",
            StatusCode::UNPROCESSABLE_ENTITY,
            "You cannot moderate your own call session",
        ),
        (KEVIN, host, Method::POST, "mute", StatusCode::FORBIDDEN, ""),
        (
            KEVIN,
            host,
            Method::DELETE,
            "mute",
            StatusCode::FORBIDDEN,
            "",
        ),
        (
            KEVIN,
            host,
            Method::POST,
            "disconnect",
            StatusCode::FORBIDDEN,
            "",
        ),
        (DAVID, 0, Method::POST, "mute", StatusCode::NOT_FOUND, ""),
    ] {
        let mut browser = test.sign_in(actor).await;
        let reply = browser
            .write(Req::new(
                method,
                &format!("/rooms/{ALL_TALK}/call_moderation/{target}/{action}"),
            ))
            .await;
        assert_eq!(
            reply.status,
            status,
            "{actor} {target} {action}: {}",
            reply.text()
        );
        assert_eq!(reply.text(), body);
        assert!(
            test.db()
                .read(move |conn| Ok(Membership::find(conn, speaker)?.server_muted_at.is_none()))
                .await
                .unwrap()
        );
    }
    let mut anonymous = test.anonymous();
    let reply = anonymous
        .send(Req::new(
            Method::POST,
            &format!("/rooms/{ALL_TALK}/call_moderation/{speaker}/mute"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
}

#[tokio::test]
async fn stage_stream_start_requires_role_host_unmuted_and_seen_grant() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let (admin, host, speaker) = stage(&test).await;
    for (actor, status, message) in [
        (
            DAVID,
            StatusCode::FORBIDDEN,
            "Only hosts and speakers can go live",
        ),
        (
            JASON,
            StatusCode::FORBIDDEN,
            "Join the stage before going live",
        ),
        (
            KEVIN,
            StatusCode::FORBIDDEN,
            "Join the stage before going live",
        ),
    ] {
        let mut browser = test.sign_in(actor).await;
        let reply = browser
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/stage/stream"))
                    .form(&[("quality", "1080p15")]),
            )
            .await;
        assert_eq!(reply.status, status, "{}", reply.text());
        assert_eq!(reply.text(), message);
    }
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='speaker' WHERE id=?",
                [host],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = test.sign_in(KEVIN).await;
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/stage/stream"))
                .form(&[("quality", "1080p15")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.text(), "The stage needs a host to go live");
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='host' WHERE id=?",
                [host],
            )?;
            Membership::find(tx.conn(), speaker)?.server_mute(tx)?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/stage/stream"))
                .form(&[("quality", "1080p15")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.text(), "Muted members cannot go live");
    let mut listener = test.sign_in(DAVID).await;
    let reply = listener
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/{ALL_TALK}/stage/stream"),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::FOUND,
        "administrator member may stop even without a live stream"
    );
    assert!(
        test.db()
            .read(
                move |conn| Ok(
                    campfire_db::models::stream::Stream::live_for_room(conn, ALL_TALK)?.is_none()
                        && Membership::find(conn, admin)?.stage_role
                            == Some(campfire_db::StageRole::Listener)
                )
            )
            .await
            .unwrap()
    );
}

fn insert(
    tx: &campfire_db::Tx<'_>,
    table: &str,
    row: &serde_json::Value,
) -> campfire_db::Result<()> {
    use rusqlite::types::Value as SqlValue;
    let row = row.as_object().unwrap();
    let columns = row
        .keys()
        .map(|key| format!("\"{key}\""))
        .collect::<Vec<_>>()
        .join(",");
    let values = row
        .values()
        .map(|value| match value {
            serde_json::Value::Null => SqlValue::Null,
            serde_json::Value::Bool(v) => SqlValue::Integer(i64::from(*v)),
            serde_json::Value::Number(v) => SqlValue::Integer(v.as_i64().unwrap()),
            serde_json::Value::String(v) => SqlValue::Text(
                campfire_db::Timestamp::parse_db(v)
                    .map(|at| at.to_db())
                    .unwrap_or_else(|| v.clone()),
            ),
            _ => SqlValue::Text(value.to_string()),
        })
        .collect::<Vec<_>>();
    tx.conn().execute(
        &format!(
            "INSERT INTO {table}({columns}) VALUES({})",
            vec!["?"; values.len()].join(",")
        ),
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}

#[tokio::test]
async fn call_moderation_http_matches_twenty_nine_rails_requests() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../db/src/models/huddle_moderation_vectors.json"
    ))
    .unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 29);
    for case in vectors["cases"].as_array().unwrap() {
        let now = jiff::Timestamp::from_second(vectors["now"].as_i64().unwrap()).unwrap();
        let config = crate::huddle::Config::from_lookup(|name| {
            Some(
                match name {
                    "LIVEKIT_URL" => "wss://public.example.test",
                    "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880",
                    "LIVEKIT_API_KEY" => "ws13-fixture-api-key",
                    "LIVEKIT_API_SECRET" => "ws13-fixture-api-secret",
                    _ => "ws13-fixture-gateway-secret",
                }
                .into(),
            )
        });
        let Some(test) = TestApp::boot_with_huddle_and_clock(
            config,
            std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(now)),
        )
        .await
        else {
            return;
        };
        let input = case["input"].clone();
        test.db()
            .write(move |tx| {
                insert(tx, "rooms", &input["room"])?;
                for (table, key) in [
                    ("memberships", "memberships"),
                    ("huddle_grants", "grants"),
                    ("streams", "streams"),
                ] {
                    for row in input[key].as_array().unwrap() {
                        insert(tx, table, row)?;
                    }
                }
                for user in input["users"].as_array().unwrap() {
                    let role =
                        campfire_db::Role::from_name(user["role"].as_str().unwrap()).unwrap();
                    tx.conn().execute_cached(
                        "UPDATE users SET role=? WHERE id=?",
                        rusqlite::params![role, user["id"].as_i64()],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = test.sign_in(case["actor_id"].as_i64().unwrap()).await;
        let action = case["action"].as_str().unwrap();
        let target = case["target_id"].as_i64().unwrap();
        let accept = match case["format"].as_str().unwrap() {
            "html" => "text/html",
            "turbo_stream" => "text/vnd.turbo-stream.html",
            _ => "application/json",
        };
        let reply = browser
            .write(
                Req::new(
                    if action == "unmute" {
                        Method::DELETE
                    } else {
                        Method::POST
                    },
                    &format!(
                        "/rooms/9001/call_moderation/{target}/{}",
                        if action == "unmute" { "mute" } else { action }
                    ),
                )
                .header("accept", accept)
                .header("x-forwarded-proto", "https"),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}: {}",
            case["name"],
            reply.text()
        );
        assert_eq!(
            reply.content_type(),
            case["content_type"].as_str(),
            "{} content type",
            case["name"]
        );
        if let Some(body) = case["body"].as_str() {
            assert_eq!(reply.text(), body, "{}", case["name"]);
        }
        if let Some(location) = case["location"].as_str() {
            assert_eq!(reply.location(), Some(location), "{}", case["name"]);
        }
        if reply.status == StatusCode::OK {
            assert!(reply.text().contains("stage_roster_rooms_stage_9001"));
            assert!(
                reply.text().contains("name=\"authenticity_token\""),
                "public forms have the actor's CSRF token"
            );
        }
    }
}

#[tokio::test]
async fn stage_stream_http_start_conflict_and_stale_stop_preserve_the_new_presenter() {
    use campfire_db::models::{
        huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream,
    };
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let (_, host, speaker) = stage(&test).await;
    test.db()
        .write(move |tx| {
            for (user, member) in [(JASON, host), (KEVIN, speaker)] {
                let session = campfire_db::Session::start(tx, user, None, None)?;
                let mut grant = HuddleGrant::issue(
                    tx,
                    session.id,
                    member,
                    ALL_TALK,
                    &HuddleConfig {
                        api_secret: Some("ws13-fixture-api-secret".into()),
                        admin_configured: false,
                    },
                )?;
                grant.record_seen(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/rooms/{ALL_TALK}/stage/stream");
    let mut presenter = test.sign_in(KEVIN).await;
    let mut moderator = test.sign_in(JASON).await;
    let invalid = presenter
        .write(Req::new(Method::POST, &path).form(&[("quality", "4k60")]))
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(invalid.text(), "Unknown stream quality");
    let started = presenter
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("quality", "1080p30")]),
        )
        .await;
    assert_eq!(started.status, StatusCode::OK, "{}", started.text());
    let first = started.header("X-Stream-Id").unwrap().to_string();
    assert!(started.text().contains("Live: Kevin"));
    assert!(started.text().contains("Stop stream"));
    assert!(started.text().contains("name=\"authenticity_token\""));
    let duplicate = moderator
        .write(Req::new(Method::POST, &path).form(&[("quality", "720p15")]))
        .await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(duplicate.text(), "Kevin is already live");
    let ended = moderator
        .write(Req::new(Method::DELETE, &path).form(&[("stream_id", &first)]))
        .await;
    assert_eq!(ended.status, StatusCode::FOUND);
    let second = moderator
        .write(Req::new(Method::POST, &path).form(&[("quality", "720p15")]))
        .await;
    assert_eq!(second.status, StatusCode::FOUND);
    let second_id = second
        .header("X-Stream-Id")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let stale = moderator
        .write(Req::new(Method::DELETE, &path).form(&[("stream_id", &first)]))
        .await;
    assert_eq!(stale.status, StatusCode::FOUND);
    let numeric_prefix = moderator
        .write(
            Req::new(Method::DELETE, &path).form(&[("stream_id", &format!("{second_id}garbage"))]),
        )
        .await;
    assert_eq!(numeric_prefix.status, StatusCode::FOUND);
    assert_eq!(
        test.db()
            .read(move |conn| Ok(Stream::live_for_room(conn, ALL_TALK)?.unwrap().id))
            .await
            .unwrap(),
        second_id
    );
    let mut speaker_browser = test.sign_in(KEVIN).await;
    let denied = speaker_browser
        .write(Req::new(Method::DELETE, &path).form(&[("stream_id", &second_id.to_string())]))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    assert_eq!(
        denied.text(),
        "Only the presenter or a host can stop the stream"
    );
    let historical = moderator.write(Req::new(Method::DELETE, &path)).await;
    assert_eq!(historical.status, StatusCode::FOUND);
    assert!(
        test.db()
            .read(move |conn| Ok(Stream::live_for_room(conn, ALL_TALK)?.is_none()))
            .await
            .unwrap()
    );
}

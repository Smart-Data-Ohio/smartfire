use crate::controllers::presenters::test_support::{ALL_TALK, BENDER_KEY, Req, TestApp};
use axum::http::{Method, StatusCode};

fn configured() -> crate::huddle::Config {
    crate::huddle::Config {
        public_url: Some("wss://public.example.test".into()),
        internal_url: Some("http://internal.example.test:7880".into()),
        api_key: Some("ws13-fixture-api-key".into()),
        api_secret: Some("ws13-fixture-api-secret".into()),
        gateway_secret: Some("ws13-fixture-gateway-secret".into()),
    }
}

#[tokio::test]
async fn public_huddle_authentication_errors_precede_csrf_and_configuration() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let mut browser = test.anonymous();
    for (method, path) in [
        (Method::GET, format!("/rooms/{ALL_TALK}/huddle")),
        (Method::POST, format!("/rooms/{ALL_TALK}/huddle")),
        (
            Method::GET,
            format!("/rooms/{ALL_TALK}/huddle/participants"),
        ),
        (Method::POST, format!("/rooms/{ALL_TALK}/huddle/leave")),
        (Method::GET, "/users/huddle_presence".into()),
    ] {
        let reply = browser.send(Req::new(method.clone(), &path)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNAUTHORIZED,
            "{path}: {}",
            reply.text()
        );
        assert_eq!(
            reply.json(),
            serde_json::json!({"error":"Authentication required"})
        );
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        assert_eq!(reply.location(), None);
        let reply = browser
            .send(Req::new(method, &path).form(&[("bot_key", BENDER_KEY)]))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.text()
        );
        assert_eq!(
            reply.json(),
            serde_json::json!({"error":"Bots cannot join huddles"})
        );
        assert_eq!(reply.header("cache-control"), Some("no-store"));
    }
    let Some(unconfigured) = TestApp::boot().await else {
        return;
    };
    let reply = unconfigured
        .anonymous()
        .get(&format!("/rooms/{ALL_TALK}/huddle"))
        .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    let reply = unconfigured
        .david()
        .get(&format!("/rooms/{ALL_TALK}/huddle"))
        .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        reply.json(),
        serde_json::json!({"error":"Huddles are not configured"})
    );
    assert_eq!(reply.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn public_huddle_http_matches_production_rails() {
    use super::call_lifecycle_tests::insert;
    use campfire_db::models::huddle_grant::HuddleGrant;
    use campfire_db::{CachedStatements, Membership, Role, Status, Timestamp};
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("public_huddle_vectors.json")).unwrap();
    let uuid = regex::Regex::new(r"\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z").unwrap();
    let identity = regex::Regex::new(r"\Acampfire-participant-[0-9a-f]{64}\z").unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let now = jiff::Timestamp::from_second(vectors["now"].as_i64().unwrap()).unwrap();
        let mut config = configured();
        if case["unconfigured"] == true {
            config.api_secret = None;
        }
        if case["aliased"] == true {
            config.public_url = Some("wss://internal.example.test:7880/client".into());
        }
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
                for (table, key) in [("rooms", "rooms"), ("memberships", "memberships")] {
                    for row in input[key].as_array().unwrap() {
                        insert(tx, table, row)?;
                    }
                }
                for user in input["users"].as_array().unwrap() {
                    tx.conn().execute_cached(
                        "UPDATE users SET name=?,role=?,status=?,updated_at=? WHERE id=?",
                        rusqlite::params![
                            user["name"].as_str(),
                            Role::from_name(user["role"].as_str().unwrap()).unwrap(),
                            Status::from_name(user["status"].as_str().unwrap()).unwrap(),
                            Timestamp::parse_db(user["updated_at"].as_str().unwrap()).unwrap(),
                            user["id"].as_i64()
                        ],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let actor = case["actor_id"].as_i64();
        let mut browser = match actor {
            Some(id) => test.sign_in(id).await,
            None => test.anonymous(),
        };
        let actor_session = match actor {
            Some(id) => Some(
                test.db()
                    .read(move |conn| {
                        Ok(conn.query_row_cached(
                            "SELECT id FROM sessions WHERE user_id=? ORDER BY id DESC LIMIT 1",
                            [id],
                            |r| r.get::<_, i64>(0),
                        )?)
                    })
                    .await
                    .unwrap(),
            ),
            None => None,
        };
        let input = case["input"].clone();
        let original_session = case["actor_session_id"].as_i64();
        test.db()
            .write(move |tx| {
                for row in input["grants"].as_array().unwrap() {
                    let mut row = row.clone();
                    if row["session_id"].as_i64() == original_session
                        && let Some(id) = actor_session
                    {
                        row["session_id"] = id.into();
                    }
                    insert(tx, "huddle_grants", &row)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let method = if case["method"] == "post" {
            Method::POST
        } else {
            Method::GET
        };
        let params = case["params"].as_object().unwrap();
        let fields: Vec<_> = params
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect();
        let request = || {
            Req::new(method.clone(), case["path"].as_str().unwrap())
                .header("x-forwarded-proto", "https")
                .form(&fields)
        };
        let reply = if method == Method::POST && actor.is_some() {
            browser.write(request()).await
        } else {
            browser.send(request()).await
        };
        let reply = if case["repeat"] == true {
            browser.write(request()).await
        } else {
            reply
        };
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            reply.text()
        );
        assert_eq!(
            reply.header("cache-control"),
            case["cache_control"].as_str(),
            "{name}"
        );
        assert_eq!(
            reply.content_type(),
            case["content_type"].as_str(),
            "{name}"
        );
        if case["claims"].is_null() {
            if case["body"].is_null() {
                assert!(reply.body.is_empty(), "{name}");
            } else {
                assert_eq!(reply.json(), case["body"], "{name}");
            }
        } else {
            let body = reply.json();
            let token = body["token"].as_str().unwrap();
            let coordinates = rails_compat::jwt::livekit::verify(
                token,
                "ws13-fixture-api-key",
                "ws13-fixture-api-secret",
                now.as_second(),
            )
            .unwrap();
            let mut claims = rails_compat::jwt::decode(
                token,
                rails_compat::jwt::Key::Hs256(b"ws13-fixture-api-secret"),
                &rails_compat::jwt::Validation::default(),
                now.as_second(),
            )
            .unwrap()
            .payload;
            let mut expected = case["claims"].clone();
            let jti = claims.as_object_mut().unwrap().remove("jti").unwrap();
            assert!(uuid.is_match(jti.as_str().unwrap()));
            expected.as_object_mut().unwrap().remove("jti");
            expected["sub"] = body["identity"].clone();
            assert_eq!(claims, expected, "{name}");
            assert_eq!(body["room"], case["body"]["room"], "{name}");
            assert_eq!(body["url"], case["body"]["url"], "{name}");
            assert_eq!(coordinates.identity, body["identity"].as_str().unwrap());
            let id = body["grant_id"].as_i64().unwrap();
            let grant = test
                .db()
                .read(move |conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap()))
                .await
                .unwrap();
            assert_eq!(grant.session_id, actor_session.unwrap(), "{name}");
            assert_eq!(grant.user_id, actor.unwrap(), "{name}");
            assert_eq!(grant.room_id, 9001, "{name}");
            assert_eq!(grant.identity, coordinates.identity, "{name}");
            assert_eq!(grant.room_name, coordinates.room_name, "{name}");
            let membership = test
                .db()
                .read(move |conn| Membership::find(conn, grant.membership_id))
                .await
                .unwrap();
            assert_eq!(membership.user_id, actor.unwrap());
            if name == "join_reused" {
                assert_eq!(id, 17);
                assert_eq!(body["identity"], "ws13-public-identity-0");
            } else {
                assert!(identity.is_match(body["identity"].as_str().unwrap()));
            }
            let recipients = test.db().read(move |conn| {
                Ok(conn.prepare_cached("SELECT user_id FROM activity_items WHERE source_type='HuddleGrant' AND source_id=? AND event_type='huddle_started' ORDER BY user_id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
            }).await.unwrap();
            if name == "join_direct" {
                assert_eq!(
                    recipients,
                    vec![super::super::presenters::test_support::JASON]
                );
            } else if name == "join_group" {
                assert_eq!(
                    recipients,
                    vec![
                        super::super::presenters::test_support::JASON,
                        super::super::presenters::test_support::KEVIN
                    ]
                );
            } else {
                assert!(recipients.is_empty(), "{name}");
            }
        }
        if case["action"] == "leave" || reply.status != StatusCode::OK {
            for expected in case["grants"].as_array().unwrap() {
                let id = expected["id"].as_i64().unwrap();
                let grant = test
                    .db()
                    .read(move |conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap()))
                    .await
                    .unwrap();
                let timestamp = |key: &str| {
                    expected[key]
                        .as_str()
                        .map(|s| Timestamp::parse_db(s).unwrap())
                };
                assert_eq!(grant.last_seen_at, timestamp("last_seen_at"), "{name} {id}");
                assert_eq!(grant.revoked_at, timestamp("revoked_at"), "{name} {id}");
            }
        }
        test.booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
    }
}

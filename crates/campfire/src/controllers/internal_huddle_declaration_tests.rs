//! Remaining request assertions in test/controllers/internal/huddle_controller_test.rb.
use super::internal_huddle_tests::{bearer, config, grant, request};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, TestApp};
use axum::http::Method;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::{CachedStatements, Timestamp};
use serde_json::{Value, json};
const GATEWAY: &str = "ws13-fixture-gateway-secret";
const SECRET: &str = "ws13-fixture-api-secret";
fn signed(claims: Value) -> String {
    format!(
        "Bearer {}",
        rails_compat::jwt::encode_hs256(claims.as_object().unwrap(), SECRET.as_bytes())
    )
}
#[tokio::test]
async fn huddle_gateway_all_ninety_eight_recorded_token_shapes_execute_through_http() {
    let vectors: Value =
        serde_json::from_str(include_str!("../huddle/protocol_vectors.json")).unwrap();
    let now = jiff::Timestamp::from_second(vectors["now"].as_i64().unwrap()).unwrap();
    let Some(test) = TestApp::boot_with_huddle_and_clock(
        config(),
        std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(now)),
    )
    .await
    else {
        return;
    };
    let grant = grant(&test).await;
    let id = grant.id;
    let identity = vectors["identity"].as_str().unwrap().to_owned();
    let room_name = vectors["room_name"].as_str().unwrap().to_owned();
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET identity=?,room_name=? WHERE id=?",
                rusqlite::params![identity, room_name, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let expected =
        json!({"grant_id":id,"identity":vectors["identity"],"room_name":vectors["room_name"]});
    assert_eq!(vectors["shapes"].as_array().unwrap().len(), 98);
    for case in vectors["shapes"].as_array().unwrap() {
        let listener = case["name"].as_str().unwrap().contains("listener");
        test.db()
            .write(move |tx| {
                tx.conn().execute_cached(
                    "UPDATE rooms SET type=? WHERE id=?",
                    rusqlite::params![
                        if listener {
                            "Rooms::Stage"
                        } else {
                            "Rooms::Closed"
                        },
                        ALL_TALK
                    ],
                )?;
                tx.conn().execute_cached(
                    "UPDATE memberships SET stage_role=? WHERE room_id=? AND user_id=?",
                    rusqlite::params![listener.then_some("listener"), ALL_TALK, DAVID],
                )?;
                tx.conn().execute_cached(
                    "UPDATE huddle_grants SET stage_role=? WHERE id=?",
                    rusqlite::params![listener.then_some("listener"), id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let token = format!("Bearer {}", case["token"].as_str().unwrap());
        let (status, body) = request(
            &test,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&token),
            json!({}),
        )
        .await;
        if case["coordinates"].is_null() {
            assert_eq!((status, body), (401, Value::Null), "{}", case["name"]);
        } else {
            assert_eq!((status, body), (200, expected.clone()), "{}", case["name"]);
        }
    }
    // The two explicitly declared refreshed-listener omission variants.
    let token = vectors["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "listener")
        .unwrap()["token"]
        .as_str()
        .unwrap();
    let mut claims = rails_compat::jwt::decode(
        token,
        rails_compat::jwt::Key::Hs256(SECRET.as_bytes()),
        &rails_compat::jwt::Validation::default(),
        now.as_second(),
    )
    .unwrap()
    .payload;
    claims["video"]
        .as_object_mut()
        .unwrap()
        .remove("canPublishSources");
    for omit_publish in [false, true] {
        if omit_publish {
            claims["video"]
                .as_object_mut()
                .unwrap()
                .remove("canPublish");
        }
        assert_eq!(
            request(
                &test,
                Method::POST,
                "/internal/huddle/authorize",
                Some(GATEWAY),
                Some(&signed(claims.clone())),
                json!({})
            )
            .await,
            (200, expected.clone())
        );
    }
    let mut missing = claims;
    missing["sub"] = json!("campfire-participant-missing");
    assert_eq!(
        request(
            &test,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&signed(missing)),
            json!({})
        )
        .await,
        (403, Value::Null)
    );
}
#[derive(Debug)]
struct Clock(std::sync::atomic::AtomicI64);
impl campfire_kit::Clock for Clock {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::from_second(self.0.load(std::sync::atomic::Ordering::SeqCst)).unwrap()
    }
}
#[tokio::test]
async fn huddle_gateway_liveness_is_exact_throttled_and_independent_of_expired_tokens() {
    let now: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
    let clock = std::sync::Arc::new(Clock(std::sync::atomic::AtomicI64::new(now.as_second())));
    let Some(test) = TestApp::boot_with_huddle_and_clock(config(), clock.clone()).await else {
        return;
    };
    let grant = grant(&test).await;
    let id = grant.id;
    let expected = grant.authorization_payload();
    let path = format!("/internal/huddle/grants/{id}");
    assert!(grant.last_seen_at.is_none());
    assert_eq!(
        request(
            &test,
            Method::GET,
            &format!("{path}?record_seen=0"),
            Some(GATEWAY),
            None,
            json!({})
        )
        .await,
        (200, expected.clone())
    );
    assert!(
        test.db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?
                .unwrap()
                .last_seen_at
                .is_none()))
            .await
            .unwrap()
    );
    let token = bearer(&test, &grant, 0);
    assert_eq!(
        request(
            &test,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&token),
            json!({})
        )
        .await,
        (200, expected.clone())
    );
    let first = test
        .db()
        .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap().last_seen_at))
        .await
        .unwrap();
    assert_eq!(first, Some(Timestamp::from_jiff(now)));
    for elapsed in [0, 9, 11, 300] {
        clock.0.store(
            now.as_second() + elapsed,
            std::sync::atomic::Ordering::SeqCst,
        );
        assert_eq!(
            request(&test, Method::GET, &path, Some(GATEWAY), None, json!({})).await,
            (200, expected.clone())
        );
        let seen = test
            .db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap().last_seen_at))
            .await
            .unwrap();
        assert_eq!(
            seen,
            Some(Timestamp::from_second(
                now.as_second() + if elapsed < 10 { 0 } else { elapsed }
            ))
        );
    }
    assert_eq!(
        request(
            &test,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&token),
            json!({})
        )
        .await,
        (401, Value::Null)
    );
}
#[tokio::test]
async fn huddle_gateway_stale_links_revoke_and_enqueue_cleanup_for_both_lookup_modes() {
    for (authorize, record_seen) in [(true, true), (false, true), (false, false)] {
        let Some(test) = TestApp::boot_with_huddle(config()).await else {
            return;
        };
        let grant = grant(&test).await;
        let id = grant.id;
        let token = bearer(&test, &grant, 0);
        test.db()
            .write(move |tx| {
                tx.conn()
                    .execute_cached("DELETE FROM memberships WHERE id=?", [grant.membership_id])?;
                Ok(())
            })
            .await
            .unwrap();
        let path = if authorize {
            "/internal/huddle/authorize".to_owned()
        } else {
            format!(
                "/internal/huddle/grants/{id}{}",
                if record_seen { "" } else { "?record_seen=0" }
            )
        };
        assert_eq!(
            request(
                &test,
                if authorize { Method::POST } else { Method::GET },
                &path,
                Some(GATEWAY),
                authorize.then_some(token.as_str()),
                json!({})
            )
            .await,
            (if authorize { 403 } else { 404 }, Value::Null)
        );
        let (revoked,seen,cleanups)=test.db().read(move |c|{let g=HuddleGrant::find_by_id(c,id)?.unwrap();Ok((g.revoked(),g.last_seen_at,c.query_row_cached("SELECT COUNT(*) FROM huddle_cleanups WHERE huddle_grant_id=? AND operation='remove_participant'",[id],|r|r.get::<_,i64>(0))?))}).await.unwrap();
        assert!(revoked);
        assert!(seen.is_none());
        assert_eq!(cleanups, 1);
        assert_eq!(
            request(
                &test,
                Method::GET,
                &format!("/internal/huddle/grants/{id}"),
                Some(GATEWAY),
                None,
                json!({})
            )
            .await,
            (404, Value::Null)
        );
        assert!(
            test.db()
                .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?
                    .unwrap()
                    .last_seen_at
                    .is_none()))
                .await
                .unwrap()
        );
    }
}
#[tokio::test]
async fn huddle_gateway_disconnect_keeps_liveness_on_bad_or_stale_floors_and_clears_missing_floor()
{
    let now: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
    let Some(test) = TestApp::boot_with_huddle_and_clock(
        config(),
        std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(now)),
    )
    .await
    else {
        return;
    };
    let grant = grant(&test).await;
    let id = grant.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                rusqlite::params![tx.now(), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/internal/huddle/grants/{id}/left");
    for floor in ["2026-13-99", "not-a-timestamp"] {
        assert_eq!(
            request(
                &test,
                Method::POST,
                &path,
                Some(GATEWAY),
                None,
                json!({"disconnected_at":floor})
            )
            .await,
            (422, Value::Null)
        );
        assert_eq!(
            test.db()
                .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap().last_seen_at))
                .await
                .unwrap(),
            Some(Timestamp::from_jiff(now))
        );
    }
    assert_eq!(
        request(
            &test,
            Method::POST,
            &path,
            Some(GATEWAY),
            None,
            json!({"disconnected_at":"2026-03-02T15:59:00Z"})
        )
        .await,
        (200, Value::Null)
    );
    assert_eq!(
        test.db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap().last_seen_at))
            .await
            .unwrap(),
        Some(Timestamp::from_jiff(now))
    );
    assert_eq!(
        request(&test, Method::POST, &path, Some(GATEWAY), None, json!({})).await,
        (200, Value::Null)
    );
    let saved = test
        .db()
        .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap()))
        .await
        .unwrap();
    assert!(saved.last_seen_at.is_none());
    assert!(!saved.revoked());
    assert_eq!(
        request(
            &test,
            Method::POST,
            "/internal/huddle/grants/-1/left",
            Some(GATEWAY),
            None,
            json!({})
        )
        .await,
        (404, Value::Null)
    );
}
#[tokio::test]
async fn huddle_gateway_disconnect_refreshes_the_room_once_without_revocation() {
    let Some(test) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let grant = grant(&test).await;
    let id = grant.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(5)), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        request(
            &test,
            Method::POST,
            &format!("/internal/huddle/grants/{id}/left"),
            Some(GATEWAY),
            None,
            json!({"disconnected_at":test.booted.app.clock.now().to_string()})
        )
        .await,
        (200, Value::Null)
    );
    let saved = test
        .db()
        .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap()))
        .await
        .unwrap();
    assert!(saved.last_seen_at.is_none());
    assert!(!saved.revoked());
}

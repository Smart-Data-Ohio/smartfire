use super::call_channel_tests::configured;
use crate::controllers::presenters::{
    sql_probe::SqlProbe,
    test_support::{ALL_TALK, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Req, TestApp},
};
use axum::http::{Method, StatusCode};
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_db::{CachedStatements, Membership, Room, RoomType, Session};
use serde_json::{Value, json};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

async fn active(test: &TestApp, room: i64, user: i64) -> HuddleGrant {
    test.db()
        .write(move |tx| {
            let session = Session::start(tx, user, None, None)?;
            let member = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap();
            let grant = HuddleGrant::issue(
                tx,
                session.id,
                member.id,
                room,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: true,
                },
            )?;
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                rusqlite::params![tx.now(), grant.id],
            )?;
            Ok(grant)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn remaining_gateway_steady_state_has_no_transaction_and_denial_revokes_once() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let test = test.without_job_runner().await;
    let grant = active(&test, ALL_TALK, DAVID).await;
    let path = format!("/internal/huddle/grants/{}", grant.id);
    let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
    let (status, _) = crate::controllers::internal_huddle_tests::request(
        &test,
        Method::GET,
        &path,
        Some("ws13-fixture-gateway-secret"),
        None,
        Value::Null,
    )
    .await;
    let statements = probe.finish().await;
    assert_eq!(status, 200);
    assert!(statements.len() <= 8, "{statements:?}");
    for s in &statements {
        assert!(!s.transaction, "{statements:?}");
        assert!(s.sql.trim_start().starts_with("SELECT"), "{statements:?}");
    }
    let member = grant.membership_id;
    test.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("DELETE FROM memberships WHERE id=?", [member])?;
            Ok(())
        })
        .await
        .unwrap();
    let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
    let (status, _) = crate::controllers::internal_huddle_tests::request(
        &test,
        Method::GET,
        &path,
        Some("ws13-fixture-gateway-secret"),
        None,
        Value::Null,
    )
    .await;
    let statements = probe.finish().await;
    assert_eq!(status, 404);
    assert!(
        statements
            .iter()
            .any(|s| s.sql.starts_with("BEGIN IMMEDIATE")),
        "{statements:?}"
    );
    for prefix in ["UPDATE huddle_grants", "INSERT INTO huddle_cleanups"] {
        let writes: Vec<_> = statements
            .iter()
            .filter(|s| s.sql.starts_with(prefix))
            .collect();
        assert_eq!(writes.len(), 1, "{statements:?}");
        assert!(writes[0].transaction);
    }
    assert!(
        test.db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, grant.id)?.unwrap().revoked()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn remaining_presence_uses_one_grants_query_and_one_batched_user_preload() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let test = test.without_job_runner().await;
    let channel = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Closed, Some("Second"), DAVID, &[DAVID, JASON]))
        .await
        .unwrap();
    for (room, user) in [
        (ALL_TALK, DAVID),
        (ALL_TALK, JASON),
        (channel.id, DAVID),
        (DIRECT_DAVID_JASON, JASON),
    ] {
        active(&test, room, user).await;
    }
    let mut browser = test.sign_in(DAVID).await;
    let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
    let response = browser.get("/users/huddle_presence").await;
    let statements = probe.finish().await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(response.json().as_array().unwrap().len(), 3);
    let selects: Vec<_> = statements
        .iter()
        .filter(|s| s.sql.starts_with("SELECT"))
        .collect();
    assert_eq!(
        selects
            .iter()
            .filter(|s| s.sql.contains("FROM huddle_grants"))
            .count(),
        1,
        "{statements:?}"
    );
    assert_eq!(
        selects
            .iter()
            .filter(|s| s.sql.contains("FROM \"users\"") && s.sql.contains(" IN ("))
            .count(),
        1,
        "{statements:?}"
    );
}
#[tokio::test]
async fn remaining_stage_edit_checks_hosts_and_inserts_members_in_the_same_immediate_transaction() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let test = test.without_job_runner().await;
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                DAVID,
                &[DAVID, JASON],
            )
        })
        .await
        .unwrap();
    let mut browser = test.sign_in(DAVID).await;
    let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
    let response = browser
        .write(
            Req::new(Method::PUT, &format!("/rooms/stages/{}", room.id)).form(&[
                ("room[name]", "Town Hall"),
                ("user_ids[]", &DAVID.to_string()),
                ("user_ids[]", &JASON.to_string()),
                ("user_ids[]", &KEVIN.to_string()),
            ]),
        )
        .await;
    let statements = probe.finish().await;
    assert_eq!(response.status, StatusCode::FOUND);
    let begin = statements
        .iter()
        .position(|s| s.sql.starts_with("BEGIN IMMEDIATE"))
        .unwrap();
    let host_check = statements
        .iter()
        .enumerate()
        .find(|(i, s)| {
            *i > begin && s.sql.contains("FROM \"memberships\"") && s.sql.contains("room_id")
        })
        .unwrap();
    let insert = statements
        .iter()
        .enumerate()
        .find(|(_, s)| s.sql.starts_with("INSERT") && s.sql.contains("memberships"))
        .unwrap();
    let commit = statements
        .iter()
        .enumerate()
        .find(|(i, s)| *i > begin && s.sql.starts_with("COMMIT"))
        .unwrap();
    assert!(
        begin < host_check.0 && host_check.0 < insert.0 && insert.0 < commit.0,
        "{statements:?}"
    );
    assert!(host_check.1.transaction && insert.1.transaction);
    assert_eq!(
        test.db()
            .read(move |c| Room::find(c, room.id)?.user_ids(c))
            .await
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
async fn remaining_huddle_membership_revocation_between_scope_and_issue_is_a_controlled_denial() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let test = test.without_job_runner().await;
    let mut browser = test.sign_in(DAVID).await;
    let member = test
        .db()
        .read(|c| {
            Ok(Membership::find_by_room_and_user(c, ALL_TALK, DAVID)?
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let (arrived, arrival) = tokio::sync::oneshot::channel();
    let arrived = Mutex::new(Some(arrived));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let callback_gate = gate.clone();
    let armed = AtomicBool::new(true);
    let probe = SqlProbe::start(
        test.db(),
        test.booted.app.config.db_readers,
        Some(Arc::new(move |sql, transaction| {
            if !transaction
                && sql.starts_with("SELECT * FROM \"rooms\" WHERE \"rooms\".\"id\" = ?")
                && armed.swap(false, Ordering::SeqCst)
            {
                arrived.lock().unwrap().take().unwrap().send(()).unwrap();
                let (lock, wake) = &*callback_gate;
                let _guard = wake
                    .wait_while(lock.lock().unwrap(), |released| !*released)
                    .unwrap();
            }
        })),
    )
    .await;
    let (response, ()) = tokio::join!(
        browser.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle"))),
        async {
            tokio::time::timeout(std::time::Duration::from_secs(3), arrival)
                .await
                .unwrap()
                .unwrap();
            test.db()
                .write(move |tx| Membership::find(tx.conn(), member)?.destroy(tx))
                .await
                .unwrap();
            let (lock, wake) = &*gate;
            *lock.lock().unwrap() = true;
            wake.notify_all();
        }
    );
    let _ = probe.finish().await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(
        response.json(),
        json!({"error":"Room not found or inaccessible"})
    );
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(
        test.db()
            .read(|c| Ok(
                c.query_row_cached("SELECT COUNT(*) FROM huddle_grants", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}

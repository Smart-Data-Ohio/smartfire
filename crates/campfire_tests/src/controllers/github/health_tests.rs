use super::{
    card_tests::Fresh,
    test_support::{request, sudo},
};
use serde_json::{Value, json};
#[tokio::test]
async fn integration_health_http_security_rejects_members_and_signed_out_visitors() {
    let member = Fresh::new(&json!({"role":0})).await;
    assert_eq!(
        request(
            &member,
            "GET",
            "/account/integrations_health",
            Value::Null,
            sudo()
        )
        .await
        .0,
        403
    );
    let fresh = Fresh::new(&json!({})).await;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::get("/account/integrations_health")
                .header("Host", "example.org")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 302);
    assert_eq!(
        response.headers()["location"],
        "http://example.org/session/new"
    );
    assert!(fresh.server.received().is_empty());
}
#[tokio::test]
async fn integration_health_snapshots_and_complete_body_match_rails_and_http_never_renders_secrets()
{
    use crate::integrations::github::{
        accounts::{Account, AccountInput},
        tests::crypto,
    };
    use rusqlite::params;
    let fresh = Fresh::new(&json!({})).await;
    fresh.app.db.write(|tx|{
 let now=tx.now();
 tx.conn().execute("INSERT INTO users(id,name,role,created_at,updated_at) VALUES(813,'Machine',2,?,?)",params![now,now])?;
 tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(881,813,811,?,?)",params![now,now])?;
 let a=Account::create(tx,&crypto(),&AccountInput{user_id:811,github_login:"oracle",access_token:"fixture-secret",refresh_token:Some("fixture-refresh"),token_expires_at:None,token_source:"app"})?;
 tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason='revoked <account>',last_error='read &failed' WHERE id=?",[a.id])?;
 tx.conn().execute("INSERT INTO google_accounts(user_id,email,disconnected_reason,created_at,updated_at) VALUES(811,'oracle@example.test','expired <grant>',?,?)",params![now,now])?;
 tx.conn().execute("INSERT INTO calendar_push_channels(user_id,channel_id,token_digest,expires_at,last_error,created_at,updated_at) VALUES(811,'fixture-channel','fixture-digest',?,'watch <error>',?,?)",params![now.since(jiff::SignedDuration::from_hours(24)),now,now])?;
 for (status,age,error) in [("pending",0,Some("delivery <error>")),("failed",-86400,None),("failed",-86401,None)]{tx.conn().execute("INSERT INTO agent_events(agent_id,event_type,webhook_status,webhook_last_error,created_at) VALUES(881,'github_action_completed',?,?,?)",params![status,error,now.since(jiff::SignedDuration::from_secs(age))])?;}
 tx.conn().execute("UPDATE rooms SET inbound_email_token='fixture-address' WHERE id=815",[])?;
 tx.conn().execute("UPDATE rooms SET inbound_email_token='fixture-deleted',deleted_at=? WHERE id=825",[now])?;
 Ok(())}).await.unwrap();
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_health_page.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let case = case.clone();
        fresh
            .app
            .db
            .read(move |conn| {
                let snapshot = crate::integrations::health::snapshot(
                    conn,
                    campfire_db::Timestamp::from_jiff("2026-01-01T12:00:00Z".parse().unwrap()),
                    |key| case["config"][key].as_str().map(str::to_owned),
                )?;
                assert_eq!(snapshot, case["snapshot"]);
                let html = campfire_views::integration_health::body(
                    &snapshot,
                    &campfire_views::time::Zone::utc(),
                );
                assert_eq!(html, case["html"]);
                Ok(())
            })
            .await
            .unwrap();
    }
    let (status, _, body) = request(
        &fresh,
        "GET",
        "/account/integrations_health",
        Value::Null,
        sudo(),
    )
    .await;
    assert_eq!(status, 200);
    for name in [
        "Integration health",
        "GitHub",
        "Google Calendar",
        "Fizzy",
        "revoked &lt;account&gt;",
        "GITHUB_APP_CLIENT_ID",
        "GOOGLE_CALENDAR_WEBHOOK_URL",
        "INBOUND_EMAIL_DOMAIN",
    ] {
        assert!(body.contains(name), "{name}");
    }
    for secret in ["fixture-secret", "fixture-refresh", "fixture-digest"] {
        assert!(!body.contains(secret));
    }
    assert!(fresh.server.received().is_empty());
}

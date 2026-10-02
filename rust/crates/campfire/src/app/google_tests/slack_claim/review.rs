//! PR #195: signed identity selection and enrollment failure against pinned Rails.
use super::*;
use campfire_db::{GoogleIdentity, NewUser, Role};

fn oracle() -> Value {
    serde_json::from_str::<Value>(ORACLE).unwrap()["review"].clone()
}
fn response(reply: &Reply) -> Value {
    json!({"status":reply.status.as_u16(),"location":reply.location(),
        "content_type":reply.content_type(),"body":reply.text()})
}
async fn sigma_case(index: usize) {
    let expected = oracle()["claims"][index].clone();
    let name = expected["name"].as_str().unwrap().to_owned();
    let subject = format!("review-{name}");
    let (a, r, _slack) = app().await;
    let input = expected.clone();
    let (floor, placeholder, rival) = a.db().write(move |tx| {
        let floor: i64 = tx.conn().query_row("SELECT MAX(id) FROM users", [], |row| row.get(0))?;
        let user = campfire_db::User::create_slack_placeholder(tx, NewUser {
            name: "Sigma placeholder".into(), email_address: Some(input["placeholder"].as_str().unwrap().into()),
            ..Default::default()
        }, !input["deactivated"].as_bool().unwrap_or(false), None, true)?;
        let rival = if let Some(email) = input["rival"].as_str() {
            let rival = campfire_db::User::create(tx, NewUser {
                name: "Sigma administrator".into(), email_address: Some(email.into()), role: Role::Administrator,
                ..Default::default()
            })?;
            tx.conn().execute("UPDATE users SET google_email_link_allowed=1 WHERE id=?", [rival.id])?;
            Some(rival.id)
        } else { None };
        Ok((floor, user.id, rival))
    }).await.unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    answer(&r, claims(&a, &q, &subject, expected["email"].as_str().unwrap()));
    let reply = callback(&mut b, &q["state"]).await;
    assert_eq!(response(&reply), expected["response"], "{name}: callback bytes");
    let (owner, rows, sessions) = a.db().read(move |conn| {
        let identity = GoogleIdentity::for_subject(conn, &subject)?;
        let owner = match identity.map(|identity| identity.user_id) {
            None => "none", Some(id) if id == placeholder => "placeholder",
            Some(id) if Some(id) == rival => "administrator", Some(_) => "new_user",
        };
        let rows = conn.prepare("SELECT users.name,users.email_address,users.role,users.status,google_identities.subject FROM users LEFT JOIN google_identities ON google_identities.user_id=users.id WHERE users.id>? ORDER BY users.id")?
            .query_map([floor], |row| Ok(json!({"name":row.get::<_,String>(0)?,"email_address":row.get::<_,Option<String>>(1)?,
                "role":row.get::<_,i64>(2)?,"status":row.get::<_,i64>(3)?,"subject":row.get::<_,Option<String>>(4)?})))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let sessions: i64 = conn.query_row("SELECT COUNT(*) FROM sessions WHERE user_id IN (SELECT id FROM users WHERE id>?)", [floor], |row| row.get(0))?;
        Ok((owner, rows, sessions))
    }).await.unwrap();
    println!("Google identity {name}: owner={owner}; users={}; sessions={sessions}", rows.len());
    assert_eq!(json!(owner), expected["owner"], "{name}: identity owner");
    assert_eq!(json!(rows), expected["users"], "{name}: account rows");
    assert_eq!(json!(sessions), expected["sessions"], "{name}: session count");
}

#[tokio::test]
async fn google_sigma_claim_matches_rails_without_a_rival() { sigma_case(0).await; }
#[tokio::test]
async fn google_sigma_claim_matches_rails_with_an_eligible_administrator() { sigma_case(1).await; }
#[tokio::test]
async fn google_sigma_deactivated_predecessor_matches_rails() { sigma_case(2).await; }

#[tokio::test]
async fn google_enrollment_audit_failure_matches_rails_over_http() {
    let expected = oracle()["enrollment_audit_failure"].clone();
    let (a, r, _slack) = app().await;
    let user = a.db().write(|tx| campfire_db::User::create_slack_placeholder(tx, NewUser {
        name: "Enrollment review".into(), email_address: Some("enrollment-review@smartdata.net".into()),
        ..Default::default()
    }, true, None, true)).await.unwrap();
    let id = user.id;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    answer(&r, claims(&a, &q, "review-enrollment", "enrollment-review@smartdata.net"));
    assert_eq!(callback(&mut b, &q["state"]).await.status, StatusCode::FOUND);
    let verified: i64 = a.db().read(move |conn| Ok(conn.query_row("SELECT COUNT(*) FROM sessions WHERE user_id=? AND two_factor_verified_at IS NOT NULL", [id], |row| row.get(0))?)).await.unwrap();
    assert_eq!(verified, 0, "real Google first factor starts unverified");
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    a.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER refuse_review_enable BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' BEGIN SELECT RAISE(ABORT,'review audit unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    let code = rails_compat::totp::at("JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP", a.booted.app.clock.now().as_second()).unwrap();
    let reply = b.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)])).await;
    assert_eq!(response(&reply), expected["response"], "enrollment failure response bytes");
    let state = a.db().read(move |conn| {
        let enrolled = campfire_db::User::find(conn, id)?.two_factor_enabled(conn)?;
        let count = |sql| conn.query_row(sql, [id], |row| row.get::<_,i64>(0));
        Ok(json!({"enrolled":enrolled,
            "backups":count("SELECT COUNT(*) FROM two_factor_backup_codes JOIN two_factor_credentials ON two_factor_credentials.id=two_factor_backup_codes.two_factor_credential_id WHERE user_id=?")?,
            "pending_secrets":count("SELECT COUNT(*) FROM two_factor_setup_secrets JOIN sessions ON sessions.id=two_factor_setup_secrets.session_id WHERE user_id=?")?,
            "verified_sessions":count("SELECT COUNT(*) FROM sessions WHERE user_id=? AND two_factor_verified_at IS NOT NULL")?,
            "sessions":count("SELECT COUNT(*) FROM sessions WHERE user_id=?")?,
            "audits":count("SELECT COUNT(*) FROM audit_logs WHERE action='two_factor.enable' AND target_id=?")?}))
    }).await.unwrap();
    println!("Google enrollment audit failure: {state}");
    assert_eq!(state, expected["state"], "enrollment audit failure: retained rows");
    assert_eq!(response(&b.get("/two_factor_setup").await), expected["after"], "enrollment audit failure: next request");
}

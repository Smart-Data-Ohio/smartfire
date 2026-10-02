//! Rails' individual enrollment commit boundaries, exercised through signed HTTP.
use super::*;
use campfire_db::{
    NewSession, NewUser, Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorSetupSecret,
};

const BOUNDARIES: &str =
    include_str!("../../../../../../vectors/slack/google_enrollment_boundaries.json");
thread_local! {
    static STATEMENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn record(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
        STATEMENTS.with(|rows| rows.borrow_mut().push(sql.into()));
    }
}
fn transactions(statements: Vec<String>) -> Vec<Value> {
    let stages = [
        ("INSERT INTO TWO_FACTOR_CREDENTIALS", "credential.insert"),
        ("UPDATE TWO_FACTOR_CREDENTIALS", "credential.confirm"),
        ("DELETE FROM TWO_FACTOR_SETUP_SECRETS", "setup.delete"),
        ("UPDATE TWO_FACTOR_SETUP_SECRETS", "setup.refresh"),
        ("DELETE FROM TWO_FACTOR_BACKUP_CODES", "backup.delete"),
        ("INSERT INTO TWO_FACTOR_BACKUP_CODES", "backup.insert"),
        ("UPDATE SESSIONS", "session.verify"),
        ("DELETE FROM SESSIONS", "session.destroy"),
        ("DELETE FROM WORKSPACE_PRESENCE_LEASES", "presence.delete"),
        ("INSERT INTO AUDIT_LOGS", "audit.insert"),
    ];
    let mut result = Vec::new();
    let mut writes = Vec::new();
    for sql in statements {
        let sql = sql.replace('"', "").to_ascii_uppercase();
        if sql.starts_with("BEGIN") {
            writes.clear();
        } else if sql.starts_with("COMMIT") || sql.starts_with("ROLLBACK") {
            if !writes.is_empty() {
                result.push(json!({"outcome":if sql.starts_with("COMMIT") {"commit"} else {"rollback"},"writes":writes}));
                writes.clear();
            }
        } else if let Some((_, stage)) = stages.iter().find(|(prefix, _)| sql.starts_with(prefix)) {
            writes.push(*stage);
        }
    }
    result
}
fn response(reply: &Reply) -> Value {
    json!({"status":reply.status.as_u16(),"location":reply.location(),"content_type":reply.content_type(),"body":reply.text()})
}
async fn boundary(name: &'static str) {
    let oracle: Value = serde_json::from_str(BOUNDARIES).unwrap();
    let expected = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
        .clone();
    let (a, r, _slack) = app().await;
    let fixture = expected.clone();
    let crypto = a.booted.app.ar_encryption.clone();
    let user = a
        .db()
        .write(move |tx| {
            let seq = fixture["user_id"].as_i64().unwrap() - 1;
            tx.conn()
                .execute("UPDATE sqlite_sequence SET seq=? WHERE name='users'", [seq])?;
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=? WHERE name='sessions'",
                [fixture["current_id"].as_i64().unwrap() - 1],
            )?;
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=? WHERE name='two_factor_credentials'",
                [fixture["credential_seq"].as_i64().unwrap()],
            )?;
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=? WHERE name='two_factor_setup_secrets'",
                [fixture["setup_seq"].as_i64().unwrap()],
            )?;
            tx.conn()
                .execute("DELETE FROM two_factor_backup_codes", [])?;
            let user = campfire_db::User::create_slack_placeholder(
                tx,
                NewUser {
                    name: "Enrollment boundary".into(),
                    email_address: Some(format!("boundary-{name}@smartdata.net")),
                    ..Default::default()
                },
                true,
                None,
                true,
            )?;
            if fixture["existing"] == true {
                let credential =
                    TwoFactorCredential::create(tx, &crypto, user.id, "KRUGS4ZANFZSAYJA")?;
                for code in ["oldcodeone", "oldcodetwo"] {
                    TwoFactorBackupCode::create(
                        tx,
                        credential.id,
                        &TwoFactorBackupCode::digest(code),
                    )?;
                }
            }
            Ok(user)
        })
        .await
        .unwrap();
    let id = user.id;
    assert_eq!(json!(id), expected["user_id"]);
    let mut browser = a.anonymous();
    browser.get("/session/new").await;
    let q = start(&mut browser, "/session/google").await;
    answer(
        &r,
        claims(
            &a,
            &q,
            &format!("boundary-{name}"),
            &format!("boundary-{name}@smartdata.net"),
        ),
    );
    assert_eq!(
        callback(&mut browser, &q["state"]).await.status,
        StatusCode::FOUND
    );
    assert_eq!(
        browser.get("/two_factor_setup").await.status,
        StatusCode::OK
    );
    let fixture = expected.clone();
    let crypto = a.booted.app.ar_encryption.clone();
    a.db()
        .write(move |tx| {
            assert_eq!(
                Session::for_user(tx.conn(), id)?.last().unwrap().id,
                fixture["current_id"].as_i64().unwrap()
            );
            for device in ["other-1", "other-2"] {
                let session = Session::start_with(
                    tx,
                    id,
                    NewSession {
                        device_id: Some(device),
                        ..Default::default()
                    },
                )?;
                TwoFactorSetupSecret::issue_for(tx, &crypto, session.id)?;
            }
            if let Some(trigger) = fixture["trigger"].as_str() {
                tx.conn().execute_batch(
                    &trigger
                        .replace("{user}", &id.to_string())
                        .replace("{current}", &fixture["current_id"].to_string()),
                )?;
            }
            STATEMENTS.with(|rows| rows.borrow_mut().clear());
            tx.conn().trace_v2(
                rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                Some(record),
            );
            Ok(())
        })
        .await
        .unwrap();
    let code = if name == "wrong_code" {
        "invalid".into()
    } else {
        rails_compat::totp::at(
            "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP",
            a.booted.app.clock.now().as_second(),
        )
        .unwrap()
    };
    let reply = with_fixed_render_secrets(
        browser.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)])),
    )
    .await;
    let statements = a
        .db()
        .write(|tx| {
            tx.conn()
                .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
            Ok(STATEMENTS.with(|rows| rows.take()))
        })
        .await
        .unwrap();
    let current = expected["current_id"].as_i64().unwrap();
    let existing = expected["existing"] == true;
    let crypto = a.booted.app.ar_encryption.clone();
    let state = a.db().read(move |conn| {
        let credential = TwoFactorCredential::for_user(conn, id)?;
        let credentials = conn.prepare("SELECT confirmed_at,last_totp_at,created_at,updated_at FROM two_factor_credentials WHERE user_id=?")?.query_map([id],|row|Ok(json!({"confirmed_at":row.get::<_,Option<String>>(0)?,"last_totp_at":row.get::<_,Option<i64>>(1)?,"created_at":row.get::<_,String>(2)?,"updated_at":row.get::<_,String>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let decryption = credential.map(|c| {
            let secret = c.secret(&crypto)?;
            Ok::<_,campfire_db::Error>(json!({"secret_present":!secret.is_empty(),"setup_secret_matches":c.enabled().then(||secret=="JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP"),"prior_secret_preserved":existing.then(||secret=="KRUGS4ZANFZSAYJA")}))
        }).transpose()?;
        let backups=conn.prepare("SELECT code_digest,used_at FROM two_factor_backup_codes WHERE two_factor_credential_id IN (SELECT id FROM two_factor_credentials WHERE user_id=?) ORDER BY code_digest")?.query_map([id],|row|Ok(json!({"code_digest":row.get::<_,String>(0)?,"used_at":row.get::<_,Option<String>>(1)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let sessions=conn.prepare("SELECT CASE WHEN id=? THEN 'current' ELSE device_id END,two_factor_verified_at,last_active_at FROM sessions WHERE user_id=? ORDER BY id")?.query_map(rusqlite::params![current,id],|row|Ok(json!({"label":row.get::<_,String>(0)?,"two_factor_verified_at":row.get::<_,Option<String>>(1)?,"last_active_at":row.get::<_,String>(2)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let pending=conn.prepare("SELECT CASE WHEN sessions.id=? THEN 'current' ELSE sessions.device_id END,expires_at FROM two_factor_setup_secrets JOIN sessions ON sessions.id=two_factor_setup_secrets.session_id WHERE user_id=? ORDER BY sessions.id")?.query_map(rusqlite::params![current,id],|row|Ok(json!({"label":row.get::<_,String>(0)?,"expires_at":row.get::<_,String>(1)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let audits=conn.prepare("SELECT details FROM audit_logs WHERE action='two_factor.enable' AND target_id=?")?.query_map([id],|row|row.get::<_,Option<String>>(0))?.map(|row|row?.map(|s|serde_json::from_str::<Value>(&s).map_err(|error| campfire_db::Error::Other(error.to_string()))).transpose()).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(json!({"credentials":credentials,"decryption":decryption,"backups":backups,"sessions":sessions,"pending":pending,"audits":audits}))
    }).await.unwrap();
    let groups = transactions(statements);
    println!(
        "Rust enrollment boundary {name}: status={}; credentials={}; backups={}; pending={}; verified={}; sessions={}; write transactions={}",
        reply.status.as_u16(),
        state["credentials"].as_array().unwrap().len(),
        state["backups"].as_array().unwrap().len(),
        state["pending"].as_array().unwrap().len(),
        state["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| !s["two_factor_verified_at"].is_null())
            .count(),
        state["sessions"].as_array().unwrap().len(),
        groups.len()
    );
    assert_eq!(
        response(&reply),
        expected["response"],
        "{name}: complete response"
    );
    assert_eq!(state, expected["state"], "{name}: retained rows");
    assert_eq!(
        json!(groups),
        expected["transactions"],
        "{name}: write transaction boundaries"
    );
    a.db()
        .write(move |tx| {
            if name != "success" && name != "wrong_code" {
                tx.conn().execute_batch("DROP TRIGGER boundary_fail")?;
            }
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        response(&with_fixed_render_secrets(browser.get("/two_factor_setup")).await),
        expected["after"],
        "{name}: next setup response"
    );
}
macro_rules! cases {
    ($($function:ident => $name:literal),* $(,)?) => { $(#[tokio::test] async fn $function() { boundary($name).await; })* };
}
cases! {
    google_enrollment_boundary_credential_insert => "credential_insert",
    google_enrollment_boundary_confirm_update => "confirm_update",
    google_enrollment_boundary_setup_delete => "setup_delete",
    google_enrollment_boundary_backup_delete => "backup_delete",
    google_enrollment_boundary_backup_insert => "backup_insert",
    google_enrollment_boundary_backup_replace_insert => "backup_replace_insert",
    google_enrollment_boundary_session_verify => "session_verify",
    google_enrollment_boundary_session_destroy_first => "session_destroy_first",
    google_enrollment_boundary_session_destroy_second => "session_destroy_second",
    google_enrollment_boundary_session_setup_delete_second => "session_setup_delete_second",
    google_enrollment_boundary_audit => "audit",
    google_enrollment_boundary_success => "success",
    google_enrollment_boundary_wrong_code => "wrong_code",
}

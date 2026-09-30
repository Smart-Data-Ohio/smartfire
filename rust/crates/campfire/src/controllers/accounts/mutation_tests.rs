use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::models::audit_log::AuditLog;
use campfire_db::{Account, User};

#[tokio::test]
async fn account_and_ban_mutations_authorize_before_writes_and_audits() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut member = app.sign_in(KEVIN).await;
    let (count, before_account, before_user) = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                Account::first(conn)?.unwrap(),
                User::find(conn, KEVIN)?,
            ))
        })
        .await
        .unwrap();
    for (method, path) in [
        (Method::PUT, "/account"),
        (Method::PUT, "/account/custom_styles"),
        (Method::POST, "/account/join_code"),
        (Method::PUT, "/account/users/712064548"),
        (Method::DELETE, "/account/users/712064548"),
        (Method::DELETE, "/account/logo"),
        (Method::POST, "/users/712064548/ban"),
        (Method::DELETE, "/users/712064548/ban"),
    ] {
        let response = member
            .write(Req::new(method, path).form(&[
                ("account[name]", "must not save"),
                ("account[custom_styles]", "must not save"),
                ("user[role]", "administrator"),
            ]))
            .await;
        assert_eq!(response.status, StatusCode::FORBIDDEN, "{path}");
    }
    let mut admin = app.david();
    for (method, path) in [
        (Method::PUT, "/account/custom_styles"),
        (Method::POST, "/account/join_code"),
        (Method::PUT, "/account/users/712064548"),
        (Method::DELETE, "/account/users/712064548"),
        (Method::POST, "/users/712064548/ban"),
        (Method::DELETE, "/users/712064548/ban"),
    ] {
        let response = admin
            .write(Req::new(method, path).form(&[
                ("account[custom_styles]", "must not save"),
                ("user[role]", "administrator"),
            ]))
            .await;
        assert_eq!(
            response.location(),
            Some("http://campfire.test/sudo/new"),
            "{path}"
        );
    }
    let (after, account, user) = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                Account::first(conn)?.unwrap(),
                User::find(conn, KEVIN)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(after, count);
    assert_eq!(account, before_account);
    assert_eq!(user, before_user);
}

#[tokio::test]
async fn account_mutations_and_audits_match_pinned_rails_http_vectors() {
    run_mutation_cases(None).await;
}

async fn run_mutation_cases(selected: Option<&str>) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_account_mutations.json"
    ))
    .unwrap();
    let mut browser = app.david();
    let confirmed = browser
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(confirmed.status, StatusCode::FOUND);
    for case in cases["cases"].as_array().unwrap().iter().filter(|case| selected.is_none_or(|name| case["name"] == name)) {
        let setup = case.clone();
        let isolated_unban = selected == Some("unban_banned");
        let (old_code,active)=app.db().write(move |tx| {
            tx.conn().execute("UPDATE accounts SET name='Signal',settings='{\"restrict_room_creation_to_administrators\":false}',custom_styles=?",[setup["styles_before"].as_str()])?;
            tx.conn().execute("UPDATE users SET role=0,status=?,theme=? WHERE id=?",rusqlite::params![setup["status_before"].as_i64().unwrap_or(0),setup["theme_before"].as_str().unwrap(),KEVIN])?;
            if let Some(ip)=setup["session_ip"].as_str() {tx.conn().execute("UPDATE sessions SET ip_address=? WHERE user_id=?",rusqlite::params![ip,KEVIN])?;}
            if setup["name"]=="ban_active" {campfire_db::Session::start(tx,KEVIN,Some("ws8br2-target"),Some("203.0.113.43"))?;}
            // In the Rails sequence this subject has already been banned, destroying sessions.
            if isolated_unban {tx.conn().execute("DELETE FROM sessions WHERE user_id=?",[KEVIN])?;}
            tx.conn().execute("DELETE FROM audit_logs",[])?;
            Ok((Account::first(tx.conn())?.unwrap().join_code,tx.conn().query_row("SELECT COUNT(*) FROM users WHERE status=0",[],|r|r.get::<_,i64>(0))?))
        }).await.unwrap();
        let response = browser
            .write(
                Req::new(
                    case["method"].as_str().unwrap().parse().unwrap(),
                    case["path"].as_str().unwrap(),
                )
                .header("content-type", "application/json")
                .header("accept", "text/html")
                .header("user-agent", "ws8br2-account-fixture")
                .header("x-forwarded-for", "127.0.0.1")
                .body(serde_json::to_vec(&case["params"]).unwrap()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}",
            case["name"]
        );
        assert_eq!(
            response.location(),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        let path=case["path"].as_str().unwrap();
        let subject_id=if path.starts_with("/account/users/") || path.starts_with("/users/") {
            path.split('/').find_map(|part|part.parse::<i64>().ok())
        } else {None};
        let (state,rows)=app.db().read(move |conn|{
            let account=Account::first(conn)?.unwrap();
            let user=User::find(conn,KEVIN)?;
            let subject=subject_id.map(|id|User::find(conn,id)).transpose()?;
            let after_active=conn.query_row("SELECT COUNT(*) FROM users WHERE status=0",[],|r|r.get::<_,i64>(0))?;
            let target_sessions=campfire_db::Session::count_for_user(conn,KEVIN)?;
            let banned_ips=campfire_db::Ban::for_user(conn,KEVIN)?.into_iter().map(|b|b.ip_address).collect::<Vec<_>>();
            let mut stmt=conn.prepare("SELECT id FROM audit_logs ORDER BY id")?;
            let ids=stmt.query_map([],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
            let rows=ids.into_iter().map(|id|AuditLog::find(conn,id).map(|r|r.snapshot())).collect::<campfire_db::Result<Vec<_>>>()?;
            Ok((serde_json::json!({"account_name":account.name,"restrict":account.settings().restrict_room_creation_to_administrators(),"styles":account.custom_styles,"code_changed":account.join_code!=old_code,"role":user.role.name(),"status":user.status.name(),"subject_status":subject.as_ref().map(|u|u.status.name()),"subject_role":subject.as_ref().map(|u|u.role.name()),"active_delta":after_active-active,"target_sessions":target_sessions,"banned_ips":banned_ips}),rows))
        }).await.unwrap();
        assert_eq!(state, case["state"], "{}", case["name"]);
        if selected.is_some() {
            assert_eq!(rows.len(),1,"{}: exactly one audit row",case["name"]);
        }
        assert_eq!(
            serde_json::json!(rows),
            case["audits"],
            "{}: complete audit snapshot",
            case["name"]
        );
    }
}

#[tokio::test]
async fn settings_name_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("settings_name")).await;
}

#[tokio::test]
async fn settings_restrict_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("settings_restrict")).await;
}

#[tokio::test]
async fn custom_styles_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("custom_styles")).await;
}

#[tokio::test]
async fn logo_removal_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("logo_delete")).await;
}

#[tokio::test]
async fn join_code_reset_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("join_reset")).await;
}

#[tokio::test]
async fn role_change_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("role_promote")).await;
}

#[tokio::test]
async fn deactivation_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("deactivate")).await;
}

#[tokio::test]
async fn ban_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("ban_active")).await;
}

#[tokio::test]
async fn unban_writes_exactly_one_rails_audit_row() {
    run_mutation_cases(Some("unban_banned")).await;
}

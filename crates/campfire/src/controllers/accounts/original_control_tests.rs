//! Original account and ban assertions through routed writes, with Rails observations.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Account, Ban, Session, User};
use serde_json::{Value, json};
const JZ: i64 = 773523953;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/account_originals.json"
    ))
    .unwrap()
}
async fn run(name: &'static str) {
    let app = TestApp::boot_frozen()
        .await
        .expect("CI seed required")
        .without_job_runner()
        .await;
    let case = oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone();
    let setup = case.clone();
    let before=app.db().write(move |tx| {
        if name.starts_with("ban_") || name=="unban" {
            tx.conn().execute("DELETE FROM sessions WHERE user_id=?",[KEVIN])?;
            tx.conn().execute("DELETE FROM bans WHERE user_id=?",[KEVIN])?;
            for ip in setup["ips"].as_array().unwrap() {Session::start(tx,KEVIN,Some("Test"),ip.as_str())?;}
            if name=="unban" {User::find(tx.conn(),KEVIN)?.ban(tx)?;}
        }
        if name=="member_unban" {tx.conn().execute("UPDATE users SET status=2 WHERE id=?",[JZ])?;}
        Ok(json!({"admin":User::find(tx.conn(),DAVID)?.is_administrator(),"member":User::find(tx.conn(),KEVIN)?.is_member(),"active":User::active(tx.conn())?.len(),"ban_total":tx.conn().query_row("SELECT COUNT(*) FROM bans",[],|r|r.get::<_,i64>(0))?,"bans":Ban::for_user(tx.conn(),KEVIN)?.len(),"sessions":Session::count_for_user(tx.conn(),KEVIN)?,"code":Account::first(tx.conn())?.unwrap().join_code,"banned":User::find(tx.conn(),KEVIN)?.is_banned()}))
    }).await.unwrap();
    let viewer = case["viewer"].as_i64().unwrap();
    let mut browser = if viewer == DAVID {
        app.david()
    } else {
        app.sign_in(viewer).await
    };
    if viewer == DAVID {
        assert_eq!(
            browser
                .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
                .await
                .status,
            StatusCode::FOUND
        );
    }
    let reply = if case["method"] == "GET" {
        browser.get(case["path"].as_str().unwrap()).await
    } else {
        browser
            .write(
                Req::new(
                    case["method"].as_str().unwrap().parse().unwrap(),
                    case["path"].as_str().unwrap(),
                )
                .header("content-type", "application/json")
                .body(serde_json::to_vec(&case["params"]).unwrap()),
            )
            .await
    };
    let state=app.db().read(move |c| {
        let account=Account::first(c)?.unwrap();let target=User::find(c,KEVIN)?;
        let active=User::active(c)?.len();
        let bans=Ban::for_user(c,KEVIN)?;
        let mut ips=bans.iter().map(|b|b.ip_address.clone()).collect::<Vec<_>>();ips.sort();
        Ok(json!({"admin_before":before["admin"],"member_before":before["member"],"admin_after":User::find(c,DAVID)?.is_administrator(),"active_delta":active as i64-before["active"].as_i64().unwrap(),"david_active":match User::find_active(c,DAVID){Ok(_)=>true,Err(campfire_db::Error::RecordNotFound(_))=>false,Err(e)=>return Err(e)},"styles":account.custom_styles,"code_changed":account.join_code!=before["code"].as_str().unwrap(),"ban_delta":c.query_row("SELECT COUNT(*) FROM bans",[],|r|r.get::<_,i64>(0))?-before["ban_total"].as_i64().unwrap(),"ips":ips,"session_delta":Session::count_for_user(c,KEVIN)?-before["sessions"].as_i64().unwrap(),"banned_before":before["banned"],"bans_before":before["bans"],"kevin_active":target.is_active()}))
    }).await.unwrap();
    let mut actual = json!({"status":reply.status.as_u16(),"location":reply.location()});
    for key in case["response"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| *k != "status" && *k != "location")
    {
        actual[key] = state[key].clone();
    }
    assert_eq!(
        actual, case["response"],
        "{name}: original routed response and persisted assertions"
    );
}
macro_rules! cases {($($name:ident => $case:literal),+ $(,)?) => {$ (#[tokio::test] async fn $name() {run($case).await;})+};}
cases!(
    original_admin_self_role_is_preserved => "role_self",
    original_self_removal_active_count_and_lookup => "remove_self",
    original_member_cannot_change_admin_role => "member_role",
    original_member_cannot_remove_admin => "member_remove",
    original_styles_exact_value_is_saved => "styles",
    original_member_styles_refused => "member_styles",
    original_join_code_changes_and_redirects => "join",
    original_jz_cannot_reset_join_code => "member_join",
    original_ban_has_two_exact_ip_records => "ban_two",
    original_ban_destroys_the_single_session => "ban_session",
    original_unban_deletes_record_and_activates => "unban",
    original_kevin_cannot_ban_jz => "member_ban",
    original_kevin_cannot_unban_jz => "member_unban",
);

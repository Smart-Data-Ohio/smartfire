use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use axum::http::StatusCode;
use campfire_db::{Account, User};
use campfire_views::{ViewContext, accounts};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_account_views.json")).unwrap()
}

async fn render(app: &TestApp, case: &Value, f: impl FnOnce(&ViewContext) -> String) -> String {
    let viewer_id = case["viewer_id"].as_i64().unwrap_or(DAVID);
    let (viewer, account, has_logo) = app.db().read(move |conn| {
        let account = Account::first(conn)?.unwrap();
        let has_logo = presenters::attachments::attached_blob(conn,"Account",account.id,"logo")?.is_some();
        Ok((User::find(conn,viewer_id)?,account,has_logo))
    }).await.unwrap();
    crate::controllers::users::people_tests::render_with(app, |ctx| {
        ctx.current_user.as_mut().unwrap().id = viewer.id;
        ctx.current_user.as_mut().unwrap().name = viewer.name;
        ctx.current_user.as_mut().unwrap().administrator = viewer.role == campfire_db::Role::Administrator;
        ctx.current_user.as_mut().unwrap().bot = viewer.role == campfire_db::Role::Bot;
        ctx.account.name = account.name.clone();
        ctx.account.logo_url = presenters::accounts::fresh_account_logo_path(Some(&account),None);
        ctx.account.has_logo = has_logo;
        ctx.last_room_visited_id = case["last_room_id"].as_i64();
        ctx.app_version = case["app_version"].as_str().unwrap().into();
    }, f)
}

fn assert_bytes(name: &str, actual: &str, expected: &str) {
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.actual"),actual).unwrap();
        std::fs::write(format!("{dir}/{name}.expected"),expected).unwrap();
    }
    assert_eq!(actual,expected,"{name}: complete Rails bytes");
}

#[tokio::test]
async fn account_member_rows_match_google_security_role_and_inactive_rails_controls() {
    for case in vectors()["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("seed required");
        let id = case["user_id"].as_i64().unwrap();
        let setup = case["setup"].clone();
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE users SET google_email_link_allowed=?,email_self_changed_at=? WHERE id=?",rusqlite::params![setup["allowed"].as_bool().unwrap_or(false), if setup["self_changed"]==true {Some(tx.now())} else {None},id])?;
            for key in ["name","email"] {
                if let Some(value) = setup[key].as_str() {
                    tx.conn().execute(&format!("UPDATE users SET {}=? WHERE id=?",if key=="email" {"email_address"} else {"name"}),rusqlite::params![value,id])?;
                }
            }
            if let Some(status) = setup["status"].as_i64() {tx.conn().execute("UPDATE users SET status=? WHERE id=?",rusqlite::params![status,id])?;}
            tx.conn().execute("DELETE FROM google_identities WHERE user_id=?",[id])?;
            if setup["identity"] == true {
                tx.conn().execute("INSERT INTO google_identities(user_id,subject,email,domain,created_at,updated_at) SELECT id,'ws8br2-row',email_address,'campfire.test',?,? FROM users WHERE id=?",rusqlite::params![tx.now(),tx.now(),id])?;
            }
            if setup["two_factor"] == true {
                tx.conn().execute("DELETE FROM two_factor_credentials WHERE user_id=?",[id])?;
                // Reuse valid seed ciphertext; this case only reads the enrollment state.
                tx.conn().execute("INSERT INTO two_factor_credentials(user_id,secret,confirmed_at,created_at,updated_at) SELECT ?,secret,?,?,? FROM two_factor_credentials WHERE user_id!=? LIMIT 1",rusqlite::params![id,tx.now(),tx.now(),tx.now(),id])?;
            }
            Ok(())
        }).await.unwrap();
        let secrets = app.booted.app.secrets.clone();
        let user = app.db().read(move |conn| presenters::account_user_summary(conn,&secrets,&User::find(conn,id)?)).await.unwrap();
        let actual = render(&app,case,|ctx| accounts::UserPartial{ctx,user}.render().unwrap()).await;
        assert_bytes(case["name"].as_str().unwrap(),&actual,case["html"].as_str().unwrap());
    }
}

#[tokio::test]
async fn account_settings_body_navigation_and_footer_match_pinned_rails() {
    for case in vectors()["pages"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("seed required");
        let viewer = case["viewer_id"].as_i64().unwrap();
        let secrets = app.booted.app.secrets.clone();
        let (account, administrators, members) = app.db().read(move |conn| {
            let summaries = presenters::accounts::account_users(conn,viewer==DAVID)?.iter().map(|user| presenters::account_user_summary(conn,&secrets,user)).collect::<campfire_db::Result<Vec<_>>>()?;
            let (admins,members): (Vec<_>,Vec<_>) = summaries.into_iter().partition(|user|user.administrator());
            Ok((Account::first(conn)?.unwrap(),admins,members))
        }).await.unwrap();
        for block in ["content","nav","footer"] {
            let actual=render(&app,case,|ctx| {
                let page = accounts::Edit {ctx,account_id:account.id,join_code:account.join_code.clone(),restrict_room_creation_to_administrators:account.settings().restrict_room_creation_to_administrators(),administrators:administrators.clone(),members:members.clone(),next_page:None};
                match block {"content"=>page.as_content().render(),"nav"=>page.as_nav().render(),"footer"=>page.as_footer().render(),_=>unreachable!()}.unwrap()
            }).await;
            let key=if block=="content" {"html"} else {block};
            assert_bytes(&format!("account-{viewer}-{block}"),&actual,case[key].as_str().unwrap());
        }
        assert_eq!(app.anonymous().get("/account/edit").await.status,StatusCode::FOUND);
        let reply=app.sign_in(viewer).await.get("/account/edit").await;
        assert_eq!(reply.status,StatusCode::OK);
        crate::controllers::users::people_tests::assert_http_fragment(&reply.text(), case["html"].as_str().unwrap(), "turbo-frame", "id", "account_users");
        if viewer == DAVID {
            let full = reply.text();
            let body = full.split_once("id=\"account_users\"").expect("account users frame").1.split_once("</turbo-frame>").unwrap().0;
            let divider = body.find("separator full-width").expect("original administrator divider");
            for name in ["David", "Jason"] { assert!(body.find(&format!("<strong>{name}</strong>")).expect("administrator row") < divider); }
            for name in ["Kevin", "JZ"] { assert!(body.find(&format!("<strong>{name}</strong>")).expect("member row") > divider); }
        }
        assert_eq!(reply.text().contains("Manage workspace icons"),viewer==DAVID);
    }
}

#[tokio::test]
async fn account_invite_markup_matches_rails_for_admin_and_member() {
    for case in vectors()["invites"].as_array().unwrap() {
        let app=TestApp::boot_frozen().await.expect("seed required");
        let actual=render(&app,case,|ctx| accounts::Invite{ctx,join_code:case["join_code"].as_str().unwrap().into()}.render().unwrap()).await;
        assert_bytes("invite",&actual,case["html"].as_str().unwrap());
    }
}

#[tokio::test]
async fn custom_style_editor_matches_rails_bytes_including_escaped_css() {
    for case in vectors()["styles"].as_array().unwrap() {
        let app=TestApp::boot_frozen().await.expect("seed required");
        let actual=render(&app,case,|ctx| accounts::CustomStylesEdit{ctx,custom_styles:case["custom_styles"].as_str().map(str::to_string)}.as_content().render().unwrap()).await;
        assert_bytes("styles",&actual,case["html"].as_str().unwrap());
        assert_eq!(app.sign_in(KEVIN).await.get("/account/custom_styles/edit").await.status,StatusCode::FORBIDDEN);
    }
}

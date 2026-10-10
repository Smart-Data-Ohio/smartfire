use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{Boost, Message, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use crate::controllers::presenters::test_support::*;

pub(crate) fn oracle() -> Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/modern-boosts.json")).unwrap()}
pub(crate) async fn fixture(app: &TestApp) -> i64 {
    app.db().write(|tx| {
        let m = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("Reaction source".into()), client_message_id: Some("modern-boosts".into()), ..Default::default()})?;
        tx.conn().execute("INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('ws8bm_icon','Workspace <&>',?,?,?)", (DAVID,tx.now(),tx.now()))?;
        assert_eq!(oracle()["message_id"],m.id);
        Ok(m.id)
    }).await.unwrap()
}
pub(crate) async fn duplicates(app: &TestApp, id: i64) {
    app.db().write(move |tx| {
        for expected in oracle()["duplicates"].as_array().unwrap() {
            tx.conn().execute("INSERT INTO boosts (message_id,booster_id,content,created_at,updated_at) VALUES (?,?,'👍',?,?)", (id,DAVID,tx.now(),tx.now()))?;
            assert_eq!(expected,tx.conn().last_insert_rowid());
        }
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn modern_boosts_reject_bots_forgery_nonmembers_and_other_boosters_first() {
    let app = TestApp::boot().await.unwrap();
    let id = fixture(&app).await;
    let path = format!("/messages/{id}/boosts");
    let input = json!({"boost":{"content":"👍"}}).to_string();
    assert_eq!(app.sign_in(KEVIN).await.write(Req::new(Method::POST,&path).header("content-type","application/json").body(input.clone())).await.status,StatusCode::NOT_FOUND);
    assert_eq!(app.anonymous().get(&format!("{path}?bot_key={BENDER_KEY}")).await.status,StatusCode::FORBIDDEN);
    assert_eq!(app.david().send(Req::new(Method::POST,&path).header("origin","https://forged.test").header("content-type","application/json").body(input)).await.status,StatusCode::UNPROCESSABLE_ENTITY);
    let boost = app.db().write(move |tx|Boost::create(tx,id,JASON,"👍")).await.unwrap();
    assert_eq!(app.david().write(Req::new(Method::DELETE,&format!("{path}/{}",boost.id))).await.status,StatusCode::NOT_FOUND);
    assert_eq!(app.db().read(move |conn|Boost::for_message(conn,id)).await.unwrap().len(),1);
    app.db().write(|tx| {tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",(ALL_TALK,DAVID))?;Ok(())}).await.unwrap();
    assert_eq!(app.david().get(&path).await.status,StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn modern_boosts_match_rails_toggle_coercion_duplicate_and_destroy_rows() {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let id = fixture(&app).await;
    for row in oracle()["rows"].as_array().unwrap() {
        if row["name"] == "duplicate_toggle" {duplicates(&app,id).await;}
        let response = app.sign_in(row["user_id"].as_i64().unwrap()).await.write(Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),row["path"].as_str().unwrap())
            .header("content-type","application/json").header("accept","application/json").body(row["input"].to_string())).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{name}: {}",response.text());
        assert_eq!(response.location(),row["location"].as_str(),"{name}");
        assert_eq!(response.header("cache-control"),row["cache_control"].as_str(),"{name}");
        assert_eq!(response.content_type(),row["content_type"].as_str(),"{name}");
        assert_eq!(response.text(),row["body"].as_str().unwrap(),"{name}");
        let rows = app.db().read(move |conn| Ok(Boost::for_message(conn,id)?.iter().map(|b|json!({"id":b.id,"booster_id":b.booster_id,"content":b.content})).collect::<Vec<_>>())).await.unwrap();
        assert_eq!(json!(rows),row["boosts"],"{name}");
    }
}

#[tokio::test]
async fn duplicate_toggle_rolls_back_on_touch_failure_and_concurrent_toggles_serialize() {
    let app = TestApp::boot().await.unwrap();
    let id = fixture(&app).await;
    app.db().write(move |tx| {
        Boost::create(tx,id,DAVID,"👍")?;
        Boost::create(tx,id,DAVID,"👍")?;
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_boost_touch BEFORE UPDATE ON messages WHEN OLD.client_message_id = 'modern-boosts' BEGIN SELECT RAISE(ABORT,'boost touch rejected'); END;")?;
        Ok(())
    }).await.unwrap();
    let path = format!("/messages/{id}/boosts");
    let response = app.david().write(Req::new(Method::POST,&path).form(&[("boost[content]",":thumbsup:")])).await;
    assert_eq!(response.status,StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.db().read(move |conn|Boost::for_message(conn,id)).await.unwrap().len(),2);
    app.db().write(move |tx| {tx.conn().execute_batch("DROP TRIGGER ws8bm_reject_boost_touch")?;
        for boost in Boost::for_message(tx.conn(),id)? {boost.destroy(tx)?;}Ok(())}).await.unwrap();
    let (mut first,mut second) = (app.david(),app.david());
    let (first,second) = tokio::join!(first.write(Req::new(Method::POST,&path).form(&[("boost[content]","👍")])),
        second.write(Req::new(Method::POST,&path).form(&[("boost[content]",":thumbsup:")])));
    assert_eq!(first.status,StatusCode::FOUND);assert_eq!(second.status,StatusCode::FOUND);
    assert!(app.db().read(move |conn|Boost::for_message(conn,id)).await.unwrap().is_empty());
}

fn pages_oracle() -> Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/boost-pages.json")).unwrap()}

#[tokio::test]
async fn boost_pages_match_complete_rails_forms_distinct_counts_and_escaped_reactors() {
    use askama::Template;
    use campfire_views::helpers::request_forgery::{self, AuthenticityTokens, RequestSecrets};
    use crate::controllers::presenters::{Presenter, page};
    struct FixedTokens;
    impl AuthenticityTokens for FixedTokens {
        fn global(&self) -> String {"GLOBAL".into()}
        fn for_form(&self, action: &str, method: &str) -> String {format!("{method}:{action}")}
    }
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let id = app.db().write(|tx| {
        let m = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("Boost pages".into()), client_message_id: Some("boost-pages".into()), ..Default::default()})?;
        assert_eq!(pages_oracle()["message_id"],m.id);
        Ok(m.id)
    }).await.unwrap();
    for row in pages_oracle()["rows"].as_array().unwrap() {
        if row["setup"] == "boosts" {
            app.db().write(move |tx| {
                for b in pages_oracle()["boosts"].as_array().unwrap() {Boost::create(tx,id,b[0].as_i64().unwrap(),b[1].as_str().unwrap())?;}
                Ok(())
            }).await.unwrap();
        } else if row["setup"] == "name" {
            app.db().write(|tx| {tx.conn().execute("UPDATE users SET name=? WHERE id=?", ("<a href=\"/unsafe\">Jason & \"quoted\"</a>",JASON))?;Ok(())}).await.unwrap();
        }
        let name = row["name"].as_str().unwrap();
        let user_id = row["user_id"].as_i64().unwrap();
        let mut browser = app.sign_in(user_id).await;
        let response = browser.send(Req::new(Method::GET,row["path"].as_str().unwrap()).header("turbo-frame","boosting")).await;
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{name}: {}",response.text());
        assert_eq!(response.content_type(),row["content_type"].as_str(),"{name}");
        if row["action"] == "actions" {
            assert_eq!(response.text(),row["body"].as_str().unwrap(),"{name}");
            continue;
        }
        let runtime = app.booted.app.clone();
        let new = row["action"] == "new";
        let html = app.db().read(move |conn| {
            let p = Presenter::new(conn,&runtime,None);
            let message = p.message(&Message::find(conn,id)?)?;
            let user = p.user_view(user_id)?;
            page::render_detached_at(&runtime,None,"http://campfire.test",|ctx| request_forgery::rendering_with(RequestSecrets {tokens: Box::new(FixedTokens),csp_nonce: None}, || {
                if new {campfire_views::messages::NewBoost {ctx,message: &message,user: &user}.render()}
                else {campfire_views::messages::BoostsIndex {ctx,message: &message}.render()}
            })).map_err(|e| campfire_db::Error::Other(e.to_string()))
        }).await.unwrap();
        if html != row["body"].as_str().unwrap() {rails_mismatch(&html,row["body"].as_str().unwrap(),name);}
        // The actual HTTP form uses its session's live token; the complete detached form above
        // gives Rails and Rust identical token inputs rather than masking generated HTML.
        let text = response.text();
        if new {
            assert!(text.contains("data-controller=\"markdown-autocomplete\""),"{name}");
            assert!(text.contains("data-profile-card-url="),"{name}");
            let token = text.split("name=\"authenticity_token\" value=\"").nth(1).unwrap().split('"').next().unwrap();
            assert!(browser.real_authenticity_token().unwrap().is_valid(token,&format!("/messages/{id}/boosts"),"post"),"{name}");
        }
    }
}

use campfire_web::controllers::presenters::{Rendering};

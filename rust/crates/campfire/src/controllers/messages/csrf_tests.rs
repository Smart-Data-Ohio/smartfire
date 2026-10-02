//! Real room/thread header security checks for every merged shared message form.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{Boost, ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter, test_support::*};

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/cached-csrf.json")).unwrap() }

async fn fixture() -> TestApp {
    // Rails queues a fetch for a new reference even when its fixture card is fresh.
    // These cache/form tests supply the fetched card and never execute network jobs.
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap().without_job_runner().await;
    app.db().write(|tx| {
        let pr=crate::integrations::github::pull_requests::PullRequest::for_reference(tx,"rails","rails",3141)?;
        tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Cached card',state='open',fetched_at=?,fetch_requested_at=NULL,updated_at=? WHERE id=?",(tx.now(),tx.now(),pr.id))?;
        let card=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("see https://github.com/rails/rails/pull/3141".into()),client_message_id:Some("csrf-pr".into()),..Default::default()})?;
        let poll_message=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Lunch?".into()),client_message_id:Some("csrf-poll".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO polls (id,message_id,created_at,updated_at) VALUES (?,?,?,?)",(oracle()["poll_id"].as_i64().unwrap(),poll_message.id,tx.now(),tx.now()))?;
        for option in oracle()["poll_options"].as_array().unwrap() {
            tx.conn().execute("INSERT INTO poll_options (id,poll_id,label,position,created_at,updated_at) VALUES (?,?,?,?,?,?)",rusqlite::params![option[0].as_i64(),oracle()["poll_id"].as_i64(),option[1].as_str(),option[2].as_i64(),tx.now(),tx.now()])?;
        }
        assert_eq!(poll_message.id,oracle()["poll_message_id"].as_i64().unwrap());
        let boosted=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Boost me".into()),client_message_id:Some("csrf-boosts".into()),..Default::default()})?;
        Boost::create(tx,boosted.id,DAVID,"👍")?;
        let legacy=Boost::create(tx,boosted.id,JASON,"Legacy text boost")?;
        let mut thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Cached thread".into()),..Default::default()})?;
        ThreadMembership::join(tx,thread.id,DAVID)?;
        let reply=thread.post_message(tx,DAVID,NewMessage {markdown_source:Some("In the thread".into()),client_message_id:Some("csrf-thread".into()),..Default::default()})?;
        Boost::create(tx,reply.id,DAVID,"🎉")?;
        for (key,id) in [("card_id",card.id),("boosted_id",boosted.id),("legacy_boost_id",legacy.id),("thread_id",thread.id),("reply_id",reply.id)] { assert_eq!(oracle()[key].as_i64(),Some(id),"{key}"); }
        Ok(())
    }).await.unwrap();
    app
}

async fn fragments(app: &TestApp, ids: Vec<i64>) -> Vec<Arc<String>> {
    let runtime=app.booted.app.clone();
    app.db().read(move |conn| {
        let p=Presenter::new(conn,&runtime,None);
        campfire_views::fragment_cache::with(&runtime.fragment_cache,|| ids.iter().map(|id| {
            let message=Message::find(conn,*id)?;
            let key=p.message_fragment_cache_key(&message,"http://campfire.test")?;
            Ok(campfire_views::fragment_cache::read(&key).expect("actual HTTP populated the shared collection cache"))
        }).collect())
    }).await.unwrap()
}

#[tokio::test]
async fn cached_pages_refreshes_and_thread_pages_reuse_tokenless_fragments_across_sessions() {
    let app=fixture().await;
    let mut first=app.david();let mut second=app.sign_in(JASON).await;
    let a=first.authenticity_token().await;let b=second.authenticity_token().await;
    assert!(!second.real_authenticity_token().unwrap().is_valid(&a,"/anything","post"));
    assert!(!first.real_authenticity_token().unwrap().is_valid(&b,"/anything","post"));
    for row in oracle()["rows"].as_array().unwrap() {
        let path=row["path"].as_str().unwrap();
        assert_eq!(first.get(path).await.status,StatusCode::OK,"{path}");
        let ids=row["message_ids"].as_array().unwrap().iter().map(|id|id.as_i64().unwrap()).collect::<Vec<_>>();
        let before=fragments(&app,ids.clone()).await;
        let response=second.get(path).await;assert_eq!(response.status,StatusCode::OK,"{path}");
        let after=fragments(&app,ids).await;
        for (old,new) in before.iter().zip(&after) {
            assert!(Arc::ptr_eq(old,new),"second HTTP request actually hits the same cached fragment");
            assert_eq!(campfire_cable::turbo::session_bound(new),None);
            let expected=oracle()["fragments"].as_array().unwrap().iter().find(|row|new.contains(&format!("data-message-id=\"{}\"",row["id"].as_i64().unwrap()))).unwrap()["html"].as_str().unwrap().to_owned();
            if new.as_str()!=expected {rails_mismatch(new,&expected,"cached CSRF fragment");}
            assert!(!new.contains("authenticity_token"));
            assert!(!new.contains(&a)&&!new.contains(&b));
            assert!(response.text().contains(new.as_str()),"cached bytes mounted unchanged");
        }
    }
}

#[tokio::test]
async fn cached_owned_forms_submit_with_real_page_header_and_reject_foreign_or_missing_tokens() {
    submit_cached_forms(false).await;
}

#[tokio::test]
async fn every_cached_form_submits_with_real_room_header_and_live_log_after_owner_merge() {
    submit_cached_forms(true).await;
}

#[tokio::test]
async fn merged_room_shell_mounts_the_shared_collection_fragment_instead_of_rebuilding_it() {
    let app=fixture().await;
    let runtime=app.booted.app.clone();
    let id=oracle()["card_id"].as_i64().unwrap();
    let cached=app.db().read(move |conn| {
        let p=Presenter::new(conn,&runtime,None);
        let message=Message::find(conn,id)?;
        let key=p.message_fragment_cache_key(&message,"http://campfire.test")?;
        // A valid tokenless cache value with an inert witness distinguishes reuse
        // from a byte-identical fresh render. No production renderer is mutated.
        let html=oracle()["fragments"][0]["html"].as_str().unwrap().to_owned();
        Ok(runtime.fragment_cache.fetch_value(&key,||Arc::new(format!("<!-- cached collection witness -->{html}"))))
    }).await.unwrap();
    for viewer in [DAVID,JASON] {
        let response=app.sign_in(viewer).await.get(&format!("/rooms/{ALL_TALK}")).await;
        assert_eq!(response.status,StatusCode::OK);
        assert!(response.text().contains(cached.as_str()),"real room shell must mount the collection cache value verbatim");
        let after=fragments(&app,vec![id]).await;
        assert!(Arc::ptr_eq(&cached,&after[0]));
    }
}

async fn submit_cached_forms(room_shell: bool) {
    let oracle=if room_shell {serde_json::from_str::<Value>(include_str!("../../../../../vectors/messaging/room-csrf.json")).unwrap()} else {oracle()};
    let app=fixture().await;
    let mut first=app.david();let foreign=first.authenticity_token().await;
    assert_eq!(first.get(&format!("/rooms/{ALL_TALK}/messages")).await.status,StatusCode::OK);
    let mut viewer=app.sign_in(JASON).await;
    let path=if room_shell {format!("/rooms/{ALL_TALK}")} else {format!("/rooms/{ALL_TALK}/threads/{}",oracle["thread_id"])};
    let page=viewer.get(&path).await;
    assert_eq!(page.status,StatusCode::OK);
    if room_shell {
        let region=regex::Regex::new(&format!(r#"<div\b[^>]*id="{}"[^>]*>"#,oracle["live_region_id"].as_str().unwrap())).unwrap();
        let body=page.text();
        let regions=region.find_iter(&body).collect::<Vec<_>>();
        assert_eq!(regions.len(),oracle["live_regions"].as_u64().unwrap() as usize);
        for attr in [r#"role="log""#,r#"aria-live="polite""#,r#"aria-relevant="additions""#] {assert!(regions[0].as_str().contains(attr),"{attr}");}
    }
    let token=page.text().split("<meta name=\"csrf-token\" content=\"").nth(1).unwrap().split('"').next().unwrap().to_owned();
    assert!(viewer.real_authenticity_token().unwrap().is_valid(&token,"/anything","post"));
    assert_eq!(viewer.get(&format!("/rooms/{ALL_TALK}/messages")).await.status,StatusCode::OK);
    assert_eq!(viewer.get(&format!("/rooms/{ALL_TALK}/threads/{}/messages",oracle["thread_id"])).await.status,StatusCode::OK);
    let keys=if room_shell {vec!["card_id","poll_message_id","boosted_id","reply_id"]} else {vec!["card_id","boosted_id","reply_id"]};
    let ids=keys.into_iter().map(|key|oracle[key].as_i64().unwrap()).collect();
    let html=fragments(&app,ids).await.into_iter().map(|s|s.to_string()).collect::<String>();
    let forms=regex::Regex::new(r#"(?s)<form\b[^>]*action="([^"]+)"[^>]*>(.*?)</form>"#).unwrap();
    let hidden=regex::Regex::new(r#"<input[^>]*type="hidden"[^>]*name="([^"]+)"[^>]*value="([^"]*)"[^>]*>"#).unwrap();
    let option=regex::Regex::new(r#"<input[^>]*type="(?:radio|checkbox)"[^>]*name="([^"]+)"[^>]*value="([^"]*)"[^>]*>"#).unwrap();
    let rendered=forms.captures_iter(&html).map(|form| {
        let mut params=hidden.captures_iter(&form[2]).map(|field|(field[1].to_owned(),field[2].to_owned())).collect::<std::collections::BTreeMap<_,_>>();
        if let Some(option)=option.captures(&form[2]) {params.insert(option[1].to_owned(),option[2].to_owned());}
        (form[1].to_owned(),params)
    }).collect::<Vec<_>>();
    assert_eq!(rendered.len(),oracle["forms"].as_array().unwrap().len(),"all real rendered forms exercised");
    for ((action,mut params),expected) in rendered.into_iter().zip(oracle["forms"].as_array().unwrap()) {
        assert!(!params.contains_key("authenticity_token"));
        let method=params.remove("_method").unwrap_or("post".into());
        assert_eq!(action,expected["action"].as_str().unwrap());assert_eq!(method,expected["method"].as_str().unwrap());
        let actual=params.iter().map(|(key,value)| (key.clone(),if key.ends_with("[]") {serde_json::json!([value])} else {serde_json::json!(value)})).collect::<serde_json::Map<_,_>>();
        assert_eq!(Value::Object(actual),expected["params"]);
        let pairs=params.iter().map(|(k,v)|(k.as_str(),v.as_str())).collect::<Vec<_>>();
        let request=|| Req::new(Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),&action).form(&pairs).header("accept","text/vnd.turbo-stream.html, text/html, application/xhtml+xml");
        let before=app.db().read(|conn| Ok((Message::count(conn)?,conn.query_row("SELECT COUNT(*) FROM boosts",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM poll_votes",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
        for csrf in [None,Some(foreign.as_str())] {
            let mut req=request();if let Some(csrf)=csrf {req=req.header("x-csrf-token",csrf);}
            assert_eq!(viewer.send(req).await.status,StatusCode::UNPROCESSABLE_ENTITY,"{method} {action}");
        }
        let after=app.db().read(|conn| Ok((Message::count(conn)?,conn.query_row("SELECT COUNT(*) FROM boosts",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM poll_votes",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
        assert_eq!(before,after,"forgery refusals do not write");
        let response=viewer.send(request().header("x-csrf-token",&token)).await;
        assert_eq!(response.status.as_u16(),expected["status"].as_u64().unwrap() as u16,"{method} {action}: {}",response.text());
    }
}

//! Named remaining root/cache declarations over the actual router and stored rows.
use std::sync::Arc;
use axum::http::{Method,StatusCode};
use campfire_db::{Boost,Message,NewMessage,MessageChanges,Timestamp};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter,page,test_support::*};

fn oracle()->Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/root-declarations.json")).unwrap()}
#[tokio::test]
async fn root_edit_markers_match_rails_for_noops_attachments_formatting_reactions_fetches_tombstones_and_zones() {
    let clock=Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let mut app=TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let ids=app.db().write(|tx| {
        let mut ids=Vec::new();
        for (index,input) in oracle()["inputs"].as_array().unwrap().iter().enumerate() {
            let message=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:input["markdown_source"].as_str().map(str::to_string),body:input["body"].as_str().map(str::to_string),client_message_id:input["client_message_id"].as_str().map(str::to_string),attachment_blob_id:(index==2).then_some(13),..Default::default()})?;
            ids.push(message.id);
        }
        tx.conn().execute("UPDATE messages SET edited_at=? WHERE id=?",(Timestamp::from_jiff("2026-09-22T12:00:00Z".parse().unwrap()),ids[3]))?;
        Message::find(tx.conn(),ids[5])?.update(tx,MessageChanges {reply_to_message_id:Some(Some(ids[4])),..Default::default()})?;
        Ok(ids)
    }).await.unwrap();
    assert_eq!(serde_json::json!(ids),oracle()["message_ids"]);
    let mut viewer=app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let id=ids[row["index"].as_u64().unwrap() as usize];
        let response=viewer.write(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/messages/{id}.json")).header("content-type","application/json").body(serde_json::json!({"message":row["input"]}).to_string())).await;
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{}: {}",row["name"],response.text());
        if response.text()!=row["body"].as_str().unwrap(){rails_mismatch(&response.text(),row["body"].as_str().unwrap(),row["name"].as_str().unwrap());}
        let (edited,updated,body)=app.db().read(move|conn|{let m=Message::find(conn,id)?;Ok((m.edited_at.map(|t|campfire_views::messages::support::json_time(t.jiff())),campfire_views::messages::support::json_time(m.updated_at.jiff()),m.body_html(conn)?.unwrap_or_default()))}).await.unwrap();
        assert_eq!(edited,row["edited_at"].as_str().map(str::to_string),"{}",row["name"]);
        assert_eq!(updated,row["updated_at"].as_str().unwrap(),"{}",row["name"]);assert_eq!(body,row["saved_body"].as_str().unwrap());
    }
    let id=ids[0];app.db().write(move|tx|Boost::create(tx,id,JASON,"👍").map(|_|())).await.unwrap();
    let card=ids[6];app.db().write(move|tx| {
        let pr=crate::integrations::github::pull_requests::PullRequest::for_message(tx.conn(),card)?.remove(0);
        crate::integrations::github::pull_requests::update(tx,pr.id,&[("private",rusqlite::types::Value::Integer(0)),("title",rusqlite::types::Value::Text("Fetched".into())),("fetched_at",rusqlite::types::Value::Text(tx.now().to_db()))])?;
        Ok(())
    }).await.unwrap();
    let source=ids[4];assert_eq!(viewer.write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/messages/{source}.turbo_stream"))).await.status,StatusCode::OK);
    for (key,id) in [("reaction_edited",ids[0]),("fetch_edited",ids[6]),("reply_edited",ids[5])] {
        assert_eq!(app.db().read(move|conn|Ok(Message::find(conn,id)?.edited_at.map(|t|campfire_views::messages::support::json_time(t.jiff())))).await.unwrap(),oracle()[key].as_str().map(str::to_owned));
        let page=viewer.get(&format!("/rooms/{ALL_TALK}/messages/{id}")).await;assert_eq!(page.status,StatusCode::OK);assert!(!page.text().contains("class=\"message__edited\""));
    }
    let id=ids[3];
    for row in oracle()["meta"].as_array().unwrap() {
        let zone=row["zone"].as_str().unwrap().to_owned();app.db().write(move|tx|{tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?",(zone,DAVID))?;Ok(())}).await.unwrap();
        let response=viewer.get(&format!("/rooms/{ALL_TALK}/messages/{id}")).await;assert_eq!(response.status,StatusCode::OK);
        let expected=row["html"].as_str().unwrap();assert!(response.text().contains(expected),"complete UTC meta independent of viewer zone");
    }
}

#[tokio::test]
async fn page_validators_change_for_off_page_replies_and_provider_fetches_without_message_touches() {
    let oracle:Value=serde_json::from_str(include_str!("../../../../../vectors/messaging/validator-declarations.json")).unwrap();
    let clock=Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let mut app=TestApp::boot_with_test_clock(clock.clone()).await.unwrap();app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let (source,card)=app.db().write(|tx| {
        let source=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Off-page source".into()),client_message_id:Some("validator-source".into()),..Default::default()})?;
        for i in 0..50 {Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some(format!("Filler {i}")),client_message_id:Some(format!("validator-filler-{i}")),..Default::default()})?;}
        Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Reply".into()),reply_to_message_id:Some(source.id),client_message_id:Some("validator-reply".into()),..Default::default()})?;
        let card=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("https://github.com/rails/rails/pull/530".into()),client_message_id:Some("validator-card".into()),..Default::default()})?;
        Ok((source.id,card.id))
    }).await.unwrap();
    assert_eq!(oracle["source_id"],source);assert_eq!(oracle["card_id"],card);
    let path=format!("/rooms/{ALL_TALK}/messages");let mut viewer=app.david();let mut previous=None::<String>;
    for row in oracle["rows"].as_array().unwrap() {
        if row["name"]=="source_edit" {
            clock.set("2026-03-02T16:00:10Z".parse().unwrap());
            assert_eq!(viewer.write(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/messages/{source}.json")).form(&[("message[markdown_source]","Edited off-page source")])).await.status,StatusCode::OK);
        }
        if row["name"]=="card_fetch" {
            clock.set("2026-03-02T16:00:20Z".parse().unwrap());
            app.db().write(move|tx| {let pr=crate::integrations::github::pull_requests::PullRequest::for_message(tx.conn(),card)?.remove(0);tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Fetched card',fetched_at=?,updated_at=? WHERE id=?",(tx.now(),tx.now(),pr.id))?;Ok(())}).await.unwrap();
        }
        let mut req=Req::new(Method::GET,&path);if let Some(before)=&previous {req=req.header("if-none-match",before);}
        let response=viewer.send(req).await;assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16);
        assert_eq!(response.header("etag"),row["etag"].as_str(),"{}",row["name"]);
        if response.status==StatusCode::OK {previous=response.header("etag").map(str::to_owned);}
    }
}

#[tokio::test]
async fn legacy_v2_fragment_and_page_validators_cannot_serve_the_vulnerable_autolink_render() {
    let oracle:Value=serde_json::from_str(include_str!("../../../../../vectors/messaging/legacy-cache.json")).unwrap();
    let mut app=TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let payload=oracle["payload"].as_str().unwrap().to_owned();
    let id=app.db().write(move|tx|Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,body:Some(payload),client_message_id:Some("legacy-cached-autolink".into()),..Default::default()}).map(|m|m.id)).await.unwrap();
    let runtime=app.booted.app.clone();let old=oracle["old_html"].as_str().unwrap().to_owned();let expected=oracle["html"].as_str().unwrap().to_owned();
    app.db().read(move|conn| {
        let mut p=Presenter::new(conn,&runtime,None);p.cache_base_url=Some("http://campfire.test".into());let message=Message::find(conn,id)?;
        let key=p.message_collection_cache_key(&message)?;let (base,_)=key.rsplit_once('/').unwrap();let old_key=campfire_views::messages::collection_fragment_key(&format!("{base}/2"),"http://campfire.test");
        runtime.fragment_cache.fetch(&old_key,||old.clone());
        // Prove the vulnerable entry was stored, with the same origin/digest.
        campfire_views::fragment_cache::with(&runtime.fragment_cache,|| {
            assert_eq!(campfire_views::fragment_cache::read(&old_key).unwrap().as_str(),old);
            let item=p.message_item(&message)?;
            let account=campfire_db::Account::first(conn)?;
            let actual=page::render_detached_at(&runtime,account.as_ref(),"http://campfire.test",|ctx|campfire_views::messages::cached_message_item(ctx,&item).to_string());
            if actual!=expected {rails_mismatch(&actual,&expected,"safe legacy collection despite stored v2");}
            Ok(())
        })
    }).await.unwrap();
    let mut viewer=app.david();
    for row in oracle["rows"].as_array().unwrap() {
        let mut req=Req::new(Method::GET,&format!("/rooms/{ALL_TALK}/messages"));for (key,value) in row["headers"].as_object().unwrap(){req=req.header(key,value.as_str().unwrap());}
        let response=viewer.send(req).await;assert_eq!(response.status,StatusCode::OK);assert_eq!(response.header("etag"),row["etag"].as_str());
        assert!(response.text().contains(oracle["html"].as_str().unwrap()));
    }
}

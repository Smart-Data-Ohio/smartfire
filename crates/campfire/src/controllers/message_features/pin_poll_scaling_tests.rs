//! Physical queries through the production pin-list and standalone poll presenters.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::page;
use askama::Template;
use campfire_db::{Account, Room};
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/pin_poll_scaling.json")).unwrap()
}
fn reads(queries: &[String]) -> usize {
    queries.iter().filter(|sql| sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH")).count()
}
#[tokio::test]
async fn pin_lists_match_rails_with_flat_physical_reads_at_4_and_16() {
    let mut previous: std::collections::HashMap<bool,Vec<usize>>=std::collections::HashMap::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app=app_rows(group["rows"].clone()).await;
        let room_id=group["room_id"].as_i64().unwrap();
        let runtime=app.booted.app.clone();
        let queries=app.db().capture_queries();
        let html=app.db().read(move |conn| {
            let room=Room::find(conn,room_id)?;
            let list=crate::controllers::rooms::pins::list(conn,&runtime,&room)?;
            Ok(page::render_detached_at(&runtime,None,"http://campfire.test",|ctx|
                campfire_views::pins::ListPartial {ctx,list:&list}.render().unwrap()))
        }).await.unwrap();
        app.db().stop_capturing_queries();
        assert_eq!(html,group["pins"]["html"].as_str().unwrap());
        let count=reads(&queries.lock().unwrap());
        println!("WS8bm2 pin-list reads populated={} size={}: Rust={count}; Rails={}",group["populated"],group["size"],group["pins"]["reads"]);
        previous.entry(group["populated"].as_bool().unwrap()).or_default().push(count);
    }
    for counts in previous.values() { assert!(counts.iter().all(|n| *n==counts[0]),"pin-list physical reads grow per pin: {counts:?}"); }
}
#[tokio::test]
async fn standalone_poll_cards_match_rails_with_flat_physical_reads_at_4_and_16() {
    let mut previous: std::collections::HashMap<bool,Vec<usize>>=std::collections::HashMap::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app=app_rows(group["rows"].clone()).await;
        for case in group["polls"].as_array().unwrap() {
            let runtime=app.booted.app.clone();
            let id=case["poll_id"].as_i64().unwrap();
            let queries=app.db().capture_queries();
            let html=app.db().read(move |conn| {
                let poll=super::poll_view(conn,&runtime,id,None)?;
                let account=Account::first(conn)?;
                Ok(page::render_detached_at(&runtime,account.as_ref(),"http://campfire.test",|ctx|
                    campfire_views::messages::parts::poll(ctx,&poll).0))
            }).await.unwrap();
            app.db().stop_capturing_queries();
            assert_eq!(html,case["html"].as_str().unwrap());
            let count=reads(&queries.lock().unwrap());
            let anonymous=case["anonymous"].as_bool().unwrap();
            println!("WS8bm2 standalone-poll reads size={} anonymous={anonymous}: Rust={count}; Rails={}",group["size"],case["reads"]);
            previous.entry(anonymous).or_default().push(count);
        }
    }
    for counts in previous.values() { assert!(counts.iter().all(|n| *n==counts[0]),"standalone poll physical reads grow per voter: {counts:?}"); }
}

#[tokio::test]
async fn pin_lists_and_message_ids_obey_sqlite_limit_with_exact_rails_order() {
    let group=oracle()["groups"].as_array().unwrap().last().unwrap().clone();
    let app=app_rows(group["rows"].clone()).await;
    let runtime=app.booted.app.clone();
    let expected=group["pins"]["html"].as_str().unwrap().to_owned();
    let room_id=group["room_id"].as_i64().unwrap();
    let html=app.db().read(move |conn| {
        let old=conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER,8)?;
        let result=(|| {
            let room=Room::find(conn,room_id)?;
            let pins=campfire_db::MessagePin::ordered_for_room(conn,room_id)?;
            let ids:Vec<_>=pins.iter().cycle().take(32768).map(|pin|pin.message_id).collect();
            assert_eq!(campfire_db::Message::for_ids(conn,&ids)?.len(),16);
            let list=crate::controllers::rooms::pins::list(conn,&runtime,&room)?;
            Ok(page::render_detached_at(&runtime,None,"http://campfire.test",|ctx|
                campfire_views::pins::ListPartial {ctx,list:&list}.render().unwrap()))
        })();
        conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER,old)?;
        result
    }).await.unwrap();
    assert_eq!(html,expected);
}

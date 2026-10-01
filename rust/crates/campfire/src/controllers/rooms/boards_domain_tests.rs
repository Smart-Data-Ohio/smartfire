//! Rails-filter and callback vectors on the real seed, with FrozenClock and RecordingSink.
use crate::controllers::presenters::test_support::*;
use campfire_db::{ChannelThread, NewChannelThread, RecordingSink, Database, Env};
use campfire_db::broadcasts::{Broadcast,Partial,TurboAction};
use std::sync::Arc;

struct FrozenDbClock(campfire_kit::FrozenClock);
impl campfire_db::Clock for FrozenDbClock {
    fn now(&self) -> campfire_db::Timestamp { campfire_db::Timestamp::from_jiff(campfire_kit::Clock::now(&self.0)) }
}

#[tokio::test]
async fn board_queries_and_after_commit_sequences_match_rails() {
    let app=TestApp::boot_frozen().await.expect("default seed required");
    let sink=RecordingSink::new();
    let db=Database::open(campfire_db::Config::new(app.db().path()),Env {clock:Arc::new(FrozenDbClock(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()))),sink:Arc::new(sink.clone()),..Env::default()}).unwrap();
    let vectors:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/boards_domain.json")).unwrap();
    for row in vectors["queries"].as_array().unwrap() {
        let args=row.clone();
        let ids=db.read(move |conn| Ok(ChannelThread::board_posts_for(conn,699448332,args["status"].as_str().unwrap(),args["owner"].as_str().unwrap(),args["tag"].as_str().unwrap(),Some(DAVID),args["page"].as_i64().unwrap())?.into_iter().map(|post|post.id).collect::<Vec<_>>())).await.unwrap();
        assert_eq!(serde_json::json!(ids),row["ids"]);
    }
    let mut thread_id=0;
    for step in vectors["steps"].as_array().unwrap() {
        sink.take();
        let name=step["name"].as_str().unwrap();
        if name=="create" {
            thread_id=db.write(|tx| ChannelThread::create(tx,NewChannelThread {room_id:699448332,creator_id:DAVID,name:Some("Created".into()),work_status:Some("planned".into()),..Default::default()})).await.unwrap().id;
        } else {
            let name=name.to_string();
            let result=db.write(move |tx| {
                let mut thread=ChannelThread::find(tx.conn(),thread_id)?;
                match name.as_str() {
                    "name_twice" => {thread.update_settings(tx,Some("First"),None)?;thread.update_settings(tx,Some("Second"),None)?;}
                    "tags" => thread.update_metadata(tx,None,None,Some(&["bug".into(),"api".into()]))?,
                    "replace_tags" => thread.update_metadata(tx,None,None,Some(&["api".into(),"docs".into()]))?,
                    "same_tags" => thread.update_metadata(tx,None,None,Some(&["docs".into(),"api".into()]))?,
                    "last_change_not_row" => {thread.update_settings(tx,Some("No broadcast"),None)?;thread.update_settings(tx,None,Some(60))?;}
                    "rollback" => {thread.update_metadata(tx,None,None,Some(&["rollback".into()]))?;return Err(campfire_db::Error::Other("intentional rollback".into()));}
                    "destroy" => thread.destroy(tx)?, _ => panic!("unknown Rails operation")
                }
                Ok(())
            }).await;
            if step["name"]=="rollback" {assert!(result.is_err());} else {result.unwrap();}
        }
        assert_eq!(thread_id,step["thread_id"].as_i64().unwrap());
        let callbacks=sink.take().into_iter().filter_map(|event| match event.as_broadcast()? {
            Broadcast::Turbo(frame) if frame.target.starts_with("board_") => Some(serde_json::json!({"action":match frame.action {TurboAction::Prepend=>"prepend",TurboAction::Replace=>"replace",TurboAction::Remove=>"remove",_=>panic!("unexpected board action")},"target":frame.target,"column":matches!(frame.partial,Some(Partial::BoardRow{column:true,..}))})), _=>None
        }).collect::<Vec<_>>();
        assert_eq!(serde_json::json!(callbacks),step["callbacks"],"{}",step["name"]);
        let tags=db.read(move |conn| Ok(campfire_db::ThreadTag::for_thread(conn,thread_id)?.into_iter().map(|tag|tag.name).collect::<Vec<_>>())).await.unwrap();
        assert_eq!(serde_json::json!(tags),step["tags"]);
    }
}

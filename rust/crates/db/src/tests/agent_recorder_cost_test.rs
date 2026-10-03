//! Full persisted Recorder facts and SQL growth, including commit callbacks.
use super::*;
use crate::models::activity_item::{ActivityEventType, ActivityRecordingFacts, ActivityRecordingSource, SourceAuthorization};
use crate::{ActivityItem, Room};
use serde_json::{Value, json};

struct RoomSource;
impl ActivityRecordingSource for RoomSource {
    fn recording_facts(&self, conn: &Connection) -> Result<Option<ActivityRecordingFacts>> {
        Ok(Room::find_by_id(conn, 486777696)?.map(|room| ActivityRecordingFacts {
            source_type: "Room", source_id: room.id, creator_id: Some(room.creator_id),
            thread_id: None, recipient_ids: Vec::new(),
        }))
    }
}

#[test]
fn ws11_recorder_real_output_and_read_growth_match_fresh_rails() {
    let oracle: Value = serde_json::from_str(include_str!("../../../../vectors/agent_recorder_cost.json")).unwrap();
    let mut growth = Vec::new();
    for size in [10_i64, 100] {
        let t = channel_thread_test::frozen();
        t.clock.travel_to(crate::Timestamp::from_second(1772467200));
        let ids = (0..size).map(|index|2115000000+index).collect::<Vec<_>>();
        let setup = ids.clone();
        t.write(move |tx| {
            for id in setup {
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Recorder recipient',0,0,?,?)",rusqlite::params![id,tx.now(),tx.now()])?;
            }
            tx.conn().execute("DELETE FROM activity_items",[])?;
            tx.conn().execute("UPDATE sqlite_sequence SET seq=2115100000 WHERE name='activity_items'",[])?;
            Ok(())
        });
        for phase in ["create", "repeat"] {
            t.sink.take();
            let queries = t.db.capture_queries();
            let recipients = ids.clone();
            let recorded = t.write(move |tx| ActivityItem::record_source_for_recipients(tx,&recipients,&RoomSource,ActivityEventType::parse("work_update")?,SourceAuthorization::CallerAuthorized));
            t.db.stop_capturing_queries();
            assert_eq!(recorded.len(),size as usize);
            let sql = queries.lock().unwrap().iter().map(|q| q.trim_start().to_lowercase().replace('"',"")).filter(|q|q.starts_with("select")).collect::<Vec<_>>();
            let sources = sql.iter().filter(|q|q.starts_with("select * from rooms ")).count();
            let users = sql.iter().filter(|q|q.starts_with("select * from users ")).count();
            assert_eq!((sources,users),(1,1),"Recorder preflight stays flat");
            let expected = oracle["rows"].as_array().unwrap().iter().find(|r|r["size"]==size && r["phase"]==phase).unwrap();
            let items = t.read(|conn| crate::sql::query_all(conn,"SELECT * FROM activity_items ORDER BY user_id",[],|row|Ok(json!({
                "id":row.get::<_,i64>("id")?,"user":row.get::<_,i64>("user_id")?,"source_type":row.get::<_,String>("source_type")?,"source_id":row.get::<_,i64>("source_id")?,"event":row.get::<_,String>("event_type")?,
                "read_at":row.get::<_,Option<crate::Timestamp>>("read_at")?.map(|v|v.as_second()),"handled_at":row.get::<_,Option<crate::Timestamp>>("handled_at")?.map(|v|v.as_second()),"created_at":row.get::<_,crate::Timestamp>("created_at")?.as_second(),"updated_at":row.get::<_,crate::Timestamp>("updated_at")?.as_second()
            }))));
            let frames = t.sink.take().into_iter().filter_map(|event|match event.as_broadcast() {
                Some(crate::broadcasts::Broadcast::Cable{stream,payload})=>Some(json!({"stream":stream,"payload":payload})),_=>None
            }).collect::<Vec<_>>();
            assert_eq!(json!({"items":items,"frames":frames}),json!({"items":expected["items"],"frames":expected["frames"]}),"Recorder complete persisted rows and committed frames");
            let rails = expected["selects"].as_u64().unwrap() as usize;
            println!("WS11_RECORDER_READS size={size} phase={phase} Rust={} Rails={rails} source={sources} users={users}",sql.len());
            growth.push((phase,size,sql.len(),rails));
        }
    }
    for phase in ["create","repeat"] {
        let rows = growth.iter().filter(|r|r.0==phase).collect::<Vec<_>>();
        assert!(rows[1].2-rows[0].2<=rows[1].3-rows[0].3,"Recorder per-size growth exceeds Rails: {rows:?}");
    }
}

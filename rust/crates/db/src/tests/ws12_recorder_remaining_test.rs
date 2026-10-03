//! Named original recorder clauses, compared after each real committed writer step.
use super::*;
use crate::{
    ChannelThread, Involvement, KeywordAlert, Membership, Message, NewMessage, ThreadInvolvement,
    ThreadMembership,
};
use serde_json::{Value, json};
fn facts(conn: &crate::Connection) -> crate::Result<Value> {
    let rows = crate::sql::query_all(
        conn,
        "SELECT user_id,source_type,event_type,read_at,handled_at FROM activity_items ORDER BY user_id,source_type,source_id",
        [],
        |r| {
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<crate::Timestamp>>(3)?
                    .map(|t| t.as_second()),
                r.get::<_, Option<crate::Timestamp>>(4)?
                    .map(|t| t.as_second())
            ]))
        },
    )?;
    Ok(json!(rows))
}
fn compare(key: &str) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_recorder_remaining.json"
    ))
    .unwrap();
    let row = corpus["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == key)
        .unwrap();
    let mut measured = Vec::new();
    for size in [10, 100] {
        let t = channel_thread_test::frozen();
        t.clock.travel_to(crate::Timestamp::from_second(1772467200));
        let setup = row["setup"].clone();
        t.write(move |tx| {
            for sql in setup.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;}
            for i in 0..size {
                let uid=901861000+i;
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Quiet roster member',0,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'mentions',?,?)",rusqlite::params![id("designers"),uid,tx.now(),tx.now()])?;
            }
            Ok(())
        });
        let mut last = None;
        let mut reads = 0;
        for (index, step) in row["steps"].as_array().unwrap().iter().enumerate() {
            let action = step["action"].clone();
            let prior = last;
            let client = format!("{key}-{index}");
            let queries = t.db.capture_queries();
            let created = t.write(move |tx| {
                let mut created = None;
                match action["kind"].as_str().unwrap() {
                    "keyword" => {
                        KeywordAlert::create(tx, action["user"].as_i64().unwrap(), "deploy")?;
                    }
                    "involvement" => {
                        let value = action["involvement"].as_str().unwrap();
                        if action["thread"] == true {
                            ThreadMembership::find_by_thread_and_user(
                                tx.conn(),
                                901860001,
                                id("david"),
                            )?
                            .unwrap()
                            .update_involvement(tx, ThreadInvolvement::from_name(value).unwrap())?;
                        } else {
                            Membership::find_by_room_and_user(
                                tx.conn(),
                                id("designers"),
                                id("david"),
                            )?
                            .unwrap()
                            .update_involvement(tx, Involvement::from_name(value).unwrap())?;
                        }
                    }
                    "message" => {
                        created = Some(
                            Message::create(
                                tx,
                                NewMessage {
                                    room_id: id("designers"),
                                    creator_id: action["creator"].as_i64().unwrap_or(id("jz")),
                                    thread_id: (action["thread"] == true).then_some(901860001),
                                    markdown_source: Some(action["text"].as_str().unwrap().into()),
                                    reply_to_message_id: if action["reply"] == true {
                                        prior
                                    } else {
                                        None
                                    },
                                    client_message_id: Some(client),
                                    ..Default::default()
                                },
                            )?
                            .id,
                        );
                    }
                    "work" => {
                        let mut thread = ChannelThread::find(tx.conn(), 901860001)?;
                        thread.update_work(
                            tx,
                            &crate::User::find(tx.conn(), id("jz"))?,
                            crate::models::channel_thread::WorkChanges {
                                status: Some(Some(action["status"].as_str().unwrap().into())),
                                ..Default::default()
                            },
                        )?;
                    }
                    other => panic!("unknown action {other}"),
                }
                Ok(created)
            });
            t.db.stop_capturing_queries();
            if created.is_some() {
                last = created;
            }
            reads += queries.lock().unwrap().len();
            assert_eq!(
                t.read(facts),
                step["items"],
                "{key} step {index}: complete original recorded recipients and state"
            );
        }
        println!("WS12_RECORDER_NAMED {key} roster={size} SELECTs={reads}");
        measured.push(reads);
    }
    assert_eq!(
        measured[0], measured[1],
        "{key}: recorder reads stay flat across the room roster"
    );
}

#[test]
fn ws12_recorder_c191_complete_named_rails_comparison() {
    compare("c191");
}

#[test]
fn ws12_recorder_c192_complete_named_rails_comparison() {
    compare("c192");
}

#[test]
fn ws12_recorder_c196_complete_named_rails_comparison() {
    compare("c196");
}

#[test]
fn ws12_recorder_c201_complete_named_rails_comparison() {
    compare("c201");
}

#[test]
fn ws12_recorder_c203_complete_named_rails_comparison() {
    compare("c203");
}

#[test]
fn ws12_recorder_c204_complete_named_rails_comparison() {
    compare("c204");
}

#[test]
fn ws12_recorder_c205_complete_named_rails_comparison() {
    compare("c205");
}

#[test]
fn ws12_recorder_c208_complete_named_rails_comparison() {
    compare("c208");
}

#[test]
fn ws12_recorder_c210_complete_named_rails_comparison() {
    compare("c210");
}

#[test]
fn ws12_recorder_c213_complete_named_rails_comparison() {
    compare("c213");
}

#[test]
fn ws12_recorder_c148_shared_presence_sets_and_clears_complete_rails_payload() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_recorder_remaining.json"
    ))
    .unwrap();
    let mut reads = Vec::new();
    for size in [10, 100] {
        let t = channel_thread_test::frozen();
        // Other agents must not add work to this single-agent writer.
        t.write(move |tx| { for i in 0..size {
            let uid=901875000+i;tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Other bot',2,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO agents(user_id,owner_id,created_at,updated_at) VALUES(?,?,?,?)",rusqlite::params![uid,id("david"),tx.now(),tx.now()])?;
        } Ok(()) });
        let queries = t.db.capture_queries();
        let results = t.write(|tx| {
            [Some("Thinking…"), Some("")]
                .into_iter()
                .map(|text| {
                    let r =
                        crate::models::agent_working_presence::set(tx, id("bender_agent"), text)?;
                    Ok(json!({"ok":r.is_ok(),"payload":r.payload}))
                })
                .collect::<crate::Result<Vec<_>>>()
        });
        t.db.stop_capturing_queries();
        reads.push(queries.lock().unwrap().len());
        assert_eq!(
            json!(results),
            oracle["extra"]["c148"],
            "c148 original shared service set and clear payloads"
        );
    }
    println!(
        "WS12_PRESENCE_READS agents=10/100 SELECTs={}/{}",
        reads[0], reads[1]
    );
    assert_eq!(reads[0], reads[1]);
}
#[test]
fn ws12_recorder_c177_agent_owner_filter_and_update_work_eligibility_match() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_recorder_remaining.json"
    ))
    .unwrap();
    let row = oracle["extra"]["c177"].clone();
    let mut counts = Vec::new();
    for size in [10, 100] {
        let t = channel_thread_test::frozen();
        let setup = row["setup"].clone();
        t.write(move |tx| {
            for sql in setup.as_array().unwrap() {
                tx.conn().execute_batch(sql.as_str().unwrap())?;
            }
            let mut post = ChannelThread::find(tx.conn(), 901860001)?;
            post.update_work(
                tx,
                &crate::User::find(tx.conn(), id("jz"))?,
                crate::models::channel_thread::WorkChanges {
                    status: Some(Some("planned".into())),
                    owner_id: Some(json!(id("bender"))),
                },
            )
        });
        t.write(move|tx|{for i in 0..size {let uid=901864000+i;
        tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Owner candidate',0,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'mentions',?,?)",rusqlite::params![id("designers"),uid,tx.now(),tx.now()])?;
    }Ok(())});
        let queries = t.db.capture_queries();
        let result=t.read(|conn| {
        let ids=ChannelThread::board_posts_for(conn,id("designers"),"open","agents","",None,1)?.into_iter().map(|p|p.id).collect::<Vec<_>>();
        let (humans,agents)=ChannelThread::work_owner_candidates_for(conn,id("designers"))?;
        Ok(json!({"ids":ids,"human_kevin":humans.iter().any(|u|u.id==id("kevin")),"agent_bender":agents.iter().any(|u|u.id==id("bender"))}))
    });
        t.db.stop_capturing_queries();
        counts.push(queries.lock().unwrap().len());
        assert_eq!(
            result,
            json!({"ids":row["ids"],"human_kevin":row["human_kevin"],"agent_bender":row["agent_bender"]}),
            "c177 agent filter preserves the same eligible work owner"
        );
    }
    println!(
        "WS12_OWNER_FILTER_READS members=10/100 SELECTs={}/{}",
        counts[0], counts[1]
    );
    assert_eq!(counts[0], counts[1]);
}
#[test]
fn ws12_recorder_c143_converting_missed_invitation_broadcasts_the_exact_recipient_frame() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_recorder_remaining.json"
    ))
    .unwrap();
    let row = oracle["extra"]["c143"].clone();
    let mut counts = Vec::new();
    for size in [10, 100] {
        let t = channel_thread_test::frozen();
        t.clock.travel_to(crate::Timestamp::from_second(1772467200));
        let setup = row["setup"].clone();
        t.write(move |tx|{for i in 0..size {tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Other recipient',0,0,?,?)",rusqlite::params![901864000+i,tx.now(),tx.now()])?;}for sql in setup.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;}Ok(())});
        t.sink.take();
        let queries = t.db.capture_queries();
        t.write(|tx| {
            crate::ActivityItem::refresh_unread(
                tx,
                id("david"),
                "HuddleGrant",
                901862001,
                "huddle_missed",
            )
        });
        t.db.stop_capturing_queries();
        counts.push(queries.lock().unwrap().len());
        let frames = t
            .sink
            .take()
            .iter()
            .filter_map(|e| match e.as_broadcast()? {
                crate::broadcasts::Broadcast::Cable { stream, payload } => {
                    Some(json!({"stream":stream,"payload":payload}))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!(frames),
            row["frames"],
            "c143 exact committed activity stream frame for event-type conversion"
        );
    }
    println!(
        "WS12_MISSED_FRAME_READS users=10/100 SELECTs={}/{}",
        counts[0], counts[1]
    );
    assert_eq!(counts[0], counts[1]);
}

#[test]
fn ws12_recorder_c199_room_candidates_and_fanout_stay_flat_at_two_sizes() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_recorder_remaining.json"
    ))
    .unwrap();
    let mut counts = Vec::new();
    for row in oracle["extra"]["c199"].as_array().unwrap() {
        let size = row["size"].as_i64().unwrap();
        let t = channel_thread_test::frozen();
        t.clock.travel_to(crate::Timestamp::from_second(1772467200));
        let message=t.write(move |tx| {
            tx.conn().execute("DELETE FROM activity_items",[])?;tx.conn().execute("DELETE FROM keyword_alerts",[])?;
            for i in 0..size {let uid=901863000+i;
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Quiet recipient',0,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'mentions',?,?)",rusqlite::params![id("designers"),uid,tx.now(),tx.now()])?;
            }
            KeywordAlert::create(tx,id("david"),"deploy")?;KeywordAlert::create(tx,id("kevin"),"deploy")?;
            Message::create(tx,NewMessage{room_id:id("designers"),creator_id:id("jz"),markdown_source:Some("Deploy now".into()),..Default::default()})
        });
        t.write(|tx| {
            tx.conn().execute("DELETE FROM activity_items", [])?;
            Ok(())
        });
        let queries = t.db.capture_queries();
        t.write(move |tx| crate::ActivityItem::record_message(tx, &message));
        t.db.stop_capturing_queries();
        counts.push(queries.lock().unwrap().len());
        assert_eq!(
            t.read(facts),
            row["items"],
            "c199 records exactly the two keyword holders"
        );
        println!(
            "WS12_KEYWORD_CANDIDATE_READS roster={size} SELECTs={}",
            counts.last().unwrap()
        );
    }
    assert!(counts[0] > 0);
    assert_eq!(
        counts[0], counts[1],
        "c199 candidate reads do not amplify the room roster"
    );
}

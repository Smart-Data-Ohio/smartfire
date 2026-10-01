//! Real model callbacks and real WS17 policy/delivery handoffs, recorded by Rails.
//! No ring delivery projection: Cable frames are observed immediately after commit.
use super::TestDb;
use crate::{ActivityItem, Event, Job, Membership, Session, Timestamp};
use crate::models::{huddle_grant::{HuddleGrant, JoinNoticeJob, PresenceJob}, huddle_invitations, huddle_notices::{self, PushInvitationJob, PushRequest}, notification_push};
use crate::models::room_delete::HuddleConfig;
use serde_json::{Value, json};

fn config() -> HuddleConfig {HuddleConfig {api_secret:Some("ws13b-review-fixture-value".into()),admin_configured:false}}
fn recipient() -> i64 {crate::fixtures::identify("jason")}
fn caller() -> i64 {crate::fixtures::identify("david")}
fn room() -> i64 {crate::fixtures::identify("david_and_jason")}

fn collect(db: &TestDb, pending: &mut Vec<crate::JobRequest>, frames: &mut Vec<Value>, pushes: &mut Vec<Value>) {
    for event in db.sink.take() {
        if let Some(crate::broadcasts::Broadcast::Cable {stream,payload}) = event.as_broadcast() {
            if (stream == format!("user_{}_activity",recipient()) && !payload["huddleInvitation"].is_null()) || (stream == format!("user_{}_huddle_notices",recipient()) && !payload["huddleJoinNotice"].is_null()) {
                frames.push(json!({"stream":stream,"payload":payload}));
            }
        }
        if let Event::Job(job) = event {
            if [PushInvitationJob::CLASS,JoinNoticeJob::CLASS,PresenceJob::CLASS].contains(&job.class) {pending.push(job);}
            else if job.class == PushRequest::CLASS {
                let request = serde_json::from_value(job.arguments).unwrap();
                db.write(move |tx| notification_push::enqueue_huddle_request(tx,request));
                collect(db,pending,frames,pushes);
            } else if [notification_push::HuddleInvitationDeliveryJob::CLASS,notification_push::HuddleJoinDeliveryJob::CLASS].contains(&job.class) {
                pushes.push(job.arguments);
            }
        }
    }
}
fn items(db: &TestDb) -> Value {
    db.read(|conn| {
        let mut q = conn.prepare("SELECT * FROM activity_items WHERE user_id=? AND source_type='HuddleGrant' ORDER BY id")?;
        let rows = q.query_map([recipient()], |r| {
            let read:Option<Timestamp> = r.get("read_at")?;
            let handled:Option<Timestamp> = r.get("handled_at")?;
            let created:Timestamp = r.get("created_at")?;
            Ok(json!({"id":r.get::<_,i64>("id")?,"source_id":r.get::<_,i64>("source_id")?,"event_type":r.get::<_,String>("event_type")?,"state":if handled.is_some(){"handled"}else if read.is_some(){"read"}else{"unread"},"created_at":format!("{:.6}",created.jiff())}))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!(rows))
    })
}
fn run_case(case: &Value) -> Vec<Value> {
    let db = TestDb::new();
    db.clock.travel_to(Timestamp::parse_db("2026-01-01 12:00:00").unwrap());
    let banner = case["spec"]["banner"].as_bool().unwrap();
    let (mut session,member) = db.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items",[])?;
        tx.conn().execute("DELETE FROM huddle_cleanups",[])?;
        tx.conn().execute("DELETE FROM huddle_grants",[])?;
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name IN ('activity_items','huddle_grants')",[])?;
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",rusqlite::params![if banner {r#"{"huddle_invitations":false}"#} else {"{}"},recipient()])?;
        Ok((Session::start(tx,caller(),None,None)?.id,Membership::find_by_room_and_user(tx.conn(),room(),caller())?.unwrap().id))
    });
    db.sink.take();
    let mut grant = 0;
    let mut recipient_session = None;
    let mut pending = Vec::<crate::JobRequest>::new();
    let mut phases = Vec::new();
    for operation in case["spec"]["steps"].as_array().unwrap() {
        db.travel(operation["seconds"].as_i64().unwrap());
        let action = operation["action"].as_str().unwrap();
        let mut frames = Vec::new();let mut pushes = Vec::new();
        if action == "drain" {
            if operation["job_order"].is_null() && (operation["order"] == "newest" || (operation["order"].is_null() && case["spec"]["newest_first"] == true)) {pending.reverse();}
            // Rails records shuffled source-job selection; use that observed order,
            // never infer it from the resulting payloads or from Rust state.
            let indices = operation["job_order"].as_array();
            let count = operation["limit"].as_u64().map_or(pending.len(), |n| (n as usize).min(pending.len()));
            let jobs = if let Some(indices) = indices {
                let old = std::mem::take(&mut pending);
                let chosen = indices.iter().map(|v|v.as_u64().unwrap() as usize).collect::<Vec<_>>();
                assert!(chosen.iter().all(|i| *i < old.len()),"{} operation {:?}: Rails chose {:?}, Rust has {:?}",case["spec"]["name"],operation,chosen,old);
                pending = old.iter().enumerate().filter(|(i,_)|!chosen.contains(i)).map(|(_,j)|j.clone()).collect();
                chosen.into_iter().map(|i|old[i].clone()).collect::<Vec<_>>()
            } else {pending.drain(..count).collect::<Vec<_>>()};
            for job in jobs {
                let args = job.arguments;
                match job.class {
                    PushInvitationJob::CLASS => {let id = args["activity_item_id"].as_i64().unwrap();db.write(move |tx| huddle_notices::push_invitation(tx,id));}
                    JoinNoticeJob::CLASS => {let id = args["grant_id"].as_i64().unwrap();db.write(move |tx| huddle_notices::notify_join(tx,id));}
                    PresenceJob::CLASS => {} // Its room stream has no invitation client frames.
                    _ => unreachable!(),
                }
                collect(&db,&mut pending,&mut frames,&mut pushes);
            }
        } else if action == "new_session" {
            session = db.write(|tx| Ok(Session::start(tx,caller(),None,None)?.id));
            grant = db.write(move |tx| HuddleGrant::issue(tx,session,member,room(),&config())).id;
        } else if action == "group_continues" {
            let live = db.write(|tx| {
                let other = crate::fixtures::identify("kevin");
                let m = Membership::create_default(tx,room(),other)?;
                let s = Session::start(tx,other,None,None)?;
                Ok(HuddleGrant::issue(tx,s.id,m.id,room(),&config())?.id)
            });
            db.write(move |tx| {
                HuddleGrant::find_by_id(tx.conn(),live)?.unwrap().record_seen(tx)?;
                let mut g = HuddleGrant::find_by_id(tx.conn(),grant)?.unwrap();g.record_seen(tx)?;g.revoke(tx,false,&config())?;Ok(())
            });
        } else if action == "rejoin" {
            let s = *recipient_session.get_or_insert_with(|| db.write(|tx| Ok(Session::start(tx,recipient(),None,None)?.id)));
            let id = db.write(move |tx| {let m = Membership::find_by_room_and_user(tx.conn(),room(),recipient())?.unwrap();Ok(HuddleGrant::issue(tx,s,m.id,room(),&config())?.id)});
            db.write(move |tx| HuddleGrant::find_by_id(tx.conn(),id)?.unwrap().record_seen(tx));
        } else {
            let owned = action.to_owned();
            let next = db.write(move |tx| {
                match owned.as_str() {
                    "issue"|"retry"|"regrant" => return Ok(Some(HuddleGrant::issue(tx,session,member,room(),&config())?.id)),
                    "banner"|"inbox"|"toggle_preferences" => {
                        let enabled = owned == "inbox" || (owned == "toggle_preferences" && !huddle_notices::invitations_enabled(tx.conn(),recipient())?);
                        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",rusqlite::params![json!({"huddle_invitations":enabled}).to_string(),recipient()])?;
                    }
                    "quiet_revoke" => {HuddleGrant::find_by_id(tx.conn(),grant)?.unwrap().revoke(tx,false,&config())?;}
                    "live_revoke"|"end"|"remove_caller"|"sign_out" => {
                        let mut g = HuddleGrant::find_by_id(tx.conn(),grant)?.unwrap();g.record_seen(tx)?;
                        match owned.as_str() {
                            "live_revoke" => {g.revoke(tx,false,&config())?;}
                            "end" => {g.mark_out_of_call(tx,None)?;}
                            "remove_caller" => {Membership::find(tx.conn(),member)?.destroy(tx)?;}
                            "sign_out" => {Session::find(tx.conn(),session)?.destroy(tx)?;}
                            _ => unreachable!(),
                        }
                    }
                    "remove_recipient" => {Membership::find_by_room_and_user(tx.conn(),room(),recipient())?.unwrap().destroy(tx)?;}
                    "read"|"dismiss"|"handled"|"unread_cycle" => {
                        let id:Option<i64> = tx.conn().query_row("SELECT MAX(id) FROM activity_items WHERE user_id=? AND source_type='HuddleGrant'",[recipient()],|r|r.get(0))?;
                        if let Some(id) = id {
                            let item = ActivityItem::find(tx.conn(),id)?;
                            match owned.as_str() {
                                "read"|"dismiss" => {item.mark_read(tx)?;}
                                "handled" => {item.mark_handled(tx)?;}
                                "unread_cycle" => {item.mark_handled(tx)?;ActivityItem::refresh_unread(tx,recipient(),"HuddleGrant",item.source_id,"huddle_started")?;}
                                _ => unreachable!(),
                            }
                        }
                    }
                    "missed" => {huddle_invitations::resolve_overdue(tx,Some(recipient()))?;}
                    _ => panic!("unknown action {owned}"),
                }
                Ok(None)
            });
            if let Some(id) = next {grant = id;}
        }
        collect(&db,&mut pending,&mut frames,&mut pushes);
        phases.push(json!({"frames":frames,"pushes":pushes,"items":items(&db)}));
    }
    phases
}
fn oracle() -> Value {
    let input = std::env::var("WS13B_OBSERVED_ORACLE").map(|path| std::fs::read_to_string(path).unwrap()).unwrap_or_else(|_|include_str!("../models/huddle_observed_matrix.json").to_owned());
    serde_json::from_str(&input).unwrap()
}
fn partition(part: usize) {
    let oracle = oracle();assert_eq!(oracle["reference_pin"],"d7c7de92");
    assert!(!oracle["cases"].as_array().unwrap().is_empty(),"Rails corpus is required");
    let mut failures = Vec::new();let mut count = 0;let mut observations = Vec::new();
    for (index,case) in oracle["cases"].as_array().unwrap().iter().enumerate() {
        if index % 8 != part {continue;}
        let actual = run_case(case);
        let mut expected = case["phases"].clone();
        for phase in expected.as_array_mut().unwrap() {phase.as_object_mut().unwrap().remove("banner");}
        observations.push(json!({"spec":case["spec"],"phases":actual}));
        if actual != *expected.as_array().unwrap() {
            let phase = actual.iter().zip(expected.as_array().unwrap()).position(|(a,e)|a!=e).unwrap();
            failures.push(json!({"name":case["spec"]["name"],"spec":case["spec"],"phase":phase,"actual":actual,"expected":case["phases"]}));
        }
        count += 1;
    }
    if let Ok(dir) = std::env::var("WS13B_DIFFERENTIAL_OUTPUT") {
        std::fs::write(format!("{dir}/observations-{part}.json"),serde_json::to_vec(&observations).unwrap()).unwrap();
        std::fs::write(format!("{dir}/mismatches-{part}.json"),serde_json::to_vec_pretty(&failures).unwrap()).unwrap();
    }
    assert!(failures.is_empty(),"differential partition {part}: {} mismatches; first: {}",failures.len(),failures.first().unwrap_or(&Value::Null));
    println!("Observed Rails differential: partition {part}: {count} sequences matched every operation");
}
macro_rules! partitions {($($name:ident:$n:literal),*)=>{$(#[test] fn $name(){partition($n);})*};}
partitions!(observed_rails_differential_0:0,observed_rails_differential_1:1,observed_rails_differential_2:2,observed_rails_differential_3:3,observed_rails_differential_4:4,observed_rails_differential_5:5,observed_rails_differential_6:6,observed_rails_differential_7:7);

fn named_regression(name: &str) {
    let oracle = oracle();
    let case = oracle["cases"].as_array().unwrap().iter().find(|case| case["spec"]["name"] == name).unwrap();
    let mut expected = case["phases"].clone();
    for phase in expected.as_array_mut().unwrap() {phase.as_object_mut().unwrap().remove("banner");}
    assert_eq!(json!(run_case(case)),expected,"{name}");
}
#[test]
fn repeated_handled_timestamp_broadcasts_once() {named_regression("shrunk/random/3620200082/68");}
#[test]
fn rejoin_answer_at_same_timestamp_broadcasts_once() {named_regression("shrunk/random/3620200082/52");}
#[test]
fn banner_to_inbox_repeated_answer_broadcasts_once() {named_regression("shrunk/random/388013012/802");}

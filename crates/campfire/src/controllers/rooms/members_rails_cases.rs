//! Named ports of our Rails members controller: owner readers and live viewer facts.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room, RoomType, Session, Timestamp, User, WorkspacePresenceLease};
use serde_json::{Value, json};
const DESIGNERS:i64=654632876;
const JZ:i64=773523953;
async fn setup()->TestApp {TestApp::boot_frozen().await.expect("seed required")}
fn request(room:i64)->Req {Req::new(Method::GET,&format!("/rooms/{room}/members.json"))}
async fn members(app:&TestApp,user:i64,room:i64)->Vec<Value> {
 let reply=app.sign_in(user).await.send(request(room)).await;
 assert_eq!(reply.status,StatusCode::OK,"{}",reply.text());
 assert_eq!(reply.header("cache-control"),Some("no-store"));
 assert_eq!(reply.header("pragma"),Some("no-cache"));
 assert!(reply.header("etag").is_some());
 assert_eq!(reply.content_type(),Some("application/json; charset=utf-8"));
 reply.json()["members"].as_array().unwrap().clone()
}
fn member(rows:&[Value],id:i64)->&Value {rows.iter().find(|v|v["id"]==id).unwrap()}
async fn lease(app:&TestApp)->(i64,i64) {app.db().write(|tx| {
 let session=Session::start(tx,JASON,None,None)?;
 let lease=WorkspacePresenceLease::establish(tx,JASON,session.id)?.unwrap();
 Ok((session.id,lease.id))
}).await.unwrap()}
#[tokio::test]
async fn requires_authentication() {
 let app=setup().await;
 let reply=app.anonymous().send(request(DESIGNERS)).await;
 assert_eq!(reply.status,StatusCode::UNAUTHORIZED);assert!(reply.body.is_empty());
 let reply=app.anonymous().get(&format!("/rooms/{DESIGNERS}/members")).await;
 assert_eq!(reply.status,StatusCode::FOUND);assert_eq!(reply.location(),Some("http://campfire.test/session/new"));
}
#[tokio::test]
async fn does_not_expose_members_of_an_inaccessible_room() {
 let app=setup().await;
 let id=app.db().write(|tx| {let room=Room::create(tx,RoomType::Closed,Some("Private"),JASON)?;room.grant_to(tx,&[JASON])?;Ok(room.id)}).await.unwrap();
 for id in [id,DIRECT_KEVIN_BENDER,999999] {let reply=app.david().send(request(id)).await;assert_eq!(reply.status,StatusCode::NOT_FOUND);assert!(reply.body.is_empty());}
}
#[tokio::test]
async fn member_reads_deny_bot_credentials() {
 let app=setup().await;
 let reply=app.anonymous().send(Req::new(Method::GET,&format!("/rooms/{DESIGNERS}/members.json?bot_key={BENDER_KEY}"))).await;
 assert_eq!(reply.status,StatusCode::FORBIDDEN);assert!(reply.body.is_empty());
}
#[tokio::test]
async fn returns_active_room_members_with_presence_and_status_fields() {
 let app=setup().await;lease(&app).await;
 app.db().write(|tx| {tx.conn().execute_cached("UPDATE users SET custom_status_emoji='🚂',custom_status_text='On a train' WHERE id=?",[JASON])?;Ok(())}).await.unwrap();
 let rows=members(&app,DAVID,DESIGNERS).await;
 let ordered=app.db().read(|conn|Ok(conn.prepare("SELECT users.id FROM users JOIN memberships ON memberships.user_id=users.id WHERE memberships.room_id=? AND users.status=0 ORDER BY LOWER(users.name),users.id")?.query_map([DESIGNERS],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?)).await.unwrap();
 assert_eq!(rows.iter().map(|v|v["id"].as_i64().unwrap()).collect::<Vec<_>>(),ordered);
 assert_eq!(member(&rows,JASON)["presence"],"online");assert_eq!(member(&rows,JASON)["status"],"🚂 On a train");assert_eq!(member(&rows,JASON)["bot"],false);
 assert_eq!(member(&rows,KEVIN)["online"],false);assert_eq!(member(&rows,KEVIN)["presence"],"offline");assert_eq!(member(&rows,KEVIN)["status"],Value::Null);
 for row in rows {assert_eq!({let mut keys=row.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>();keys.sort();keys},vec!["avatar_url","bot","id","name","online","presence","starred","status"]);assert!(row["avatar_url"].as_str().unwrap().starts_with("http://campfire.test/users/"));}
}
#[tokio::test]
async fn flags_bots_so_the_picker_can_exclude_them_from_huddles() {
 let app=setup().await;let rows=members(&app,DAVID,ALL_TALK).await;
 assert_eq!(member(&rows,BENDER)["bot"],true);assert_eq!(member(&rows,DAVID)["bot"],false);
}
#[tokio::test]
async fn returns_the_meeting_label_for_members_in_a_meeting() {
 let app=setup().await;
 app.db().write(|tx| {
 tx.conn().execute_cached("UPDATE users SET meeting_status_enabled=1 WHERE id=?",[JASON])?;
 let pairs=json!([[tx.now().ago(jiff::SignedDuration::from_mins(5)).jiff().to_string(),tx.now().since(jiff::SignedDuration::from_mins(55)).jiff().to_string()]]).to_string();
 tx.conn().execute_cached("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,fetched_at,created_at,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET busy_intervals=excluded.busy_intervals,fetched_at=excluded.fetched_at",rusqlite::params![JASON,pairs,tx.now(),tx.now(),tx.now()])?;Ok(())
 }).await.unwrap();
 assert_eq!(member(&members(&app,DAVID,DESIGNERS).await,JASON)["status"],"📅 In a meeting");
}
#[tokio::test]
async fn returns_the_ooo_label_for_members_out_of_office() {
 let app=TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new("2026-09-20T12:00:00Z".parse().unwrap()))).await.unwrap();
 app.db().write(|tx| {tx.conn().execute_cached("UPDATE users SET time_zone='UTC',ooo_until=?,ooo_note='Back soon' WHERE id=?",rusqlite::params![Timestamp::from_jiff("2026-09-24T12:00:00Z".parse().unwrap()),JASON])?;Ok(())}).await.unwrap();
 assert_eq!(member(&members(&app,DAVID,DESIGNERS).await,JASON)["status"],"🌴 Out of office until September 24, 2026 — Back soon");
}
#[tokio::test]
async fn reports_idle_and_do_not_disturb_presence() {
 let app=setup().await;let (_,id)=lease(&app).await;
 app.db().write(move|tx| {tx.conn().execute_cached("UPDATE workspace_presence_leases SET last_active_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(11)),id])?;Ok(())}).await.unwrap();
 for (setting,presence,online) in [("online","idle",true),("dnd","dnd",true),("invisible","offline",false)] {
  app.db().write(move|tx| {tx.conn().execute_cached("UPDATE users SET presence_setting=? WHERE id=?",rusqlite::params![setting,JASON])?;Ok(())}).await.unwrap();
  let rows=members(&app,DAVID,DESIGNERS).await;assert_eq!(member(&rows,JASON)["presence"],presence);assert_eq!(member(&rows,JASON)["online"],online);
 }
}
#[tokio::test]
async fn does_not_return_inactive_users() {
 let app=setup().await;app.db().write(|tx|User::find(tx.conn(),JASON)?.deactivate(tx)).await.unwrap();
 assert!(members(&app,DAVID,DESIGNERS).await.iter().all(|v|v["id"]!=JASON));
}
#[tokio::test]
async fn starred_is_per_viewer_and_never_leaks_between_viewers() {
 let app=setup().await;
 app.db().write(|tx| {for (viewer,target) in [(DAVID,KEVIN),(JASON,JZ)] {tx.conn().execute_cached("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![viewer,target,tx.now(),tx.now()])?;}Ok(())}).await.unwrap();
 for (viewer,target,other) in [(DAVID,KEVIN,JZ),(JASON,JZ,KEVIN),(DAVID,KEVIN,JZ)] {let rows=members(&app,viewer,DESIGNERS).await;assert_eq!(member(&rows,target)["starred"],true);assert_eq!(member(&rows,other)["starred"],false);assert_eq!(member(&rows,viewer)["starred"],false);}
 app.db().write(|tx|{tx.conn().execute_cached("DELETE FROM user_stars WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
 assert_eq!(member(&members(&app,DAVID,DESIGNERS).await,KEVIN)["starred"],false);
}
async fn bot(app:&TestApp,seen:bool,suspended:bool) {
 app.db().write(move|tx| {Room::find(tx.conn(),DESIGNERS)?.grant_to(tx,&[BENDER])?;tx.conn().execute_cached("UPDATE agents SET last_seen_at=?,suspended_at=? WHERE user_id=?",rusqlite::params![seen.then(||tx.now()),suspended.then(||tx.now()),BENDER])?;Ok(())}).await.unwrap();
 let rows=members(app,DAVID,DESIGNERS).await;let member=member(&rows,BENDER);
 assert_eq!(member["online"],seen&&!suspended);assert_eq!(member["presence"],if seen&&!suspended {"agent"} else {"offline"});assert_eq!(member["status"],"Idle");
}
#[tokio::test] async fn reports_a_bot_with_a_checked_in_agent_as_online() {let app=setup().await;bot(&app,true,false).await;}
#[tokio::test] async fn reports_a_bot_as_offline_until_its_agent_checks_in() {let app=setup().await;bot(&app,false,false).await;}
#[tokio::test] async fn reports_a_bot_with_a_suspended_agent_as_offline() {let app=setup().await;bot(&app,true,true).await;}
#[tokio::test]
async fn revoked_sessions_immediately_make_a_member_offline() {
 let app=setup().await;let(session,_)=lease(&app).await;
 assert_eq!(member(&members(&app,DAVID,DESIGNERS).await,JASON)["online"],true);
 app.db().write(move|tx|Session::find(tx.conn(),session)?.destroy(tx)).await.unwrap();
 assert_eq!(member(&members(&app,DAVID,DESIGNERS).await,JASON)["online"],false);
}

#[tokio::test]
async fn complete_member_json_matches_rails_for_each_viewer_and_room() {
 let app=setup().await;
 let fixtures:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-support/legacy-contracts/rooms/members.json"))).unwrap();
 for row in fixtures["rows"].as_array().unwrap() {
  let reply=app.sign_in(row["viewer_id"].as_i64().unwrap()).await.send(request(row["room_id"].as_i64().unwrap())).await;
  assert_eq!(reply.status,StatusCode::OK);
  assert_eq!(reply.text(),row["body"].as_str().unwrap());
  for (name,value) in row["headers"].as_object().unwrap() {assert_eq!(reply.header(name),value.as_str());}
 }
}

//! Named Rails refresh cases; pins are composed solely through WS8bm2's list seam.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method,StatusCode};
use askama::Template;
use campfire_views::helpers::request_forgery::{AuthenticityTokens,RequestSecrets,rendering_with};
use campfire_db::{CachedStatements, Membership, Message, MessagePin, NewMessage, Room, Timestamp};
use campfire_kit::{Clock,FrozenClock};
use serde_json::Value;
use std::sync::Arc;
fn request(since:jiff::Timestamp)->Req {Req::new(Method::GET,&format!("/rooms/{ALL_TALK}/refresh.turbo_stream?since={}",since.as_millisecond()))}
async fn setup()->(TestApp,Arc<FrozenClock>) {
 let clock=Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
 let app=TestApp::boot_with_test_clock(clock.clone()).await.unwrap();(app,clock)
}
async fn create(app:&TestApp,name:&str)->Message {
 let name=name.to_string();app.db().write(move|tx|Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,body:Some(name.clone()),client_message_id:Some(name.replace(' ',"-")),..Default::default()})).await.unwrap()
}
#[tokio::test]
async fn refresh_includes_new_messages_since_the_last_known() {
 let (app,clock)=setup().await;let old=create(&app,"old").await;
 clock.advance(jiff::SignedDuration::from_hours(24));let new=create(&app,"new").await;
 app.db().write(move|tx|{tx.conn().execute_cached("UPDATE messages SET updated_at=? WHERE id=?",rusqlite::params![tx.now(),old.id])?;Ok(())}).await.unwrap();
 let reply=app.david().send(request(clock.now().checked_sub(jiff::SignedDuration::from_mins(10)).unwrap())).await;
 assert_eq!(reply.status,StatusCode::OK,"{}",reply.text());
 assert!(reply.text().contains("action=\"append\" target=\"messages_rooms_closed_486777696\""));
 assert!(reply.text().contains("id=\"message_new\""));assert!(reply.text().contains("action=\"replace\" target=\"message_old\""));
 assert!(reply.text().contains("Old")||reply.text().contains("old"));assert_eq!(new.client_message_id,"new");
}
#[tokio::test]
async fn refresh_includes_pin_and_unpin_changes_since_the_last_sync() {
 let(app,clock)=setup().await;let pinned=create(&app,"Stays pinned").await;let unpinned=create(&app,"Gets unpinned").await;
 let prior=unpinned.clone();let pin=app.db().write(move|tx|Ok(MessagePin::pin(tx,&prior,DAVID)?.unwrap())).await.unwrap();
 let since=clock.now();clock.advance(jiff::SignedDuration::from_mins(1));
 let target=pinned.clone();app.db().write(move|tx|{MessagePin::pin(tx,&target,DAVID)?.unwrap();pin.unpin(tx)}).await.unwrap();
 let reply=app.david().send(request(since)).await;let html=reply.text();assert_eq!(reply.status,StatusCode::OK,"{html}");
 for message in [&pinned,&unpinned] {assert!(html.contains(&format!("action=\"replace\" target=\"message_{}\"",message.client_message_id)));}
 assert!(html.contains("id=\"pin_badge_message_Stays-pinned\""));
 assert!(html.contains("id=\"pin_badge_message_Gets-unpinned\" class=\"message__pin-badge\" hidden"));
 assert!(html.contains("action=\"replace\" target=\"pins_count_rooms_closed_486777696\""));
 assert!(html.contains("aria-label=\"1 pinned messages\">1</span>"));
 assert!(html.contains("action=\"replace\" target=\"pins_list_rooms_closed_486777696\""));
 assert!(html.contains("class=\"pins-panel__excerpt\">Stays pinned</p>"));
 assert!(!html.contains("class=\"pins-panel__excerpt\">Gets unpinned</p>"));
}
#[tokio::test]
async fn refresh_with_no_changes_is_a_quiet_204() {
 let(app,clock)=setup().await;let reply=app.david().send(request(clock.now())).await;assert_eq!(reply.status,StatusCode::NO_CONTENT);assert!(reply.body.is_empty());
}
#[tokio::test]
async fn refreshing_a_room_the_user_no_longer_belongs_to_is_a_quiet_404() {
 let(app,_)=setup().await;assert_eq!(app.david().send(request(jiff::Timestamp::UNIX_EPOCH)).await.status,StatusCode::OK);
 app.db().write(|tx|Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().destroy(tx)).await.unwrap();
 assert_eq!(app.david().send(request(jiff::Timestamp::UNIX_EPOCH)).await.status,StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn pin_only_refresh_matches_rails_bytes_and_request_token_ownership() {
 let(app,clock)=setup().await;
 let mut browser=app.sign_in(DAVID).await;
 let oracle:Value=serde_json::from_str(include_str!("../../../../views/tests/golden/rooms/pin_refresh.json")).unwrap();
 let tokens=regex::Regex::new(r#"name="authenticity_token" value="([^"]+)""#).unwrap();
 for row in oracle["rows"].as_array().unwrap() {
  if row["pinned"]==true {let id=row["message_id"].as_i64().unwrap();app.db().write(move|tx|{let message=Message::find(tx.conn(),id)?;MessagePin::pin(tx,&message,DAVID)?.unwrap();Ok(())}).await.unwrap();}
  app.db().write(|tx|{tx.conn().execute_cached("UPDATE rooms SET pins_changed_at=? WHERE id=?",rusqlite::params![Timestamp::from_jiff(tx.now().jiff().checked_add(jiff::SignedDuration::from_mins(1)).unwrap()),ALL_TALK])?;Ok(())}).await.unwrap();
  let reply=browser.send(request(clock.now())).await;assert_eq!(reply.status,StatusCode::OK);
  if row["pinned"]==false {assert_eq!(reply.text(),row["body"].as_str().unwrap());}
  else {
   let html=reply.text();
   let token=tokens.captures(&html).unwrap()[1].to_string();
   let path=format!("/messages/{}/pin",row["message_id"].as_i64().unwrap());
   assert!(browser.real_authenticity_token().unwrap().is_valid(&token,&path,"delete"));
   let mut other=app.sign_in(JASON).await;other.get("/rooms/654632876").await;
   assert!(!other.real_authenticity_token().unwrap().is_valid(&token,&path,"delete"));
  }
  // No response bytes are normalized: compare the actual owner factory/view with fixed
  // request secrets, just as the Rails rendering instrumentation does.
  let state=app.booted.app.clone();
  let rendered=app.db().read(move|conn| {
   let room=Room::find(conn,ALL_TALK)?;
   let refresh=campfire_views::rooms::RefreshView{room_id:ALL_TALK,room_kind:campfire_views::messages::RoomKind::Closed,new_messages:vec![],updated_messages:vec![],pins:Some(super::pins::list(conn,&state,&room)?)};
   let account=campfire_db::Account::first(conn)?;
   rendering_with(RequestSecrets{tokens:Box::new(FixedTokens),csp_nonce:None},||crate::controllers::presenters::page::render_detached_at(&state,account.as_ref(),"http://campfire.test",|ctx|campfire_views::rooms::RefreshShow{ctx,refresh:&refresh}.render())).map_err(|e|campfire_db::Error::Other(e.to_string()))
  }).await.unwrap();
  assert_eq!(rendered,row["body"].as_str().unwrap());
 }
}

struct FixedTokens;
impl AuthenticityTokens for FixedTokens {fn global(&self)->String {"GLOBAL".into()} fn for_form(&self,_:&str,_:&str)->String {"GLOBAL".into()}}

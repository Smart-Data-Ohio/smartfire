//! One named port for every built-in DispatcherTest case, through the app adapter.
use super::quote_integration_tests::{insert_rows,stream};
use crate::controllers::presenters::{Presenter,test_support::*};
use campfire_db::{Message,Timestamp,slash_commands::{self,Context}};
use serde_json::{Value,json};
fn oracle()->Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/slash_named.json")).unwrap()}
fn stamp(t:Timestamp)->String {let s=t.to_db();if s.contains('.') {s} else {format!("{s}.000000")}}
fn sql(v:&Value)->rusqlite::types::Value {match v {Value::Null=>rusqlite::types::Value::Null,Value::Bool(b)=>rusqlite::types::Value::Integer(i64::from(*b)),Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),Value::String(s)=>rusqlite::types::Value::Text(s.clone()),_=>panic!("scalar fixture")}}
async fn run(index:usize) {
 let data=oracle();let case=data["cases"][index].clone();let name=case["name"].as_str().unwrap();
 if index==0 {assert_eq!(serde_json::to_value(slash_commands::registry()).unwrap(),case["observations"],"{name}");return;}
 if index==1 {for row in case["observations"].as_array().unwrap() {assert_eq!(slash_commands::command_text(row["text"].as_str().unwrap()),row["recognized"].as_bool().unwrap(),"{name}");}return;}
 let app=TestApp::boot_with_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new(data["now"].as_str().unwrap().parse().unwrap()))).await.unwrap().without_job_runner().await;
 insert_rows(&app,case["rows"].clone()).await;
 let mut socket=None;
 if index==21 {
  let (mut client,server)=stream(&app).await;
  for suffix in ["status","ooo_notice"] {
   let gid=campfire_views::helpers::gid_param("User",DAVID);
   let signed=rails_compat::turbo::signed_stream_name(&app.booted.app.secrets,&[&gid,suffix]);
   client.confirm(&crate::channels::tests::support::identifier(json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}))).await;
  }
  socket=Some((client,server));
 }
 let initial=case["initial"].clone();
 app.db().write(move |tx| {
  for(k,v)in initial.as_object().unwrap() {tx.conn().execute(&format!("UPDATE users SET {k}=? WHERE id=?"),rusqlite::params![sql(v),DAVID])?;}
  tx.conn().execute("DELETE FROM background_jobs",[])?;Ok(())
 }).await.unwrap();
 for row in case["observations"].as_array().unwrap() {
  let context=Context{user_id:DAVID,room_id:case["room_id"].as_i64().unwrap(),thread_id:row["thread_id"].as_i64(),huddles_configured:case["huddle"].as_bool().unwrap()};
  let text=row["text"].as_str().unwrap().to_owned();let runtime=app.booted.app.clone();
  app.db().write(|tx|{tx.conn().execute("DELETE FROM background_jobs",[])?;Ok(())}).await.unwrap();
  let capture=app.db().capture_queries();
  let result=app.db().write(move |tx|crate::controllers::rooms::slash_commands::dispatch(tx,&context,&text,runtime.storage.clone())).await.unwrap();
  app.db().stop_capturing_queries();
  let attachment_calls=capture.lock().unwrap().iter().filter(|q|q.contains("WHERE a.record_type = 'Message'")&&q.contains("a.name = 'attachment'")&&q.contains("ORDER BY a.id LIMIT 1")).count();
  let id=result.message_id;let mut result=serde_json::to_value(&result).unwrap();result["payload"]=result_payload(id,result["kind"].as_str().unwrap(),case["room_id"].as_i64().unwrap());
  let runtime=app.booted.app.clone();
  let mut actual=app.db().read(move |conn| {
   let p=Presenter::new(conn,&runtime,None);let mut state=serde_json::Map::new();
   for key in ["custom_status_emoji","custom_status_text","custom_status_expires_at","dnd_enabled","dnd_until","ooo_until","ooo_note"] {
    let v:rusqlite::types::Value=conn.query_row(&format!("SELECT {key} FROM users WHERE id=?"),[DAVID],|r|r.get(0))?;
    let v=match v {rusqlite::types::Value::Null=>Value::Null,rusqlite::types::Value::Integer(n)=>json!(n!=0),rusqlite::types::Value::Text(s)=>if key.ends_with("_at")||key.ends_with("_until") {json!(stamp(Timestamp::parse_db(&s).unwrap()))}else{json!(s)},_=>panic!("state scalar")};state.insert(key.into(),v);
   }
   let message=if let Some(id)=id {let m=Message::find(conn,id)?;let plain=p.plain_text_body(&m)?;json!({"id":id,"markdown_source":m.markdown_source,"action":m.action,"streaming":m.streaming,"thread_id":m.thread_id,"sound":m.sound(conn,runtime.db.env().rich_text.as_ref())?.map(|s|s.name),"plain":plain})}else{Value::Null};
   let saved=id.map(|id|campfire_db::SavedItem::find_by_user_and_message(conn,DAVID,id)).transpose()?.flatten().map(|s|json!({"status":s.status,"remind_at":s.remind_at.map(stamp)})).unwrap_or(Value::Null);
   let jobs:i64=conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Bot::WebhookJob'",[],|r|r.get(0))?;
   Ok(json!({"result":result,"state":state,"message":message,"saved":saved,"legacy_jobs":jobs,"attachment_calls":attachment_calls,"frames":null}))
  }).await.unwrap();
  if let Some((client,_))=&mut socket {
   let mut frames=vec![];
   for frame in row["frames"].as_array().unwrap() {let wire:Value=serde_json::from_str(&client.next_text().await).unwrap();frames.push(json!({"stream":frame["stream"],"html":wire["message"]}));}
   client.assert_silent().await;actual["frames"]=json!(frames);
  }
  let expected=json!({"result":row["result"],"state":row["state"],"message":row["message"],"saved":row["saved"],"legacy_jobs":row["legacy_jobs"],"attachment_calls":row["attachment_calls"],"frames":row["frames"]});
  assert_eq!(actual,expected,"{name}: {}",row["text"]);
 }
 if let Some((mut client,server))=socket {client.socket.close(None).await.unwrap();server.abort();}
 println!("WS8bm2 named slash: {name} matched Rails");
}
fn result_payload(id:Option<i64>,kind:&str,room:i64)->Value {if let Some(id)=id {json!({"message_id":id})}else if kind=="start_huddle" {json!({"room_id":room})}else{json!({})}}
macro_rules! named {($name:ident,$index:literal)=> {#[tokio::test] async fn $name(){run($index).await;}};}
named!(builtin_registry_holds_every_shipped_command_with_metadata,0);
named!(builtin_command_text_matches_slash_commands_but_not_escapes_or_play_passthrough,1);
named!(builtin_huddle_starts_a_call_when_configured,2);
named!(builtin_huddle_errors_when_unconfigured,3);
named!(builtin_event_opens_the_prefilled_form_url,4);
named!(builtin_event_without_a_time_prefills_the_title_only,5);
named!(builtin_event_rejects_past_times,6);
named!(builtin_bare_event_opens_the_blank_form,7);
named!(builtin_poll_opens_the_builder_in_channels_but_not_threads,8);
named!(builtin_remind_posts_and_saves_with_a_reminder,9);
named!(builtin_remind_rejects_unusable_input_without_posting,10);
named!(builtin_status_sets_emoji_and_text_until_end_of_day,11);
named!(builtin_status_rejects_blank_arguments,12);
named!(builtin_dnd_toggles_takes_durations_and_turns_off,13);
named!(builtin_dnd_rejects_garbage_durations,14);
named!(builtin_ooo_sets_an_end_with_a_note_and_off_clears_it,15);
named!(builtin_ooo_takes_week_durations_dates_and_datetimes,16);
named!(builtin_ooo_bare_tomorrow_and_weekdays_run_to_the_end_of_the_day,17);
named!(builtin_ooo_bare_dates_run_to_the_end_of_the_day,18);
named!(builtin_ooo_bare_month_dates_roll_to_next_year_when_this_year_s_passed,19);
named!(builtin_ooo_day_durations_stay_exact,20);
named!(builtin_ooo_broadcasts_the_badge_and_the_notice,21);
named!(builtin_ooo_off_while_calendar_ooo_covers_says_the_calendar_still_shows_it,22);
named!(builtin_ooo_rejects_blank_arguments_garbage_past_times_and_long_notes,23);
named!(builtin_shrug_posts_with_the_shrug,24);
named!(builtin_posting_commands_in_a_board_answer_an_error_without_posting,25);
named!(builtin_posting_commands_in_a_board_thread_still_post,26);
named!(builtin_slash_posts_in_threads_skip_the_legacy_webhook_fanout,27);
named!(builtin_slash_posts_in_channels_fan_out_to_legacy_webhooks,28);
named!(builtin_slash_posts_in_threads_process_attachments_once,29);
named!(builtin_me_posts_an_action_line,30);
named!(builtin_me_requires_an_action,31);
named!(builtin_play_posts_through_the_normal_message_path,32);
named!(builtin_slash_posts_never_start_a_stream,33);
named!(builtin_unknown_commands_error_with_the_available_list,34);

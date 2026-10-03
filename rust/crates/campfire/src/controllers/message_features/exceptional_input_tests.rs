//! Exact exceptional grammar, DST and structured request responses from pinned Rails.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::test_support::*;
use campfire_db::slash_commands::time_parser;
use serde_json::{Value,json};
fn oracle()->Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/exceptional_inputs.json")).unwrap()}
fn stamp(t:campfire_db::Timestamp)->String {
 let s=t.to_db();if s.contains('.') {s} else {format!("{s}.000000")}
}
#[test]
fn exceptional_calendar_grammar_and_dst_match_rails_exact_results_and_messages() {
 let mut failures=vec![];
 for row in oracle()["cases"].as_array().unwrap() {
  let zone=time_parser::zone(row["zone"].as_str().unwrap());
  let result=time_parser::parse_calendar(row["input"].as_str().unwrap(),&zone,campfire_db::Timestamp::parse_db(SEED_NOW).unwrap());
  let actual=match result {
   Ok(t)=>json!({"result":t.map(stamp)}),
   Err(e)=>{let message=e.to_string();let(class,message)=message.split_once(": ").filter(|(c,_)| *c=="RangeError" || *c=="ArgumentError").unwrap_or(("ArgumentError",&message));json!({"error":class,"message":message})},
  };
  if actual!=row["calendar"] { failures.push(json!({"input":row["input"],"zone":row["zone"],"rails":row["calendar"],"rust":actual})); }
 }
 for f in failures.iter().take(20) {eprintln!("{f}");}
 println!("WS8bm2 exceptional calendar: {} matched; {} differed",315-failures.len(),failures.len());
 assert!(failures.is_empty(),"calendar differential");
}
#[test]
fn exceptional_slash_fallback_and_dst_match_rails() {
 let mut failures=vec![];
 for row in oracle()["cases"].as_array().unwrap() {
  let actual=json!({"result":time_parser::parse(row["input"].as_str().unwrap(),row["zone"].as_str().unwrap(),campfire_db::Timestamp::parse_db(SEED_NOW).unwrap()).map(stamp)});
  if actual!=row["slash"] {failures.push(json!({"input":row["input"],"zone":row["zone"],"rails":row["slash"],"rust":actual}));}
 }
 for f in failures.iter().take(20) {eprintln!("{f}");}
 println!("WS8bm2 exceptional slash: {} matched; {} differed",315-failures.len(),failures.len());
 assert!(failures.is_empty(),"slash differential");
}
#[tokio::test]
async fn exceptional_structured_http_parameters_match_rails_status_type_and_bytes() {
 let data=oracle();let mut failures=vec![];
 for step in data["steps"].as_array().unwrap() {
  let app=app_rows(data["rows"].clone()).await;
  let method:hyper::Method=step["method"].as_str().unwrap().parse().unwrap();
  let mut req=Req::new(method.clone(),step["path"].as_str().unwrap());
  if method!=hyper::Method::GET {req=req.header("accept","application/json").header("content-type","application/json").body(serde_json::to_vec(&step["params"]).unwrap());}
  if let Some(accept)=step["accept"].as_str() {req=req.header("accept",accept);}
  let mut browser=app.david();
  let response=if method==hyper::Method::GET {browser.send(req).await} else {browser.write(req).await};
  let mut body=response.text().to_owned();
  if let Some(marker)=step["marker"].as_str() { if let Some(start)=body.find(marker) {
   let mut depth=0;let mut finish=None;
   for tag in regex::Regex::new(r"<section\b[^>]*>|</section>").unwrap().find_iter(&body[start..]) {
    depth+=if tag.as_str().starts_with("</") {-1} else {1};
    if depth==0 {finish=Some(start+tag.end());break;}
   }
   body=body[start..finish.expect("balanced owned section")].into();
  } }
  let actual=json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"body":body});
  let expected=json!({"status":step["status"],"content_type":step["content_type"],"body":step["body"]});
  if actual!=expected {failures.push(json!({"step":step,"rust":actual}));}
 }
 for f in &failures {eprintln!("{f}");}
 println!("WS8bm2 exceptional structured HTTP: {} matched; {} differed",data["steps"].as_array().unwrap().len()-failures.len(),failures.len());
 assert!(failures.is_empty(),"structured params differential");
}

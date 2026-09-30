use askama::Template;
use campfire_views::{helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with}, rooms};
#[path = "support/context.rs"] mod common;
struct Tokens;
impl AuthenticityTokens for Tokens {
 fn global(&self)->String {"GLOBAL".into()}
 fn for_form(&self,action:&str,method:&str)->String {format!("{method}:{action}")}
}
#[test]
fn complete_direct_settings_and_picker_match_rails_bytes() {
 let oracle:serde_json::Value=serde_json::from_str(include_str!("../../../vectors/direct_forms.json")).unwrap();
 let asset=|name:&str|campfire_assets::asset_path(name);let signer=|_:&[&str]|String::new();
 let ctx=common::context(&asset,&signer);let mut failures=0;
 for row in oracle["forms"].as_array().unwrap() {
 let actual=rendering_with(RequestSecrets{tokens:Box::new(Tokens),csp_nonce:None},||{
 if row["kind"]=="edit" {
 let edit:rooms::DirectEditView=serde_json::from_value(row["edit"].clone()).unwrap();
 rooms::DirectsEdit{ctx:&ctx,edit:&edit}.as_content().render().unwrap()
 } else {let users:Vec<rooms::DirectPickerUser>=serde_json::from_value(row["users"].clone()).unwrap();rooms::DirectsNew{ctx:&ctx,users:&users}.as_content().render().unwrap()}
 });
 let expected=row["html"].as_str().unwrap();
 if actual!=expected {failures+=1;if let Ok(dir)=std::env::var("WS8BR_DIFF_DIR") {
 std::fs::create_dir_all(&dir).unwrap();let name=row["name"].as_str().unwrap();
 std::fs::write(format!("{dir}/{name}.actual"),actual).unwrap();
 std::fs::write(format!("{dir}/{name}.expected"),expected).unwrap();
 }}
 }
 assert_eq!(failures,0,"complete direct form body mismatches");
}

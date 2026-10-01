use askama::Template;
use campfire_views::{rooms,layouts::Page};
use campfire_views::helpers::request_forgery::{AuthenticityTokens,RequestSecrets,rendering_with};
#[path="support/context.rs"] mod common;
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self)->String {"GLOBAL".into()}
    fn for_form(&self,action:&str,method:&str)->String {format!("{method}:{action}")}
}
#[test]
fn join_page_regions_match_rails_without_exposing_messages() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../vectors/rooms_join.json")).unwrap();
    let row=&fixture["golden"];
    let asset=|name:&str|campfire_assets::asset_path(name);
    let signer=|_:&[&str]|String::new();
    let mut ctx=common::context(&asset,&signer);
    ctx.last_room_visited_id=Some(104393281);
    let page=rooms::JoinPage{ctx:&ctx,id:row["id"].as_i64().unwrap(),name:row["name"].as_str().unwrap()};
    let (body,nav)=rendering_with(RequestSecrets{tokens:Box::new(Tokens),csp_nonce:None},||(page.as_content().render().unwrap(),page.as_nav().render().unwrap()));
    assert_eq!(body,row["parts"]["body"].as_str().unwrap());
    assert_eq!(nav,row["parts"]["nav"].as_str().unwrap());
    assert_eq!(page.page_title(),Some("Join #Join <&>".into()));
    assert!(page.has_sidebar());
}

use crate::controllers::presenters::test_support::*;
use axum::http::Method;
async fn run(name: &str) {
    let app=TestApp::boot_frozen().await.expect("CI seed required");
    let oracle:serde_json::Value=serde_json::from_str(include_str!("../../../../../../vectors/avatar_originals.json")).unwrap();
    let case=oracle["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();
    let reply=app.david().send(Req::new(Method::GET,case["path"].as_str().unwrap()).header("accept","image/svg+xml")).await;
    assert_eq!(reply.status.as_u16(),case["status"].as_u64().unwrap() as u16,"original {name} avatar status");
    if name=="initials" {assert_eq!(reply.text(),case["body"].as_str().unwrap(),"original complete Kevin initials SVG");}
}
#[tokio::test]async fn original_kevin_initials_text(){run("initials").await;}
#[tokio::test]async fn original_invalid_avatar_token_is_not_found(){run("invalid").await;}

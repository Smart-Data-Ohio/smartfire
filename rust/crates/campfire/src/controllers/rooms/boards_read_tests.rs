//! Actual presenter and HTTP responses compared against Rails, including shared chrome.
use askama::Template;
use crate::controllers::presenters::{boards, page, test_support::*};

#[tokio::test]
async fn board_rows_match_both_rails_renderings() {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle:serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/boards_read.json")).unwrap();
    for row in oracle["fragments"].as_array().unwrap() {
        let (id,column)=(row["thread_id"].as_i64().unwrap(),row["column"].as_bool().unwrap());
        let state=app.booted.app.clone();
        let actual=app.db().read(move |conn| {
            let thread=campfire_db::ChannelThread::find(conn,id)?;
            let presenter=crate::controllers::presenters::Presenter::new(conn,&state,None);
            let facts=boards::rows(&presenter,thread.room_id,&[thread])?;
            page::render_detached_at(&state,None,"http://campfire.test",|ctx| campfire_views::rooms::boards::RowPartial {ctx,row:&facts[0],column}.render()).map_err(|error| campfire_db::Error::Other(error.to_string()))
        }).await.unwrap();
        assert_eq!(actual,row["html"].as_str().unwrap(),"thread {id} column {column}");
    }
}

#[tokio::test]
async fn board_list_and_columns_match_complete_rails_http_responses() {
    let env=std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../parity/.env.reference")).unwrap();
    let vapid=env.lines().filter_map(|line|line.split_once('=')).filter(|(key,_)|matches!(*key,"VAPID_PUBLIC_KEY"|"VAPID_PRIVATE_KEY")).collect::<Vec<_>>();
    let oracle:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/boards_read.json")).unwrap();
    let app=TestApp::boot_frozen_with_env(&vapid).await.expect("default seed required");
    for row in oracle["rows"].as_array().unwrap() {
        let mut browser=app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let response=with_fixed_render_secrets(browser.send(Req::new(axum::http::Method::GET,row["path"].as_str().unwrap()).header("user-agent","Mozilla"))).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16);
        let actual=response.text();let expected=row["html"].as_str().unwrap();
        if actual != expected {
            rails_mismatch(&actual,expected,row["name"].as_str().unwrap());
        }
    }
}

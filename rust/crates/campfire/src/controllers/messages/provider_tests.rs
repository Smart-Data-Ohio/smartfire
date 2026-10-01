//! Complete response/row checks for the provider edit declarations.
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Tx};
use serde_json::{Value,json};
use crate::controllers::presenters::test_support::*;
pub(crate) fn oracle() -> Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/provider-declarations.json")).unwrap()}
pub(crate) fn seed(tx:&mut Tx<'_>)->campfire_db::Result<()> {
    let root=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Original".into()),client_message_id:Some("provider-root".into()),..Default::default()})?;
    let mut thread=ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("Provider thread".into()),..Default::default()})?;
    let reply=thread.post_message(tx,DAVID,NewMessage{markdown_source:Some("Original".into()),client_message_id:Some("provider-reply".into()),..Default::default()})?;
    let pr=crate::integrations::github::pull_requests::PullRequest::for_reference(tx,"rails","rails",511)?;
    tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Seeded PR card',state='open',fetched_at=?,fetch_requested_at=NULL,updated_at=? WHERE id=?",(tx.now(),tx.now(),pr.id))?;
    for (name,id) in [("root_id",root.id),("thread_id",thread.id),("reply_id",reply.id)]{assert_eq!(oracle()[name],id);}
    Ok(())
}
pub(crate) async fn assert_references(app:&TestApp,row:&Value) {
    let id=row["path"].as_str().unwrap().rsplit('/').next().unwrap().parse::<i64>().unwrap();
    let refs=app.db().read(move|conn| {
        let github=crate::integrations::github::pull_requests::PullRequest::for_message(conn,id)?.into_iter().map(|pr|json!([pr.owner,pr.repo,pr.number])).collect::<Vec<_>>();
        let twitter=crate::integrations::twitter::post::Post::for_message(conn,id)?.into_iter().map(|post|post.post_id).collect::<Vec<_>>();
        Ok(json!({"github":github,"twitter":twitter}))
    }).await.unwrap();
    assert_eq!(refs["github"],row["github"]);assert_eq!(refs["twitter"],row["twitter"]);
}

// WS14e's merged callback must also run through the shared legacy edit path.
#[tokio::test]
async fn legacy_rich_text_edit_synchronizes_event_reference_through_ws14e() {
    use axum::http::Method;
    use campfire_kit::clock::FrozenClock;
    let row:Value=serde_json::from_str(include_str!("../../../../../vectors/messaging/event-reference-declaration.json")).unwrap();
    let mut app=TestApp::boot_with_test_clock(std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let id=app.db().write(|tx|Message::create(tx,NewMessage{room_id:654632876,creator_id:DAVID,body:Some("<div>no links here</div>".into()),client_message_id:Some("legacy-resync-event".into()),..Default::default()}).map(|m|m.id)).await.unwrap();
    assert_eq!(row["message_id"],id);
    let response=app.david().write(Req::new(Method::PATCH,row["path"].as_str().unwrap()).header("content-type","application/json").body(json!({"message":row["input"]}).to_string())).await;
    assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16);
    assert_eq!(response.location(),row["location"].as_str());
    let events=app.db().read(move|conn| {
        let mut stmt=conn.prepare("SELECT event_id FROM event_references WHERE message_id=? ORDER BY id")?;
        let ids=stmt.query_map([id],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        Ok(ids)
    }).await.unwrap();
    assert_eq!(json!(events),row["events"]);
}

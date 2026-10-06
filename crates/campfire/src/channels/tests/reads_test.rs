//! The reads controller publishes only to the requesting user's other sessions.
use super::*;
use super::directory::connect_user;
use crate::controllers::presenters::test_support::{ALL_TALK,JASON};
use campfire_db::CachedStatements;

#[tokio::test]
async fn read_http_frames_match_rails_and_failures_publish_nothing() {
    let hub=boot().await.expect("seed required");
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/reads_refresh.json")).unwrap();
    let reads=identifier(json!({"channel":"ReadRoomsChannel"}));
    let unreads=identifier(json!({"channel":"UnreadRoomsChannel"}));
    let (mut first,mut second,mut other)=(hub.david().await,hub.david().await,connect_user(&hub,JASON).await);
    for client in [&mut first,&mut second,&mut other] {client.confirm(&reads).await;client.confirm(&unreads).await;}
    let mut browser=hub.app.david();
    let path=format!("/rooms/{ALL_TALK}/read");
    for (key,method,index,channel) in [("create",Method::POST,None,&reads),("second",Method::DELETE,Some(1),&unreads),("first",Method::DELETE,Some(0),&unreads)] {
        let req=Req::new(method,&path);
        let req=if let Some(i)=index {req.form(&[("message_id",&fixture["root_ids"][i].to_string())])} else {req};
        let reply=browser.write(req).await;
        assert_eq!(reply.status,200,"{}",reply.text());
        let frame=&fixture["cases"][key]["frames"][0];
        assert_eq!(frame["stream"].as_str().unwrap(),format!("user_{DAVID}_{}",if key=="create" {"reads"} else {"unreads"}));
        let expected=delivery(channel,&serde_json::to_string(&frame["message"]).unwrap());
        assert_eq!(first.next_text().await,expected);
        assert_eq!(second.next_text().await,expected);
        other.assert_silent().await;
    }
    assert_eq!(browser.write(Req::new(Method::DELETE,&path).form(&[("message_id","9999999999")])).await.status,404);
    hub.app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_http_read BEFORE UPDATE ON memberships BEGIN SELECT RAISE(ABORT,'injected read failure'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(browser.write(Req::new(Method::POST,&path)).await.status,500);
    // Revoke the requester's room access without invoking unrelated removal callbacks.
    hub.app.db().write(|tx| {tx.conn().execute_cached("DELETE FROM memberships WHERE room_id=? AND user_id=?",(ALL_TALK,DAVID))?;Ok(())}).await.unwrap();
    assert_eq!(browser.write(Req::new(Method::POST,&path)).await.status,404);
    first.assert_silent().await;
    second.assert_silent().await;
    other.assert_silent().await;
}

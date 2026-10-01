use super::*;
use super::directory::connect_user;
use campfire_db::{CachedStatements,Membership};
use crate::controllers::presenters::test_support::{HQ,JASON};
#[tokio::test]
async fn join_sends_one_rails_row_only_to_the_joiner() {
    let hub=boot().await.expect("seed required");
    hub.app.db().write(|tx|{tx.conn().execute_cached("DELETE FROM memberships WHERE room_id=? AND user_id=?",(HQ,DAVID))?;Ok(())}).await.unwrap();
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/rooms_join.json")).unwrap();
    let own=hub.turbo(&[&user_gid(DAVID).to_param(),"rooms"]);
    let mut joiner=hub.david().await;
    joiner.confirm(&own).await;
    let mut other=connect_user(&hub,JASON).await;
    other.confirm(&hub.turbo(&[&user_gid(JASON).to_param(),"rooms"])).await;
    let path=format!("/rooms/{HQ}/join");
    let mut browser=hub.app.david();
    assert_eq!(browser.write(Req::new(Method::POST,&path)).await.status,302);
    let html=fixture["cases"]["join"]["frames"][0]["html"].as_str().unwrap();
    assert!(campfire_cable::turbo::session_bound(html).is_none());
    let actual:serde_json::Value=serde_json::from_str(&joiner.next_text().await).unwrap();
    assert_eq!(actual,json!({"identifier":own,"message":html}));
    assert_eq!(browser.write(Req::new(Method::POST,&path)).await.status,302);
    hub.app.db().read(|conn|{assert_eq!(Membership::for_room(conn,HQ)?.iter().filter(|m|m.user_id==DAVID).count(),1);Ok(())}).await.unwrap();
    joiner.assert_silent().await;
    other.assert_silent().await;
}

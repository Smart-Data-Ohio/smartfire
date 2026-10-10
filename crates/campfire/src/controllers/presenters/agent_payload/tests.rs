use super::super::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use super::*;
use campfire_db::Message;
use campfire_db::User;
use campfire_runtime::presenters::{Presenter, Result};
use serde_json::Value;
use std::sync::Arc;
use campfire_db::NewMessage;
struct Adapter;
impl MessagePayload for Adapter {
    fn message(
        &self,
        p: &Presenter<'_>,
        m: &Message,
        viewer: &User,
        base: &str,
    ) -> Result<Value> {
        assert!(p.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?)",
            [m.id],
            |r| r.get::<_, bool>(0)
        )?);
        Ok(serde_json::json!({"message":m.id,"viewer":viewer.id,"base":base}))
    }
}
#[tokio::test]
async fn ws11_production_presenter_requires_current_user_and_forwards_uncached_request_context()
{
    let test = TestApp::boot().await.expect("default seed");
    let app = test.booted.app.clone();
    let mid = app
        .db
        .write(|tx| {
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Payload".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let copy = app.clone();
    app.db
        .read(move |conn| {
            let mut p = Presenter::new(conn, &copy, None);
            let m = Message::find(conn, mid)?;
            assert!(
                p.agent_message_payload(&m)
                    .unwrap_err()
                    .to_string()
                    .contains("Current.user")
            );
            p.current_user_id = Some(BENDER);
            p.cache_base_url = Some("https://first.test".into());
            let uninstalled = State::default();
            assert!(
                uninstalled
                    .message(&p, &m)
                    .unwrap_err()
                    .to_string()
                    .contains("not installed")
            );
            copy.agent_message_payload.install(Arc::new(Adapter));
            assert_eq!(
                p.agent_message_payload(&m)?,
                serde_json::json!({"message":mid,"viewer":BENDER,"base":"https://first.test"})
            );
            p.current_user_id = Some(DAVID);
            p.cache_base_url = Some("https://second.test".into());
            assert_eq!(
                p.agent_message_payload(&m)?,
                serde_json::json!({"message":mid,"viewer":DAVID,"base":"https://second.test"})
            );
            Ok(())
        })
        .await
        .unwrap();
}

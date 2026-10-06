use super::super::test_support::{ALL_TALK, DAVID, TestApp};
use campfire_db::callbacks::Phase;
use campfire_db::{Message, NewMessage};
#[tokio::test]
async fn ws11_production_jobs_dispatches_peer_hooks_and_rejects_the_originating_write() {
    let test = TestApp::boot().await.expect("default seed");
    let app = &test.booted.app;
    app.jobs
        .model_callbacks
        .install(Phase::MessageActivity, |tx, c| {
            assert!(Message::find_by_id(tx.conn(), c.record_id)?.is_some());
            Err(campfire_db::Error::Other(
                "WS12 adapter rejected activity".into(),
            ))
        });
    assert!(
        app.db
            .write(|tx| Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    client_message_id: Some("ws11-production-peer-rejected".into()),
                    markdown_source: Some("Peer callback".into()),
                    ..Default::default()
                }
            ))
            .await
            .is_err()
    );
    app.db.read(|c|{assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE client_message_id='ws11-production-peer-rejected'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
}

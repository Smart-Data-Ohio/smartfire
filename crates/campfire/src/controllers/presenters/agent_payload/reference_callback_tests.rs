use super::super::test_support::{ALL_TALK, BENDER, TestApp};
use campfire_db::{Message, MessageChanges, NewMessage};
fn expected(phase: &str) -> i64 {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/agents_stream_references_contract.json"
    ))
    .unwrap();
    oracle[phase].as_i64().unwrap()
}
#[tokio::test]
async fn ws11_stream_case_link_references_sync_only_at_finalize() {
    let (app, _dir) = TestApp::boot()
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let mid = app
        .db
        .write(|tx| {
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    streaming: true,
                    markdown_source: Some("See https://example.test/some/page".into()),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",
                    [m.id],
                    |r| r.get::<_, i64>(0)
                )?,
                expected("start")
            );
            Ok(m.id)
        })
        .await
        .unwrap();
    app.db.write(move|tx| {
        let mut m=Message::find(tx.conn(),mid)?;
        m.update(tx,MessageChanges{markdown_source:Some("See https://example.test/some/page and more".into()),..Default::default()})?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("append"));
        assert!(m.finalize_stream(tx)?);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("finalize"));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,1);
        assert!(!m.finalize_stream(tx)?);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("repeat"));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,1);Ok(())
    }).await.unwrap();
    app.db
        .write(|tx| {
            let mut quiet = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    streaming: true,
                    markdown_source: Some("https://example.test/quiet-page".into()),
                    ..Default::default()
                },
            )?;
            assert!(quiet.finalize_stream_quietly(tx)?);
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",
                    [quiet.id],
                    |r| r.get::<_, i64>(0)
                )?,
                expected("quiet")
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_stream_reference_fetch_failure_keeps_claim_and_reference_atomic() {
    let (app, _dir) = TestApp::boot()
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let mid=app.db.write(|tx| {
        let m=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:BENDER,streaming:true,markdown_source:Some("https://example.test/rejected-fetch".into()),..Default::default()})?;
        tx.conn().execute_batch("CREATE TRIGGER ws11_reject_ref_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='LinkEmbed::FetchJob' BEGIN SELECT RAISE(ABORT,'reference fetch queue failure'); END")?;Ok(m.id)
    }).await.unwrap();
    assert!(
        app.db
            .write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
            .await
            .is_err()
    );
    app.db.read(move|c| {
        assert!(Message::find(c,mid)?.streaming);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())
    }).await.unwrap();
}

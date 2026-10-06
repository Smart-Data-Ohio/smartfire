use super::super::test_support::{ALL_TALK, BENDER, TestApp};
use campfire_db::{Agent, Message, NewMessage};
#[tokio::test]
async fn ws11_finalization_durable_job_failure_rolls_back_claim_and_earlier_effects() {
    let test = TestApp::boot().await.expect("default seed");
    let app = &test.booted.app;
    let mid=app.db.write(|tx|{
        let mut a=Agent::for_user(tx.conn(),BENDER)?.unwrap();a.set_working_presence(tx,Some("Keep working"))?;
        let m=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:BENDER,markdown_source:Some("Draft".into()),streaming:true,..Default::default()})?;
        tx.conn().execute_batch("CREATE TRIGGER ws11_reject_finalize BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::PushMessageJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected finalize queue'); END;")?;Ok(m.id)
    }).await.unwrap();
    assert!(
        app.db
            .write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
            .await
            .is_err()
    );
    app.db
        .read(move |c| {
            assert!(Message::find(c, mid)?.streaming);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM message_search_index WHERE rowid=?",
                    [mid],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                Agent::for_user(c, BENDER)?
                    .unwrap()
                    .working_presence
                    .as_deref(),
                Some("Keep working")
            );
            Ok(())
        })
        .await
        .unwrap();
}

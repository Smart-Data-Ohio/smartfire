use super::*;

#[tokio::test]
async fn origin_is_present_for_commit_callbacks_and_restored_after_errors_and_panics() {
    let app = crate::controllers::presenters::test_support::TestApp::boot()
        .await
        .expect("WS8bm2 requires default seed");
    app.db()
        .write_scoped(
            || slash_origin("http://one.test"),
            |tx| {
                tx.after_commit(|_| {
                    assert!(SLASH.with(|flag| flag.get()));
                    assert_eq!(
                        ORIGIN.with(|origin| origin.borrow().clone()).as_deref(),
                        Some("http://one.test")
                    );
                    Ok(())
                });
                Ok(())
            },
        )
        .await
        .unwrap();
    let clean = app
        .db()
        .write(|_| {
            Ok(ORIGIN.with(|origin| origin.borrow().is_none())
                && !SLASH.with(|flag| flag.get()))
        })
        .await
        .unwrap();
    assert!(clean);
    let failed: campfire_db::Result<()> = app
        .db()
        .write_scoped(
            || slash_origin("http://two.test"),
            |_| Err(campfire_db::Error::Other("refused".into())),
        )
        .await;
    assert!(failed.is_err());
    assert!(
        app.db()
            .write(|_| Ok(ORIGIN.with(|origin| origin.borrow().is_none())
                && !SLASH.with(|flag| flag.get())))
            .await
            .unwrap()
    );
    let panicked: campfire_db::Result<()> = app
        .db()
        .write_scoped(
            || slash_origin("http://three.test"),
            |_| panic!("deliberate writer panic"),
        )
        .await;
    assert!(panicked.is_err());
    assert!(
        app.db()
            .write(|_| Ok(ORIGIN.with(|origin| origin.borrow().is_none())
                && !SLASH.with(|flag| flag.get())))
            .await
            .unwrap()
    );
}

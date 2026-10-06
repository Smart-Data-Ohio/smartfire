// Generated test host only: the explicit perform_enqueued_jobs(only: ...) in
// sending_messages_test.rb. Ordinary queues remain stopped. No release route.
fn ws8bm_attachment_job_router(app: &TestApp) -> axum::Router {
    let app = app.booted.app.clone();
    axum::Router::new().route("/__ws8bm__/attachment-processing", axum::routing::post(move || {
        let app = app.clone();
        async move {
            let rows = app.db.read(|conn| {
                conn.prepare("SELECT id,arguments,created_at,run_at FROM background_jobs WHERE job_class='Message::AttachmentProcessingJob' AND status='ready' ORDER BY id")?
                    .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, campfire_db::Timestamp>(2)?, row.get::<_, campfire_db::Timestamp>(3)?)))?
                    .collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }).await.unwrap();
            let count = rows.len();
            assert!(count > 0, "original perform_enqueued_jobs must encounter the upload's real job");
            for (id, arguments, enqueued_at, scheduled_at) in rows {
                let job = serde_json::from_str(&arguments).unwrap();
                let outcome = crate::jobs::attachment_processing::perform(app.clone(), job,
                    campfire_jobs::Execution { id, executions: 1, enqueued_at, scheduled_at }).await.unwrap();
                assert_eq!(outcome, campfire_jobs::Outcome::Done);
                app.db.write(move |tx| {
                    tx.conn().execute("DELETE FROM background_jobs WHERE id=? AND job_class='Message::AttachmentProcessingJob'", [id])?;
                    Ok(())
                }).await.unwrap();
            }
            axum::Json(serde_json::json!({"performed": count, "only": "Message::AttachmentProcessingJob"}))
        }
    }))
}

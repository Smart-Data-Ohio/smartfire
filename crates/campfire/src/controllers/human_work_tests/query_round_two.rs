//! PR #193 round two: actual reader SQL for distinct associations and inbox sources.
use super::*;

async fn fixture(inbox: bool) -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let oracle: Value = if inbox {
        serde_json::from_str(include_str!("../../../../../vectors/inbox-http.json")).unwrap()
    } else {
        let oracle: Value =
            serde_json::from_str(include_str!("../../../../../vectors/human_work_http.json"))
                .unwrap();
        oracle["rows"][0].clone()
    };
    setup(&app, &oracle).await;
    app
}

async fn measure(app: &TestApp, path: &str, inbox: bool, json: bool) -> (Vec<String>, String) {
    let mut browser = app.david();
    browser.authenticity_token().await;
    let mut req = Req::new(Method::GET, path).header(
        "accept",
        if json {
            "application/json"
        } else {
            "text/html"
        },
    );
    if inbox {
        req = req.header("turbo-frame", "activity_test");
    }
    let log = app.db().capture_read_queries();
    let response = browser.send(req).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(response.status, 200, "{}", response.text());
    let queries = log.lock().unwrap().clone();
    (queries, response.text().to_owned())
}

#[tokio::test]
async fn inbox_batches_work_events_and_associations_in_json_and_html() {
    let mut counts = [Vec::new()];
    for size in [10_i64, 100] {
        let app = fixture(true).await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM activity_items")?;
            for i in 0..size {
                let event = 8_700_000_000+i;
                tx.conn().execute("INSERT INTO work_thread_events(id,actor_id,channel_thread_id,created_at,event_type,from_owner_id,from_owner_name,from_status,metadata,to_owner_id,to_owner_name,to_status,updated_at) SELECT ?,actor_id,channel_thread_id,created_at,event_type,from_owner_id,from_owner_name,from_status,metadata,to_owner_id,to_owner_name,to_status,updated_at FROM work_thread_events WHERE id=8300000001",[event])?;
                tx.conn().execute("INSERT INTO activity_items(id,user_id,source_id,source_type,event_type,created_at,updated_at) VALUES(?,127326141,?,'WorkThreadEvent','work_update','2026-03-02 16:00:00','2026-03-02 16:00:00')",(8_800_000_000+i,event))?;
            }
            Ok(())
        }).await.unwrap();
        for (index, json) in [true].into_iter().enumerate() {
            let (queries, body) = measure(&app, "/activity", true, json).await;
            if json {
                let value: Value = serde_json::from_str(&body).unwrap();
                assert_eq!(
                    value["activity_items"].as_array().unwrap().len(),
                    size as usize
                );
            } else {
                assert_eq!(
                    body.matches("class=\"activity-item__title\"").count(),
                    size as usize
                );
            }
            println!(
                "WS12 R2 inbox {} rows={size}: {} reader SQL",
                if json { "JSON" } else { "HTML" },
                queries.len()
            );
            counts[index].push(queries.len());
        }
    }
    assert!(
        counts.iter().all(|sizes| sizes[0] == sizes[1]),
        "work sources and their associations must be preloaded in both formats: {counts:?}"
    );
}

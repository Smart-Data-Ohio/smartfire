//! PR #193: count actual SQL across the real seeded HTTP path at two sizes.
use super::*;

fn rails_queries(surface: &str, size: i64) -> usize {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/work_query_counts.json"
    ))
    .unwrap();
    oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["surface"] == surface && row["size"] == size)
        .unwrap()["queries"]
        .as_u64()
        .unwrap() as usize
}

async fn query_fixture(size: i64) -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../../vectors/human_work_http.json")).unwrap();
    setup(&app, &oracle["rows"][0]).await;
    app.db().write(move |tx| {
        tx.conn().execute_batch("UPDATE channel_threads SET work_status=NULL; DELETE FROM work_thread_links; DELETE FROM work_thread_events;")?;
        for i in 0..size {
            tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(?,486777696,127326141,?,'planned',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",(1000+i,format!("Query work {i}")))?;
        }
        Ok(())
    }).await.unwrap();
    app
}

#[tokio::test]
async fn work_index_queries_grow_no_faster_than_rails_and_skip_event_options() {
    let mut counts = Vec::new();
    let mut option_counts = Vec::new();
    for size in [10, 100] {
        let app = query_fixture(size).await;
        let mut browser = app.david();
        browser.authenticity_token().await;
        let log = app.db().capture_read_queries();
        let response = browser.get("/work?state=all").await;
        app.db().stop_capturing_read_queries();
        assert_eq!(response.status, 200, "{}", response.text());
        assert_eq!(
            response
                .text()
                .matches("class=\"work-threads__item\"")
                .count(),
            size as usize
        );
        let queries = log.lock().unwrap();
        let options = queries
            .iter()
            .filter(|sql| sql.starts_with("SELECT id FROM events WHERE room_id="))
            .count();
        println!(
            "WS12 index rows={size}: {} reader SQL; {options} unused event-option queries",
            queries.len()
        );
        counts.push(queries.len());
        option_counts.push(options);
    }
    // Actual pinned Rails requests: 25 / 113 uncached SQL at 10 / 100 rows.
    // Check both absolute counts and the slope, after measuring both sizes.
    let (small, large) = (rails_queries("index", 10), rails_queries("index", 100));
    assert!(
        counts[0] <= small
            && counts[1] <= large
            && counts[1].saturating_sub(counts[0]) <= large - small,
        "index must stay within Rails' 25/113 query counts: {counts:?}"
    );
    assert_eq!(
        option_counts,
        [0, 0],
        "row context never uses event-picker options"
    );
}

#[tokio::test]
async fn ordinary_and_board_history_queries_are_constant_for_repeated_actors() {
    let mut counts = [Vec::new(), Vec::new()];
    for size in [10, 100] {
        let app = query_fixture(size).await;
        let mut browser = app.david();
        browser.authenticity_token().await;
        // Reproduce the review's preceding work-index reads before its pane request.
        assert_eq!(browser.get("/work?state=all").await.status, 200);
        assert_eq!(browser.get("/work.json?state=all").await.status, 200);
        app.db().write(move |tx| {
            tx.conn().execute_batch("UPDATE channel_threads SET work_status='planned' WHERE id IN (90,91)")?;
            for thread in [90, 91] {
                for _ in 0..size {
                    tx.conn().execute("INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,metadata,created_at,updated_at) VALUES(?,149087659,'work_update','planned','blocked','{}','2026-03-02 16:00:00','2026-03-02 16:00:00')",[thread])?;
                }
            }
            Ok(())
        }).await.unwrap();
        for (index, (surface, path)) in [
            ("ordinary", "/rooms/486777696/threads/91"),
            ("board", "/rooms/699448332/threads/90"),
        ]
        .into_iter()
        .enumerate()
        {
            let log = app.db().capture_read_queries();
            let response = browser.get(path).await;
            app.db().stop_capturing_read_queries();
            assert_eq!(response.status, 200, "{}", response.text());
            assert_eq!(
                response.text().matches("status Planned").count(),
                size as usize
            );
            let count = log.lock().unwrap().len();
            println!("WS12 {surface} history={size}: {count} reader SQL");
            counts[index].push(count);
        }
    }
    // Rails' ordinary pane stays at 24 queries. Both surfaces share history_records.
    for surface in ["ordinary-history", "board-history"] {
        assert_eq!(rails_queries(surface, 10), rails_queries(surface, 100));
    }
    assert_eq!(
        counts[0][0], counts[0][1],
        "ordinary history must preload repeated actors: {:?}",
        counts[0]
    );
    assert_eq!(
        counts[1][0], counts[1][1],
        "board history must preload repeated actors: {:?}",
        counts[1]
    );
}

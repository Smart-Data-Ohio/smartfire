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

async fn distinct_owners_and_rooms(bot: bool) {
    let mut counts = Vec::new();
    for size in [10_i64, 100] {
        let app = fixture(false).await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("UPDATE channel_threads SET work_status=NULL; DELETE FROM work_thread_links; DELETE FROM work_thread_events;")?;
            for i in 0..size {
                let (user, room, thread) = (2_000_000_000 + i, 2_100_000_000 + i, 1000 + i);
                tx.conn().execute("INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(?,127326141,?,'Rooms::Closed','2026-03-02 16:00:00','2026-03-02 16:00:00')", (room,format!("Review room {i}")))?;
                tx.conn().execute("INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(?,?,?,?, '2026-03-02 16:00:00','2026-03-02 16:00:00')", (user,format!("Review owner {i}"),format!("review-{i}@example.test"),if bot {2} else {0}))?;
                for member in [127326141, user] {
                    tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')",(room,member))?;
                }
                if bot {
                    let agent = 2_200_000_000 + i;
                    tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(?,?,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(agent,user))?;
                    tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(?,?,127326141,'post_messages','2026-03-02 16:00:00','2026-03-02 16:00:00')",(agent,room))?;
                }
                tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(?,?,127326141,?,'planned',?,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",(thread,room,format!("Query work {i}"),user))?;
            }
            tx.conn().execute_batch("INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(2100000999,149087659,'Hidden room','Rooms::Closed','2026-03-02 16:00:00','2026-03-02 16:00:00'); INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(9999,2100000999,149087659,'HIDDEN_SENTINEL','planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00');")?;
            Ok(())
        }).await.unwrap();
        let (queries, body) = measure(&app, "/work?state=all", false, false).await;
        assert_eq!(
            body.matches("class=\"work-threads__item\"").count(),
            size as usize
        );
        assert!(!body.contains("HIDDEN_SENTINEL"));
        assert_eq!(body.matches("Owner: Review owner").count(), size as usize);
        println!(
            "WS12 R2 index {} rows={size}: {} reader SQL",
            if bot { "bots" } else { "humans" },
            queries.len()
        );
        counts.push(queries.len());
    }
    let rails_growth = if bot { 513 - 63 } else { 213 - 33 };
    assert!(
        counts[1].saturating_sub(counts[0]) <= rails_growth,
        "distinct associations must grow no faster than Rails: {counts:?}"
    );
    assert_eq!(
        counts[0], counts[1],
        "page associations must be batched across rooms and owners"
    );
}

#[tokio::test]
async fn index_batches_distinct_human_owners_and_rooms() {
    distinct_owners_and_rooms(false).await;
}
#[tokio::test]
async fn index_batches_distinct_agent_owners_and_rooms() {
    distinct_owners_and_rooms(true).await;
}

#[tokio::test]
async fn pane_batches_upcoming_unlinked_events() {
    let mut counts = Vec::new();
    for size in [10_i64, 100] {
        let app = fixture(false).await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM work_thread_links; DELETE FROM work_thread_events; UPDATE events SET cancelled_at='2026-03-02 16:00:00' WHERE room_id=486777696;")?;
            for i in 0..size {
                tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,486777696,127326141,?,'2026-03-03 16:00:00','UTC','2026-03-02 16:00:00','2026-03-02 16:00:00')",(2_300_000_000+i,format!("Option {i}")))?;
            }
            Ok(())
        }).await.unwrap();
        let (queries, body) = measure(&app, "/rooms/486777696/threads/91", false, false).await;
        assert_eq!(body.matches("Option ").count(), size as usize);
        println!(
            "WS12 R2 pane event options rows={size}: {} reader SQL",
            queries.len()
        );
        counts.push(queries.len());
    }
    assert_eq!(
        counts[0], counts[1],
        "event-picker reads must be constant at 10/100 choices: {counts:?}"
    );
}

#[tokio::test]
async fn inbox_batches_work_events_and_associations_in_json_and_html() {
    let mut counts = [Vec::new(), Vec::new()];
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
        for (index, json) in [true, false].into_iter().enumerate() {
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

#[tokio::test]
async fn ordinary_pane_skips_unused_board_facts_and_duplicate_pr_association() {
    let mut fact_counts = Vec::new();
    let mut pr_counts = Vec::new();
    for size in [10_i64, 100] {
        let app = fixture(false).await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM work_thread_events; DELETE FROM work_thread_links; DELETE FROM github_pull_request_threads WHERE channel_thread_id=91; INSERT INTO github_pull_requests(id,owner,repo,number,private,state,created_at,updated_at) VALUES(2300000999,'review','work',1,1,'open','2026-03-02 16:00:00','2026-03-02 16:00:00'); INSERT INTO github_pull_request_threads(room_id,channel_thread_id,github_pull_request_id,created_at,updated_at) VALUES(486777696,91,2300000999,'2026-03-02 16:00:00','2026-03-02 16:00:00');")?;
            for _ in 0..size {
                tx.conn().execute_batch("INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,metadata,created_at,updated_at) VALUES(91,149087659,'work_update','planned','blocked','{}','2026-03-02 16:00:00','2026-03-02 16:00:00');")?;
            }
            Ok(())
        }).await.unwrap();
        let (queries, body) = measure(&app, "/rooms/486777696/threads/91", false, false).await;
        assert!(body.contains("github-pr-thread-header"));
        let unused = queries
            .iter()
            .filter(|sql| {
                sql.contains("GROUP BY thread_id")
                    || sql.contains("GROUP BY channel_thread_id")
                    || sql.contains("FROM thread_tags")
            })
            .count();
        let prs = queries
            .iter()
            .filter(|sql| {
                sql.contains("FROM github_pull_request_threads")
                    || sql.contains("FROM github_pull_requests")
            })
            .count();
        println!(
            "WS12 R2 pane fixed reads history={size}: {} reader SQL; {unused} unused board facts; {prs} PR association reads",
            queries.len()
        );
        fact_counts.push(unused);
        pr_counts.push(prs);
    }
    assert_eq!(
        fact_counts,
        [0, 0],
        "ordinary panes do not render board counts or tags"
    );
    assert_eq!(
        pr_counts,
        [2, 2],
        "load the PR and its thread association once"
    );
}

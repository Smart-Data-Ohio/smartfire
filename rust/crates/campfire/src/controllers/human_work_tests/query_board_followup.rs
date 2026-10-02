//! #191/#195 board probes: trace real SQLite statements at 10/200 associations.
use super::*;
use crate::controllers::presenters::{Presenter, board_posts};
use campfire_db::{ChannelThread, NewChannelThread};

async fn fixture() -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../../vectors/human_work_http.json")).unwrap();
    setup(&app, &oracle["rows"][0]).await;
    app
}

async fn members(app: &TestApp, size: i64, bots: bool, explicit: bool) {
    app.db().write(move |tx| {
        tx.conn().execute_batch("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696; DELETE FROM memberships WHERE room_id=486777696; DELETE FROM work_thread_links; DELETE FROM activity_items;")?;
        for i in 0..size {
            let user = if !bots && i==0 { DAVID } else if !bots && i==1 { JASON } else { 2_400_000_000+i };
            if user!=DAVID && user!=JASON {
                tx.conn().execute("INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(?,?,?,?, '2026-03-02 16:00:00','2026-03-02 16:00:00')",(user,format!("Board owner {i:03}"),format!("board-owner-{i}@example.test"),if bots {2} else {0}))?;
            }
            tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(user,if user==JASON {"everything"} else {"invisible"}))?;
            if bots {
                let agent=2_500_000_000+i;
                tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(?,?,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(agent,user))?;
                if explicit {
                    tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(?,486777696,127326141,'post_messages','2026-03-02 16:00:00','2026-03-02 16:00:00')",[agent])?;
                }
                tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(?,486777696,127326141,?,'planned',?,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",(2000+i,format!("Agent post {i}"),user))?;
            }
        }
        Ok(())
    }).await.unwrap();
}

async fn agent_availability(explicit: bool) {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        members(&app, size, true, explicit).await;
        let posts = app
            .db()
            .read(|conn| ChannelThread::for_ids(conn, &(2000..2200).collect::<Vec<_>>()))
            .await
            .unwrap();
        let log = app.db().capture_read_queries();
        let available = app
            .db()
            .read(move |conn| ChannelThread::board_owner_active_map(conn, ALL_TALK, posts.iter()))
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        assert_eq!(available.len(), size as usize);
        assert!(available.values().all(|value| *value));
        let count = log.lock().unwrap().len();
        println!(
            "WS12 board agent availability {} size={size}: {count} SELECTs",
            if explicit { "explicit" } else { "legacy" }
        );
        counts.push(count);
    }
    assert_eq!(
        counts,
        [4, 4],
        "agent availability must meet Rails' constant four reads"
    );
}

#[tokio::test]
async fn agent_availability_batches_legacy_permissions() {
    agent_availability(false).await;
}
#[tokio::test]
async fn agent_availability_batches_explicit_grants() {
    agent_availability(true).await;
}

#[tokio::test]
async fn owner_choices_bulk_load_human_members() {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        members(&app, size, false, false).await;
        let log = app.db().capture_read_queries();
        let (humans, agents) = app
            .db()
            .read(|conn| ChannelThread::work_owner_candidates_for(conn, ALL_TALK))
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        assert_eq!(humans.len(), size as usize);
        assert!(agents.is_empty());
        assert!(
            humans
                .windows(2)
                .all(|users| rails_compat::unicode::downcase(&users[0].name)
                    <= rails_compat::unicode::downcase(&users[1].name))
        );
        let count = log.lock().unwrap().len();
        println!("WS12 board owner choices size={size}: {count} SELECTs");
        counts.push(count);
    }
    assert_eq!(
        counts[0], counts[1],
        "human choices must bulk-load members: {counts:?}"
    );
    assert!(counts[1] <= 3, "Rails uses three reads");
}

#[tokio::test]
async fn owner_choices_batch_agent_policy() {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        members(&app, size, true, true).await;
        let log = app.db().capture_read_queries();
        let (humans, agents) = app
            .db()
            .read(|conn| ChannelThread::work_owner_candidates_for(conn, ALL_TALK))
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        assert!(humans.is_empty());
        assert_eq!(agents.len(), size as usize);
        let count = log.lock().unwrap().len();
        println!("WS12 board agent owner choices size={size}: {count} SELECTs");
        counts.push(count);
    }
    assert_eq!(
        counts,
        [4, 4],
        "agent owner choices must share the batched policy facts"
    );
}

thread_local! {
    static WRITER_READS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn record_writer_read(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
        && (sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH"))
    {
        WRITER_READS.with(|log| log.borrow_mut().push(sql.to_owned()));
    }
}

#[tokio::test]
async fn opening_message_bulk_loads_members_and_preserves_recipients() {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        members(&app, size, false, false).await;
        let (thread, queries) = app
            .db()
            .write(|tx| {
                WRITER_READS.with(|log| log.borrow_mut().clear());
                tx.conn().trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(record_writer_read),
                );
                let result = ChannelThread::create_board_post(
                    tx,
                    NewChannelThread {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        name: Some("Opening query probe".into()),
                        work_status: Some("planned".into()),
                        ..Default::default()
                    },
                    Some("Opening message".into()),
                );
                tx.conn()
                    .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                Ok((
                    result?,
                    WRITER_READS.with(|log| std::mem::take(&mut *log.borrow_mut())),
                ))
            })
            .await
            .unwrap();
        let recipients=app.db().read(move |conn| {
            Ok(conn.prepare("SELECT DISTINCT user_id FROM activity_items WHERE source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?) ORDER BY user_id")?.query_map([thread.id], |row|row.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await.unwrap();
        assert_eq!(
            recipients,
            [JASON],
            "invisible members and the creator must not receive an opening notification"
        );
        let users = queries
            .iter()
            .filter(|sql| sql.contains("FROM \"users\"") || sql.contains("FROM users"))
            .count();
        println!(
            "WS12 board opening notification size={size}: {} SELECTs; {users} user reads",
            queries.len()
        );
        counts.push(queries.len());
    }
    assert_eq!(
        counts[0], counts[1],
        "nonrecipients must not cause per-member user reads: {counts:?}"
    );
}

async fn event_links(kind: &str) {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        let kind = kind.to_owned();
        let source_kind = kind.clone();
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM work_thread_links; UPDATE events SET cancelled_at='2026-03-02 16:00:00' WHERE room_id=486777696;")?;
            for i in 0..size {
                let id=2_600_000_000+i;
                if source_kind=="pull_request" {
                    tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,private,state,title,created_at,updated_at) VALUES(?,'query','board',?,0,'open',?,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(id,i+1,format!("Linked PR {i}")))?;
                } else {
                    tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,486777696,127326141,?,'2026-03-03 16:00:00','UTC','2026-03-02 16:00:00','2026-03-02 16:00:00')",(id,format!("Query event {i}")))?;
                }
                if source_kind!="unlinked" {
                    tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,event_id,github_pull_request_id,created_at,updated_at) VALUES(91,127326141,?,?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(source_kind.as_str(),(source_kind=="event").then_some(id),(source_kind=="pull_request").then_some(id)))?;
                }
            }
            Ok(())
        }).await.unwrap();
        let state = app.booted.app.clone();
        let thread = app
            .db()
            .read(|conn| ChannelThread::find(conn, 91))
            .await
            .unwrap();
        let log = app.db().capture_read_queries();
        let links = app
            .db()
            .read(move |conn| board_posts::links(&Presenter::new(conn, &state, None), &thread))
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        if kind == "unlinked" {
            assert_eq!(links.events.len(), size as usize);
            assert!(links.items.is_empty());
        } else {
            assert_eq!(links.items.len(), size as usize);
            assert!(links.events.is_empty());
        }
        let count = log.lock().unwrap().len();
        println!("WS12 board {kind} event/link choices size={size}: {count} SELECTs");
        counts.push(count);
    }
    assert_eq!(
        counts[0], counts[1],
        "link associations must be preloaded: {counts:?}"
    );
    assert!(
        counts[1] <= 3,
        "one read each for links, associated rows and event choices"
    );
}
#[tokio::test]
async fn event_choices_fetch_upcoming_rows_together() {
    event_links("unlinked").await;
}
#[tokio::test]
async fn links_preload_existing_calendar_events() {
    event_links("event").await;
}
#[tokio::test]
async fn links_preload_existing_pull_requests() {
    event_links("pull_request").await;
}

#[tokio::test]
async fn work_index_preloads_link_sources_across_the_whole_page() {
    let mut counts = Vec::new();
    for size in [10_i64, 200] {
        let app = fixture().await;
        app.db().write(move |tx| {
            tx.conn().execute_batch("UPDATE channel_threads SET work_status=NULL; DELETE FROM work_thread_links;")?;
            for i in 0..size {
                let (thread,pr)=(3000+i,2_700_000_000+i);
                tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(?,486777696,127326141,?,'planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",(thread,format!("Linked work {i}")))?;
                tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,private,state,created_at,updated_at) VALUES(?,'page','board',?,0,'open','2026-03-02 16:00:00','2026-03-02 16:00:00')",(pr,i+1))?;
                tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,github_pull_request_id,created_at,updated_at) VALUES(?,127326141,'pull_request',?,'2026-03-02 16:00:00','2026-03-02 16:00:00')",(thread,pr))?;
            }
            Ok(())
        }).await.unwrap();
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
        assert_eq!(
            response
                .text()
                .matches("Remove link to pull request page/board#")
                .count(),
            size as usize
        );
        let count = log.lock().unwrap().len();
        println!("WS12 board linked work index size={size}: {count} SELECTs");
        counts.push(count);
    }
    assert_eq!(
        counts[0], counts[1],
        "preload associations across all rows, not once per thread: {counts:?}"
    );
}

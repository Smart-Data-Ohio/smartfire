use super::super::client::tests::fake;
use super::super::jobs::tests::{run, setup, start};
use super::super::{client::Client, jobs::perform_import, runner::Runner};
use super::*;
use crate::integrations::test_support::Route;
use std::time::Duration;

pub(crate) fn routes(personal: bool) -> Vec<Route> {
    let route = |path: &str, body: &str| {
        Route::new("GET", "slack.com", path, 200).body(body.as_bytes().to_vec())
    };
    let mut routes = vec![
        route(
            "/api/users.list?limit=200",
            include_str!("../fixtures/users.json"),
        ),
        if personal {
            route(
                "/api/conversations.list?types=im%2Cmpim%2Cprivate_channel&exclude_archived=false&limit=200",
                include_str!("../fixtures/conversations_personal.json"),
            )
        } else {
            route(
                "/api/conversations.list?types=public_channel%2Cprivate_channel&exclude_archived=false&limit=200",
                include_str!("../fixtures/conversations_workspace.json"),
            )
        },
    ];
    for (id, members, history) in [
        (
            "CARCH",
            include_str!("../fixtures/members_CARCH.json"),
            include_str!("../fixtures/history_CARCH.json"),
        ),
        (
            "CCHAN",
            include_str!("../fixtures/members_CCHAN.json"),
            include_str!("../fixtures/history_CCHAN_p1.json"),
        ),
        (
            "CPRIV",
            include_str!("../fixtures/members_CPRIV.json"),
            include_str!("../fixtures/history_CPRIV.json"),
        ),
        (
            "DIM",
            include_str!("../fixtures/members_DIM.json"),
            include_str!("../fixtures/history_DIM.json"),
        ),
        (
            "GMPIM",
            include_str!("../fixtures/members_GMPIM.json"),
            include_str!("../fixtures/history_GMPIM.json"),
        ),
    ] {
        routes.push(route(
            &format!("/api/conversations.members?channel={id}&limit=1000"),
            members,
        ));
        routes.push(route(
            &format!("/api/conversations.history?channel={id}&limit=200"),
            history,
        ));
    }
    routes.push(route(
        "/api/conversations.history?channel=CCHAN&cursor=cchan-page-2&limit=200",
        include_str!("../fixtures/history_CCHAN_p2.json"),
    ));
    routes.push(route(
        "/api/conversations.replies?channel=CCHAN&ts=1700000002.000002&limit=200",
        include_str!("../fixtures/replies_CCHAN_parent.json"),
    ));
    routes.push(route(
        "/api/conversations.replies?channel=GMPIM&ts=1700000050.000050&limit=200",
        include_str!("../fixtures/replies_GMPIM_parent.json"),
    ));
    routes
}
pub(crate) async fn import(
    db: &Database,
    crypto: std::sync::Arc<rails_compat::ar_encryption::ArEncryption>,
    id: i64,
    network: crate::integrations::net::Network,
) -> SlackImport {
    for _ in 0..100 {
        let store_db = db.clone();
        let network = network.clone();
        perform_import(
            db.clone(),
            crypto.clone(),
            id,
            move |run, lease, token| async move {
                Runner::new(
                    SqlStore {
                        db: store_db,
                        lease,
                        allowed_domains: ["example.com".into()].into(),
                    },
                    Client::with_network(token, None, false, network),
                    run,
                )
                .with_budget(Duration::ZERO)
                .step()
                .await
            },
        )
        .await
        .unwrap();
        let row = run(db, id).await;
        if row.status != "running" && row.status != "queued" {
            assert_eq!(row.status, "completed", "{}", row.error.unwrap_or_default());
            return row;
        }
    }
    panic!("fixture import did not finish");
}
#[tokio::test]
async fn slack_sql_store_full_workspace_import_persists_pages_and_finishes_all_rooms() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        Ok(())
    })
    .await
    .unwrap();
    let (server, network) = fake(routes(false)).await;
    let row = import(&db, crypto, id, network).await;
    assert_eq!(row.state["phase"], "done");
    assert!(row.state.get("step_lease_token").is_none());
    assert_eq!(row.stats["counts"]["rooms_created"], 3);
    assert!(
        array(&row.stats["conversations"])
            .iter()
            .all(|c| c["done"] == true)
    );
    assert_eq!(row.stats["api_calls"], server.received().len());
    db.read(|c|{
        let rooms=Room::all(c)?;assert_eq!(rooms.len(),3);
        for room in rooms{
            let last:Timestamp=c.query_row("SELECT MAX(created_at) FROM messages WHERE room_id=?",[room.id],|r|r.get(0))?;
            assert_eq!(room.updated_at,last);
            let unread:i64=c.query_row("SELECT count(*) FROM memberships WHERE room_id=? AND (unread_at IS NOT NULL OR last_read_message_id IS NULL)",[room.id],|r|r.get(0))?;assert_eq!(unread,0);
        }
        assert_eq!(c.query_row("SELECT count(*) FROM activity_items",[],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(c.query_row("SELECT count(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,1);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn slack_sql_store_dry_run_collects_samples_without_domain_rows_or_reply_fetches() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_imports SET mode='dry_run' WHERE id=?", [id])?;
        Ok(())
    })
    .await
    .unwrap();
    let (server, network) = fake(routes(false)).await;
    let row = import(&db, crypto, id, network).await;
    assert!(array(&row.stats["samples"]).len() <= 20);
    assert!(!array(&row.stats["samples"]).is_empty());
    assert!(
        !server
            .received()
            .iter()
            .any(|r| r.target.starts_with("/api/conversations.replies"))
    );
    db.read(|c| {
        for table in [
            "rooms",
            "messages",
            "slack_import_records",
            "channel_threads",
        ] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
        }
        assert_eq!(
            c.query_row("SELECT count(*) FROM users", [], |r| r.get::<_, i64>(0))?,
            1
        );
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_sql_store_rejects_cancelled_or_replaced_lease_before_domain_writes() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let lease = db
        .write(move |tx| {
            SlackImport::claim_running(tx, id)?;
            Ok(SlackImport::acquire_step_lease(
                tx,
                id,
                campfire_db::models::slack_import::StepStatus::Running,
            )?
            .unwrap())
        })
        .await
        .unwrap();
    let row = run(&db, id).await;
    let p = Progress::new(row);
    let wrong = SqlStore {
        db: db.clone(),
        lease: "wrong owner".into(),
        allowed_domains: HashSet::new(),
    };
    assert!(
        wrong
            .commit(
                p.clone(),
                Operation::Users(json!([{"id":"UWRITE","is_bot":true}]))
            )
            .await
            .unwrap()
            .is_none()
    );
    db.write(move |tx| SlackImport::cancel(tx, id))
        .await
        .unwrap();
    let store = SqlStore {
        db: db.clone(),
        lease,
        allowed_domains: HashSet::new(),
    };
    assert!(
        store
            .commit(p, Operation::Users(json!([{"id":"UWRITE","is_bot":true}])))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        db.read(|c| Ok(
            c.query_row("SELECT count(*) FROM slack_import_records", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        0
    );
}

async fn selected_run(db: &Database, options: Value) -> i64 {
    let id = start(db).await;
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        tx.conn().execute(
            "UPDATE slack_imports SET options=? WHERE id=?",
            params![options.to_string(), id],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    id
}
fn window_routes(oldest: Option<&str>, latest: Option<&str>) -> Vec<Route> {
    let mut result = routes(false);
    result.retain(|route| {
        !route
            .path
            .starts_with("/api/conversations.history?channel=CCHAN")
    });
    for (cursor, next, ts, text) in [
        (None, Some("page-2"), "1719792000.000001", "summer 2024"),
        (
            Some("page-2"),
            Some("page-3"),
            "1685577600.000002",
            "summer 2023",
        ),
        (Some("page-3"), None, "1640995200.000003", "new year 2022"),
    ] {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("channel", "CCHAN");
        if let Some(oldest) = oldest {
            query.append_pair("oldest", oldest);
        }
        if let Some(latest) = latest {
            query.append_pair("latest", latest);
        }
        if let Some(cursor) = cursor {
            query.append_pair("cursor", cursor);
        }
        query.append_pair("limit", "200");
        result.push(Route::new("GET","slack.com",&format!("/api/conversations.history?{}",query.finish()),200).body(json!({"ok":true,"messages":[{"type":"message","user":"U001","ts":ts,"text":text}],"has_more":next.is_some(),"response_metadata":{"next_cursor":next.unwrap_or("")}}).to_string()));
    }
    result
}
fn middle_window() -> Value {
    json!({"conversation_ids":["CCHAN"],"oldest":"2023-05-01T00:00:00Z","latest":"2023-07-01T00:00:00Z"})
}
#[tokio::test]
async fn slack_history_windows_full_catchup_finish_earlier_room_and_keep_forward_unread_pointers() {
    let (db, crypto, _dir) = setup().await;
    let bounded = selected_run(&db, middle_window()).await;
    let (_server, network) = fake(window_routes(
        Some("1682899200.000000"),
        Some("1688169600.000000"),
    ))
    .await;
    let first = import(&db, crypto.clone(), bounded, network).await;
    assert_eq!(first.stats["counts"]["messages"], 1);
    let (room, newer_member, unread_member, newer) = db
        .write(move |tx| {
            let room = users::mapped_id(tx.conn(), 1, "conversation", "CCHAN")?.unwrap();
            let mut members = Room::find(tx.conn(), room)?.memberships(tx.conn())?;
            members.sort_by_key(|m| m.id);
            let newer = campfire_db::Message::create_imported(
                tx,
                campfire_db::NewMessage {
                    room_id: room,
                    creator_id: 1,
                    markdown_source: Some("newer live chat".into()),
                    ..Default::default()
                },
                tx.now(),
                None,
            )?;
            tx.conn().execute(
                "UPDATE memberships SET last_read_message_id=? WHERE id=?",
                params![newer.id, members[0].id],
            )?;
            tx.conn().execute(
                "UPDATE memberships SET unread_at=? WHERE id=?",
                params![tx.now(), members[1].id],
            )?;
            Ok((room, members[0].id, members[1].id, newer.id))
        })
        .await
        .unwrap();
    let full = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
    let (server, network) = fake(window_routes(None, None)).await;
    let second = import(&db, crypto.clone(), full, network).await;
    assert_eq!(second.stats["counts"]["messages"], 2);
    assert_eq!(
        server
            .received()
            .iter()
            .filter(|request| request.target.starts_with("/api/conversations.history"))
            .count(),
        3
    );
    db.read(move |conn| {
        assert_eq!(
            Room::find(conn, room)?.updated_at,
            writer::slack_time("1719792000.000001")?
        );
        let newest = users::mapped_id(conn, 1, "message", "CCHAN:1719792000.000001")?.unwrap();
        for member in Room::find(conn, room)?.memberships(conn)? {
            if member.id == newer_member {
                assert_eq!(member.last_read_message_id, Some(newer));
            } else if member.id == unread_member {
                assert!(member.unread_at.is_some());
                assert_ne!(member.last_read_message_id, Some(newest));
            } else {
                assert_eq!(member.last_read_message_id, Some(newest));
                assert!(member.unread_at.is_none());
            }
        }
        Ok(())
    })
    .await
    .unwrap();
    let catchup = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
    let (_server, network) = fake(window_routes(Some("1717200000.000001"), None)).await;
    let third = import(&db, crypto, catchup, network).await;
    assert_eq!(third.stats["counts"]["messages"], 0);
}
#[tokio::test]
async fn slack_catchup_coverage_is_invalidated_by_undoing_an_earlier_bounded_run() {
    let (db, crypto, _dir) = setup().await;
    let bounded = selected_run(&db, middle_window()).await;
    let (_server, network) = fake(window_routes(
        Some("1682899200.000000"),
        Some("1688169600.000000"),
    ))
    .await;
    import(&db, crypto.clone(), bounded, network).await;
    let full = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
    let (_server, network) = fake(window_routes(None, None)).await;
    import(&db, crypto.clone(), full, network).await;
    db.write(move |tx| {
        tx.conn().execute("UPDATE slack_imports SET started_at=?,status='undoing',state='{\"phase\":\"undo\"}',finished_at=NULL WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_secs(120)),bounded])?;
        tx.conn().execute("UPDATE slack_imports SET finished_at=? WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_secs(60)),full])?;Ok(())
    }).await.unwrap();
    for _ in 0..50 {
        let work_db = db.clone();
        super::super::jobs::perform_undo(db.clone(), bounded, move |_, lease| async move {
            super::super::undoer::Undoer {
                db: work_db,
                id: bounded,
                lease,
            }
            .step()
            .await
        })
        .await
        .unwrap();
        if run(&db, bounded).await.status == "undone" {
            break;
        }
    }
    assert_eq!(run(&db, bounded).await.status, "undone");
    let again = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
    let (server, network) = fake(window_routes(None, None)).await;
    let row = import(&db, crypto, again, network).await;
    assert_eq!(row.stats["counts"]["messages"], 1);
    assert!(
        server
            .received()
            .iter()
            .filter(|request| request.target.starts_with("/api/conversations.history"))
            .all(|request| !request.target.contains("oldest="))
    );
}
#[tokio::test]
async fn slack_catchup_new_message_late_reply_and_deleted_mapped_thread_match_rails() {
    for deleted in [false, true] {
        let (db, crypto, _dir) = setup().await;
        let first = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
        let (_server, network) = fake(routes(false)).await;
        import(&db, crypto.clone(), first, network).await;
        let thread = db
            .read(|conn| users::mapped_id(conn, 1, "thread", "CCHAN:1700000002.000002"))
            .await
            .unwrap()
            .unwrap();
        if deleted {
            db.write(move |tx| {
                campfire_db::ChannelThread::find(tx.conn(), thread)?.destroy_imported(tx)
            })
            .await
            .unwrap();
        }
        let mut responses = routes(false);
        responses.retain(|route| {
            !route
                .path
                .starts_with("/api/conversations.history?channel=CCHAN")
                && !route
                    .path
                    .starts_with("/api/conversations.replies?channel=CCHAN")
        });
        let original: Value =
            serde_json::from_str(include_str!("../fixtures/history_CCHAN_p2.json")).unwrap();
        let parent = original["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["ts"] == "1700000002.000002")
            .unwrap();
        responses.push(Route::new("GET","slack.com","/api/conversations.history?channel=CCHAN&oldest=1697408102.000102&limit=200",200).body(json!({"ok":true,"messages":[{"type":"message","user":"U001","text":"Fresh news","ts":"1700000099.000099"},parent],"has_more":false}).to_string()));
        responses.push(Route::new("GET","slack.com","/api/conversations.replies?channel=CCHAN&ts=1700000002.000002&oldest=1697408102.000102&limit=200",200).body(json!({"ok":true,"messages":[{"type":"message","user":"U002","text":"parent","ts":"1700000002.000002"},{"type":"message","user":"U001","text":"Late reply","ts":"1700000200.000200","thread_ts":"1700000002.000002"}],"has_more":false}).to_string()));
        let second = selected_run(&db, json!({"conversation_ids":["CCHAN"]})).await;
        let (_server, network) = fake(responses).await;
        let row = import(&db, crypto, second, network).await;
        assert_eq!(row.stats["counts"]["messages"], 1);
        assert_eq!(row.stats["counts"]["replies"], if deleted { 0 } else { 1 });
        db.read(move |conn| {
            if deleted {assert!(campfire_db::ChannelThread::find_by_id(conn,thread)?.is_none());assert_eq!(conn.query_row("SELECT count(*) FROM slack_import_issues WHERE slack_import_id=? AND message LIKE '%was deleted%'",[second],|row|row.get::<_,i64>(0))?,1);}
            else {let thread=campfire_db::ChannelThread::find(conn,thread)?;assert_eq!(thread.messages_count,3);assert_eq!(thread.last_activity_at,writer::slack_time("1700000200.000200")?);}
            Ok(())
        }).await.unwrap();
    }
}
#[tokio::test]
async fn slack_finishing_per_conversation_seeks_identity_index_and_renews_heartbeat_lease() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    db.write(move |tx| {
        SlackImport::claim_running(tx,id)?;
        let lease=SlackImport::acquire_step_lease(tx,id,campfire_db::models::slack_import::StepStatus::Running)?.unwrap();
        tx.conn().execute("UPDATE slack_imports SET heartbeat_at=? WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_secs(600)),id])?;
        let mut p=Progress::new(SlackImport::find(tx.conn(),id)?.unwrap());p.state["written_conversation_ids"]=json!(["C1","C2"]);
        finish_rooms(tx,&p)?;
        let row=SlackImport::find(tx.conn(),id)?.unwrap();assert_eq!(row.heartbeat_at,Some(tx.now()));assert_eq!(row.state["step_lease_token"],lease);
        let mut explain=tx.conn().prepare("EXPLAIN QUERY PLAN SELECT record_id FROM slack_import_records WHERE slack_workspace_id=1 AND slack_kind='message' AND slack_key>='C1:' AND slack_key<'C1;'")?;
        let details=explain.query_map([],|row|row.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?.join(" ");assert!(details.contains("USING INDEX index_slack_import_records_on_slack_identity"),"{details}");assert!(details.contains("slack_key"));Ok(())
    }).await.unwrap();
}

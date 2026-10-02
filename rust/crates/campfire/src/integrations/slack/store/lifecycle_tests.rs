//! Direct ports of the remaining pinned Rails job/model regression declarations.
use super::super::client::{Client, tests::fake};
use super::super::jobs::{
    perform_import,
    tests::{run, setup, start},
};
use super::super::runner::Runner;
use super::tests::{import, routes};
use super::*;
use crate::integrations::test_support::Route;
use campfire_db::models::slack_import::StepStatus;
use campfire_jobs::inspect;
use std::time::Duration;

async fn configured(db: &Database, options: Value, kind: &str, mode: &str) -> i64 {
    let id = start(db).await;
    let kind = kind.to_owned();
    let mode = mode.to_owned();
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        tx.conn().execute(
            "UPDATE slack_imports SET options=?,kind=?,mode=? WHERE id=?",
            params![options.to_string(), kind, mode, id],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    id
}
fn history_routes(messages: Value) -> Vec<Route> {
    let mut response = routes(false);
    response.retain(|r| {
        !r.path
            .starts_with("/api/conversations.history?channel=CCHAN")
    });
    response.push(
        Route::new(
            "GET",
            "slack.com",
            "/api/conversations.history?channel=CCHAN&limit=200",
            200,
        )
        .body(json!({"ok":true,"messages":messages,"has_more":false}).to_string()),
    );
    response
}
async fn step(
    db: &Database,
    crypto: std::sync::Arc<rails_compat::ar_encryption::ArEncryption>,
    id: i64,
    network: crate::integrations::net::Network,
) {
    let worker_db = db.clone();
    perform_import(
        db.clone(),
        crypto,
        id,
        move |row, lease, token| async move {
            Runner::new(
                SqlStore {
                    db: worker_db,
                    lease: Some(lease),
                    allowed_domains: HashSet::new(),
                },
                Client::with_network(token, None, false, network),
                row,
            )
            .with_budget(Duration::ZERO)
            .step()
            .await
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_lifecycle_mapped_deleted_room_is_skipped_and_other_room_finishes() {
    let (db, crypto, _dir) = setup().await;
    let first = configured(
        &db,
        json!({"conversation_ids":["CCHAN","CARCH"]}),
        "workspace",
        "import",
    )
    .await;
    let (_server, network) = fake(routes(false)).await;
    import(&db, crypto.clone(), first, network.clone()).await;
    db.write(|tx| {
        let id = users::mapped_id(tx.conn(), 1, "conversation", "CCHAN")?.unwrap();
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), id],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let second = configured(
        &db,
        json!({"conversation_ids":["CCHAN","CARCH"]}),
        "workspace",
        "import",
    )
    .await;
    let mut responses = routes(false);
    responses.push(
        Route::new(
            "GET",
            "slack.com",
            "/api/conversations.history?channel=CARCH&oldest=1697408020.000020&limit=200",
            200,
        )
        .body(include_str!("../fixtures/history_CARCH.json")),
    );
    let (_server, network) = fake(responses).await;
    let row = import(&db, crypto, second, network).await;
    for (id, action) in [("CCHAN", "skip"), ("CARCH", "merge")] {
        let entry = array(&row.stats["conversations"])
            .iter()
            .find(|e| e["id"] == id)
            .unwrap();
        assert_eq!(entry["target"]["action"], action);
        assert_eq!(entry["done"], true);
    }
    let issue = db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT message FROM slack_import_issues WHERE slack_import_id=?",
                [second],
                |r| r.get::<_, String>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        issue,
        "mapped room was deleted; undo the earlier run or remove the mapping to re-import"
    );
}

#[tokio::test]
async fn slack_lifecycle_large_groups_personal_targets_and_dry_preview_follow_rails() {
    // Workspace auto, workspace explicit, personal auto/explicit, preview explicit,
    // and personal private-channel explicit are six distinct Rails declarations.
    for (kind, mode, explicit, large) in [
        ("workspace", "import", false, true),
        ("workspace", "import", true, true),
        ("personal", "import", true, true),
        ("workspace", "dry_run", true, true),
        ("personal", "import", true, false),
    ] {
        let (db, crypto, _dir) = setup().await;
        let (existing, target) = db
            .write(|tx| {
                let old = Room::create(
                    tx,
                    campfire_db::RoomType::Closed,
                    Some("Unknown Slack user, Unknown Slack user, Unknown Slack user +8"),
                    1,
                )?;
                let target = Room::create(tx, campfire_db::RoomType::Closed, Some("Target"), 1)?;
                Ok((old.id, target.id))
            })
            .await
            .unwrap();
        let channel = if large { "GMPIMBIG" } else { "CPRIV" };
        let options = if explicit {
            json!({"conversation_ids":[channel],"room_targets":{channel:target}})
        } else {
            json!({"conversation_ids":[channel]})
        };
        let id = configured(&db, options, kind, mode).await;
        let mut response = routes(false);
        response.retain(|r| !r.path.starts_with("/api/conversations.list"));
        let types = if kind == "personal" {
            "im%2Cmpim%2Cprivate_channel"
        } else {
            "public_channel%2Cprivate_channel"
        };
        response.push(Route::new("GET","slack.com",&format!("/api/conversations.list?types={types}&exclude_archived=false&limit=200"),200)
            .body(json!({"ok":true,"channels":[{"id":channel,"name":if large{"big-group"}else{"secret"},"is_private":true,"is_mpim":large,"num_members":11}]}).to_string()));
        if large {
            response.push(Route::new("GET","slack.com","/api/conversations.members?channel=GMPIMBIG&limit=1000",200)
                .body(json!({"ok":true,"members":(101..=111).map(|n|format!("U{n}")).collect::<Vec<_>>()} ).to_string()));
            response.push(Route::new("GET","slack.com","/api/conversations.history?channel=GMPIMBIG&limit=200",200)
                .body(json!({"ok":true,"messages":[{"type":"message","user":"U001","text":"hello big group","ts":"1700000060.000060"}]}).to_string()));
        }
        let (_server, network) = fake(response).await;
        let row = import(&db, crypto, id, network).await;
        let merged = kind == "workspace" && explicit;
        assert_eq!(
            row.stats["conversations"][0]["target"]["action"],
            if merged { "merge" } else { "create" }
        );
        db.read(move |c| {
            assert_eq!(campfire_db::Message::for_room(c,existing)?.len(),0);
            assert_eq!(campfire_db::Message::for_room(c,target)?.len(),usize::from(merged&&mode=="import"));
            if mode=="import" {
                let record=users::mapped_id(c,1,"conversation",channel)?.unwrap();
                assert_eq!(record==target,merged);
                let created:bool=c.query_row("SELECT created_record FROM slack_import_records WHERE slack_kind='conversation' AND slack_key=?",[channel],|r|r.get(0))?;
                assert_eq!(created,!merged);
            }else { assert_eq!(row.stats["conversations"][0]["target"]["room_id"],target); }
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn slack_lifecycle_room_membership_mapping_crash_rolls_back_then_resumes_cleanly() {
    let (db, crypto, _dir) = setup().await;
    db.write(|tx|{tx.conn().execute_batch("CREATE TRIGGER membership_crash BEFORE INSERT ON slack_import_records WHEN NEW.slack_kind='membership' BEGIN SELECT RAISE(ABORT,'simulated crash writing membership records'); END;")?;Ok(())}).await.unwrap();
    let failed = configured(
        &db,
        json!({"conversation_ids":["CCHAN"]}),
        "workspace",
        "import",
    )
    .await;
    let (_server, network) = fake(routes(false)).await;
    for _ in 0..20 {
        step(&db, crypto.clone(), failed, network.clone()).await;
        if run(&db, failed).await.status == "failed" {
            break;
        }
    }
    assert_eq!(run(&db, failed).await.status, "failed");
    db.write(|tx|{
        assert!(Room::all(tx.conn())?.is_empty());
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM slack_import_records WHERE slack_kind IN ('conversation','membership')",[],|r|r.get::<_,i64>(0))?,0);
        tx.conn().execute_batch("DROP TRIGGER membership_crash")?;Ok(())
    }).await.unwrap();
    let resumed = configured(
        &db,
        json!({"conversation_ids":["CCHAN"]}),
        "workspace",
        "import",
    )
    .await;
    let row = import(&db, crypto, resumed, network).await;
    assert_eq!(row.stats["users"]["total"], 0);
    db.read(move |c| {
        let room=Room::all(c)?.pop().unwrap();
        let mapped:i64=c.query_row("SELECT COUNT(*) FROM slack_import_records WHERE slack_import_id=? AND slack_kind='membership'",[resumed],|r|r.get(0))?;
        assert!(mapped>0);assert_eq!(mapped as usize,room.memberships(c)?.len());Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn slack_lifecycle_date_bounds_filter_all_rows_and_are_sent_on_every_history_page() {
    let (db, crypto, _dir) = setup().await;
    let id = configured(
        &db,
        json!({"oldest":"2023-11-14T22:13:30.000010Z","latest":"2023-11-14T22:13:30.500000Z"}),
        "workspace",
        "import",
    )
    .await;
    let mut response = routes(false);
    for route in &mut response {
        if route.path.starts_with("/api/conversations.history?") {
            let (path, query) = route.path.split_once('?').unwrap();
            let mut pairs: url::form_urlencoded::Serializer<String> =
                url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                pairs.append_pair(&key, &value);
                if key == "channel" {
                    pairs.append_pair("oldest", "1700000010.000010");
                    pairs.append_pair("latest", "1700000010.500000");
                }
            }
            route.path = format!("{path}?{}", pairs.finish());
        }
    }
    let (server, network) = fake(response).await;
    let row = import(&db, crypto, id, network).await;
    assert_eq!(row.stats["counts"]["messages"], 1);
    db.read(|c| {
        let rows = campfire_db::Message::ordered(c)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].created_at, writer::slack_time("1700000010.000010")?);
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(server.received().iter().filter(|r|r.target.starts_with("/api/conversations.history?channel=CCHAN&oldest=1700000010.000010&latest=1700000010.500000")).count(),2);
}

#[tokio::test]
async fn slack_lifecycle_scope_transient_and_auth_failures_preserve_resumable_mappings() {
    for failure in ["scope", "http", "auth"] {
        let (db, crypto, _dir) = setup().await;
        let id = configured(
            &db,
            json!({"conversation_ids":["CCHAN"]}),
            "workspace",
            "import",
        )
        .await;
        let mut response = routes(false);
        response.retain(|r| {
            !r.path
                .starts_with("/api/conversations.history?channel=CCHAN&limit")
        });
        let (status,body)=match failure {"scope"=>(200,json!({"ok":false,"error":"missing_scope","needed":"channels:history","provided":"channels:read"}).to_string()),"auth"=>(200,json!({"ok":false,"error":"invalid_auth"}).to_string()),_=>(500,"boom".into())};
        response.push(
            Route::new(
                "GET",
                "slack.com",
                "/api/conversations.history?channel=CCHAN&limit=200",
                status,
            )
            .body(body),
        );
        let (server, network) = fake(response).await;
        for _ in 0..30 {
            step(&db, crypto.clone(), id, network.clone()).await;
            if run(&db, id).await.status == "failed" {
                break;
            }
        }
        let failed = run(&db, id).await;
        assert_eq!(failed.status, "failed");
        let expected = match failure {
            "scope" => {
                "Slack token is missing a required scope for conversations.history (needed: channels:history)"
            }
            "auth" => "Slack authentication failed for conversations.history (invalid_auth)",
            _ => "Slack HTTP 500 for conversations.history",
        };
        assert_eq!(failed.error.as_deref(), Some(expected));
        assert_eq!(
            server
                .received()
                .iter()
                .filter(|r| r.target == "/api/conversations.history?channel=CCHAN&limit=200")
                .count(),
            if failure == "http" { 4 } else { 1 }
        );
        db.write(move |tx| {
            let reason: Option<String> = tx.conn().query_row(
                "SELECT disconnected_reason FROM slack_connections",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(reason.is_some(), failure == "auth");
            tx.conn()
                .execute("UPDATE slack_connections SET disconnected_reason=NULL", [])?;
            Ok(())
        })
        .await
        .unwrap();
        let next = configured(
            &db,
            json!({"conversation_ids":["CCHAN"]}),
            "workspace",
            "import",
        )
        .await;
        let (_server, network) = fake(routes(false)).await;
        let row = import(&db, crypto, next, network).await;
        assert_eq!(row.stats["users"]["total"], 0);
        assert!(integer(&row.stats["counts"]["messages"]) > 0);
    }
}

#[tokio::test]
async fn slack_workspace_deleted_queued_parent_and_truncated_reactions_match_rails() {
    let (db, crypto, _dir) = setup().await;
    let id = configured(
        &db,
        json!({"conversation_ids":["CCHAN"]}),
        "workspace",
        "import",
    )
    .await;
    let messages = json!([{"type":"message","user":"U001","text":"parent","ts":"1700000002.000002","reply_count":1}]);
    let (_server, network) = fake(history_routes(messages)).await;
    let mut deleted = false;
    for _ in 0..40 {
        step(&db, crypto.clone(), id, network.clone()).await;
        let row = run(&db, id).await;
        if !deleted && !array(&row.state["convo"]["thread_queue"]).is_empty() {
            db.write(|tx| {
                let id =
                    users::mapped_id(tx.conn(), 1, "message", "CCHAN:1700000002.000002")?.unwrap();
                campfire_db::Message::find(tx.conn(), id)?.destroy_imported(tx)
            })
            .await
            .unwrap();
            deleted = true;
        }
        if row.status == "completed" {
            break;
        }
    }
    assert!(deleted);
    assert_eq!(run(&db, id).await.status, "completed");
    db.read(move |c| {assert_eq!(campfire_db::Message::count(c)?,0);assert_eq!(c.query_row("SELECT COUNT(*) FROM slack_import_issues WHERE slack_import_id=? AND message LIKE '%was deleted%'",[id],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
    let (db, crypto, _dir) = setup().await;
    let id = configured(
        &db,
        json!({"conversation_ids":["CCHAN"]}),
        "workspace",
        "import",
    )
    .await;
    let (_server,network)=fake(history_routes(json!([{"type":"message","user":"U001","text":"popular","ts":"1700000001.000001","reactions":[{"name":"thumbsup","users":["U002","UADMIN"],"count":5},{"name":"+1","users":["U001"],"count":3}]}]))).await;
    let row = import(&db, crypto, id, network).await;
    assert_eq!(row.stats["counts"]["reactions"], 3);
    db.read(move |c| {assert_eq!(c.query_row("SELECT COUNT(*) FROM boosts",[],|r|r.get::<_,i64>(0))?,3);let issue:String=c.query_row("SELECT message FROM slack_import_issues WHERE slack_import_id=?",[id],|r|r.get(0))?;
        assert_eq!(issue,"Slack truncated the reaction list on CCHAN:1700000001.000001 (thumbsup, +1); imported the listed users only");Ok(())}).await.unwrap();
}

#[tokio::test]
async fn slack_model_commit_refreshes_owned_lease_and_preserves_unleased_stamp() {
    for leased in [false, true] {
        let (db, _, _dir) = setup().await;
        let id = start(&db).await;
        let token=db.write(move |tx|{SlackImport::claim_running(tx,id)?;let token=SlackImport::acquire_step_lease(tx,id,StepStatus::Running)?.unwrap();
            tx.conn().execute("UPDATE slack_imports SET state=json_set(state,'$.step_started_at','2026-03-02T15:57:00.000000Z'),heartbeat_at='2026-03-02 15:57:00' WHERE id=?",[id])?;Ok(token)}).await.unwrap();
        let before = run(&db, id).await;
        let store = SqlStore {
            db: db.clone(),
            lease: leased.then_some(token.clone()),
            allowed_domains: HashSet::new(),
        };
        let mut p = Progress::new(before.clone());
        p.state["users_done"] = json!(true);
        store.commit(p, Operation::Save).await.unwrap().unwrap();
        let after = run(&db, id).await;
        assert_eq!(after.state["step_lease_token"], token);
        if leased {
            assert!(
                after.state["step_started_at"].as_str().unwrap()
                    > before.state["step_started_at"].as_str().unwrap()
            );
        } else {
            assert_eq!(
                after.state["step_started_at"],
                before.state["step_started_at"]
            );
        }
        assert!(after.heartbeat_at > before.heartbeat_at);
        assert_eq!(after.state["users_done"], true);
    }
}

#[tokio::test]
async fn slack_model_finishing_cancel_failure_and_undo_kick_next_queued_run() {
    for terminal in ["completed", "cancelled", "failed", "undone"] {
        let (db, crypto, _dir) = setup().await;
        let first = configured(
            &db,
            json!({"conversation_ids":["CARCH"]}),
            "workspace",
            "import",
        )
        .await;
        let (_server, network) = fake(routes(false)).await;
        if terminal == "undone" {
            import(&db, crypto.clone(), first, network).await;
            assert!(
                db.write(move |tx| SlackImport::undo(tx, first))
                    .await
                    .unwrap()
            );
        } else {
            db.write(move |tx| {
                SlackImport::claim_running(tx, first)?;
                Ok(())
            })
            .await
            .unwrap();
        }
        let second = start(&db).await;
        db.write(move |tx| {
            SlackImport::clear_pending_step_job(tx, second)?;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
        match terminal {
            "cancelled" => {
                db.write(move |tx| SlackImport::cancel(tx, first))
                    .await
                    .unwrap();
                let store = SqlStore {
                    db: db.clone(),
                    lease: None,
                    allowed_domains: HashSet::new(),
                };
                assert!(
                    store
                        .commit(Progress::new(run(&db, first).await), Operation::FinishRooms)
                        .await
                        .unwrap()
                        .is_none()
                );
            }
            "failed" => {
                db.write(move |tx| SlackImport::mark_failed(tx, first, "boom"))
                    .await
                    .unwrap();
            }
            "undone" => {
                for _ in 0..20 {
                    let worker = db.clone();
                    super::super::jobs::perform_undo(
                        db.clone(),
                        first,
                        move |_, lease| async move {
                            super::super::undoer::Undoer {
                                db: worker,
                                id: first,
                                lease,
                            }
                            .step()
                            .await
                        },
                    )
                    .await
                    .unwrap();
                    if run(&db, first).await.status == "undone" {
                        break;
                    }
                }
            }
            _ => {
                let mut p = Progress::new(run(&db, first).await);
                p.state["phase"] = json!("finishing");
                let store = SqlStore {
                    db: db.clone(),
                    lease: None,
                    allowed_domains: HashSet::new(),
                };
                store
                    .commit(p, Operation::FinishRooms)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(run(&db, first).await.stats["phase"], "done");
            }
        }
        assert_eq!(run(&db, first).await.status, terminal);
        let jobs = db.read(inspect::all).await.unwrap();
        let next: Vec<_> = jobs
            .iter()
            .filter(|j| j.arguments == json!({"import_id":second}))
            .collect();
        assert_eq!(next.len(), 1);
        assert!(run(&db, second).await.state.get("enqueued_at").is_some());
        assert_eq!(next[0].class, "SlackImport::StepJob");
    }
}

#[tokio::test]
async fn slack_workspace_full_import_completion_hands_off_to_next_queued_job() {
    let (db, crypto, _dir) = setup().await;
    let first = configured(
        &db,
        json!({"conversation_ids":["CARCH"]}),
        "workspace",
        "import",
    )
    .await;
    let second = start(&db).await;
    db.write(move |tx| {
        SlackImport::clear_pending_step_job(tx, second)?;
        tx.conn().execute("DELETE FROM background_jobs", [])?;
        Ok(())
    })
    .await
    .unwrap();
    let (_server, network) = fake(routes(false)).await;
    import(&db, crypto, first, network).await;
    assert!(run(&db, second).await.state.get("enqueued_at").is_some());
    let jobs = db.read(inspect::all).await.unwrap();
    assert_eq!(
        jobs.iter()
            .filter(|j| j.arguments == json!({"import_id":second}))
            .count(),
        1
    );
    assert_eq!(run(&db, second).await.status, "queued");
}

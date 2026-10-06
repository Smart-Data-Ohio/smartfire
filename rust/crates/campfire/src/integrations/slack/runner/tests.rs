use super::super::client::tests::fake;
use super::super::jobs::tests::{run, setup, start};
use super::*;
use crate::integrations::test_support::Route;
use campfire_db::{Database, models::slack_import::SlackImport};

/// A transactional operation recorder for testing the engine independently of mapping policy.
/// It persists the engine's actual progress and rejects cancelled pages in SQLite, just as
/// every Store implementation is required to do; it does not emulate the HTTP client/runner.
#[derive(Clone)]
struct Recorder(Database);
impl Store for Recorder {
    async fn load(&self, id: i64) -> anyhow::Result<Option<SlackImport>> {
        Ok(self.0.read(move |c| SlackImport::find(c, id)).await?)
    }
    async fn commit(
        &self,
        mut progress: Progress,
        operation: Operation,
    ) -> anyhow::Result<Option<Progress>> {
        Ok(self
            .0
            .write(move |tx| {
                let id = progress.run.id;
                if SlackImport::find(tx.conn(), id)?.is_none_or(|r| r.status != "running") {
                    return Ok(None);
                }
                let operation_name = match &operation {
                    Operation::Users(_) => "users",
                    Operation::Resolve => "resolve",
                    Operation::Bounds => "bounds",
                    Operation::History(_) => "history",
                    Operation::Replies(_) => "replies",
                    Operation::FinishThread => "finish_thread",
                    Operation::FinishRooms => "finish_rooms",
                    Operation::Save => "save",
                };
                tx.conn()
                    .execute("INSERT INTO operations (name) VALUES (?)", [operation_name])?;
                match operation {
                    Operation::Users(members) => {
                        progress.stats["users"]["total"] = json!(array(&members).len());
                    }
                    Operation::Resolve => {
                        progress.state["convo"]["resolved"] = json!(true);
                        progress.state["convo"]["room_id"] = json!(123);
                    }
                    Operation::Bounds => {
                        progress.state["convo"]["bounds"] = json!({"oldest":null,"latest":null});
                    }
                    Operation::History(messages) => {
                        progress.entry_mut()["messages"] = json!(array(&messages).len());
                        if !progress.dry_run() {
                            for message in array(&messages) {
                                if integer(&message["reply_count"]) > 0 {
                                    progress.state["convo"]["thread_queue"]
                                        .as_array_mut()
                                        .unwrap()
                                        .push(json!({"ts":message["ts"],"message_id":1234}));
                                }
                            }
                        }
                    }
                    Operation::FinishRooms => {
                        progress.state["phase"] = json!("done");
                        progress.stats["phase"] = json!("done");
                    }
                    _ => (),
                }
                let status = if operation_name == "finish_rooms" {
                    "completed"
                } else {
                    "running"
                };
                tx.conn().execute(
                    "UPDATE slack_imports SET state=?,stats=?,status=? WHERE id=?",
                    rusqlite::params![
                        progress.state.to_string(),
                        progress.stats.to_string(),
                        status,
                        id
                    ],
                )?;
                Ok(Some(progress))
            })
            .await?)
    }
}
async fn started(db: &Database, state: Value, options: Value) -> i64 {
    let id = start(db).await;
    db.write(move |tx| {
        tx.conn()
            .execute_batch("CREATE TABLE operations (id INTEGER PRIMARY KEY,name TEXT NOT NULL)")?;
        tx.conn().execute(
            "UPDATE slack_imports SET status='running',state=?,options=? WHERE id=?",
            rusqlite::params![state.to_string(), options.to_string(), id],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    id
}
fn route(path: &str, body: &str) -> Route {
    Route::new("GET", "slack.com", path, 200).body(body.as_bytes().to_vec())
}

#[tokio::test]
async fn slack_runner_lease_only_conflict_is_not_mistaken_for_saved_progress() {
    struct Conflict(Database, bool);
    impl Store for Conflict {
        async fn load(&self, id: i64) -> anyhow::Result<Option<SlackImport>> {
            Ok(self.0.read(move |c| SlackImport::find(c, id)).await?)
        }
        async fn commit(&self, p: Progress, _: Operation) -> anyhow::Result<Option<Progress>> {
            let id = p.run.id;
            let saved = self.1;
            self.0.write(move |tx|{
                tx.conn().execute("UPDATE slack_imports SET state=json_set(state,'$.step_lease_token','replacement','$.step_started_at','later') WHERE id=?",[id])?;
                if saved {tx.conn().execute("UPDATE slack_imports SET state=json_set(state,'$.phase','messages') WHERE id=?",[id])?;}
                Ok(())
            }).await?;
            Err(campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
                Some("fixture identity conflict".into()),
            ))
            .into())
        }
    }
    for saved in [false, true] {
        let (db, _, _dir) = setup().await;
        let id = started(&db, json!({"phase":"users","users_done":true}), json!({})).await;
        // Rails transition_to saves the state defaults before the competing lease write.
        let state = Progress::new(run(&db, id).await).state;
        db.write(move |tx| {
            tx.conn().execute(
                "UPDATE slack_imports SET state=? WHERE id=?",
                rusqlite::params![state.to_string(), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        let result = Runner::new(
            Conflict(db.clone(), saved),
            Client::new("fixture-conflict-grant".into(), None),
            run(&db, id).await,
        )
        .step()
        .await;
        if saved {
            assert_eq!(result.unwrap(), Outcome::Continue);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .downcast_ref::<campfire_db::Error>()
                    .unwrap()
                    .is_record_not_unique()
            );
        }
    }
}

#[tokio::test]
async fn slack_runner_users_cursor_defaults_and_explicit_transition() {
    let (db, _, _dir) = setup().await;
    let id = started(&db, json!({}), json!({})).await;
    let (server, network) = fake(vec![route(
        "/api/users.list?limit=200",
        include_str!("../fixtures/users.json"),
    )])
    .await;
    let client = Client::with_network("fixture-token".into(), None, false, network);
    assert_eq!(
        Runner::new(Recorder(db.clone()), client, run(&db, id).await)
            .step()
            .await
            .unwrap(),
        Outcome::Continue
    );
    let row = run(&db, id).await;
    assert_eq!(row.state["phase"], "users");
    assert_eq!(row.state["users_done"], true);
    assert_eq!(row.stats["api_calls"], 1);
    assert!(integer(&row.stats["users"]["total"]) > 0);
    let client = Client::with_network(
        "fixture-token".into(),
        None,
        false,
        crate::net::Network::system(),
    );
    assert_eq!(
        Runner::new(Recorder(db.clone()), client, row)
            .step()
            .await
            .unwrap(),
        Outcome::Continue
    );
    assert_eq!(run(&db, id).await.state["phase"], "conversations");
    assert_eq!(server.received().len(), 1);
}
#[tokio::test]
async fn slack_runner_discovers_sorted_scope_and_does_not_page_history_in_discovery() {
    let (db, _, _dir) = setup().await;
    let id = started(
        &db,
        json!({"phase":"conversations"}),
        json!({"conversation_ids":["CCHAN","CARCH"],"include_private":false}),
    )
    .await;
    let (server, network) = fake(vec![route(
        "/api/conversations.list?types=public_channel&exclude_archived=false&limit=200",
        include_str!("../fixtures/conversations_workspace.json"),
    )])
    .await;
    let client = Client::with_network("fixture-token".into(), None, false, network);
    assert_eq!(
        Runner::new(Recorder(db.clone()), client, run(&db, id).await)
            .step()
            .await
            .unwrap(),
        Outcome::Continue
    );
    let row = run(&db, id).await;
    assert_eq!(row.state["phase"], "messages");
    assert_eq!(row.state["conversation_ids"], json!(["CARCH", "CCHAN"]));
    assert_eq!(row.stats["conversations"][0]["target"]["action"], "pending");
    assert_eq!(server.received().len(), 1);
}
#[tokio::test]
async fn slack_runner_zero_budget_stops_after_page_and_preserves_reply_cursor() {
    let (db, _, _dir) = setup().await;
    let id=started(&db,json!({"phase":"messages","conversation_ids":["CCHAN"],
        "conversations":[{"id":"CCHAN","name":"general"}],"convo":{"id":"CCHAN","member_ids":[],"members_done":true,
        "resolved":true,"room_id":123,"history_done":false,"thread_queue":[],"thread_ts":null,"bounds":{"oldest":null,"latest":null}}}),json!({})).await;
    db.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET stats=? WHERE id=?",
            rusqlite::params![
                json!({"conversations":[{"id":"CCHAN","name":"general","messages":0,"threads":0}]})
                    .to_string(),
                id
            ],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let mut replies: Value =
        serde_json::from_str(include_str!("../fixtures/replies_CCHAN_parent.json")).unwrap();
    replies["response_metadata"]["next_cursor"] = json!("thread-page-2");
    let (server, network) = fake(vec![
        route(
            "/api/conversations.history?channel=CCHAN&limit=200",
            include_str!("../fixtures/history_CCHAN_p2.json"),
        ),
        route(
            "/api/conversations.replies?channel=CCHAN&ts=1700000002.000002&limit=200",
            &replies.to_string(),
        ),
    ])
    .await;
    for _ in 0..2 {
        let client = Client::with_network("fixture-token".into(), None, false, network.clone());
        assert_eq!(
            Runner::new(Recorder(db.clone()), client, run(&db, id).await)
                .with_budget(Duration::ZERO)
                .step()
                .await
                .unwrap(),
            Outcome::Continue
        );
    }
    let row = run(&db, id).await;
    assert_eq!(row.state["convo"]["thread_ts"], "1700000002.000002");
    assert_eq!(row.state["convo"]["thread_cursor"], "thread-page-2");
    assert_eq!(server.received().len(), 2);
    assert_eq!(row.stats["api_calls"], 2);
}
#[tokio::test]
async fn slack_runner_cancelled_during_fetch_does_not_commit_page_or_progress() {
    let (db, _, _dir) = setup().await;
    let id = started(&db, json!({}), json!({})).await;
    let mut page = route(
        "/api/users.list?limit=200",
        include_str!("../fixtures/users.json"),
    );
    page.delay = Duration::from_millis(250);
    let (server, network) = fake(vec![page]).await;
    let client = Client::with_network("fixture-token".into(), None, false, network);
    let execution =
        tokio::spawn(Runner::new(Recorder(db.clone()), client, run(&db, id).await).step());
    tokio::time::timeout(Duration::from_secs(5), async {
        while server.received().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    db.write(move |tx| SlackImport::cancel(tx, id))
        .await
        .unwrap();
    assert_eq!(execution.await.unwrap().unwrap(), Outcome::Stopped);
    assert_eq!(run(&db, id).await.state, json!({}));
    assert_eq!(
        db.read(
            |c| Ok(c.query_row("SELECT count(*) FROM operations", [], |r| r
                .get::<_, i64>(0))?)
        )
        .await
        .unwrap(),
        0
    );
}
#[tokio::test]
async fn slack_runner_finishing_and_skip_targets_complete_without_network() {
    let (db, _, _dir) = setup().await;
    let id=started(&db,json!({"phase":"messages","conversation_ids":["CCHAN"],"conversations":[{"id":"CCHAN","name":"general"}]}),json!({"room_targets":{"CCHAN":"skip"}})).await;
    db.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET stats=? WHERE id=?",
            rusqlite::params![
                json!({"conversations":[{"id":"CCHAN","name":"general"}]}).to_string(),
                id
            ],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let client = Client::new("fixture-token".into(), None);
    assert_eq!(
        Runner::new(Recorder(db.clone()), client, run(&db, id).await)
            .step()
            .await
            .unwrap(),
        Outcome::Continue
    );
    let row = run(&db, id).await;
    assert_eq!(row.state["phase"], "finishing");
    assert_eq!(row.stats["conversations"][0]["done"], true);
    assert_eq!(
        Runner::new(
            Recorder(db.clone()),
            Client::new("fixture-token".into(), None),
            row
        )
        .step()
        .await
        .unwrap(),
        Outcome::Done
    );
    assert_eq!(run(&db, id).await.status, "completed");
}

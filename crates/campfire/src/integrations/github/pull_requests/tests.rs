use super::*;
use crate::integrations::{
    github::{
        subscriptions::{self, RepositorySubscription},
        threads::PullRequestThread,
    },
    test_support::TestDb,
};
use campfire_db::{
    ChannelThread, Env, Message, NewChannelThread, NewMessage, RecordingSink, TestClock, fixtures,
};
use std::sync::Arc;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/github_domain.json")).unwrap()
}
fn attrs(value: &Value) -> Vec<(&'static str, SqlValue)> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            let name = match k.as_str() {
                "owner" => "owner",
                "repo" => "repo",
                "number" => "number",
                "title" => "title",
                "private" => "private",
                _ => panic!("unsupported oracle input {k}"),
            };
            (
                name,
                match v {
                    Value::Null => SqlValue::Null,
                    Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
                    Value::String(s) => SqlValue::Text(s.clone()),
                    Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap()),
                    _ => panic!("unsupported type"),
                },
            )
        })
        .collect()
}
fn errors(error: Option<campfire_db::Error>) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(error) = error {
        let campfire_db::Error::RecordInvalid(errors) = error else {
            panic!("{error}")
        };
        for (key, message) in errors.0 {
            map.entry(key)
                .or_insert(json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!(message));
        }
    }
    Value::Object(map)
}
async fn database() -> (TestDb, TestClock, RecordingSink) {
    let clock = TestClock::frozen_at(Timestamp::from_jiff(
        "2026-01-01T12:00:00Z".parse().unwrap(),
    ));
    let sink = RecordingSink::new();
    let env = Env {
        clock: Arc::new(clock.clone()),
        sink: Arc::new(sink.clone()),
        bcrypt_cost: 4,
        message_reference_syncs: vec![super::super::references::sync],
        ..Default::default()
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g");
    let db = tokio::task::spawn_blocking(move || TestDb::with_env(env, &dir))
        .await
        .unwrap();
    (db, clock, sink)
}
fn message(tx: &mut Tx<'_>, room: i64, source: &str) -> Result<Message> {
    Message::create_markdown(
        tx,
        NewMessage {
            room_id: room,
            creator_id: fixtures::identify("david"),
            ..Default::default()
        },
        source,
    )
}
fn thread(tx: &mut Tx<'_>, room: i64, name: &str) -> Result<ChannelThread> {
    let parent = message(tx, room, "Discuss")?;
    ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: room,
            creator_id: fixtures::identify("david"),
            parent_message_id: Some(parent.id),
            name: Some(name.into()),
            ..Default::default()
        },
    )
}

#[tokio::test]
async fn github_pr_identity_display_files_and_save_callbacks_match_rails() {
    let (fixture, clock, sink) = database().await;
    for case in vectors()["updates"].as_array().unwrap() {
        clock.travel_to(Timestamp::from_jiff(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ));
        let pr = fixture
            .db
            .write(|tx| PullRequest::for_reference(tx, "Rails", "Rails", 1))
            .await
            .unwrap();
        assert_eq!((pr.owner.as_str(), pr.repo.as_str()), ("rails", "rails"));
        assert_eq!(
            fixture
                .db
                .write(|tx| PullRequest::for_reference(tx, "RAILS", "rails", 1))
                .await
                .unwrap()
                .id,
            pr.id
        );
        sink.take();
        clock.travel_to(Timestamp::from_jiff(
            "2026-01-01T12:01:00Z".parse().unwrap(),
        ));
        let attributes = attrs(&case["attributes"]);
        let result = fixture
            .db
            .write(move |tx| update(tx, pr.id, &attributes))
            .await;
        assert_eq!(result.is_ok(), case["ok"], "{case}");
        assert_eq!(errors(result.err()), case["errors"], "{case}");
        let stored = fixture
            .db
            .read(move |conn| PullRequest::find(conn, pr.id))
            .await
            .unwrap();
        assert_eq!(
            stored.updated_at.jiff().strftime("%F %T.%6f").to_string(),
            case["updated_at"],
            "{case}"
        );
        assert_eq!(
            sink.take()
                .iter()
                .filter(|e| matches!(e,Event::Broadcast(b) if b.kind==CardUpdated::KIND))
                .count(),
            case["broadcasts"].as_u64().unwrap() as usize
        );
        fixture
            .db
            .write(move |tx| {
                tx.conn()
                    .execute("DELETE FROM github_pull_requests WHERE id=?", [pr.id])?;
                Ok(())
            })
            .await
            .unwrap();
    }
    let mut pr = fixture
        .db
        .write(|tx| PullRequest::for_reference(tx, "stored", "names", 5))
        .await
        .unwrap();
    for case in vectors()["display"].as_array().unwrap() {
        pr.payload = case["attributes"]
            .get("payload")
            .filter(|v| !v.is_null())
            .cloned();
        pr.html_url = case["attributes"]["html_url"].as_str().map(str::to_owned);
        let result = pr.display_full_name();
        if case.get("error").is_some() {
            assert!(result.is_err(), "{case}");
        } else {
            assert_eq!(result.unwrap(), case["display"], "{case}");
        }
    }
    for case in vectors()["files"].as_array().unwrap() {
        pr.changed_files = case["storage"].as_str().map(str::to_owned);
        let result = pr.changed_files_summary();
        if case.get("error").is_some() {
            assert!(result.is_err(), "{case}");
        } else {
            assert_eq!(result.unwrap(), case["summary"], "{case}");
        }
    }
}

#[tokio::test]
async fn github_pr_staleness_claim_boundaries_and_concurrent_upserts_are_quiet() {
    let (fixture, _, sink) = database().await;
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let db = fixture.db.clone();
        tasks.push(tokio::spawn(async move {
            db.write(|tx| PullRequest::for_reference(tx, "Rails", "Rails", 50))
                .await
                .unwrap()
                .id
        }));
    }
    let mut ids = Vec::new();
    for task in tasks {
        ids.push(task.await.unwrap());
    }
    assert!(ids.iter().all(|id| *id == ids[0]));
    let id = ids[0];
    fixture
        .db
        .write(move |tx| {
            let mut pr = PullRequest::find(tx.conn(), id)?;
            assert!(pr.stale(tx.now()));
            pr.fetched_at = Some(tx.now().ago(STALE_AFTER));
            assert!(!pr.stale(tx.now()));
            pr.fetched_at = Some(
                tx.now()
                    .ago(STALE_AFTER + jiff::SignedDuration::from_micros(1)),
            );
            assert!(pr.stale(tx.now()));
            assert!(pr.claim_fetch_request(tx)?);
            assert!(!pr.claim_fetch_request(tx)?);
            assert!(pr.fetch_requested_recently(tx.now()));
            pr.release_fetch_request(tx)?;
            assert!(pr.claim_fetch_request(tx)?);
            let cutoff = tx.now().ago(STALE_AFTER);
            tx.conn().execute(
                "UPDATE github_pull_requests SET fetch_requested_at=? WHERE id=?",
                params![cutoff, id],
            )?;
            pr = PullRequest::find(tx.conn(), id)?;
            assert!(!pr.claim_fetch_request(tx)?);
            tx.conn().execute(
                "UPDATE github_pull_requests SET fetch_requested_at=? WHERE id=?",
                params![cutoff.ago(jiff::SignedDuration::from_micros(1)), id],
            )?;
            pr = PullRequest::find(tx.conn(), id)?;
            assert!(pr.claim_fetch_request(tx)?);
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        sink.take()
            .iter()
            .all(|e| !matches!(e,Event::Broadcast(b) if b.kind==CardUpdated::KIND))
    );
}

#[tokio::test]
async fn github_pr_case_collapse_repoints_links_and_mappings_without_destroying_threads() {
    let (fixture, _, _) = database().await;
    fixture.db.write(|tx| {
        let winner=PullRequest::for_reference(tx,"rails","rails",12)?;
        let solo=PullRequest::for_reference(tx,"rails","rails",13)?;
        let loser=PullRequest::for_reference(tx,"rails","rails",14)?;
        tx.conn().execute("UPDATE github_pull_requests SET owner='Rails',repo='Rails',number=12 WHERE id=?",[loser.id])?;
        tx.conn().execute("UPDATE github_pull_requests SET owner='Rails',repo='Rails' WHERE id=?",[solo.id])?;
        let room=fixtures::identify("designers");let other=fixtures::identify("watercooler");
        let a=message(tx,room,"linked")?;let b=message(tx,room,"double")?;
        for (message,pr) in [(a.id,loser.id),(b.id,loser.id),(b.id,winner.id)] {tx.conn().execute("INSERT INTO github_pull_request_references (message_id,github_pull_request_id,created_at,updated_at) VALUES (?,?,?,?)",params![message,pr,tx.now(),tx.now()])?;}
        let repoint=thread(tx,other,"Repoint")?;PullRequestThread::create(tx,loser.id,other,repoint.id)?;
        let clash=thread(tx,room,"Clash")?;PullRequestThread::create(tx,loser.id,room,clash.id)?;
        let kept=thread(tx,room,"Kept")?;PullRequestThread::create(tx,winner.id,room,kept.id)?;
        collapse_case_duplicates(tx)?;
        assert!(PullRequest::find(tx.conn(),loser.id).is_err());assert_eq!(PullRequest::find(tx.conn(),solo.id)?.owner,"rails");
        for message in [a,b] {assert_eq!(PullRequest::for_message(tx.conn(),message.id)?.iter().map(|p|p.id).collect::<Vec<_>>(),vec![winner.id]);}
        assert_eq!(PullRequestThread::for_room_pr(tx.conn(),other,winner.id)?.unwrap().channel_thread_id,repoint.id);
        assert_eq!(PullRequestThread::for_room_pr(tx.conn(),room,winner.id)?.unwrap().channel_thread_id,kept.id);
        assert!(ChannelThread::find_by_id(tx.conn(),clash.id)?.is_some());Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_pr_thread_uniqueness_reuse_cleanup_and_unrelated_errors_match_rails() {
    let (fixture, _, _) = database().await;
    fixture.db.write(|tx| {
        let room=fixtures::identify("designers");let other=fixtures::identify("watercooler");let pr=PullRequest::for_reference(tx,"rails","rails",12)?;let other_pr=PullRequest::for_reference(tx,"rails","rails",13)?;
        let winner_thread=thread(tx,room,"Winner")?;let winner=PullRequestThread::create_or_reuse(tx,pr.id,room,winner_thread.id)?;
        let loser=thread(tx,room,"Loser")?;
        let errors=PullRequestThread::validate(tx.conn(),pr.id,room,loser.id)?;assert_eq!(errors.on("github_pull_request_id"),vec!["has already been taken"]);
        let reused=PullRequestThread::create_or_reuse(tx,pr.id,room,loser.id)?;assert_eq!(winner.id,reused.id);assert!(ChannelThread::find_by_id(tx.conn(),loser.id)?.is_none());
        let indexed_loser=thread(tx,room,"Index race")?;
        let index_error=campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),None));
        assert_eq!(PullRequestThread::recover_creation(tx,pr.id,room,indexed_loser.id,Err(index_error))?.id,winner.id);
        assert!(ChannelThread::find_by_id(tx.conn(),indexed_loser.id)?.is_none());
        let occupied=thread(tx,room,"Occupied")?;PullRequestThread::create(tx,other_pr.id,room,occupied.id)?;
        let combined=PullRequestThread::create_or_reuse(tx,pr.id,room,occupied.id).unwrap_err();assert!(matches!(combined,campfire_db::Error::RecordInvalid(e) if e.0.len()==2));assert!(ChannelThread::find_by_id(tx.conn(),occupied.id)?.is_some());
        assert!(PullRequestThread::create_or_reuse(tx,other_pr.id,room,winner_thread.id).is_err());assert!(ChannelThread::find_by_id(tx.conn(),winner_thread.id)?.is_some());
        let other_thread=thread(tx,other,"Other room")?;let cross=PullRequestThread::create(tx,pr.id,other,other_thread.id)?;assert_eq!(cross.pull_request_id,pr.id);
        let indexed=tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (?,?,?,?,?)",params![pr.id,room,other_thread.id,tx.now(),tx.now()]);assert!(indexed.is_err());
        other_thread.destroy(tx)?;assert!(PullRequestThread::for_room_pr(tx.conn(),other,pr.id)?.is_none());Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_subscriptions_validation_and_bot_membership_callbacks_match_rails() {
    let (fixture, _, _) = database().await;
    for case in vectors()["subscriptions"].as_array().unwrap() {
        let case = case.clone();
        fixture
            .db
            .write(move |tx| {
                let room = fixtures::identify("designers");
                let a = &case["attributes"];
                let result = if case["create"] == true {
                    RepositorySubscription::create(
                        tx,
                        room,
                        a["owner"].as_str().unwrap(),
                        a["repo"].as_str().unwrap(),
                        a["events"].clone(),
                        None,
                        false,
                    )
                } else {
                    let mut sub = RepositorySubscription::create(
                        tx,
                        room,
                        "rails",
                        "rails",
                        Value::Null,
                        None,
                        false,
                    )?;
                    let result = sub
                        .update(
                            tx,
                            a["owner"].as_str().unwrap(),
                            a["repo"].as_str().unwrap(),
                            a["events"].clone(),
                            false,
                        )
                        .map(|_| sub.clone());
                    if result.is_err() {
                        assert_eq!(
                            RepositorySubscription::find(tx.conn(), sub.id)?.events,
                            json!(subscriptions::DEFAULT_EVENTS)
                        );
                    }
                    sub.destroy(tx)?;
                    result
                };
                assert_eq!(result.is_ok(), case["valid"], "{case}");
                match result {
                    Ok(sub) => {
                        assert_eq!(sub.owner, case["owner"]);
                        assert_eq!(sub.repo, case["repo"]);
                        assert_eq!(sub.events, case["events"]);
                        if case["create"] == true {
                            sub.destroy(tx)?;
                        }
                    }
                    Err(e) => assert_eq!(errors(Some(e)), case["errors"], "{case}"),
                };
                Ok(())
            })
            .await
            .unwrap();
    }
    fixture
        .db
        .write(|tx| {
            let room = fixtures::identify("designers");
            let other = fixtures::identify("watercooler");
            let first = RepositorySubscription::create(
                tx,
                room,
                "Rails",
                "Rails",
                Value::Null,
                Some(fixtures::identify("david")),
                false,
            )?;
            assert_eq!(first.events, json!(subscriptions::DEFAULT_EVENTS));
            assert!(first.subscribed_to("opened"));
            assert!(!first.subscribed_to("closed"));
            assert!(
                RepositorySubscription::create(tx, room, "RAILS", "rails", json!([]), None, false)
                    .is_err()
            );
            let cross = RepositorySubscription::create(
                tx,
                other,
                "rails",
                "rails",
                Value::Null,
                None,
                false,
            )?;
            assert!(
                RepositorySubscription::create(
                    tx,
                    fixtures::identify("david_and_jason"),
                    "o",
                    "r",
                    Value::Null,
                    None,
                    false
                )
                .is_err()
            );
            let second = RepositorySubscription::create(
                tx,
                room,
                "rails",
                "propshaft",
                Value::Null,
                None,
                false,
            )?;
            let bot = super::super::notifier::bot_user(tx)?;
            let user = campfire_db::User::find(tx.conn(), bot)?;
            assert!(user.bot_token_digest.is_some());
            assert!(!tx.conn().query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE user_id=?)",
                [bot],
                |r| r.get::<_, bool>(0)
            )?);
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM memberships WHERE user_id=?",
                    [bot],
                    |r| r.get::<_, i64>(0)
                )?,
                2
            );
            first.destroy(tx)?;
            assert!(
                campfire_db::Membership::find_by_room_and_user(tx.conn(), room, bot)?.is_some()
            );
            second.destroy(tx)?;
            assert!(
                campfire_db::Membership::find_by_room_and_user(tx.conn(), room, bot)?.is_none()
            );
            cross.destroy(tx)?;
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM memberships WHERE user_id=?",
                    [bot],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn github_notification_claims_validate_and_share_one_concurrent_winner_per_subscription() {
    let (fixture, _, _) = database().await;
    let (a, b) = fixture
        .db
        .write(|tx| {
            Ok((
                RepositorySubscription::create(
                    tx,
                    fixtures::identify("designers"),
                    "o",
                    "r",
                    Value::Null,
                    None,
                    false,
                )?
                .id,
                RepositorySubscription::create(
                    tx,
                    fixtures::identify("watercooler"),
                    "o",
                    "r",
                    Value::Null,
                    None,
                    false,
                )?
                .id,
            ))
        })
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let db = fixture.db.clone();
        tasks.push(tokio::spawn(async move {
            db.write(move |tx| subscriptions::claim_notification(tx, a, "opened:o/r#12"))
                .await
                .unwrap()
        }));
    }
    let mut winners = 0;
    for task in tasks {
        winners += usize::from(task.await.unwrap().is_some());
    }
    assert_eq!(winners, 1);
    fixture
        .db
        .write(move |tx| {
            assert!(subscriptions::claim_notification(tx, b, "opened:o/r#12")?.is_some());
            assert!(subscriptions::claim_notification(tx, a, "")?.is_none());
            assert!(subscriptions::claim_notification(tx, 9999, "opened")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn github_card_cardset_header_and_files_html_are_byte_identical_to_rails() {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../../../vectors/github_cards.json")).unwrap();
    let (app, _dir) = super::super::references::tests::application().await;
    app.db.write(|tx| {
        let now=tx.now();tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (811,'Oracle',1,?,?)",params![now,now])?;
        tx.conn().execute("INSERT INTO rooms (id,type,creator_id,name,created_at,updated_at) VALUES (815,'Rooms::Closed',811,'Cards',?,?)",params![now,now])?;
        tx.conn().execute("INSERT INTO messages (id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES (818,815,811,'card-parent',?,?)",params![now,now])?;
        tx.conn().execute("INSERT INTO channel_threads (id,room_id,creator_id,parent_message_id,name,last_activity_at,created_at,updated_at) VALUES (817,815,811,818,'Discussion',?,?,?)",params![now,now,now])?;Ok(())
    }).await.unwrap();
    for case in vectors.as_array().unwrap() {
        let case = case.clone();
        let expected = case.clone();
        let app_read = app.clone();
        app.db.write(move|tx| {
            tx.conn().execute_batch("DELETE FROM github_pull_request_references; DELETE FROM github_pull_request_threads; DELETE FROM github_pull_requests;")?;
            let mut columns=vec!["created_at","updated_at"];let mut values=vec![SqlValue::Text(tx.now().to_db()),SqlValue::Text(tx.now().to_db())];
            for (k,v) in case["attributes"].as_object().unwrap() {columns.push(k);values.push(match v {Value::Null=>SqlValue::Null,Value::Bool(b)=>SqlValue::Integer(i64::from(*b)),Value::Number(n)=>SqlValue::Integer(n.as_i64().unwrap()),Value::String(s)=>SqlValue::Text(if k.ends_with("_at"){s.replace(" UTC","Z").parse::<jiff::Timestamp>().unwrap().strftime("%Y-%m-%d %H:%M:%S.%6f").to_string()}else{s.clone()}),_=>SqlValue::Text(v.to_string())});}
            tx.conn().execute(&format!("INSERT INTO github_pull_requests ({}) VALUES ({})",columns.join(","),vec!["?";columns.len()].join(",")),rusqlite::params_from_iter(values))?;
            tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id,message_id,created_at,updated_at) VALUES (816,818,?,?)",params![tx.now(),tx.now()])?;
            if case["mapped"]==true{PullRequestThread::create(tx,816,815,817)?;}
            tx.conn().execute("UPDATE messages SET thread_id=? WHERE id=818",[if case["reply"]==true{Some(817)}else{None}])?;Ok(())
        }).await.unwrap();
        app.db
            .read(move |conn| {
                let pr = PullRequest::find(conn, 816)?;
                let data = crate::controllers::presenters::github::card_with_files(conn, &pr, 815)?;
                let message = Message::find(conn, 818)?;
                let account = campfire_db::Account::first(conn)?;
                crate::controllers::presenters::page::render_detached(
                    &app_read,
                    account.as_ref(),
                    |ctx| {
                        let cm = campfire_views::github::CardMessage {
                            id: 818,
                            room_id: 815,
                            thread_id: message.thread_id,
                        };
                        for (key, actual) in [
                            ("card", campfire_views::github::card(ctx, &data, Some(&cm))),
                            (
                                "cards",
                                campfire_views::github::cards(
                                    ctx,
                                    &message.client_message_id,
                                    &cm,
                                    std::slice::from_ref(&data),
                                ),
                            ),
                            (
                                "header",
                                campfire_views::github::thread_header(ctx, 815, 817, &data),
                            ),
                            ("files", campfire_views::github::files_summary(&data)),
                        ] {
                            assert_eq!(actual, expected[key], "{} {key}", expected["name"]);
                            if key == "cards" || key == "header" {
                                assert!(campfire_cable::turbo::session_bound(&actual).is_none());
                                if pr.private != Some(false) {
                                    assert!(!actual.contains("Secret title"));
                                    assert!(actual.contains("turbo-frame"));
                                }
                            }
                        }
                    },
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn github_pr_agent_payload_security_hides_private_and_unknown_details_without_owner_access() {
    use super::super::{
        accounts::AccountInput,
        client::AppClient,
        tests::{crypto, fake},
    };
    use crate::integrations::test_support::Route;
    let (fixture, _, _) = database().await;
    let (server, network) = fake(vec![
        Route::new("GET", "api.github.com", "/repos/rails/rails", 200).body("{}"),
    ])
    .await;
    let accounts = Accounts::with_network(
        fixture.db.clone(),
        crypto(),
        AppClient::with_network(None, None, network.clone()),
        network,
    );
    let (pr,reply,parent)=fixture.db.write(|tx| {
        let room=fixtures::identify("designers");let pr=PullRequest::for_reference(tx,"rails","rails",12)?;
        let pr=update(tx,pr.id,&[("title",SqlValue::Text("Secret title".into())),("head_branch",SqlValue::Text("shiny".into())),("base_branch",SqlValue::Text("main".into())),("state",SqlValue::Text("open".into())),("review_decision",SqlValue::Text("approved".into())),("check_status",SqlValue::Text("passing".into()))])?;
        let mut thread=thread(tx,room,"Payload")?;PullRequestThread::create(tx,pr.id,room,thread.id)?;
        let parent=Message::find(tx.conn(),thread.parent_message_id.unwrap())?;
        let reply=thread.post_message(tx,fixtures::identify("david"),NewMessage {markdown_source:Some("looks good".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO agents (id,user_id,owner_id,created_at,updated_at) VALUES (810,?,?,?,?)",params![fixtures::identify("kevin"),fixtures::identify("david"),tx.now(),tx.now()])?;Ok((pr,reply,parent))
    }).await.unwrap();
    for private in [None, Some(true)] {
        let mut candidate = pr.clone();
        candidate.private = private;
        let p = candidate
            .agent_payload(&fixture.db, &accounts, None)
            .await
            .unwrap();
        for key in ["title", "head_branch", "base_branch"] {
            assert!(p[key].is_null());
        }
        assert_eq!(p["url"], "https://github.com/rails/rails/pull/12");
        assert_eq!(p["checks_state"], "passing");
        assert!(
            !candidate
                .visible_to(&fixture.db, &accounts, Some(fixtures::identify("david")))
                .await
                .unwrap()
        );
    }
    let mut public = pr.clone();
    public.private = Some(false);
    let p = public
        .agent_payload(&fixture.db, &accounts, None)
        .await
        .unwrap();
    assert_eq!(p["title"], "Secret title");
    assert!(server.received().is_empty());
    assert_eq!(p,json!({"url":"https://github.com/rails/rails/pull/12","owner":"rails","repo":"rails","number":12,"title":"Secret title","state":"open","head_branch":"shiny","base_branch":"main","review_decision":"approved","checks_state":"passing"}));
    let encryption = crypto();
    fixture
        .db
        .write(move |tx| {
            Account::create(
                tx,
                &encryption,
                &AccountInput {
                    user_id: fixtures::identify("david"),
                    github_login: "david",
                    access_token: "fixture-owner-token",
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let p = payload_for_message(&fixture.db, &accounts, &reply, Some(810))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p["title"], "Secret title");
    assert_eq!(p["head_branch"], "shiny");
    assert_eq!(p["base_branch"], "main");
    assert_eq!(p["checks_state"], "passing");
    assert!(
        payload_for_message(&fixture.db, &accounts, &parent, None)
            .await
            .unwrap()
            .is_none()
    );
    let ordinary=fixture.db.write(|tx|{let mut t=thread(tx,fixtures::identify("designers"),"Ordinary")?;t.post_message(tx,fixtures::identify("david"),NewMessage {markdown_source:Some("hello".into()),..Default::default()})}).await.unwrap();
    assert!(payload_for_message(&fixture.db,&accounts,&ordinary,None).await.unwrap().is_none());
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn github_pr_commit_callbacks_coalesce_and_refresh_queue_failure_rolls_back_save() {
    let (fixture,_,sink)=database().await;
    fixture.db.write(|tx| {let pr=PullRequest::for_reference(tx,"o","r",1)?;update(tx,pr.id,&[("title",SqlValue::Text("First".into()))])?;update(tx,pr.id,&[("title",SqlValue::Text("Last".into()))])?;Ok(())}).await.unwrap();
    assert_eq!(sink.take().iter().filter(|e|matches!(e,Event::Broadcast(b) if b.kind==CardUpdated::KIND)).count(),1);
    let (app,_dir)=super::super::references::tests::application().await;
    let pr=app.db.write(|tx|{let m=message(tx,fixtures::identify("designers"),"https://github.com/o/r/pull/2")?;let pr=PullRequest::for_message(tx.conn(),m.id)?.remove(0);tx.conn().execute("UPDATE github_pull_requests SET fetch_requested_at=NULL WHERE id=?",[pr.id])?;tx.conn().execute_batch("CREATE TRIGGER reject_card_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue down'); END;")?;Ok(pr)}).await.unwrap();
    app.db.write(move|tx|update(tx,pr.id,&[("private",SqlValue::Integer(0)),("title",SqlValue::Text("Rejected with failed render refresh".into()))])).await.unwrap_err();
    app.db.read(move|conn|{let stored=PullRequest::find(conn,pr.id)?;assert!(stored.fetch_requested_at.is_none());assert_eq!(stored.title,None);assert_eq!(stored.private,None);Ok(())}).await.unwrap();
}

#[tokio::test]
async fn github_pr_thread_concurrent_losers_leave_one_mapping_and_one_thread() {
    let (fixture, _, _) = database().await;
    let pr = fixture
        .db
        .write(|tx| PullRequest::for_reference(tx, "o", "r", 12))
        .await
        .unwrap();
    let room = fixtures::identify("designers");
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let db = fixture.db.clone();
        tasks.push(tokio::spawn(async move {
            db.write(move |tx| {
                let provisional = thread(tx, room, "Race")?;
                PullRequestThread::create_or_reuse(tx, pr.id, room, provisional.id)
            })
            .await
            .unwrap()
        }));
    }
    let mut ids = Vec::new();
    for task in tasks {
        ids.push(task.await.unwrap().channel_thread_id);
    }
    assert!(ids.iter().all(|id| *id == ids[0]));
    fixture
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM channel_threads WHERE name='Race'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn github_pr_registered_card_callbacks_publish_public_and_private_room_and_thread_replacements()
 {
    use campfire_kit::Crypto;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{Message as WsMessage, client::IntoClientRequest};
    let (app, _dir) = super::super::references::tests::application().await;
    let (pr, message, thread, session, room) = app
        .db
        .write(|tx| {
            let room = campfire_db::Room::find(tx.conn(), fixtures::identify("designers"))?;
            let message = message(tx, room.id, "https://github.com/rails/rails/pull/12")?;
            let pr = PullRequest::for_message(tx.conn(), message.id)?.remove(0);
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: fixtures::identify("david"),
                    parent_message_id: Some(message.id),
                    name: Some("Cards".into()),
                    ..Default::default()
                },
            )?;
            PullRequestThread::create(tx, pr.id, room.id, thread.id)?;
            let session = campfire_db::Session::start_with(
                tx,
                fixtures::identify("david"),
                campfire_db::NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )?;
            Ok((pr, message, thread, session, room))
        })
        .await
        .unwrap();
    let mut listener = None;
    for port in 51500..=51549 {
        if let Ok(l) = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
            listener = Some(l);
            break;
        }
    }
    let listener = listener.unwrap();
    let address = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let cookie = campfire_kit::RailsCrypto::new(app.secrets.clone())
        .sign_cookie("session_token", &session.token, None)
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D");
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", format!("session_token={cookie}").parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    async fn frame(
        socket: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Value {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let next = tokio::time::timeout_at(deadline, socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let WsMessage::Text(text) = next {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] != "ping" {
                    return value;
                }
            }
        }
    }
    assert_eq!(frame(&mut socket).await["type"], "welcome");
    let other_room = app.db.read(|c| campfire_db::Room::find(c, fixtures::identify("watercooler"))).await.unwrap();
    let mut identifiers = Vec::new();
    for gid in [
        crate::channels::room_gid(&room),
        crate::channels::threads::thread_gid(thread.id),
        crate::channels::room_gid(&other_room),
    ] {
        let identifier=json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&app.secrets,&[&gid.to_param(),"messages"])}).to_string();
        socket
            .send(WsMessage::Text(
                json!({"command":"subscribe","identifier":identifier})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert_eq!(frame(&mut socket).await["type"], "confirm_subscription");
        identifiers.push(identifier);
    }
    let publications = app.cable.capture_publications();
    for private in [false, true] {
        app.db
            .write(move |tx| {
                update(
                    tx,
                    pr.id,
                    &[
                        (
                            "title",
                            SqlValue::Text(
                                if private {
                                    "Secret title"
                                } else {
                                    "Public title"
                                }
                                .into(),
                            ),
                        ),
                        ("private", SqlValue::Integer(i64::from(private))),
                        ("changed_files", SqlValue::Text(json!({"files":[{"filename":"app/a.rb","status":"modified","additions":4,"deletions":2}],"total_count":3}).to_string())),
                    ],
                )
            })
            .await
            .unwrap();
        let mut seen = Vec::new();
        for _ in 0..2 {
            let f = frame(&mut socket).await;
            let html = f["message"].as_str().unwrap();
            assert!(html.contains("action=\"replace\""));
            assert!(html.contains("maintain_scroll=\"true\""));
            assert!(campfire_cable::turbo::session_bound(html).is_none());
            if private {
                assert!(!html.contains("Secret title"));
                assert!(html.contains("turbo-frame"));
            } else {
                assert!(html.contains("Public title"));
            }
            let identifier = f["identifier"].as_str().unwrap();
            assert!(identifiers.iter().any(|id| id == identifier));
            let target = if identifier == identifiers[0] {
                crate::channels::broadcasts::message_dom_id(&message, Some("github_pr_cards"))
            } else {
                {
                    let cards:Value=serde_json::from_str(include_str!("../../../../../../vectors/github_cards.json")).unwrap();
                    let expected=cards.as_array().unwrap().iter().find(|c|c["name"]=="public").unwrap()["files"].as_str().unwrap();
                    if private {assert!(!html.contains("app/a.rb"));} else {assert!(html.contains(expected),"actual thread cable replacement discarded pinned file summary: {html}");}
                    crate::channels::broadcasts::thread_dom_id(thread.id, "github_pr_header")
                }
            };
            assert!(html.contains(&format!("target=\"{target}\"")));
            seen.push(identifier.to_owned());
        }
        assert_ne!(seen[0], seen[1]);
        let published = publications.take();
        assert_eq!(published.len(), 2, "only the referencing room and mapped thread may receive updates");
        let other_stream = format!("{}:messages", crate::channels::room_gid(&other_room).to_param());
        assert!(published.iter().all(|(stream, _)| stream != &other_stream));
    }
    socket.close(None).await.unwrap();
    serving.abort();
}

#[tokio::test]
async fn github_pr_and_thread_stamps_invalidate_message_fragments_without_touching_message() {
    let (app, _dir) = super::super::references::tests::application().await;
    let message = app
        .db
        .write(|tx| {
            let m = message(
                tx,
                fixtures::identify("designers"),
                "https://github.com/o/r/pull/12",
            )?;
            let pr = PullRequest::for_message(tx.conn(), m.id)?.remove(0);
            update(
                tx,
                pr.id,
                &[
                    ("private", SqlValue::Integer(0)),
                    ("title", SqlValue::Text("Initial".into())),
                ],
            )?;
            Ok(m)
        })
        .await
        .unwrap();
    let app_read = app.clone();
    let m = message.clone();
    let first = app
        .db
        .read(move |conn| {
            let view = crate::controllers::presenters::Presenter::new(conn, &app_read, None)
                .message(&m)?;
            let account = campfire_db::Account::first(conn)?;
            Ok(crate::controllers::presenters::page::render_detached(
                &app_read,
                account.as_ref(),
                |ctx| campfire_views::messages::message(ctx, &view),
            ))
        })
        .await
        .unwrap();
    assert!(first.contains("Initial"));
    let m = message.clone();
    app.db
        .write(move |tx| {
            let pr = PullRequest::for_message(tx.conn(), m.id)?.remove(0);
            tx.conn().execute(
                "UPDATE github_pull_requests SET title='Updated',updated_at=? WHERE id=?",
                params![tx.now().since(jiff::SignedDuration::from_secs(1)), pr.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let app_read = app.clone();
    app.db
        .read(move |conn| {
            let stored = Message::find(conn, message.id)?;
            assert_eq!(stored.updated_at, message.updated_at);
            let view = crate::controllers::presenters::Presenter::new(conn, &app_read, None)
                .message(&stored)?;
            let account = campfire_db::Account::first(conn)?;
            let html = crate::controllers::presenters::page::render_detached(
                &app_read,
                account.as_ref(),
                |ctx| campfire_views::messages::message(ctx, &view),
            );
            assert!(html.contains("Updated"));
            assert!(!html.contains("Initial"));
            Ok(())
        })
        .await
        .unwrap();
}


#[tokio::test]
async fn review_refresh_queue_is_atomic_with_triggering_save() {
    let (app,_dir)=super::super::references::tests::application().await;
    let pr=app.db.write(|tx| {
        let m=message(tx,fixtures::identify("designers"),"https://github.com/review/atomic/pull/2")?;
        let pr=PullRequest::for_message(tx.conn(),m.id)?.remove(0);
        tx.conn().execute("UPDATE github_pull_requests SET fetch_requested_at=NULL,title='Before' WHERE id=?",[pr.id])?;
        tx.conn().execute_batch("DELETE FROM background_jobs; CREATE TRIGGER reject_review_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'review queue failure'); END;")?;
        super::super::tests::enqueue_retention_job(tx)?;
        Ok(pr)
    }).await.unwrap();
    let result=app.db.write(move|tx|update(tx,pr.id,&[("title",SqlValue::Text("After".into()))])).await;
    assert!(result.is_err(),"queue insertion failure must roll back the triggering PR save");
    app.db.read(move|conn|{let stored=PullRequest::find(conn,pr.id)?;assert_eq!(stored.title.as_deref(),Some("Before"));assert!(stored.fetch_requested_at.is_none());let jobs:i64=conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class GLOB 'Github::*'",[],|r|r.get(0))?;assert_eq!(jobs,0);Ok(())}).await.unwrap();
}

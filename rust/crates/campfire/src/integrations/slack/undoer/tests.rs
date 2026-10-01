use super::super::client::tests::fake;
use super::super::jobs::perform_undo;
use super::super::jobs::tests::{run, setup, start};
use super::super::store::tests::{import, routes};
use super::super::users;
use super::*;
use campfire_db::Timestamp;

pub(crate) async fn undo(db: &Database, id: i64) -> SlackImport {
    assert!(db.write(move |tx| SlackImport::undo(tx, id)).await.unwrap());
    for _ in 0..100 {
        let worker_db = db.clone();
        perform_undo(db.clone(), id, move |_, lease| async move {
            Undoer {
                db: worker_db,
                id,
                lease,
            }
            .step()
            .await
        })
        .await
        .unwrap();
        let row = run(db, id).await;
        if row.status == "undone" {
            return row;
        }
        assert_eq!(row.status, "undoing");
    }
    panic!("undo did not finish");
}
async fn imported() -> (
    Database,
    std::sync::Arc<rails_compat::ar_encryption::ArEncryption>,
    tempfile::TempDir,
    i64,
) {
    let (db, crypto, dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        Ok(())
    })
    .await
    .unwrap();
    let (_server, network) = fake(routes(false)).await;
    import(&db, crypto.clone(), id, network).await;
    (db, crypto, dir, id)
}
#[tokio::test]
async fn slack_undo_deletes_data_search_and_mappings_then_reimports_fixture() {
    let (db, crypto, _dir, id) = imported().await;
    let before = run(&db, id).await;
    let row = undo(&db, id).await;
    assert_eq!(row.state, json!({"phase":"done"}));
    db.read(|c| {
        for table in [
            "rooms",
            "messages",
            "channel_threads",
            "memberships",
            "boosts",
            "message_pins",
            "thread_memberships",
            "slack_import_records",
            "message_search_index",
            "action_text_rich_texts",
            "message_references",
            "link_embed_references",
        ] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))?,
                0,
                "{table}"
            );
        }
        assert_eq!(User::count(c)?, 1);
        Ok(())
    })
    .await
    .unwrap();
    let again = start(&db).await;
    let (_server, network) = fake(routes(false)).await;
    let after = import(&db, crypto, again, network).await;
    assert_eq!(before.stats["counts"], after.stats["counts"]);
    assert_eq!(before.stats["users"], after.stats["users"]);
}
#[tokio::test]
async fn slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies() {
    for scheduled in [false, true] {
        let (db, _, _dir, id) = imported().await;
        let (thread_id,parent,room)=db.write(move |tx|{
            let thread_id=users::mapped_id(tx.conn(),1,"thread","CCHAN:1700000002.000002")?.unwrap();
            let thread=ChannelThread::find(tx.conn(),thread_id)?;
            let parent=thread.parent_message_id.unwrap();
            if scheduled {
                tx.conn().execute("INSERT INTO scheduled_messages (markdown_source,reply_to_message_id,thread_id,room_id,user_id,send_at,created_at,updated_at) VALUES ('pending',?,?,?,?,?,?,?)",params![parent,thread.id,thread.room_id,1,tx.now(),tx.now(),tx.now()])?;
            } else {
                Message::create_imported(tx,campfire_db::NewMessage{room_id:thread.room_id,creator_id:1,thread_id:Some(thread.id),markdown_source:Some("foreign reply".into()),..Default::default()},tx.now(),None)?;
            }
            Ok((thread.id,parent,thread.room_id))
        }).await.unwrap();
        undo(&db, id).await;
        db.read(move |c| {
            assert!(ChannelThread::find_by_id(c, thread_id)?.is_some());
            assert!(Message::find_by_id(c, parent)?.is_some());
            assert!(Room::find_by_id(c, room)?.is_some());
            assert!(users::mapped_id(c, 1, "message", "CCHAN:1700000002.000002")?.is_some());
            let warnings: i64 = c.query_row(
                "SELECT count(*) FROM slack_import_issues WHERE slack_ref=?",
                [format!("thread:{thread_id}")],
                |r| r.get(0),
            )?;
            assert_eq!(warnings, 1);
            let memberships: i64 = c.query_row(
                "SELECT count(*) FROM memberships WHERE room_id=?",
                [room],
                |r| r.get(0),
            )?;
            assert!(memberships > 0);
            Ok(())
        })
        .await
        .unwrap();
    }
}
#[tokio::test]
async fn slack_undo_keeps_poll_saved_item_foreign_pin_and_pending_root_reply() {
    for kind in ["poll", "saved", "pin", "scheduled"] {
        let (db, crypto, _dir, id) = imported().await;
        let (message,room)=db.write(move |tx|{
            let message=users::mapped_id(tx.conn(),1,"message","CCHAN:1700000007.000007")?.unwrap();
            let room=Message::find(tx.conn(),message)?.room_id;
            match kind {
                "poll"=>{tx.conn().execute("INSERT INTO polls (message_id,created_at,updated_at) VALUES (?,?,?)",params![message,tx.now(),tx.now()])?;},
                "saved"=>{tx.conn().execute("INSERT INTO saved_items (message_id,user_id,created_at,updated_at) VALUES (?,1,?,?)",params![message,tx.now(),tx.now()])?;},
                "pin"=>{tx.conn().execute("INSERT INTO message_pins (message_id,room_id,pinner_id,created_at,updated_at) VALUES (?,?,1,?,?)",params![message,room,tx.now(),tx.now()])?;},
                _=>{tx.conn().execute("INSERT INTO scheduled_messages (markdown_source,reply_to_message_id,room_id,user_id,send_at,created_at,updated_at) VALUES ('pending',?,?,1,?,?,?)",params![message,room,tx.now(),tx.now(),tx.now()])?;},
            } Ok((message,room))
        }).await.unwrap();
        undo(&db, id).await;
        db.read(move |c| {
            assert!(Message::find_by_id(c, message)?.is_some());
            assert!(Room::find_by_id(c, room)?.is_some());
            assert_eq!(
                users::mapped_id(c, 1, "message", "CCHAN:1700000007.000007")?,
                Some(message)
            );
            Ok(())
        })
        .await
        .unwrap();
        let again = start(&db).await;
        let (_server, network) = fake(routes(false)).await;
        import(&db, crypto, again, network).await;
        db.read(move |c| {
            assert_eq!(
                users::mapped_id(c, 1, "message", "CCHAN:1700000007.000007")?,
                Some(message)
            );
            Ok(())
        })
        .await
        .unwrap();
    }
}
#[tokio::test]
async fn slack_undo_keeps_claimed_placeholders_and_their_mapping() {
    let (db, _, _dir, id) = imported().await;
    let user = db
        .write(|tx| {
            let user = users::mapped_id(tx.conn(), 1, "user", "U003")?.unwrap();
            tx.conn().execute(
                "UPDATE users SET password_digest='claimed' WHERE id=?",
                [user],
            )?;
            Ok(user)
        })
        .await
        .unwrap();
    undo(&db, id).await;
    db.read(move |c| {
        assert!(User::find_by_id(c, user)?.is_some());
        assert_eq!(users::mapped_id(c, 1, "user", "U003")?, Some(user));
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_undo_failed_destroy_rolls_back_model_and_records_issue_before_continuing() {
    let (db, _, _dir, id) = imported().await;
    let message=db.write(|tx|{let id=users::mapped_id(tx.conn(),1,"message","CCHAN:1700000006.000006")?.unwrap();tx.conn().execute_batch(&format!("CREATE TRIGGER ws16_reject_delete BEFORE DELETE ON messages WHEN OLD.id={id} BEGIN SELECT RAISE(ABORT,'fixture deletion rejected'); END"))?;Ok(id)}).await.unwrap();
    let row = undo(&db, id).await;
    assert!(integer(&row.stats["issues_count"]) > 0);
    db.read(move |c| {
        let message = Message::find(c, message)?;
        assert!(message.body_html(c)?.is_some());
        assert!(Room::find_by_id(c, message.room_id)?.is_some());
        let errors: i64 = c.query_row(
            "SELECT count(*) FROM slack_import_issues WHERE level='error' AND slack_ref=?",
            [format!("message:{}", message.id)],
            |r| r.get(0),
        )?;
        assert_eq!(errors, 1);
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_undo_batch_cursor_and_lease_block_duplicate_execution() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let lease = db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE slack_imports SET status='completed' WHERE id=?",
                [id],
            )?;
            SlackImport::undo(tx, id)?;
            for n in 0..201 {
                users::record(
                    tx,
                    &SlackImport::find(tx.conn(), id)?.unwrap(),
                    "reaction",
                    &format!("fake:{n}"),
                    "Boost",
                    1000 + n,
                    true,
                )?;
            }
            Ok(SlackImport::acquire_step_lease(
                tx,
                id,
                campfire_db::models::slack_import::StepStatus::Undoing,
            )?
            .unwrap())
        })
        .await
        .unwrap();
    assert!(matches!(
        Undoer {
            db: db.clone(),
            id,
            lease: "wrong".into()
        }
        .step()
        .await
        .unwrap(),
        Outcome::Stopped
    ));
    let count = |state: Value| integer(&state["undo_cursor"]);
    assert!(matches!(
        Undoer {
            db: db.clone(),
            id,
            lease: lease.clone()
        }
        .step()
        .await
        .unwrap(),
        Outcome::Continue
    ));
    assert_eq!(count(run(&db, id).await.state), 200);
    Undoer {
        db: db.clone(),
        id,
        lease: lease.clone(),
    }
    .step()
    .await
    .unwrap();
    assert_eq!(count(run(&db, id).await.state), 201);
    Undoer {
        db: db.clone(),
        id,
        lease,
    }
    .step()
    .await
    .unwrap();
    assert_eq!(run(&db, id).await.state["undo_step"], "messages");
}

#[tokio::test]
async fn slack_undo_actual_overlapping_imports_require_lifo_and_name_later_importer() {
    let (db, crypto, _dir, first) = imported().await;
    let before_messages = db.read(Message::count).await.unwrap();
    let connection_crypto = crypto.clone();
    let second = db
        .write(move |tx| {
            let actor = User::create(
                tx,
                campfire_db::NewUser {
                    name: "Later importer".into(),
                    role: campfire_db::user::Role::Administrator,
                    ..Default::default()
                },
            )?;
            let connection = campfire_db::models::slack::SlackConnection::create(
                tx,
                &connection_crypto,
                campfire_db::models::slack::NewConnection {
                    workspace_id: 1,
                    user_id: actor.id,
                    slack_user_id: "UOTHER",
                    access_token: Some("fixture-user-token"),
                    scopes: None,
                },
            )?;
            Ok(SlackImport::create(
                tx,
                campfire_db::models::slack_import::NewImport {
                    workspace_id: 1,
                    connection_id: Some(connection.id),
                    user_id: actor.id,
                    kind: campfire_db::models::slack_import::Kind::Workspace,
                    mode: campfire_db::models::slack_import::Mode::Import,
                    options: json!({}),
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let oldests=db.read(|c| {
        let mut values=std::collections::HashMap::new();
        for channel in ["CARCH","CCHAN","CPRIV"] {
            let latest:f64=c.query_row("SELECT MAX(CAST(substr(slack_key,?) AS REAL)) FROM slack_import_records WHERE slack_kind='message' AND slack_key>=? AND slack_key<?",params![channel.len()+2,format!("{channel}:"),format!("{channel};")],|r|r.get(0))?;
            values.insert(channel.to_owned(),format!("{:.6}",latest-super::super::runner::CATCHUP_LOOKBACK as f64));
        } Ok(values)
    }).await.unwrap();
    let mut catchup_routes = routes(false);
    for route in &mut catchup_routes {
        if route.path.starts_with("/api/conversations.history?")
            || route.path.starts_with("/api/conversations.replies?")
        {
            let url = url::Url::parse(&format!("https://slack.com{}", route.path)).unwrap();
            let channel = url.query_pairs().find(|(k, _)| k == "channel").unwrap().1;
            if let Some(oldest) = oldests.get(channel.as_ref()) {
                let mut pairs = url
                    .query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect::<Vec<_>>();
                pairs.insert(
                    if route.path.starts_with("/api/conversations.replies?") {
                        2
                    } else {
                        1
                    },
                    ("oldest".into(), oldest.clone()),
                );
                let query = url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(pairs)
                    .finish();
                route.path = format!("{}?{query}", url.path());
            }
        }
    }
    let (server, network) = fake(catchup_routes).await;
    let later = import(&db, crypto, second, network).await;
    assert_eq!(later.stats["counts"]["messages"], 0);
    assert_eq!(later.stats["counts"]["replies"], 0);
    assert_eq!(db.read(Message::count).await.unwrap(), before_messages);
    assert!(
        server
            .received()
            .iter()
            .any(|request| request.target.contains("&oldest="))
    );
    assert!(
        !db.write(move |tx| SlackImport::undo(tx, first))
            .await
            .unwrap()
    );
    let reason = db
        .read(move |c| {
            SlackImport::find(c, first)?.unwrap().undo_blocked_reason(
                c,
                Timestamp::parse_db("2026-03-02 16:00:00.123456").unwrap(),
            )
        })
        .await
        .unwrap()
        .unwrap();
    assert!(
        reason
            == "A later import by Later importer also imported some of these conversations. It has to be undone first; ask them or an administrator.",
        "{reason}"
    );
    undo(&db, second).await;
    db.read(|c| {
        assert!(!Room::all(c)?.is_empty());
        assert!(Message::count(c)? > 0);
        Ok(())
    })
    .await
    .unwrap();
    undo(&db, first).await;
    db.read(|c| {
        assert!(Room::all(c)?.is_empty());
        assert_eq!(Message::count(c)?, 0);
        Ok(())
    })
    .await
    .unwrap();
}

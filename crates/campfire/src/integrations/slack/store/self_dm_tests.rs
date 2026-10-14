use super::super::client::tests::fake;
use super::super::jobs::tests::{run, setup, start};
use super::super::undoer::tests::undo;
use super::tests::import;
use crate::integrations::test_support::Route;
use campfire_app::integrations::slack::{conversations, users};
use campfire_db::models::slack::{NewConnection, SlackConnection};
use campfire_db::models::slack_import::SlackImport;
use campfire_db::models::slack_import::{Kind, Mode, NewImport};
use campfire_db::{Database, Message, NewMessage, NewUser, Role, Room, RoomType, User};
use rusqlite::params;
use serde_json::{Value, json};

async fn owner(db: &Database, administrator: bool) -> i64 {
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET role=1 WHERE id=1", [])?;
        let id = if administrator {
            1
        } else {
            let user = User::create(
                tx,
                NewUser {
                    name: "Member".into(),
                    ..Default::default()
                },
            )?;
            assert_eq!(user.role, Role::Member);
            assert_ne!(user.id, 1);
            user.id
        };
        tx.conn()
            .execute("UPDATE slack_connections SET user_id=? WHERE id=1", [id])?;
        Ok(id)
    })
    .await
    .unwrap()
}

async fn personal(db: &Database, user: i64, dry: bool) -> i64 {
    let id = start(db).await;
    db.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET user_id=?,kind='personal',mode=?,options=? WHERE id=?",
            params![
                user,
                if dry { "dry_run" } else { "import" },
                json!({"oldest":"2023-01-01T00:00:00Z"}).to_string(),
                id
            ],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    id
}

fn route(path: &str, body: Value) -> Route {
    Route::new("GET", "slack.com", path, 200).body(body.to_string())
}

fn provider(with_peer: bool, late_reply: bool) -> Vec<Route> {
    let mut channels = vec![
        json!({"id":"DSELF","is_im":true,"user":"UOWNER","num_members":1}),
        json!({"id":"DALIAS","is_im":true,"user":"UALIAS","num_members":1}),
        json!({"id":"DSLACKBOT","is_im":true,"user":"USLACKBOT","num_members":1}),
    ];
    if with_peer {
        channels.push(json!({"id":"DPEER","is_im":true,"user":"UPEER","num_members":2}));
    }
    let mut routes = vec![
        route(
            "/api/users.list?limit=200",
            json!({"ok":true,"members":[
                {"id":"UOWNER","name":"owner","profile":{"email":"different@example.com"}},
                {"id":"UALIAS","name":"alias"},
                {"id":"UPEER","name":"peer","profile":{"email":"peer@example.com"}},
                {"id":"USLACKBOT","is_bot":true}
            ]}),
        ),
        route(
            "/api/conversations.list?types=im%2Cmpim%2Cprivate_channel&exclude_archived=false&limit=200",
            json!({"ok":true,"channels":channels}),
        ),
    ];
    for (id, members, author, text, ts) in [
        (
            "DSELF",
            json!(["UOWNER"]),
            "UOWNER",
            "slackselfnote",
            "1700000001.000001",
        ),
        (
            "DALIAS",
            json!(["UALIAS"]),
            "UALIAS",
            "slackaliasnote",
            "1700000002.000002",
        ),
        (
            "DSLACKBOT",
            json!(["USLACKBOT"]),
            "USLACKBOT",
            "bot note",
            "1700000003.000003",
        ),
        (
            "DPEER",
            json!(["UOWNER", "UPEER"]),
            "UPEER",
            "ordinarydmparent",
            "1700000010.000010",
        ),
    ] {
        routes.push(route(
            &format!("/api/conversations.members?channel={id}&limit=1000"),
            json!({"ok":true,"members":members}),
        ));
        routes.push(route(&format!("/api/conversations.history?channel={id}&oldest=1672531200.000000&limit=200"),
            json!({"ok":true,"messages":[{"type":"message","user":author,"text":text,"ts":ts,"reply_count":if id=="DPEER"||id=="DSELF"{1}else{0}}],"has_more":false})));
    }
    let mut replies = vec![
        json!({"type":"message","user":"UOWNER","text":"slackselfnote","ts":"1700000001.000001"}),
        json!({"type":"message","user":"UOWNER","text":"slackselfreply","ts":"1700000001.000004","thread_ts":"1700000001.000001"}),
    ];
    if late_reply {
        replies.push(json!({"type":"message","user":"UOWNER","text":"lateselfreply","ts":"1700000001.000005","thread_ts":"1700000001.000001"}));
    }
    routes.push(route("/api/conversations.replies?channel=DSELF&ts=1700000001.000001&oldest=1672531200.000000&limit=200",
        json!({"ok":true,"messages":replies,"has_more":false})));
    routes.push(route("/api/conversations.replies?channel=DPEER&ts=1700000010.000010&oldest=1672531200.000000&limit=200",
        json!({"ok":true,"messages":[
            {"type":"message","user":"UPEER","text":"ordinarydmparent","ts":"1700000010.000010"},
            {"type":"message","user":"UOWNER","text":"ordinarydmreply","ts":"1700000011.000011","thread_ts":"1700000010.000010"}
        ],"has_more":false})));
    routes
}

async fn prior_aliases(db: &Database, user: i64, stale_owner: bool) {
    let id = personal(db, user, false).await;
    let row = run(db, id).await;
    db.write(move |tx| {
        users::record(tx, &row, "user", "UALIAS", "User", user, false)?;
        if stale_owner {
            let placeholder = User::create_slack_placeholder(
                tx,
                NewUser {
                    name: "Old placeholder".into(),
                    ..Default::default()
                },
                false,
                None,
                false,
            )?;
            users::record(tx, &row, "user", "UOWNER", "User", placeholder.id, true)?;
        }
        tx.conn().execute(
            "UPDATE slack_imports SET status='completed' WHERE id=?",
            [id],
        )?;
        Ok(())
    })
    .await
    .unwrap();
}

async fn own_note(db: &Database, room: i64, user: i64) -> i64 {
    db.write(move |tx| {
        Ok(Message::create_imported(
            tx,
            NewMessage {
                room_id: room,
                creator_id: user,
                markdown_source: Some("myretainednote".into()),
                ..Default::default()
            },
            tx.now(),
            None,
        )?
        .id)
    })
    .await
    .unwrap()
}

async fn transferred_run(db: &Database, user: i64, connection: i64, dry: bool) -> i64 {
    db.write(move |tx| {
        Ok(SlackImport::create(
            tx,
            NewImport {
                workspace_id: 1,
                connection_id: Some(connection),
                user_id: user,
                kind: Kind::Personal,
                mode: if dry { Mode::DryRun } else { Mode::Import },
                options: json!({"conversation_ids":["DSELF"]}),
            },
        )?
        .id)
    })
    .await
    .unwrap()
}

fn transferred_provider(late: bool) -> Vec<Route> {
    let mut routes = provider(false, late);
    for route in &mut routes {
        if route
            .path
            .starts_with("/api/conversations.history?channel=DSELF")
        {
            let mut body: Value = serde_json::from_slice(&route.body).unwrap();
            body["messages"][0]["reactions"] =
                json!([{"name":"heart","users":["UOWNER"],"count":1}]);
            body["messages"][0]["pinned_to"] = json!(["DSELF"]);
            route.body = body.to_string().into_bytes();
        }
    }
    let originals = routes.clone();
    for route in originals {
        if route.path.contains("channel=DSELF") && route.path.contains("oldest=1672531200.000000&")
        {
            for oldest in ["", "oldest=1697408001.000004&", "oldest=1697408001.000005&"] {
                let mut route = route.clone();
                route.path = route.path.replace("oldest=1672531200.000000&", oldest);
                routes.push(route);
            }
        }
    }
    routes
}

#[tokio::test]
async fn slack_self_dm_reconnect_isolates_owners_history_replies_and_undo() {
    for existing in [false, true] {
        for undo_b_first in [false, true] {
            let (db, crypto, _dir) = setup().await;
            let a = 1;
            let b = db
                .write(|tx| {
                    Ok(User::create(
                        tx,
                        NewUser {
                            name: "Second owner".into(),
                            ..Default::default()
                        },
                    )?
                    .id)
                })
                .await
                .unwrap();
            let b_room = if existing {
                Some(
                    db.write(move |tx| Ok(Room::find_or_create_direct_for(tx, &[b], b)?.id))
                        .await
                        .unwrap(),
                )
            } else {
                None
            };
            let (server, network) = fake(transferred_provider(false)).await;
            let first = transferred_run(&db, a, 1, false).await;
            import(&db, crypto.clone(), first, network.clone()).await;
            let a_room = db
                .read(move |c| Ok(Room::note_to_self_for(c, a)?.unwrap().id))
                .await
                .unwrap();
            let legacy = db.write(move |tx| {
            let prefix = format!("notes:{a}:DSELF");
            tx.conn().execute("UPDATE slack_import_records SET slack_key='DSELF' || substr(slack_key,?) WHERE slack_import_id=? AND (slack_key=? OR slack_key LIKE ?)", params![prefix.len() as i64+1, first, prefix, format!("{prefix}:%")])?;
            tx.conn().execute("UPDATE slack_imports SET stats=json_remove(stats,'$.conversations[0].mapping_key','$.conversations[0].notes_owner_id'),state=json_remove(state,'$.convo.mapping_key','$.convo.notes_owner_id') WHERE id=?", [first])?;
            SlackConnection::destroy(tx, 1)?;
            let mut q = tx.conn().prepare("SELECT slack_kind,slack_key,record_id,created_record FROM slack_import_records WHERE slack_import_id=? ORDER BY id")?;
            Ok(q.query_map([first], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,bool>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await.unwrap();
            assert_eq!(run(&db, first).await.slack_connection_id, None);
            let key = crypto.clone();
            let b_connection = db
                .write(move |tx| {
                    Ok(SlackConnection::create(
                        tx,
                        &key,
                        NewConnection {
                            workspace_id: 1,
                            user_id: b,
                            slack_user_id: "UOWNER",
                            access_token: Some("fixture-user-token"),
                            scopes: None,
                        },
                    )?
                    .id)
                })
                .await
                .unwrap();
            let preview = transferred_run(&db, b, b_connection, true).await;
            let preview = import(&db, crypto.clone(), preview, network.clone()).await;
            assert_eq!(
                preview.stats["conversations"][0]["target"]["room_id"],
                json!(b_room)
            );
            assert_eq!(preview.stats["conversations"][0]["name"], "your notes");
            let second = transferred_run(&db, b, b_connection, false).await;
            let imported = import(&db, crypto.clone(), second, network.clone()).await;
            assert_ne!(
                imported.stats["conversations"][0]["target"]["room_id"], a_room,
                "the reconnected owner must never reuse the previous owner's notes"
            );
            assert_eq!(imported.stats["counts"]["messages"], 1);
            assert_eq!(imported.stats["counts"]["replies"], 1);
            assert_eq!(imported.stats["counts"]["reactions"], 1);
            assert_eq!(imported.stats["counts"]["pins"], 1);
            let b_room = db
                .read(move |c| {
                    let room = Room::note_to_self_for(c, b)?.unwrap();
                    let messages = Message::for_room(c, room.id)?;
                    assert_eq!(messages.len(), 2);
                    assert!(messages.iter().all(|m| m.creator_id == b));
                    assert_eq!(messages[1].reply_to_message_id, Some(messages[0].id));
                    assert_eq!(Message::for_room(c, a_room)?.len(), 2);
                    Ok(room.id)
                })
                .await
                .unwrap();
            let own_a = own_note(&db, a_room, a).await;
            let own_b = own_note(&db, b_room, b).await;
            server.replace_routes(transferred_provider(true));
            let again_b = transferred_run(&db, b, b_connection, false).await;
            let repeated = import(&db, crypto.clone(), again_b, network.clone()).await;
            assert_eq!(repeated.stats["counts"]["messages"], 0);
            assert_eq!(repeated.stats["counts"]["replies"], 1);
            db.read(move |c| {
                let parent = Message::search_in_room(c, b_room, "slackselfnote")?
                    .pop()
                    .unwrap();
                let late = Message::search_in_room(c, b_room, "lateselfreply")?
                    .pop()
                    .unwrap();
                assert_eq!(late.reply_to_message_id, Some(parent.id));
                assert_eq!(late.creator_id, b);
                Ok(())
            })
            .await
            .unwrap();
            db.write(move |tx| {
                assert!(!SlackImport::undo(tx, second)?);
                Ok(())
            })
            .await
            .unwrap();
            let key = crypto.clone();
            let a_connection = db
                .write(move |tx| {
                    SlackConnection::destroy(tx, b_connection)?;
                    Ok(SlackConnection::create(
                        tx,
                        &key,
                        NewConnection {
                            workspace_id: 1,
                            user_id: a,
                            slack_user_id: "UOWNER",
                            access_token: Some("fixture-user-token"),
                            scopes: None,
                        },
                    )?
                    .id)
                })
                .await
                .unwrap();
            server.replace_routes(transferred_provider(false));
            let again_a = transferred_run(&db, a, a_connection, false).await;
            let repeated = import(&db, crypto, again_a, network).await;
            assert_eq!(repeated.stats["counts"]["messages"], 0);
            assert_eq!(repeated.stats["counts"]["replies"], 0);
            db.read(move |c| {
            let mut q=c.prepare("SELECT slack_kind,slack_key,record_id,created_record FROM slack_import_records WHERE slack_import_id=? ORDER BY id")?;
            let actual=q.query_map([first],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,bool>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            assert_eq!(actual,legacy,"the previous owner's ledger is immutable");
            Ok(())
        }).await.unwrap();
            db.write(move |tx| {
                assert!(!SlackImport::undo(tx, first)?);
                Ok(())
            })
            .await
            .unwrap();
            if undo_b_first {
                undo(&db, again_b).await;
                undo(&db, second).await;
                db.read(move |c| {
                    assert_eq!(Message::for_room(c, a_room)?.len(), 3);
                    Ok(())
                })
                .await
                .unwrap();
                undo(&db, again_a).await;
                undo(&db, first).await;
            } else {
                undo(&db, again_a).await;
                undo(&db, first).await;
                db.read(move |c| {
                    assert_eq!(Message::for_room(c, b_room)?.len(), 4);
                    Ok(())
                })
                .await
                .unwrap();
                undo(&db, again_b).await;
                undo(&db, second).await;
            }
            db.read(move |c| {
            for (user,room,own) in [(a,a_room,own_a),(b,b_room,own_b)] {
                assert_eq!(Room::note_to_self_for(c,user)?.unwrap().id,room);
                assert_eq!(Room::find(c,room)?.user_ids(c)?,vec![user]);
                assert_eq!(Message::for_room(c,room)?.iter().map(|m|m.id).collect::<Vec<_>>(),vec![own]);
            }
            assert_eq!(c.query_row("SELECT COUNT(*) FROM slack_import_records WHERE slack_import_id IN (?,?,?,?)",params![first,second,again_a,again_b],|r|r.get::<_,i64>(0))?,0);
            Ok(())
        }).await.unwrap();
        }
    }
}

#[tokio::test]
async fn slack_self_dm_resume_replaces_foreign_cached_room_parent_and_bounds() {
    let (db, crypto, _dir) = setup().await;
    let (_server, network) = fake(transferred_provider(false)).await;
    let first = transferred_run(&db, 1, 1, false).await;
    import(&db, crypto.clone(), first, network.clone()).await;
    let (a_room, parent, b, connection) = {
        let key = crypto.clone();
        db.write(move |tx| {
            let a_room = Room::note_to_self_for(tx.conn(), 1)?.unwrap().id;
            let parent = Message::search_in_room(tx.conn(), a_room, "slackselfnote")?
                .pop()
                .unwrap()
                .id;
            SlackConnection::destroy(tx, 1)?;
            let b = User::create(
                tx,
                NewUser {
                    name: "Resumed owner".into(),
                    ..Default::default()
                },
            )?
            .id;
            let connection = SlackConnection::create(
                tx,
                &key,
                NewConnection {
                    workspace_id: 1,
                    user_id: b,
                    slack_user_id: "UOWNER",
                    access_token: Some("fixture-user-token"),
                    scopes: None,
                },
            )?
            .id;
            Ok((a_room, parent, b, connection))
        })
        .await
        .unwrap()
    };
    let second = transferred_run(&db, b, connection, false).await;
    db.write(move |tx| {
        assert!(SlackImport::claim_running(tx,second)?);
        let state = json!({"phase":"messages","conversation_ids":["DSELF"],"convo_index":0,
            "conversations":[{"id":"DSELF","is_im":true,"user":"UOWNER"}],
            "convo":{"id":"DSELF","member_ids":["UOWNER"],"members_done":true,
                "resolved":true,"room_id":a_room,"direct":true,"history_done":true,
                "thread_queue":[],"thread_ts":"1700000001.000001","thread_message_id":parent,
                "thread_state":{},"bounds":{"oldest":1800000000.0,"latest":null}}});
        let stats = json!({"conversations":[{"id":"DSELF","name":"Direct message","type":"im","messages":0,"threads":0,"target":{"action":"merge","room_id":a_room},"done":false}]});
        tx.conn().execute("UPDATE slack_imports SET state=?,stats=? WHERE id=?",params![state.to_string(),stats.to_string(),second])?;
        Ok(())
    }).await.unwrap();
    let imported = import(&db, crypto, second, network).await;
    assert_eq!(imported.stats["counts"]["messages"], 1);
    assert_eq!(imported.stats["counts"]["replies"], 1);
    db.read(move |c| {
        let b_room = Room::note_to_self_for(c, b)?.unwrap().id;
        let messages = Message::for_room(c, b_room)?;
        assert_eq!(messages.len(), 2);
        assert!(messages.iter().all(|message| message.creator_id == b));
        assert_eq!(messages[1].reply_to_message_id, Some(messages[0].id));
        assert_ne!(messages[1].reply_to_message_id, Some(parent));
        assert_eq!(Message::for_room(c, a_room)?.len(), 2);
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_self_dm_reuses_legacy_kept_message_keys_only_for_their_notes_owner() {
    let (db, crypto, _dir) = setup().await;
    let (_server, network) = fake(transferred_provider(false)).await;
    let first = transferred_run(&db, 1, 1, false).await;
    import(&db, crypto.clone(), first, network.clone()).await;
    let (room,parent) = db.write(move |tx| {
        let prefix = "notes:1:DSELF";
        tx.conn().execute("UPDATE slack_import_records SET slack_key='DSELF' || substr(slack_key,?) WHERE slack_import_id=? AND (slack_key=? OR slack_key LIKE ?)",params![prefix.len() as i64+1,first,prefix,format!("{prefix}:%")])?;
        tx.conn().execute("UPDATE slack_imports SET stats=json_remove(stats,'$.conversations[0].mapping_key'),state=json_remove(state,'$.convo.mapping_key') WHERE id=?",[first])?;
        let room = Room::note_to_self_for(tx.conn(),1)?.unwrap().id;
        let parent = Message::search_in_room(tx.conn(),room,"slackselfnote")?.pop().unwrap().id;
        tx.conn().execute("INSERT INTO saved_items(message_id,user_id,created_at,updated_at) VALUES (?,1,?,?)",params![parent,tx.now(),tx.now()])?;
        Ok((room,parent))
    }).await.unwrap();
    undo(&db, first).await;
    db.read(move |c| {
        assert_eq!(users::mapped_id(c, 1, "conversation", "DSELF")?, None);
        assert_eq!(
            users::mapped_id(c, 1, "message", "DSELF:1700000001.000001")?,
            Some(parent)
        );
        assert_eq!(Message::for_room(c, room)?.len(), 1);
        Ok(())
    })
    .await
    .unwrap();
    let again = transferred_run(&db, 1, 1, false).await;
    let repeated = import(&db, crypto.clone(), again, network.clone()).await;
    assert_eq!(repeated.stats["counts"]["messages"], 0);
    assert_eq!(repeated.stats["counts"]["replies"], 1);
    assert_eq!(repeated.stats["conversations"][0]["mapping_key"], "DSELF");
    let key = crypto.clone();
    let (b, connection) = db
        .write(move |tx| {
            SlackConnection::destroy(tx, 1)?;
            let b = User::create(
                tx,
                NewUser {
                    name: "Other kept-message owner".into(),
                    ..Default::default()
                },
            )?
            .id;
            let connection = SlackConnection::create(
                tx,
                &key,
                NewConnection {
                    workspace_id: 1,
                    user_id: b,
                    slack_user_id: "UOWNER",
                    access_token: Some("fixture-user-token"),
                    scopes: None,
                },
            )?
            .id;
            Ok((b, connection))
        })
        .await
        .unwrap();
    let second = transferred_run(&db, b, connection, false).await;
    let imported = import(&db, crypto, second, network).await;
    assert_eq!(imported.stats["counts"]["messages"], 1);
    assert_eq!(imported.stats["counts"]["replies"], 1);
    db.read(move |c| {
        let own = Room::note_to_self_for(c, b)?.unwrap();
        let messages = Message::for_room(c, own.id)?;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].reply_to_message_id, Some(messages[0].id));
        assert_ne!(messages[0].id, parent);
        assert_eq!(Message::for_room(c, room)?.len(), 2);
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_self_dm_writer_rejects_foreign_notes_and_foreign_reply_parents() {
    use campfire_app::integrations::slack::writer;
    let (db, crypto, _dir) = setup().await;
    let (_server, network) = fake(transferred_provider(false)).await;
    let first = transferred_run(&db, 1, 1, false).await;
    import(&db, crypto.clone(), first, network).await;
    let key = crypto.clone();
    let (b, connection) = db
        .write(move |tx| {
            SlackConnection::destroy(tx, 1)?;
            let b = User::create(
                tx,
                NewUser {
                    name: "Writer owner".into(),
                    ..Default::default()
                },
            )?
            .id;
            let connection = SlackConnection::create(
                tx,
                &key,
                NewConnection {
                    workspace_id: 1,
                    user_id: b,
                    slack_user_id: "UOWNER",
                    access_token: Some("fixture-user-token"),
                    scopes: None,
                },
            )?
            .id;
            Ok((b, connection))
        })
        .await
        .unwrap();
    let second = transferred_run(&db, b, connection, false).await;
    let row = run(&db, second).await;
    db.write(move |tx| {
        let a_room = Room::note_to_self_for(tx.conn(), 1)?.unwrap();
        let parent = Message::search_in_room(tx.conn(), a_room.id, "slackselfnote")?
            .pop()
            .unwrap();
        let b_room = Room::find_or_create_direct_for(tx, &[b], b)?;
        let mut authors = indexmap::IndexMap::from([("UOWNER".into(), User::find(tx.conn(), b)?)]);
        let messages = [json!({"user":"UOWNER","text":"muststayprivate","ts":"1700000001.000009"})];
        assert!(
            writer::history(
                tx,
                &row,
                &a_room,
                "DSELF",
                &messages,
                writer::Bounds::default(),
                &mut authors
            )
            .is_err()
        );
        let mut state = json!({});
        let counts = writer::replies(
            tx,
            &row,
            &b_room,
            writer::Replies {
                conversation: "DSELF",
                parent_ts: "1700000001.000001",
                parent_id: parent.id,
                messages: &messages,
                bounds: writer::Bounds::default(),
                state: &mut state,
            },
            &mut authors,
        )?;
        assert_eq!(counts["replies"], 0);
        assert_eq!(counts["skipped"], 1);
        assert_eq!(state["deleted_reported"], true);
        assert_eq!(Message::for_room(tx.conn(), a_room.id)?.len(), 2);
        assert!(Message::for_room(tx.conn(), b_room.id)?.is_empty());
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_self_dm_import_reuses_canonical_notes_for_admin_and_member_and_undo_keeps_own_content()
 {
    for administrator in [true, false] {
        for existing in [false, true] {
            let (db, crypto, _dir) = setup().await;
            let user = owner(&db, administrator).await;
            let original = if existing {
                Some(
                    db.write(move |tx| Ok(Room::find_or_create_direct_for(tx, &[user], user)?.id))
                        .await
                        .unwrap(),
                )
            } else {
                None
            };
            let before = if let Some(room) = original {
                Some(own_note(&db, room, user).await)
            } else {
                None
            };
            prior_aliases(&db, user, !administrator).await;
            let id = personal(&db, user, false).await;
            let (server, network) = fake(provider(true, false)).await;
            let imported = import(&db, crypto.clone(), id, network.clone()).await;
            let notes = db.read(move |c| {
                let notes = Room::note_to_self_for(c, user)?.expect("self-DM must resolve to canonical notes");
                if let Some(original) = original { assert_eq!(notes.id, original); }
                assert_eq!(notes.user_ids(c)?, vec![user]);
                for key in ["DSELF", "DALIAS"] {
                    let mapping_key = users::notes_key(c, &imported, key)?;
                    assert_eq!(users::mapped_id(c, 1, "conversation", &mapping_key)?, Some(notes.id));
                    let created: bool = c.query_row("SELECT created_record FROM slack_import_records WHERE slack_kind='conversation' AND slack_key=?", [&mapping_key], |r| r.get(0))?;
                    assert!(!created, "notes are never owned by the import");
                    let entry = imported.stats["conversations"].as_array().unwrap().iter().find(|entry| entry["id"]==key).unwrap();
                    assert_eq!(entry["name"], "your notes");
                    assert_eq!(entry["target"]["room_name"], "your notes");
                }
                assert_eq!(c.query_row("SELECT COUNT(*) FROM slack_import_records WHERE slack_kind='membership' AND record_id IN (SELECT id FROM memberships WHERE room_id=?)", [notes.id], |r| r.get::<_,i64>(0))?, 0);
                for text in ["slackselfnote", "slackaliasnote"] {
                    let messages = Message::search_in_room(c, notes.id, text)?;
                    assert_eq!(messages.len(), 1);
                    assert_eq!(messages[0].creator_id, user);
                }
                let parent = Message::search_in_room(c, notes.id, "slackselfnote")?.pop().unwrap();
                let reply = Message::search_in_room(c, notes.id, "slackselfreply")?.pop().unwrap();
                assert_eq!(reply.creator_id, user);
                assert_eq!(reply.reply_to_message_id, Some(parent.id));
                assert!(parent.thread_id.is_none() && reply.thread_id.is_none());
                assert_eq!(c.query_row("SELECT COUNT(*) FROM channel_threads WHERE room_id=?", [notes.id], |r|r.get::<_,i64>(0))?, 0);
                assert!(users::mapped_id(c, 1, "conversation", "DSLACKBOT")?.is_none());
                if !administrator {
                    assert_ne!(users::mapped_id(c, 1, "user", "UOWNER")?, Some(user));
                }
                let peer = users::mapped_id(c, 1, "conversation", "DPEER")?.unwrap();
                assert_ne!(peer, notes.id);
                let messages = Message::for_room(c, peer)?;
                assert_eq!(messages.len(), 2);
                assert_eq!(messages[0].creator_id, users::mapped_id(c, 1, "user", "UPEER")?.unwrap());
                assert_eq!(messages[1].creator_id, users::mapped_id(c, 1, "user", "UOWNER")?.unwrap());
                assert_eq!(messages[1].reply_to_message_id, Some(messages[0].id));
                assert!(messages.iter().all(|m| m.thread_id.is_none()));
                Ok(notes.id)
            }).await.unwrap();
            let own = match before {
                Some(id) => id,
                None => own_note(&db, notes, user).await,
            };
            let again = personal(&db, user, false).await;
            db.write(move |tx| {
                tx.conn().execute("UPDATE slack_imports SET options=json_set(options,'$.conversation_ids',json(?)) WHERE id=?", params![json!(["DSELF","DALIAS"]).to_string(),again])?;
                Ok(())
            }).await.unwrap();
            server.replace_routes(provider(true, true));
            let repeated = import(&db, crypto.clone(), again, network.clone()).await;
            assert_eq!(repeated.stats["counts"]["messages"], 0);
            assert_eq!(repeated.stats["counts"]["replies"], 1);
            assert_eq!(repeated.stats["conversations"].as_array().unwrap().len(), 2);
            for entry in repeated.stats["conversations"].as_array().unwrap() {
                assert_eq!(entry["name"], "your notes");
                assert_eq!(entry["target"]["room_name"], "your notes");
                assert_eq!(entry["target"]["room_id"], notes);
            }
            db.read(move |c| {
                let parent = Message::search_in_room(c, notes, "slackselfnote")?
                    .pop()
                    .unwrap();
                let late = Message::search_in_room(c, notes, "lateselfreply")?
                    .pop()
                    .unwrap();
                assert_eq!(late.creator_id, user);
                assert_eq!(late.reply_to_message_id, Some(parent.id));
                assert!(late.thread_id.is_none());
                Ok(())
            })
            .await
            .unwrap();
            db.write(move |tx| {
                assert!(
                    !SlackImport::undo(tx, id)?,
                    "later import must be undone first"
                );
                Ok(())
            })
            .await
            .unwrap();
            let third = personal(&db, user, false).await;
            let unchanged = import(&db, crypto, third, network).await;
            assert_eq!(unchanged.stats["counts"]["messages"], 0);
            assert_eq!(unchanged.stats["counts"]["replies"], 0);
            undo(&db, third).await;
            undo(&db, again).await;
            undo(&db, id).await;
            db.read(move |c| {
                assert_eq!(Room::note_to_self_for(c, user)?.unwrap().id, notes);
                assert_eq!(Room::find(c, notes)?.user_ids(c)?, vec![user]);
                let remaining = Message::for_room(c, notes)?;
                assert_eq!(
                    remaining.iter().map(|m| m.id).collect::<Vec<_>>(),
                    vec![own]
                );
                assert!(
                    remaining[0]
                        .body_html(c)?
                        .unwrap()
                        .contains("myretainednote")
                );
                assert_eq!(
                    Message::search_in_room(c, notes, "myretainednote")?.len(),
                    1
                );
                assert!(Message::search_in_room(c, notes, "slackselfnote")?.is_empty());
                Ok(())
            })
            .await
            .unwrap();
        }
    }
}

#[tokio::test]
async fn slack_self_dm_dry_preview_names_your_notes_without_domain_writes() {
    for existing in [false, true] {
        let (db, crypto, _dir) = setup().await;
        let user = owner(&db, false).await;
        prior_aliases(&db, user, false).await;
        let original = if existing {
            Some(
                db.write(move |tx| Ok(Room::find_or_create_direct_for(tx, &[user], user)?.id))
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let id = personal(&db, user, true).await;
        let (_server, network) = fake(provider(false, false)).await;
        let preview = import(&db, crypto, id, network).await;
        let entry = preview.stats["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == "DSELF")
            .unwrap();
        assert_eq!(entry["name"], "your notes");
        assert_eq!(entry["target"]["room_name"], "your notes");
        assert_eq!(entry["target"]["room_id"], json!(original));
        assert_eq!(
            entry["target"]["action"],
            if existing { "merge" } else { "create" }
        );
        let alias = preview.stats["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == "DALIAS")
            .unwrap();
        assert_eq!(alias["name"], "your notes");
        assert_eq!(alias["target"]["room_id"], json!(original));
        assert!(
            preview.stats["samples"]
                .as_array()
                .unwrap()
                .iter()
                .any(|sample| sample["conversation"] == "your notes")
        );
        db.read(move |c| {
            assert_eq!(Room::all(c)?.len(), usize::from(existing));
            assert_eq!(Room::note_to_self_for(c, user)?.map(|r| r.id), original);
            for table in ["messages", "channel_threads"] {
                assert_eq!(
                    c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))?,
                    0
                );
            }
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM slack_import_records", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM memberships", [], |r| r
                    .get::<_, i64>(0))?,
                i64::from(existing)
            );
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?,
                2
            );
            Ok(())
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn slack_self_dm_keeps_missing_peer_skips_numeric_target_issue_and_canonical_identity() {
    let (db, _, _dir) = setup().await;
    let user = owner(&db, false).await;
    let id = personal(&db, user, false).await;
    let mut row = run(&db, id).await;
    row.kind = "workspace".into();
    row.options = json!({"room_targets":{"DSELF":99999}});
    db.write(move |tx| {
        let group = Room::create_for(tx, RoomType::Direct, Some("Former group"), user, &[user])?;
        tx.conn().execute(
            "UPDATE rooms SET direct_member_key=direct_member_key || ':group' WHERE id=?",
            [group.id],
        )?;
        let target = conversations::resolve(
            tx,
            &row,
            &json!({"id":"DSELF","is_im":true,"user":"UOWNER"}),
            &[json!("UOWNER")],
            &indexmap::IndexMap::new(),
            false,
        )?;
        let notes = target.room.expect("self DM must target notes");
        assert_ne!(notes.id, group.id);
        assert_eq!(
            Room::note_to_self_for(tx.conn(), user)?.unwrap().id,
            notes.id
        );
        let issue: String = tx.conn().query_row(
            "SELECT message FROM slack_import_issues WHERE slack_import_id=?",
            [id],
            |r| r.get(0),
        )?;
        assert_eq!(
            issue,
            "Room targets only apply to channels; this DM keeps its own Direct room"
        );
        for conversation in [
            json!({"id":"DMISSING","is_im":true}),
            json!({"id":"DUNKNOWN","is_im":true,"user":"UNKNOWN"}),
        ] {
            let result = conversations::resolve(
                tx,
                &row,
                &conversation,
                &[],
                &indexmap::IndexMap::new(),
                true,
            )?;
            assert_eq!(result.action, "skip");
        }
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_self_dm_owner_from_conversation_needs_no_placeholder_when_members_and_users_are_absent()
 {
    let (db, crypto, _dir) = setup().await;
    let id = personal(&db, 1, false).await;
    let mut response = provider(false, false);
    response.retain(|r| {
        !r.path.starts_with("/api/users.list")
            && !r.path.starts_with("/api/conversations.list")
            && !r
                .path
                .starts_with("/api/conversations.members?channel=DSELF")
    });
    response.extend([
        route("/api/users.list?limit=200", json!({"ok":true,"members":[]})),
        route("/api/conversations.list?types=im%2Cmpim%2Cprivate_channel&exclude_archived=false&limit=200",
            json!({"ok":true,"channels":[{"id":"DSELF","is_im":true,"user":"UOWNER"}]})),
        route("/api/conversations.members?channel=DSELF&limit=1000", json!({"ok":true,"members":[]})),
    ]);
    let (_server, network) = fake(response).await;
    import(&db, crypto, id, network).await;
    let notes = db
        .read(|c| {
            let notes = Room::note_to_self_for(c, 1)?.unwrap();
            assert_eq!(Message::for_room(c, notes.id)?.len(), 2);
            assert!(
                Message::for_room(c, notes.id)?
                    .iter()
                    .all(|m| m.creator_id == 1)
            );
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?,
                1
            );
            assert!(users::mapped_id(c, 1, "user", "UOWNER")?.is_none());
            Ok(notes.id)
        })
        .await
        .unwrap();
    undo(&db, id).await;
    db.read(move |c| {
        assert_eq!(Room::note_to_self_for(c, 1)?.unwrap().id, notes);
        assert_eq!(Room::find(c, notes)?.user_ids(c)?, vec![1]);
        assert!(Message::for_room(c, notes)?.is_empty());
        assert!(Message::search_in_room(c, notes, "slackselfnote")?.is_empty());
        Ok(())
    })
    .await
    .unwrap();
}

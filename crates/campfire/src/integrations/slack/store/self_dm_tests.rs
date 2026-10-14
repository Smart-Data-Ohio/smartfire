use super::super::client::tests::fake;
use super::super::jobs::tests::{run, setup, start};
use super::super::undoer::tests::undo;
use super::tests::import;
use crate::integrations::test_support::Route;
use campfire_app::integrations::slack::{conversations, users};
use campfire_db::models::slack_import::SlackImport;
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
                    assert_eq!(users::mapped_id(c, 1, "conversation", key)?, Some(notes.id));
                    let created: bool = c.query_row("SELECT created_record FROM slack_import_records WHERE slack_kind='conversation' AND slack_key=?", [key], |r| r.get(0))?;
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

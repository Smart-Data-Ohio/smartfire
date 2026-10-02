use super::super::jobs::tests::{run, setup, start};
use super::*;
use campfire_jobs::inspect;

#[tokio::test]
async fn slack_ordering_mentions_keep_existing_then_rails_mapping_order() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/ordering.json"
    ))
    .unwrap();
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        for (key, name) in oracle["keys"]
            .as_array()
            .unwrap()
            .iter()
            .zip(oracle["names"].as_array().unwrap())
            .rev()
        {
            let user = User::create(
                tx,
                campfire_db::NewUser {
                    name: name.as_str().unwrap().into(),
                    ..Default::default()
                },
            )?;
            users::record(
                tx,
                &run,
                "user",
                key.as_str().unwrap(),
                "User",
                user.id,
                true,
            )?;
        }
        let mut mapped = users::users_for(tx.conn(), &run, &[json!("U10")])?;
        enrich(
            tx,
            &run,
            oracle["messages"].as_array().unwrap(),
            &mut mapped,
        )?;
        assert_eq!(
            json!(mapped.keys().collect::<Vec<_>>()),
            oracle["mention_keys"]
        );
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_converter_real_saves_match_rails_bodies_and_mentionees() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/rendering.json"
    ))
    .unwrap();
    let (db, _, _dir) = super::super::jobs::tests::setup_sequence().await;
    db.write(move |tx| {
        let jane = User::create(
            tx,
            campfire_db::NewUser {
                name: "Jane Doe".into(),
                email_address: Some("conv-jane@example.com".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(jane.id, 2);
        let room = Room::create_for(
            tx,
            campfire_db::RoomType::Closed,
            Some("Render fixture"),
            1,
            &[1, 2],
        )?;
        let users = serde_json::from_value(oracle["users"].clone()).unwrap();
        for case in oracle["cases"].as_array().unwrap() {
            let source =
                super::super::markdown::convert(&json!({"text":case["text"]}), &users).markdown;
            assert_eq!(source, case["markdown"]);
            let message = Message::create_imported(
                tx,
                campfire_db::NewMessage {
                    room_id: room.id,
                    creator_id: 1,
                    markdown_source: Some(source),
                    ..Default::default()
                },
                tx.now(),
                None,
            )?;
            assert_eq!(
                message.body_html(tx.conn())?.unwrap(),
                case["body"],
                "{}",
                case["text"]
            );
            let mentioned = message
                .mentionees(tx.conn(), tx.rich_text())?
                .into_iter()
                .map(|u| u.id)
                .collect::<Vec<_>>();
            assert_eq!(json!(mentioned), case["mentionees"]);
        }
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM activity_items", [], |r| r
                    .get::<_, i64>(0))?,
            0
        );
        Ok(())
    })
    .await
    .unwrap();
    println!(
        "Slack real-save rendering: 4 persisted body and mention goldens matched byte for byte"
    );
}

#[tokio::test]
async fn slack_writer_mapped_nonmember_mentions_render_tokens_and_unknown_labels_fall_back() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let jane=User::create_slack_placeholder(tx,campfire_db::NewUser{name:"Robin Returner".into(),..Default::default()},false,None,false)?;
        users::record(tx,&run,"user","URET","User",jane.id,true)?;
        let room=Room::create_for(tx,campfire_db::RoomType::Closed,Some("secret"),1,&[1])?;
        let mut mapped=IndexMap::new();
        history(tx,&run,&room,"CPRIV",&[json!({"type":"message","user":"UOWNER","text":"hi <@URET|robin> and <@U999|ghost>","ts":"1700000031.000031"})],Bounds::default(),&mut mapped)?;
        let message=Message::for_room(tx.conn(),room.id)?.pop().unwrap();
        assert_eq!(message.markdown_source.as_deref(),Some("hi @[Robin Returner] and @ghost"));
        assert!(message.body_html(tx.conn())?.unwrap().contains("@[Robin Returner]"));
        assert!(message.mentionees(tx.conn(),tx.rich_text())?.is_empty());Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        let members: Value = serde_json::from_str(include_str!("../fixtures/users.json")).unwrap();
        users::map_page(tx, &run, &members["members"], false, &HashSet::new())?;
        let member_ids: Value =
            serde_json::from_str(include_str!("../fixtures/members_CCHAN.json")).unwrap();
        let mut mapped = users::users_for(tx.conn(), &run, array(&member_ids["members"]))?;
        let room = super::super::conversations::resolve(
            tx,
            &run,
            &json!({"id":"CCHAN","name":"general"}),
            array(&member_ids["members"]),
            &mapped,
            false,
        )?
        .room
        .unwrap();
        let stamp = room.updated_at;
        let first: Value =
            serde_json::from_str(include_str!("../fixtures/history_CCHAN_p1.json")).unwrap();
        let second: Value =
            serde_json::from_str(include_str!("../fixtures/history_CCHAN_p2.json")).unwrap();
        let one = history(
            tx,
            &run,
            &room,
            "CCHAN",
            array(&first["messages"]),
            Bounds::default(),
            &mut mapped,
        )?;
        let two = history(
            tx,
            &run,
            &room,
            "CCHAN",
            array(&second["messages"]),
            Bounds::default(),
            &mut mapped,
        )?;
        assert!(integer(&one.counts["messages"]) > 0);
        assert_eq!(two.parents.len(), 1);
        let parent = &two.parents[0];
        let parent_id = integer(&parent["message_id"]);
        let parent_ts = string(&parent["ts"]);
        let page: Value =
            serde_json::from_str(include_str!("../fixtures/replies_CCHAN_parent.json")).unwrap();
        let mut thread_state = json!({});
        let delta = replies(
            tx,
            &run,
            &room,
            Replies {
                conversation: "CCHAN",
                parent_ts: &parent_ts,
                parent_id,
                messages: array(&page["messages"]),
                bounds: Bounds::default(),
                state: &mut thread_state,
            },
            &mut mapped,
        )?;
        assert_eq!(delta["replies"], 2);
        assert_eq!(delta["threads"], 1);
        let thread = ChannelThread::find(tx.conn(), integer(&thread_state["thread_id"]))?;
        assert_eq!(thread.messages_count, 0); // refreshed once, after the last replies page
        finish_thread(tx, &run, "CCHAN", &parent_ts)?;
        assert_eq!(ChannelThread::find(tx.conn(), thread.id)?.messages_count, 2);
        assert_eq!(
            ChannelThread::find(tx.conn(), thread.id)?.last_activity_at,
            slack_time("1700000102.000102")?
        );
        assert_eq!(Room::find(tx.conn(), room.id)?.updated_at, stamp);
        assert!(
            room.memberships(tx.conn())?
                .iter()
                .all(|m| m.unread_at.is_none())
        );
        let parent = Message::find(tx.conn(), parent_id)?;
        assert_eq!(parent.created_at, slack_time("1700000002.000002")?);
        let dup = history(
            tx,
            &run,
            &room,
            "CCHAN",
            array(&second["messages"]),
            Bounds::default(),
            &mut mapped,
        )?;
        assert_eq!(dup.counts["messages"], 0);
        assert_eq!(dup.parents.len(), 1);
        let index: i64 =
            tx.conn()
                .query_row("SELECT count(*) FROM message_search_index", [], |r| {
                    r.get(0)
                })?;
        let messages: i64 = tx
            .conn()
            .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?;
        assert_eq!(index, messages);
        let pins: i64 = tx
            .conn()
            .query_row("SELECT count(*) FROM message_pins", [], |r| r.get(0))?;
        assert_eq!(pins, 1);
        let boosts: i64 = tx
            .conn()
            .query_row("SELECT count(*) FROM boosts", [], |r| r.get(0))?;
        assert!(boosts > 0);
        let activity: i64 =
            tx.conn()
                .query_row("SELECT count(*) FROM activity_items", [], |r| r.get(0))?;
        assert_eq!(activity, 0);
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(db.read(inspect::all).await.unwrap().len(), 1); // only the run's original StepJob
}
#[tokio::test]
async fn slack_writer_direct_replies_flatten_and_deleted_parent_skips_channel_thread() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let peer = User::create(
            tx,
            campfire_db::NewUser {
                name: "Peer".into(),
                ..Default::default()
            },
        )?;
        let room = Room::create_for(tx, campfire_db::RoomType::Direct, None, 1, &[1, peer.id])?;
        let mut mapped: IndexMap<_, _> = [
            ("UONE".into(), User::find(tx.conn(), 1)?),
            ("UTWO".into(), peer),
        ]
        .into();
        let roots = history(
            tx,
            &run,
            &room,
            "DIM",
            &[json!({"user":"UONE","ts":"100.000001","text":"parent","reply_count":1})],
            Bounds::default(),
            &mut mapped,
        )?;
        let parent = integer(&roots.parents[0]["message_id"]);
        let mut state = json!({});
        let delta = replies(
            tx,
            &run,
            &room,
            Replies {
                conversation: "DIM",
                parent_ts: "100.000001",
                parent_id: parent,
                messages: &[json!({"user":"UTWO","ts":"101.000002","text":"reply"})],
                bounds: Bounds::default(),
                state: &mut state,
            },
            &mut mapped,
        )?;
        assert_eq!(delta["replies"], 1);
        assert_eq!(delta["threads"], 0);
        let reply = Message::for_room(tx.conn(), room.id)?.pop().unwrap();
        assert_eq!(reply.reply_to_message_id, Some(parent));
        assert!(reply.thread_id.is_none());
        let channel = Room::create(tx, campfire_db::RoomType::Closed, Some("channel"), 1)?;
        let missing = replies(
            tx,
            &run,
            &channel,
            Replies {
                conversation: "CDELETED",
                parent_ts: "200.000001",
                parent_id: 999999,
                messages: &[json!({"user":"UTWO","ts":"201.000001","text":"reply"})],
                bounds: Bounds::default(),
                state: &mut state,
            },
            &mut mapped,
        )?;
        assert_eq!(missing["skipped"], 1);
        assert_eq!(state["deleted_reported"], true);
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_writer_mapping_failure_rolls_back_page_and_search_rows() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    let result = db
        .write(move |tx| {
            let room = Room::create(tx, campfire_db::RoomType::Closed, Some("failure"), 1)?;
            let mut users = [("UOWNER".into(), User::find(tx.conn(), 1)?)].into();
            history(
                tx,
                &run,
                &room,
                "FAIL",
                &[
                    json!({"user":"UOWNER","ts":"1.000001","text":"one"}),
                    json!({"user":"UOWNER","ts":"1.000001","text":"two"}),
                ],
                Bounds::default(),
                &mut users,
            )?;
            Ok(())
        })
        .await;
    assert!(result.unwrap_err().is_record_not_unique());
    assert_eq!(
        db.read(|c| Ok(c.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        db.read(|c| Ok(
            c.query_row("SELECT count(*) FROM message_search_index", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        0
    );
}
#[test]
fn slack_writer_timestamp_microseconds_do_not_round_through_float() {
    assert_eq!(
        slack_time("1700000002.000002").unwrap().as_microsecond(),
        1_700_000_002_000_002
    );
    assert_eq!(
        slack_time("1700000002.9999999").unwrap().as_microsecond(),
        1_700_000_002_999_999
    );
    assert_eq!(slack_time("-0.0000001").unwrap().as_microsecond(), -1);
    assert!(slack_time("NaN").is_err());
}

#[test]
fn slack_writer_bounds_match_actual_rails_rational_float_comparisons() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/sequence.json"
    ))
    .unwrap();
    for case in array(&vector["bounds"]) {
        assert_eq!(
            Bounds::from_value(&case["bounds"])
                .contains(case["ts"].as_str().unwrap())
                .unwrap(),
            case["contains"].as_bool().unwrap(),
            "{case}"
        );
    }
}

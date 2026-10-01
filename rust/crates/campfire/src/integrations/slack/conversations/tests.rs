use super::super::jobs::tests::{run, setup, start};
use super::*;
use serde_json::json;

#[tokio::test]
async fn slack_conversations_only_public_workspace_channels_auto_merge() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let open = Room::create(tx, RoomType::Open, Some("GENERAL"), 1)?;
        let private = Room::create(tx, RoomType::Closed, Some("secret"), 1)?;
        let public = resolve(
            tx,
            &run,
            &json!({"id":"CPUB","name":"general"}),
            &[],
            &HashMap::new(),
            false,
        )?;
        assert_eq!(public.action, "merge");
        assert_eq!(public.room.unwrap().id, open.id);
        let closed = resolve(
            tx,
            &run,
            &json!({"id":"CPRIV","name":"secret","is_private":true}),
            &[],
            &HashMap::new(),
            false,
        )?;
        assert_eq!(closed.action, "create");
        assert_ne!(closed.room.unwrap().id, private.id);
        let mut personal = run.clone();
        personal.kind = "personal".into();
        personal.options = json!({"room_targets":{"PERSONAL":open.id}});
        assert_eq!(
            resolve(
                tx,
                &personal,
                &json!({"id":"PERSONAL","name":"general"}),
                &[],
                &HashMap::new(),
                false
            )?
            .action,
            "create"
        );
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_conversations_archive_and_open_nonmembers_are_invisible_and_recorded() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let member = User::create_slack_placeholder(
            tx,
            campfire_db::NewUser {
                name: "Slack member".into(),
                ..Default::default()
            },
            false,
            None,
            false,
        )?;
        let users: HashMap<_, _> = [("UMEMBER".into(), member.clone())].into();
        let channel = json!({"id":"COPEN","name":"open"});
        let room = resolve(tx, &run, &channel, &[json!("UMEMBER")], &users, false)?
            .room
            .unwrap();
        let rows = room.memberships(tx.conn())?;
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter().find(|m| m.user_id == 1).unwrap().involvement,
            Some(campfire_db::Involvement::Invisible)
        );
        assert_eq!(
            rows.iter()
                .find(|m| m.user_id == member.id)
                .unwrap()
                .involvement,
            Some(campfire_db::Involvement::Mentions)
        );
        let archived = resolve(
            tx,
            &run,
            &json!({"id":"CARCH","name":"old","is_archived":true}),
            &[json!("UMEMBER")],
            &users,
            false,
        )?
        .room
        .unwrap();
        assert_eq!(archived.name.as_deref(), Some("old (archived)"));
        assert!(
            archived
                .memberships(tx.conn())?
                .iter()
                .all(|m| m.involvement == Some(campfire_db::Involvement::Invisible))
        );
        let mapped: i64 = tx.conn().query_row(
            "SELECT count(*) FROM slack_import_records WHERE slack_kind='membership'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(mapped, 4);
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let mut run = run(&db, id).await;
    db.write(move |tx| {
        let room = Room::create(tx, RoomType::Closed, Some("existing"), 1)?;
        run.options = json!({"room_targets":{"CTARGET":room.id,"CBAD":9999}});
        assert_eq!(
            resolve(
                tx,
                &run,
                &json!({"id":"CTARGET","name":"slack-private","is_private":true}),
                &[json!("UOWNER")],
                &[("UOWNER".into(), User::find(tx.conn(), 1)?)].into(),
                false
            )?
            .action,
            "merge"
        );
        assert!(room.memberships(tx.conn())?.is_empty());
        assert_eq!(
            resolve(
                tx,
                &run,
                &json!({"id":"CBAD","name":"missing"}),
                &[],
                &HashMap::new(),
                false
            )?
            .reason,
            Some("invalid room target")
        );
        Ok(())
    })
    .await
    .unwrap();
    let issue: String = db
        .read(|c| Ok(c.query_row("SELECT message FROM slack_import_issues", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(
        issue,
        "Room target 9999 for #missing is not an alive Open or Closed room; skipped"
    );
}
#[tokio::test]
async fn slack_conversations_directs_reuse_member_sets_skip_self_and_slackbot() {
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
        let owner = User::find(tx.conn(), 1)?;
        let users: HashMap<_, _> = [("UOWNER".into(), owner), ("UPEER".into(), peer)].into();
        let members = vec![json!("UOWNER"), json!("UPEER")];
        let first = resolve(
            tx,
            &run,
            &json!({"id":"D1","is_im":true}),
            &members,
            &users,
            false,
        )?;
        let second = resolve(
            tx,
            &run,
            &json!({"id":"D2","is_im":true}),
            &members,
            &users,
            false,
        )?;
        assert_eq!(first.action, "create");
        assert_eq!(second.action, "merge");
        assert_eq!(first.room.unwrap().id, second.room.unwrap().id);
        assert_eq!(
            resolve(
                tx,
                &run,
                &json!({"id":"DSELF","is_im":true,"user":"UOWNER"}),
                &[json!("UOWNER")],
                &users,
                false
            )?
            .reason,
            Some("self DM")
        );
        assert_eq!(
            resolve(
                tx,
                &run,
                &json!({"id":"DSLACK","is_im":true,"user":"USLACKBOT"}),
                &[json!("USLACKBOT")],
                &dry_users(tx, &run, &[json!("USLACKBOT")])?,
                true
            )?
            .reason,
            Some("Slackbot DM")
        );
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_conversations_dry_group_preview_uses_negative_members_without_writes() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let members = vec![json!("UONE"), json!("UTWO")];
        let users = dry_users(tx, &run, &members)?;
        let target = resolve(
            tx,
            &run,
            &json!({"id":"GDM","is_mpim":true}),
            &members,
            &users,
            true,
        )?;
        assert_eq!(target.action, "create");
        assert!(target.room.is_none());
        assert_eq!(
            tx.conn()
                .query_row("SELECT count(*) FROM rooms", [], |r| r.get::<_, i64>(0))?,
            0
        );
        assert_eq!(
            tx.conn()
                .query_row("SELECT count(*) FROM slack_import_records", [], |r| r
                    .get::<_, i64>(0))?,
            0
        );
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn slack_review_room_matching_uses_rails_sqlite_lower() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/review_regressions.json"
    ))
    .unwrap();
    for case in vectors["rooms"].as_array().unwrap() {
        let (db, _, _dir) = setup().await;
        let id = start(&db).await;
        let run = run(&db, id).await;
        let case = case.clone();
        db.write(move |tx| {
            tx.conn().execute("INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(861,?,'Rooms::Open',1,?,?)",rusqlite::params![case["existing"].as_str().unwrap(),tx.now(),tx.now()])?;
            Room::find(tx.conn(), 861)?.grant_to(tx, &[1])?;
            let conversation = json!({"id":"CREVIEW", "name":case["incoming"]});
            let preview = resolve(tx, &run, &conversation, &[], &HashMap::new(), true)?;
            assert_eq!(json!({"action":preview.action,"room_id":preview.room.map(|r|r.id)}),case["preview"],"{} into {} preview",case["incoming"],case["existing"]);
            let result = resolve(tx, &run, &conversation, &[], &HashMap::new(), false)?;
            let room = result.room.unwrap();
            let created: bool = tx.conn().query_row("SELECT created_record FROM slack_import_records WHERE slack_kind='conversation'",[],|r|r.get(0))?;
            let actual = json!({"action":result.action,"room_id":room.id,"name":room.name,"created":created,"memberships":room.memberships(tx.conn())?.len()});
            assert_eq!(actual,case["result"],"{} into {} import",case["incoming"],case["existing"]);
            Ok(())
        }).await.unwrap();
    }
    println!("Slack review room parity: 11 Rails preview/import cases matched");
}

#[tokio::test]
async fn slack_review_group_name_keeps_ruby_unicode_downcase_order() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/review_regressions.json"
    ))
    .unwrap();
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        let mut users = HashMap::new();
        let mut members = Vec::new();
        for (i, name) in vectors["group"]["names"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let key = format!("U{i}");
            members.push(json!(key));
            users.insert(
                key,
                User::create(
                    tx,
                    campfire_db::NewUser {
                        name: name.as_str().unwrap().into(),
                        ..Default::default()
                    },
                )?,
            );
        }
        let result = resolve(
            tx,
            &run,
            &json!({"id":"GREVIEW","is_mpim":true}),
            &members,
            &users,
            false,
        )?;
        assert_eq!(json!(result.room.unwrap().name), vectors["group"]["result"]);
        Ok(())
    })
    .await
    .unwrap();
}

use super::super::jobs::tests::{run, setup, start};
use super::*;

#[tokio::test]
async fn slack_users_preview_import_repeat_match_pinned_rails_fixture_rows() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx|{
        tx.conn().execute("UPDATE slack_connections SET slack_user_id='UADMIN'",[])?;
        tx.conn().execute("INSERT INTO users (id,name,email_address,bio,status,google_email_link_allowed,created_at,updated_at) VALUES (2,'Existing Kevin','kevin@37signals.com','Programmer',1,1,?,?)",params![tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    let run = run(&db, id).await;
    let members: Value = serde_json::from_str(include_str!("../fixtures/users.json")).unwrap();
    let run_copy = run.clone();
    let page = members["members"].clone();
    let preview = db
        .write(move |tx| {
            map_page(
                tx,
                &run_copy,
                &page,
                true,
                &["example.com".to_owned()].into(),
            )
        })
        .await
        .unwrap();
    assert_eq!(
        db.read(|c| Ok(
            c.query_row("SELECT COUNT(*) FROM slack_import_records", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        0
    );
    let page = members["members"].clone();
    let run_copy = run.clone();
    let delta = db
        .write(move |tx| {
            map_page(
                tx,
                &run_copy,
                &page,
                false,
                &["example.com".to_owned()].into(),
            )
        })
        .await
        .unwrap();
    let expected: Value =
        serde_json::from_str(include_str!("../../../../../../vectors/slack/users.json")).unwrap();
    assert_eq!(preview, expected["preview"]);
    assert_eq!(delta, expected["stats"]);
    let rows=db.read(|c|{
        let mut statement=c.prepare("SELECT r.slack_key,r.created_record,u.name,u.email_address,u.status,u.bio,u.time_zone,u.google_email_link_allowed FROM slack_import_records r JOIN users u ON u.id=r.record_id WHERE r.slack_kind='user' ORDER BY r.slack_key")?;
        Ok(statement.query_map([],|row|Ok(json!({"key":row.get::<_,String>(0)?,"created":row.get::<_,bool>(1)?,"name":row.get::<_,String>(2)?,"email":row.get::<_,Option<String>>(3)?,"status":if row.get::<_,i64>(4)?==0{"active"}else{"deactivated"},"bio":row.get::<_,Option<String>>(5)?,"zone":row.get::<_,Option<String>>(6)?,"claimable":row.get::<_,bool>(7)?})))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    assert_eq!(json!(rows), expected["users"]);
    let page = members["members"].clone();
    let run_copy = run.clone();
    let repeated = db
        .write(move |tx| map_page(tx, &run_copy, &page, false, &HashSet::new()))
        .await
        .unwrap();
    assert_eq!(repeated, expected["repeat"]);
}
#[tokio::test]
async fn slack_users_humans_receive_open_rooms_bots_guests_and_unknown_authors_do_not() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx| {
        campfire_db::Room::create(tx, campfire_db::RoomType::Open, Some("everyone"), 1)?;
        Ok(())
    })
    .await
    .unwrap();
    let run = run(&db, id).await;
    db.write(move |tx|{
        map_page(tx,&run,&json!([{ "id":"UCLAIM","profile":{"real_name":"Claim","email":"claim@example.com"},"tz":"America/New_York"},
            {"id":"UGUEST","profile":{"email":"guest@example.com"},"is_restricted":true},
            {"id":"UNOMAIL","name":"No Mail"}]),false,&["example.com".into()].into())?;
        let unknown=ensure_author(tx,&run,"UUNKNOWN")?;
        let again=ensure_author(tx,&run,"UUNKNOWN")?;assert_eq!(unknown.id,again.id);
        let bot=bot_user(tx,&run,&json!({"bot_id":"B42","username":"builder"}))?;
        assert_eq!(bot_user(tx,&run,&json!({"bot_id":"B42","username":"new name"}))?.id,bot.id);
        Ok(())
    }).await.unwrap();
    let memberships = db
        .read(|c| {
            Ok(c.query_row("SELECT count(*) FROM memberships", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    assert_eq!(memberships, 2); // original owner plus the active claimable human
    let zone: Option<String> = db
        .read(|c| {
            Ok(
                c.query_row("SELECT time_zone FROM users WHERE name='Claim'", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(zone.as_deref(), Some("America/New_York"));
    let issue: String = db
        .read(|c| Ok(c.query_row("SELECT message FROM slack_import_issues", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(
        issue,
        "Slack user No Mail has no email address; imported as deactivated"
    );
}
#[tokio::test]
async fn slack_users_duplicate_page_identity_rolls_back_all_users_and_mappings() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    let before = db
        .read(|c| Ok(c.query_row("SELECT count(*) FROM users", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let result = db
        .write(move |tx| {
            map_page(
                tx,
                &run,
                &json!([{"id":"USAME","is_bot":true},{"id":"USAME","is_bot":true}]),
                false,
                &HashSet::new(),
            )
        })
        .await;
    assert!(result.unwrap_err().is_record_not_unique());
    assert_eq!(
        db.read(|c| Ok(c.query_row("SELECT count(*) FROM users", [], |r| r.get::<_, i64>(0))?))
            .await
            .unwrap(),
        before
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

#[tokio::test]
async fn slack_review_email_matching_keeps_sqlite_lower_and_ruby_downcase() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/review_regressions.json"
    ))
    .unwrap();
    for case in vectors["emails"].as_array().unwrap() {
        let (db, _, _dir) = setup().await;
        let id = start(&db).await;
        let run = run(&db, id).await;
        let case = case.clone();
        db.write(move |tx| {
            // Use Rails-recorded ids to compare the actual mapping and placeholder row.
            review_owner_id(tx)?;
            tx.conn().execute("INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES(812,'Existing',?,?,?)",params![case["existing"].as_str().unwrap(),tx.now(),tx.now()])?;
            let members = json!([{"id":"UREVIEW","name":"Incoming","profile":{"email":case["incoming"]}}]);
            assert_eq!(map_page(tx,&run,&members,true,&HashSet::new())?,case["preview"],"{} preview",case["incoming"]);
            let imported = map_page(tx,&run,&members,false,&HashSet::new());
            if let Some(error) = case["error"].as_str() {
                assert!(imported.unwrap_err().to_string().contains(error), "{} error", case["incoming"]);
                let counts = tx.conn().query_row("SELECT (SELECT count(*) FROM slack_import_records),(SELECT count(*) FROM users)",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)))?;
                assert_eq!(counts,(case["records"].as_i64().unwrap(),case["users"].as_i64().unwrap()));
                return Ok(());
            }
            assert_eq!(imported?,case["stats"],"{} import",case["incoming"]);
            let actual = tx.conn().query_row("SELECT u.id,u.name,u.email_address,r.created_record FROM slack_import_records r JOIN users u ON u.id=r.record_id WHERE r.slack_key='UREVIEW'",[],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?,"email":r.get::<_,Option<String>>(2)?,"created":r.get::<_,bool>(3)?})))?;
            assert_eq!(actual,case["user"],"{} row",case["incoming"]);
            Ok(())
        }).await.unwrap();
    }
    println!("Slack review email parity: 7 Rails preview/import cases matched");
}

#[tokio::test]
async fn slack_review_bot_handles_remain_exact_case_sensitive_keys() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/review_regressions.json"
    ))
    .unwrap();
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    db.write(move |tx| {
        review_owner_id(tx)?;
        let actual = vectors["handles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| {
                bot_user(tx, &run, &json!({"username":case["name"]}))
                    .map(|user| json!({"name":case["name"],"id":user.id}))
            })
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(json!(actual), vectors["handles"]);
        Ok(())
    })
    .await
    .unwrap();
}

fn review_owner_id(tx: &mut Tx<'_>) -> Result<()> {
    tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON; UPDATE users SET id=811 WHERE id=1; UPDATE slack_workspaces SET configured_by_id=811 WHERE configured_by_id=1; UPDATE slack_connections SET user_id=811 WHERE user_id=1; UPDATE slack_imports SET user_id=811 WHERE user_id=1")?;
    Ok(())
}

use super::*;
use crate::{NewWorkThreadLink, WorkThreadLink};
use serde_json::{Value, json};

fn attributes(value: &Value) -> NewWorkThreadLink {
    NewWorkThreadLink {
        channel_thread_id: value["channel_thread_id"].as_i64().unwrap(),
        created_by_id: value["created_by_id"].as_i64().unwrap(),
        kind: value["kind"].as_str().map(str::to_owned),
        github_pull_request_id: value["github_pull_request_id"].as_i64(),
        event_id: value["event_id"].as_i64(),
        url: value["url"].as_str().map(str::to_owned),
        title: value["title"].as_str().map(str::to_owned),
    }
}
#[test]
fn work_link_models_match_rails_validations_and_persistence() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/work_link_model.json")).unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let t = channel_thread_test::frozen();
        let setup = oracle["setup"].as_array().unwrap().clone();
        let row = row.clone();
        t.write(move |tx| {
            for sql in setup {
                tx.conn().execute_batch(sql.as_str().unwrap())?;
            }
            if !row["duplicate"].is_null() {
                WorkThreadLink::create(tx, attributes(&row["duplicate"]))?;
            }
            let input = attributes(&row["input"]);
            let errors = WorkThreadLink::validate(tx.conn(), &input)?;
            let mut errors_json = serde_json::Map::new();
            for (key, message) in &errors.0 {
                errors_json
                    .entry(key.to_string())
                    .or_insert(json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(json!(message));
            }
            assert_eq!(json!(errors_json), row["errors"], "{}", row["name"]);
            assert_eq!(
                json!(errors.full_messages()),
                row["full_messages"],
                "{}",
                row["name"]
            );
            assert_eq!(errors.is_empty(), row["valid"].as_bool().unwrap());
            let before: i64 =
                tx.conn()
                    .query_row("SELECT COUNT(*) FROM activity_items", [], |row| row.get(0))?;
            let time = crate::ChannelThread::find_by_id(tx.conn(), input.channel_thread_id)?
                .map(|thread| thread.updated_at);
            match WorkThreadLink::create(tx, input.clone()) {
                Ok(link) => {
                    assert!(row["stored"].as_bool().unwrap());
                    assert_eq!(json!(link.title), row["title"]);
                    assert_eq!(json!(link.url), row["url"]);
                    assert_eq!(
                        WorkThreadLink::for_thread(tx.conn(), link.channel_thread_id)?
                            .last()
                            .unwrap()
                            .id,
                        link.id
                    );
                    link.destroy(tx)?;
                    assert!(WorkThreadLink::find(tx.conn(), link.id)?.is_none());
                }
                Err(crate::Error::RecordInvalid(actual)) => assert_eq!(actual, errors),
                Err(error) => return Err(error),
            }
            assert_eq!(
                tx.conn()
                    .query_row("SELECT COUNT(*) FROM activity_items", [], |row| row
                        .get::<_, i64>(0))?,
                before
            );
            assert_eq!(
                crate::ChannelThread::find_by_id(tx.conn(), input.channel_thread_id)?
                    .map(|thread| thread.updated_at),
                time
            );
            Ok(())
        });
    }
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let thread = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Linked work".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        let link = WorkThreadLink::create(
            tx,
            NewWorkThreadLink {
                channel_thread_id: thread.id,
                created_by_id: id("david"),
                kind: Some("drive_file".into()),
                url: Some("https://drive.google.com/file/d/abcdefghij".into()),
                ..Default::default()
            },
        )?;
        thread.destroy(tx)?;
        assert!(WorkThreadLink::find(tx.conn(), link.id)?.is_none());
        Ok(())
    });
}

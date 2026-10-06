use super::*;
use crate::{Error, Message, NewMessage, Room};
use serde_json::{Value, json};

fn golden() -> Value {
    serde_json::from_str(include_str!("ws8_mail_merge_vectors.json")).unwrap()
}

#[test]
fn mail_markdown_entry_uses_shared_rails_validation_and_source() {
    for case in golden()["validation"].as_array().unwrap() {
        let t = TestDb::new();
        let source = case["unit"]
            .as_str()
            .unwrap()
            .repeat(case["count"].as_u64().unwrap() as usize);
        let saved_source = source.clone();
        let attributes = NewMessage {
            room_id: id("designers"),
            creator_id: id("david"),
            body: Some("<p>old supplied body</p>".into()),
            markdown_source: Some("old supplied source".into()),
            streaming: case["streaming"].as_bool().unwrap_or(false),
            drive_file_ids: case["drive_file_ids"]
                .as_array()
                .map(|ids| ids.iter().map(|s| s.as_str().unwrap().to_owned()).collect())
                .unwrap_or_default(),
            action: true,
            embeds_suppressed: true,
            reply_notify_author: Some(false),
            ..Default::default()
        };
        match t.try_write(move |tx| Message::create_markdown(tx, attributes, &source)) {
            Ok(message) => {
                assert_eq!(case["errors"], json!([]), "{}", case["name"]);
                assert_eq!(
                    message.markdown_source.as_deref(),
                    Some(saved_source.as_str()),
                    "{}",
                    case["name"]
                );
                assert_eq!(
                    json!({"action": message.action, "embeds_suppressed": message.embeds_suppressed, "reply_notify_author": message.reply_notify_author}),
                    case["metadata"]
                );
            }
            Err(Error::RecordInvalid(errors)) => {
                assert_eq!(json!(errors.0), case["errors"], "{}", case["name"])
            }
            Err(error) => panic!("{}: {error}", case["name"]),
        }
    }
}

#[test]
fn mail_token_rotation_preserves_direct_name_validation() {
    let t = TestDb::new();
    let rid = id("designers");
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE rooms SET type='Rooms::Direct', name=? WHERE id=?",
            rusqlite::params!["é".repeat(101), rid],
        )?;
        Ok(())
    });
    let error = t
        .try_write(move |tx| Room::find(tx.conn(), rid)?.regenerate_inbound_email_token(tx))
        .unwrap_err();
    let Error::RecordInvalid(errors) = error else {
        panic!("{error}")
    };
    assert_eq!(json!(errors.0), golden()["token_errors"]);
}

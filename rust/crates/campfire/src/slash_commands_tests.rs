//! Verify slash rows through the real app Markdown/Action Text adapter, without parity seeds.
use campfire_db::slash_commands::{Context, dispatch};
use campfire_db::{Config, Database, Env, Message, RecordingSink, TestClock, Timestamp, fixtures};
use serde_json::{Value, json};
use std::sync::Arc;
fn vectors() -> Value {
    let mut vectors: Value =
        serde_json::from_str(include_str!("../../db/src/tests/ws8_slash_vectors.json")).unwrap();
    let review: Value = serde_json::from_str(include_str!(
        "../../db/src/tests/ws8_slash_review_vectors.json"
    ))
    .unwrap();
    vectors["rows"]
        .as_array_mut()
        .unwrap()
        .extend(review["rows"].as_array().unwrap().iter().cloned());
    vectors
}
#[test]
fn slash_runtime_richtext_and_index_rows_match_rails() {
    let vectors = vectors();
    let mut checked = 0;
    for case in vectors["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| !c["message"].is_null())
    {
        let dir = tempfile::tempdir().unwrap();
        let now = Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
        let secrets = rails_compat::Secrets::new(
            "5335c3b1ad35b4ad170c3413bd651ef3b6ed64e257261871a6de3f978cf3868ee417a927040935fb30b0f7debdedb34a2a403e9f34b16cf594c917c2ecd4a995",
        );
        let rich = crate::rich_text::AppRichText::new(
            Arc::new(secrets),
            Arc::new(campfire_kit::clock::FrozenClock::new(now.jiff())),
        );
        let db = Database::open(
            Config::new(dir.path().join("slash.sqlite3")),
            Env {
                clock: Arc::new(TestClock::frozen_at(now)),
                sink: Arc::new(RecordingSink::new()),
                rich_text: Arc::new(rich),
                bcrypt_cost: 4,
                ..Env::default()
            },
        )
        .unwrap();
        let case = case.clone();
        let expected = case.clone();
        let result = db
            .write_blocking(move |tx| {
                fixtures::load(
                    tx.conn(),
                    &fixtures::reference_dir(),
                    &fixtures::Options {
                        now,
                        bcrypt_cost: 4,
                    },
                )?;
                let user = fixtures::identify("david");
                let room = fixtures::identify("watercooler");
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![case["zone"].as_str().unwrap(), user],
                )?;
                let thread = if case["place"] == "thread" {
                    Some(
                        campfire_db::ChannelThread::create(
                            tx,
                            campfire_db::NewChannelThread {
                                room_id: room,
                                creator_id: user,
                                name: Some("Slash test".into()),
                                ..Default::default()
                            },
                        )?
                        .id,
                    )
                } else {
                    None
                };
                dispatch(
                    tx,
                    &Context {
                        user_id: user,
                        room_id: room,
                        thread_id: thread,
                        huddles_configured: false,
                    },
                    case["text"].as_str().unwrap(),
                )
            })
            .unwrap();
        let id = result.message_id.expect("Rails posted this case");
        let(actual,index)=db.read_blocking(|conn| {
            let message=Message::find(conn,id)?;let rich=message.body(conn)?.unwrap();
            let index:String=conn.query_row("SELECT body FROM message_search_index WHERE rowid=?",[id],|r|r.get(0))?;
            Ok((json!({"name":rich.name,"body":rich.body,"record_type":rich.record_type,"created_at":rich.created_at.to_db(),"updated_at":rich.updated_at.to_db()}),json!(index)))
        }).unwrap();
        assert_eq!(actual, expected["rich_text"], "{expected}");
        assert_eq!(index, expected["index"], "{expected}");
        checked += 1;
        if let Ok(output) = std::env::var("WS8_SLASH_EXPORT_DIR") {
            std::fs::create_dir_all(&output).unwrap();
            let export = std::path::Path::new(&output).join(format!("case-{checked}.sqlite3"));
            if export.exists() {
                std::fs::remove_file(&export).unwrap();
            }
            campfire_db::Connection::open(db.path())
                .unwrap()
                .execute("VACUUM INTO ?", [export.to_str().unwrap()])
                .unwrap();
            std::fs::write(export.with_extension("json"),json!({"now":expected["now"],"message_id":id,"user_id":fixtures::identify("david"),"body":expected["rich_text"]["body"],"index":expected["index"]}).to_string()).unwrap();
        }
    }
    assert!(
        checked > 0,
        "the generated corpus must contain posted messages"
    );
    println!("WS8 slash runtime: {checked} Rails-generated rich-text/FTS row comparisons");
}

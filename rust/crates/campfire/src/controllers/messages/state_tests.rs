//! Whole message states from actual records, including cold/warm collection rendering.
use std::sync::Arc;
use campfire_db::{Message, NewMessage, Tx};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter, page, test_support::*};

pub(crate) fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/message-states.json")).unwrap()
}

pub(crate) fn seed(tx: &mut Tx<'_>) -> campfire_db::Result<()> {
    tx.conn().execute("UPDATE users SET icon_name = 'github' WHERE id = ?", [BENDER])?;
    let agent: i64 = tx.conn().query_row("SELECT id FROM agents WHERE user_id = ?", [BENDER], |row| row.get(0))?;
    assert_eq!(agent, oracle()["agent_id"].as_i64().unwrap());
    let source = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
        markdown_source: Some(format!("A long reply source: {}", "🎈 word ".repeat(40))), client_message_id: Some("states-source".into()), ..Default::default() })?;
    assert_eq!(source.id, oracle()["source_id"].as_i64().unwrap());
    Ok(())
}

pub(crate) fn create_state(tx: &mut Tx<'_>, row: &Value) -> campfire_db::Result<Message> {
    let input = &row["input"];
    let name = row["name"].as_str().unwrap();
    let text = |key: &str| input[key].as_str().map(str::to_owned);
    let message = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: input["creator_id"].as_i64().unwrap_or(DAVID),
        client_message_id: Some(format!("states-{name}")), body: text("body"), markdown_source: text("markdown_source"),
        system_note: input["system_note"].as_bool().unwrap_or(false), streaming: input["streaming"].as_bool().unwrap_or(false),
        action: input["action"].as_bool().unwrap_or(false), forwarded_markdown: input["forwarded_markdown"].as_bool().unwrap_or(false),
        forwarded_at: text("forwarded_at").map(|_| tx.now()), forwarded_from_message_id: input["forwarded_from_message_id"].as_i64(),
        forward_note: text("forward_note"), reply_to_message_id: input["reply_to_message_id"].as_i64(), ..Default::default() })?;
    assert_eq!(message.id, row["id"].as_i64().unwrap());
    match name {
        "deleted_reply" => Message::find(tx.conn(), oracle()["source_id"].as_i64().unwrap())?.destroy(tx)?,
        "steps" => {
            for (index, step) in oracle()["steps"].as_array().unwrap().iter().enumerate() {
                tx.conn().execute("INSERT INTO agent_steps (agent_id, message_id, position, name, status, duration_ms, input_summary, output_summary, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![oracle()["agent_id"].as_i64().unwrap(), message.id, index as i64, step["name"].as_str(), step["status"].as_str(), step["duration_ms"].as_i64(), step["input_summary"].as_str(), step["output_summary"].as_str(), tx.now(), tx.now()])?;
            }
        },
        _ => {},
    }
    Message::find(tx.conn(), message.id)
}

#[tokio::test]
async fn complete_message_states_match_rails_on_cache_misses_and_hits() {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    app.db().write(seed).await.unwrap();
    for row in oracle()["rows"].as_array().unwrap() {
        let fixture = row.clone();
        let message = app.db().write(move |tx| create_state(tx, &fixture)).await.unwrap();
        for _ in 0..2 {
            let runtime = app.booted.app.clone();
            let record = message.clone();
            let html = app.db().read(move |conn| {
                let mut presenter = Presenter::new(conn, &runtime, None);
                presenter.cache_base_url = Some("http://campfire.test".into());
                campfire_views::fragment_cache::with(&runtime.fragment_cache, || {
                    let item = presenter.message_item(&record)?;
                    let account = campfire_db::Account::first(conn)?;
                    Ok(page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx|
                        campfire_views::messages::cached_message_item(ctx, &item).to_string()))
                })
            }).await.unwrap();
            let expected = row["html"].as_str().unwrap();
            if html != expected { rails_mismatch(&html, expected, row["name"].as_str().unwrap()); }
            assert_eq!(campfire_cable::turbo::session_bound(&html), None);
        }
    }
}

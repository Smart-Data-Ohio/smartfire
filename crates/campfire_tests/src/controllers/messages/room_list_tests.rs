use std::sync::Arc;
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Timeline};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter, test_support::*};

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/room-list.json")).unwrap() }

#[tokio::test]
async fn room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages() {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let (ids, child, foreign) = app.db().write(|tx| {
        let ids = (0..85).map(|index| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some(format!("Unread {index}")), client_message_id: Some(format!("unread-{index}")), ..Default::default() }).map(|m| m.id)).collect::<campfire_db::Result<Vec<_>>>()?;
        let thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, parent_message_id: Some(ids[0]), name: Some("Unread thread".into()), ..Default::default() })?;
        let child = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id), markdown_source: Some("Child".into()), client_message_id: Some("unread-child".into()), ..Default::default() })?;
        let foreign = Message::create(tx, NewMessage { room_id: QUIET_CORNER, creator_id: DAVID, markdown_source: Some("Foreign".into()), client_message_id: Some("unread-foreign".into()), ..Default::default() })?;
        Ok((ids, child.id, foreign.id))
    }).await.unwrap();
    assert_eq!(serde_json::json!(ids), oracle()["message_ids"]);
    assert_eq!(child, oracle()["child_id"].as_i64().unwrap());
    assert_eq!(foreign, oracle()["foreign_id"].as_i64().unwrap());
    for row in oracle()["rows"].as_array().unwrap() {
        let row = row.clone();
        let expected = row["html"].as_str().unwrap().to_owned();
        let name = row["name"].as_str().unwrap().to_owned();
        let runtime = app.booted.app.clone();
        let html = app.db().read(move |conn| {
            // The same root-only selection WS8b-r's room_shell::find_messages supplies.
            let anchor = row["anchor"].as_i64().map(|id| Message::find_by_id(conn, id)).transpose()?.flatten()
                .filter(|m| m.room_id == ALL_TALK && m.thread_id.is_none());
            let records = if let Some(anchor) = anchor { Message::page_around(conn, Timeline::Room(ALL_TALK), &anchor)? }
                else { Message::last_page(conn, Timeline::Room(ALL_TALK))? };
            assert_eq!(serde_json::json!(records.iter().map(|m| m.id).collect::<Vec<_>>()), row["ids"]);
            let mut presenter = Presenter::new(conn, &runtime, None);
            presenter.cache_base_url = Some("http://campfire.test".into());
            campfire_views::fragment_cache::with(&runtime.fragment_cache, ||
                presenter.room_message_list(&records, row["divider_id"].as_i64(), row["count"].as_i64().unwrap()))
        }).await.unwrap();
        if html != expected { rails_mismatch(&html, &expected, &name); }
        assert_eq!(campfire_cable::turbo::session_bound(&html), None);
    }
}

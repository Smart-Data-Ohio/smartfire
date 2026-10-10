//! Complete owned room slots, using the merged GitHub renderer and pinned Rails collections.
use std::sync::Arc;
use campfire_db::{Message, Timeline};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter, test_support::*};

#[tokio::test]
async fn complete_github_containers_match_rails_in_room_lists_on_cold_and_warm_caches() {
    let oracle: Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/room-components.json")).unwrap();
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let room_id = row["room_id"].as_i64().unwrap();
        for warm in [false, true] {
            let runtime = app.booted.app.clone();
            let row = row.clone();
            app.db().read(move |conn| {
                let mut p = Presenter::new(conn, &runtime, Some("campfire.test".into()));
                p.cache_base_url = Some("http://campfire.test".into());
                let messages = Message::last_page(conn, Timeline::Room(room_id))?;
                assert_eq!(serde_json::json!(messages.iter().map(|m| m.id).collect::<Vec<_>>()), row["root_ids"]);
                let html = campfire_views::fragment_cache::with(&runtime.fragment_cache, || p.room_message_list(&messages, None, 0))?;
                let expected = row["message_list"].as_str().unwrap();
                for message in &messages {
                    let github = format!("<div id=\"github_pr_cards_message_{}\"", message.client_message_id);
                    let twitter = format!("<div id=\"twitter_cards_message_{}\"", message.client_message_id);
                    let slot = |body: &str| body.find(&github).map(|start| {
                        let end = body[start..].find(&twitter).expect("following Twitter container") + start;
                        body[start..end].to_owned()
                    });
                    // This owns the entire card container, including blank/suppressed states.
                    // Other message features are compared by their separately owned fixtures.
                    let actual = slot(&html);
                    let wanted = slot(expected);
                    if actual != wanted { rails_mismatch(actual.as_deref().unwrap_or(""), wanted.as_deref().unwrap_or(""), &format!("room {room_id}, message {}, warm={warm}", message.id)); }
                }

                Ok(())
            }).await.unwrap();
        }
    }
}

use campfire_web::controllers::presenters::RoomList;

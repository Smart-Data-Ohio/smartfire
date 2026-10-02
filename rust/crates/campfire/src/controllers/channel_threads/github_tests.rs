//! The authorized thread show caller consumes WS15g's real public/private header.
use std::sync::Arc;
use campfire_kit::clock::FrozenClock;
use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn github_thread_show_matches_complete_rails_public_private_and_unknown_bodies() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/github-thread-page.json")).unwrap();
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let (room, thread, pr) = (oracle["room_id"].as_i64().unwrap(), oracle["thread_id"].as_i64().unwrap(), oracle["pull_request_id"].as_i64().unwrap());
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?", (room, KEVIN))?;
        Ok(())
    }).await.unwrap();
    let path = format!("/rooms/{room}/threads/{thread}");
    for row in oracle["rows"].as_array().unwrap() {
        let private = row["private"].as_bool();
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE github_pull_requests SET private=? WHERE id=?", (private, pr))?;
            Ok(())
        }).await.unwrap();
        let response = app.david().get(&path).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16);
        let actual = response.text();
        let body = actual.split("<main class=\"thread\"").nth(1).expect("standalone thread").split("</main>").next().unwrap();
        let body = format!("\n<main class=\"thread\"{body}</main>");
        let expected = row["body"].as_str().unwrap();
        if body != expected { rails_mismatch(&body, expected, &format!("PR private={private:?}")); }
        assert!(actual.contains("id=\"github_write_actions_channel_thread_8\""));
        assert_eq!(app.sign_in(KEVIN).await.get(&path).await.status, 404);
        assert!(!app.sign_in(KEVIN).await.get(&path).await.text().contains("Port the launch checklist"));
    }
}

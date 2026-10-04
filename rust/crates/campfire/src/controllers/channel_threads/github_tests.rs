//! The authorized thread show caller consumes WS15g's real public/private header.
use crate::controllers::presenters::test_support::*;
use campfire_kit::clock::FrozenClock;
use std::sync::Arc;

#[tokio::test]
async fn github_thread_show_matches_complete_rails_public_private_and_unknown_bodies() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/github-thread-page.json"
    ))
    .unwrap();
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap())))
        .await
        .unwrap();
    let (room, thread, pr) = (
        oracle["room_id"].as_i64().unwrap(),
        oracle["thread_id"].as_i64().unwrap(),
        oracle["pull_request_id"].as_i64().unwrap(),
    );
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                (room, KEVIN),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/rooms/{room}/threads/{thread}");
    for row in oracle["rows"].as_array().unwrap() {
        let private = row["private"].as_bool();
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE github_pull_requests SET private=? WHERE id=?",
                    (private, pr),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let response = app.david().get(&path).await;
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16
        );
        let actual = response.text();
        let body = actual
            .split("<main class=\"thread\"")
            .nth(1)
            .expect("standalone thread")
            .split("</main>")
            .next()
            .unwrap();
        let body = format!("\n<main class=\"thread\"{body}</main>");
        let expected = row["body"].as_str().unwrap();
        if body != expected {
            rails_mismatch(&body, expected, &format!("PR private={private:?}"));
        }
        assert!(actual.contains("id=\"github_write_actions_channel_thread_8\""));
        assert_eq!(app.sign_in(KEVIN).await.get(&path).await.status, 404);
        assert!(
            !app.sign_in(KEVIN)
                .await
                .get(&path)
                .await
                .text()
                .contains("Port the launch checklist")
        );
    }
}

// test/integration/github_pr_threads_test.rb:35,122: persisted files must reach the
// public thread header, while a private header exposes only its lazy card frame.
#[tokio::test]
async fn github_thread_http_renders_populated_files_and_hides_private_filenames() {
    use serde_json::{Value, json};
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/github-thread-page.json"
    ))
    .unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let (room, thread, pr) = (
        oracle["room_id"].as_i64().unwrap(),
        oracle["thread_id"].as_i64().unwrap(),
        oracle["pull_request_id"].as_i64().unwrap(),
    );
    let files = json!({"files":[
        {"filename":"app/models/user.rb","additions":10,"deletions":2,"status":"modified"},
        {"filename":"app/models/new.rb","additions":5,"deletions":0,"status":"added"}
    ],"total_count":5})
    .to_string();
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Add shiny things',changed_files=?,changed_files_fetched_at=?,fetched_at=? WHERE id=?", (files,tx.now(),tx.now(),pr))?;
        let mut conversation = campfire_db::ChannelThread::find(tx.conn(), thread)?;
        conversation.post_message(tx, DAVID, campfire_db::NewMessage {
            markdown_source: Some("first reply".into()),
            ..Default::default()
        })?;
        Ok(())
    }).await.unwrap();
    let path = format!("/rooms/{room}/threads/{thread}");
    let response = app.david().get(&path).await;
    assert_eq!(response.status, 200);
    let html = response.text();
    // The files follow the card's nested divs; fence the whole header at its write frame.
    let header = html
        .split("class=\"github-pr-thread-header\"")
        .nth(1)
        .unwrap()
        .split("<turbo-frame class=\"github-pr-write\"")
        .next()
        .unwrap();
    assert!(header.contains("class=\"github-pr-card github-pr-card--open\""));
    assert_eq!(header.matches("class=\"github-pr-card ").count(), 1);
    assert!(header.contains("class=\"github-pr-card__title\">Add shiny things</p>"));
    assert!(header.contains("class=\"github-pr-files__heading\">Files changed</h2>"));
    assert_eq!(header.matches("class=\"github-pr-files__file\"").count(), 2);
    for text in [
        "app/models/user.rb",
        "app/models/new.rb",
        ">Modified</span>",
        ">Added</span>",
        "+10 −2</span>",
        "+5 −0</span>",
        "and 3 more on GitHub",
    ] {
        assert!(header.contains(text), "missing {text}: {header}");
    }
    assert!(!header.contains("Loading files"));
    assert!(html.contains("first reply"));
    assert!(
        html.find("github-pr-thread-header").unwrap()
            < html.find("aria-label=\"Thread messages\"").unwrap()
    );
    assert!(html.find("github-pr-thread-header").unwrap() < html.find("first reply").unwrap());

    app.db().write(move |tx| {
        let files = json!({"files":[{"filename":"app/models/secret.rb","additions":3,"deletions":1,"status":"modified"}],"total_count":1}).to_string();
        tx.conn().execute("UPDATE github_pull_requests SET private=1,changed_files=? WHERE id=?", (files,pr))?;
        Ok(())
    }).await.unwrap();
    let response = app.david().get(&path).await;
    assert_eq!(response.status, 200);
    let html = response.text();
    let frame = format!(
        "<turbo-frame loading=\"lazy\" class=\"github-pr-card-frame\" id=\"card_for_thread_{thread}_github_pull_request_{pr}\" src=\"/rooms/{room}/github/pull_requests/{pr}/card?thread_id={thread}\"></turbo-frame>"
    );
    assert_eq!(html.matches(&frame).count(), 1);
    assert!(html.contains(&format!(
        "src=\"/rooms/{room}/github/pull_requests/{pr}/card?thread_id={thread}\""
    )));
    assert!(html.contains("loading=\"lazy\""));
    for hidden in [
        "class=\"github-pr-card ",
        "class=\"github-pr-files\"",
        "Add shiny things",
        "app/models/secret.rb",
    ] {
        assert!(!html.contains(hidden), "private thread leaked {hidden}");
    }
}

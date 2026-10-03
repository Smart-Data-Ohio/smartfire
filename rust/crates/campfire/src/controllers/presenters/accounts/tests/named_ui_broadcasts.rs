//! UI-owned names from WS11's inventory. Rails generator uses the same pubsub as its named tests.
use super::agent_broadcasts::{human_socket, receive};
use super::{Test, boot_seed_with_clock};
use futures_util::SinkExt;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-named-ui-broadcasts.json"
    ))
    .unwrap()
}
async fn boot() -> Test {
    boot_seed_with_clock(
        "default",
        std::sync::Arc::new(campfire_kit::FrozenClock::new(
            "2026-09-23T16:00:00Z".parse().unwrap(),
        )),
    )
    .await
    .unwrap()
}
fn equivalent(actual: &str, expected: &str) -> bool {
    actual == expected
}
fn no_private(html: &str, values: &[Value]) -> bool {
    values.iter().all(|v| !html.contains(v.as_str().unwrap()))
}
async fn agent_frames(secret_case: bool) {
    let t = boot().await;
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = t.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut socket = human_socket(&t, address, 127326141).await;
    let identifier = json!({"channel":"AgentsChannel"}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
    t.booted
        .app
        .db
        .write(|tx| {
            let mut a = campfire_db::Agent::find(tx.conn(), 773018776)?.unwrap();
            a.update(
                tx,
                campfire_db::AgentChanges {
                    status: Some("working".into()),
                    status_note: Some(Some("on it".into())),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let corpus = oracle();
    for row in corpus["status"].as_array().unwrap() {
        let frame = receive(&mut socket).await;
        assert_eq!(frame["identifier"], identifier);
        let html = frame["message"].as_str().unwrap();
        assert!(
            equivalent(html, row["html"].as_str().unwrap()),
            "{html}\nRails {}",
            row["html"]
        );
        // Wrong target/action/content all fail the byte comparator independently.
        for broken in [
            html.replacen("replace", "update", 1),
            html.replacen("agent_773018776", "agent_0", 1),
            html.replace("Working", "Idle"),
        ] {
            assert!(!equivalent(&broken, row["html"].as_str().unwrap()));
        }
    }
    if secret_case {
        t.booted
            .app
            .db
            .write(|tx| {
                campfire_db::AgentGrant::create(
                    tx,
                    campfire_db::NewGrant {
                        agent_id: 773018776,
                        granted_by_id: 127326141,
                        capability: "post_messages".into(),
                        ..Default::default()
                    },
                )?;
                let mut a = campfire_db::Agent::find(tx.conn(), 773018776)?.unwrap();
                a.update(
                    tx,
                    campfire_db::AgentChanges {
                        status: Some("failed".into()),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        for row in corpus["secrets"].as_array().unwrap() {
            let frame = receive(&mut socket).await;
            let html = frame["message"].as_str().unwrap();
            assert!(equivalent(html, row["html"].as_str().unwrap()), "{html}");
            let private = corpus["private_values"].as_array().unwrap();
            assert!(no_private(html, private));
            for value in private {
                assert!(
                    !no_private(&format!("{html}{}", value.as_str().unwrap()), private),
                    "secret-leak control was accepted"
                );
            }
        }
    }
    // A committed wrong status comes through the same writer and live renderer, and is rejected.
    t.booted
        .app
        .db
        .write(|tx| {
            let mut a = campfire_db::Agent::find(tx.conn(), 773018776)?.unwrap();
            a.update(
                tx,
                campfire_db::AgentChanges {
                    status: Some("waiting".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    for row in corpus["status"].as_array().unwrap() {
        let frame = receive(&mut socket).await;
        assert!(!equivalent(
            frame["message"].as_str().unwrap(),
            row["html"].as_str().unwrap()
        ));
    }
    socket.close(None).await.unwrap();
    server.abort();
}
#[tokio::test]
async fn ws11ui_status_change_broadcasts_badge_and_directory_row_replaces_to_agents_all() {
    agent_frames(false).await;
}
#[tokio::test]
async fn ws11ui_status_broadcasts_carry_no_credentials_or_grants() {
    agent_frames(true).await;
}
#[tokio::test]
async fn ws11ui_ooo_broadcasts_the_badge_and_the_notice() {
    let t = boot().await;
    t.booted.app.db.write(|tx|{tx.conn().execute("UPDATE users SET time_zone='America/New_York',ooo_until=NULL,ooo_note=NULL WHERE id=127326141",[])?;Ok(())}).await.unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = t.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut socket = human_socket(&t, address, 127326141).await;
    let gid = crate::channels::user_gid(127326141).to_param();
    let mut identifiers = Vec::new();
    for stream in ["status", "ooo_notice"] {
        let signed =
            rails_compat::turbo::signed_stream_name(&t.booted.app.secrets, &[&gid, stream]);
        let identifier =
            json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}).to_string();
        socket
            .send(Message::Text(
                json!({"command":"subscribe","identifier":identifier})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
        identifiers.push(identifier);
    }
    let mut browser = t.browser("198.51.100.219");
    browser.sign_in(&t.label("emails.david")).await;
    let response = browser
        .request(
            axum::http::Method::POST,
            &format!("/rooms/{}/slash_commands", t.label("rooms.watercooler")),
            &[("accept", "text/vnd.turbo-stream.html")],
            Some((
                "application/x-www-form-urlencoded",
                "text=%2Fooo+tomorrow+Back+soon".into(),
            )),
        )
        .await;
    assert_eq!(
        response.status,
        axum::http::StatusCode::OK,
        "{}",
        response.text()
    );
    let corpus = oracle();
    // Independent cable streams can arrive in either order. Check each stream once.
    let mut received = std::collections::BTreeMap::new();
    for _ in 0..2 {
        let frame = receive(&mut socket).await;
        let id = frame["identifier"].as_str().unwrap().to_owned();
        assert!(identifiers.contains(&id));
        assert!(
            received.insert(id, frame).is_none(),
            "duplicate stream broadcast"
        );
    }
    for (index, row) in corpus["ooo"].as_array().unwrap().iter().enumerate() {
        let frame = &received[&identifiers[index]];
        assert_eq!(frame["identifier"], identifiers[index]);
        let html = frame["message"].as_str().unwrap();
        assert!(
            equivalent(html, row["html"].as_str().unwrap()),
            "{html}\nRails {}",
            row["html"]
        );
        assert!(!equivalent(
            &html.replacen("update", "replace", 1),
            row["html"].as_str().unwrap()
        ));
    }
    socket.close(None).await.unwrap();
    server.abort();
}

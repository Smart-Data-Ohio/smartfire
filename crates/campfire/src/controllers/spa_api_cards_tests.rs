//! S3 polls and cards on `/api/v1` (`campfire_api::cards`): posting and voting on polls with
//! their `poll.updated` and `poll.ballot` twins, event responses, the cards a message carries
//! with their `message.cards` twin, and the per-viewer previews.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve};
use crate::controllers::presenters::test_support::{
    Browser, DAVID, HQ, JASON, KEVIN, Reply, Req, TestApp,
};

/// The seed's card fixtures, all in Designers (David, Jason and Kevin are members; Kevin isn't
/// in All Pets, where the quoted message is).
const DESIGNERS: i64 = 654632876;
const ALL_PETS: i64 = 104393281;
/// Single choice, open until the next day; David voted "Online" (option 1).
const OPEN_POLL: i64 = 1;
const OPEN_POLL_MESSAGE: i64 = 935962047;
/// Closed by the periodic closer.
const CLOSED_POLL: i64 = 2;
const CLOSED_POLL_MESSAGE: i64 = 935962048;
const PR_MESSAGE: i64 = 935962053;
const FIZZY_MESSAGE: i64 = 935962054;
const LINK_MESSAGE: i64 = 935962055;
const LINKEDIN_MESSAGE: i64 = 935962056;
/// Quotes a message in All Pets (reference 1).
const QUOTE_MESSAGE: i64 = 935962057;
/// A post on X in All Pets.
const X_MESSAGE: i64 = 935961886;
const LAUNCH_PARTY: i64 = 390339825;
const SPRINT_RETRO_CANCELLED: i64 = 439674037;
const WATERCOOLER_IN_ALL_TALK: i64 = 411254270;

async fn send(b: &mut Browser<'_>, method: Method, path: &str, body: Value) -> Reply {
    b.write(json_body(method, path, &body)).await
}

fn ok<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(reply)
}

fn fields(reply: &Reply) -> Vec<String> {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    fields.into_keys().collect()
}

fn poll_body(client_message_id: &str, options: &[&str]) -> Value {
    json!({
        "clientMessageId": client_message_id,
        "question": "Where should we eat?",
        "options": options,
        "multiple": false,
        "anonymous": false,
        "closesAt": null,
    })
}

async fn vote(b: &mut Browser<'_>, room_id: i64, poll_id: i64, option_ids: &[i64]) -> Reply {
    send(
        b,
        Method::POST,
        &format!("/api/v1/rooms/{room_id}/polls/{poll_id}/vote"),
        json!({ "optionIds": option_ids }),
    )
    .await
}

async fn end_poll(b: &mut Browser<'_>, room_id: i64, poll_id: i64) -> Reply {
    send(
        b,
        Method::POST,
        &format!("/api/v1/rooms/{room_id}/polls/{poll_id}/end"),
        json!({}),
    )
    .await
}

#[tokio::test]
async fn parity_poll_author_and_admin_can_end_once_and_stop_voting() {
    let Some(a) = app(true).await else { return };
    sql(&a, "UPDATE users SET role = 0 WHERE id = ?", vec![JASON]).await;
    let mut author = a.sign_in(KEVIN).await;
    let mut admin = a.sign_in(DAVID).await;
    let mut other = a.sign_in(JASON).await;
    for actor in [KEVIN, DAVID] {
        let reply = send(
            &mut author,
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/polls"),
            poll_body(&format!("end-{actor}"), &["Yes", "No"]),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let poll = parse::<api::MessageDTO>(&reply).poll.unwrap();
        let choice = poll.options[0].id;
        ok::<api::PollResults>(&vote(&mut author, HQ, poll.id, &[choice]).await);
        let denied = end_poll(&mut other, HQ, poll.id).await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.text());
        let open: api::PollResults = ok(&author
            .send(get(&format!("/api/v1/rooms/{HQ}/polls/{}", poll.id)))
            .await);
        assert!(!open.poll.closed);
        let ended: api::PollResults = ok(&end_poll(
            if actor == KEVIN {
                &mut author
            } else {
                &mut admin
            },
            HQ,
            poll.id,
        )
        .await);
        assert!(ended.poll.closed && ended.poll.closed_at.is_some());
        assert_eq!(ended.poll.total_votes, 1);
        let again: api::PollResults = ok(&end_poll(&mut author, HQ, poll.id).await);
        assert_eq!(again.poll.closed_at, ended.poll.closed_at);
        assert_eq!(counts(&again.poll), counts(&ended.poll));
        assert_eq!(fields(&vote(&mut author, HQ, poll.id, &[]).await), ["poll"]);
        assert_eq!(
            end_poll(&mut other, HQ, poll.id).await.status,
            StatusCode::FORBIDDEN
        );
    }
    let before: api::PollResults = ok(&admin
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/polls/{CLOSED_POLL}"
        )))
        .await);
    let already: api::PollResults = ok(&end_poll(&mut admin, DESIGNERS, CLOSED_POLL).await);
    assert_eq!(already.poll.closed_at, before.poll.closed_at);
    assert_eq!(
        end_poll(&mut admin, HQ, OPEN_POLL).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        end_poll(&mut author, ALL_PETS, OPEN_POLL).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn parity_poll_thread_creation_and_end_reach_live_viewers() {
    for threaded in [false, true] {
        let Some(a) = app(true).await else { return };
        let thread_id = if threaded {
            Some(
                a.db()
                    .write(|tx| {
                        Ok(campfire_db::ChannelThread::create(
                            tx,
                            campfire_db::NewChannelThread {
                                room_id: DESIGNERS,
                                creator_id: DAVID,
                                name: Some("Poll thread".into()),
                                ..Default::default()
                            },
                        )?
                        .id)
                    })
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let topic =
            thread_id.map_or_else(|| format!("room:{DESIGNERS}"), |id| format!("thread:{id}"));
        let (addr, server) = serve(&a).await;
        let mut david = a.sign_in(DAVID).await;
        let mut kevin = a.sign_in(KEVIN).await;
        let mut sync =
            Sync::connect(addr, &kevin.cookie_header(), std::slice::from_ref(&topic)).await;
        sync.welcome().await;
        let mut body = poll_body("thread-poll", &["Yes", "No"]);
        body["threadId"] = json!(thread_id);
        body["anonymous"] = json!(true);
        let reply = send(
            &mut david,
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/polls"),
            body.clone(),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let posted: api::MessageDTO = parse(&reply);
        assert_eq!(posted.thread_id, thread_id);
        let poll = posted.poll.unwrap();
        let event = sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == posted.id), |_| false).await;
        assert_eq!(event.topic, topic);
        let again: api::MessageDTO = ok(&send(
            &mut david,
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/polls"),
            body,
        )
        .await);
        assert_eq!(again.id, posted.id);
        ok::<api::PollResults>(&vote(&mut david, DESIGNERS, poll.id, &[poll.options[0].id]).await);
        sync.until(poll_updated(poll.id), any_ballot).await;
        let ended: api::PollResults = ok(&end_poll(&mut david, DESIGNERS, poll.id).await);
        let event = sync.until(|event| matches!(&event.payload, api::SyncPayload::PollUpdated(updated) if updated.poll.id == poll.id && updated.poll.closed), any_ballot).await;
        assert_eq!(event.topic, topic);
        let api::SyncPayload::PollUpdated(updated) = event.payload else {
            unreachable!()
        };
        assert_eq!((updated.room_id, updated.thread_id), (DESIGNERS, thread_id));
        assert_eq!(counts(&updated.poll), [(1, vec![]), (0, vec![])]);
        assert_eq!(updated.poll.closed_at, ended.poll.closed_at);
        let final_results: api::PollResults = ok(&kevin
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/polls/{}", poll.id)))
            .await);
        assert!(final_results.poll.closed && final_results.my_option_ids.is_empty());
        assert_eq!(
            fields(&vote(&mut kevin, DESIGNERS, poll.id, &[poll.options[1].id]).await),
            ["poll"]
        );
        server.abort();
    }
}

#[tokio::test]
async fn parity_poll_rejects_locked_closed_and_foreign_threads_without_posting() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    for (thread_id, room_id, status) in [
        (3, DESIGNERS, StatusCode::FORBIDDEN),
        (2, DESIGNERS, StatusCode::FORBIDDEN),
        (1, HQ, StatusCode::NOT_FOUND),
        (99999999, DESIGNERS, StatusCode::NOT_FOUND),
    ] {
        let client_id = format!("denied-thread-{thread_id}-{room_id}");
        let mut body = poll_body(&client_id, &["Yes", "No"]);
        body["threadId"] = json!(thread_id);
        let reply = send(
            &mut david,
            Method::POST,
            &format!("/api/v1/rooms/{room_id}/polls"),
            body,
        )
        .await;
        assert_eq!(reply.status, status, "{}", reply.text());
        a.db()
            .read(move |conn| {
                assert!(
                    campfire_db::Message::find_duplicate(conn, room_id, DAVID, &client_id)?
                        .is_none()
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}

async fn message(b: &mut Browser<'_>, id: i64) -> api::MessageDTO {
    ok::<api::MessageRead>(&b.send(get(&format!("/api/v1/messages/{id}"))).await).message
}

fn poll_updated(poll_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::PollUpdated(updated) if updated.poll.id == poll_id)
}

fn ballot(poll_id: i64, mine: Vec<i64>) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::PollBallot(ballot) if ballot.poll_id == poll_id && ballot.my_option_ids == mine)
}

fn any_ballot(event: &api::SyncEvent) -> bool {
    matches!(event.payload, api::SyncPayload::PollBallot(_))
}

fn counts(poll: &api::Poll) -> Vec<(i64, Vec<i64>)> {
    poll.options
        .iter()
        .map(|option| (option.votes, option.voter_ids.clone()))
        .collect()
}

async fn sql(a: &TestApp, statement: &'static str, params: Vec<i64>) {
    a.db()
        .write(move |tx| {
            tx.conn()
                .execute(statement, rusqlite::params_from_iter(params))?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn posting_a_poll_answers_its_question_once_and_checks_it() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{HQ}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/rooms/{HQ}/polls");

    let reply = send(
        &mut david,
        Method::POST,
        &path,
        poll_body("poll-1", &["Pizza", " Tacos ", "  "]),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: api::MessageDTO = parse(&reply);
    assert_eq!(posted.client_message_id, "poll-1");
    assert_eq!(
        posted.markdown_source.as_deref(),
        Some("Where should we eat?")
    );
    let poll = posted.poll.clone().expect("the poll");
    assert_eq!(poll.message_id, posted.id);
    assert_eq!(
        poll.options
            .iter()
            .map(|option| option.label.as_str())
            .collect::<Vec<_>>(),
        ["Pizza", "Tacos"]
    );
    assert!(!poll.multiple && !poll.anonymous && !poll.closed && poll.total_votes == 0);
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == posted.id),
            |_| false,
        )
        .await;
    let api::SyncPayload::MessageCreated(created) = event.payload else {
        unreachable!()
    };
    assert_eq!(created.poll.map(|poll| poll.id), Some(poll.id));

    // A retry answers the question already posted, and posts no second poll.
    let reply = send(
        &mut david,
        Method::POST,
        &path,
        poll_body("poll-1", &["Pizza", "Tacos"]),
    )
    .await;
    let again: api::MessageDTO = ok(&reply);
    assert_eq!(again.id, posted.id);
    let polls: i64 = a
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM polls WHERE message_id = ?",
                [posted.id],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(polls, 1);

    let mut invalid = poll_body("poll-2", &["Pizza", "Tacos"]);
    invalid["question"] = json!("   ");
    assert_eq!(
        fields(&send(&mut david, Method::POST, &path, invalid).await),
        ["question"]
    );
    for options in [
        vec!["Only one".to_string(), " ".into()],
        (0..11).map(|n| format!("Option {n}")).collect(),
        vec!["Fine".into(), "x".repeat(201)],
    ] {
        let options: Vec<&str> = options.iter().map(String::as_str).collect();
        let reply = send(
            &mut david,
            Method::POST,
            &path,
            poll_body("poll-2", &options),
        )
        .await;
        assert_eq!(fields(&reply), ["options"]);
    }
    let mut past = poll_body("poll-2", &["Pizza", "Tacos"]);
    past["closesAt"] = json!("2026-03-01T12:00:00.000Z");
    assert_eq!(
        fields(&send(&mut david, Method::POST, &path, past).await),
        ["closesAt"]
    );
    let reply = send(
        &mut david,
        Method::POST,
        &path,
        poll_body(" ", &["Pizza", "Tacos"]),
    )
    .await;
    assert_eq!(fields(&reply), ["clientMessageId"]);

    // Bots don't post polls.
    sql(&a, "UPDATE users SET role = 2 WHERE id = ?", vec![DAVID]).await;
    let reply = send(
        &mut david,
        Method::POST,
        &path,
        poll_body("poll-3", &["Pizza", "Tacos"]),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    server.abort();
}

#[tokio::test]
async fn votes_tell_the_room_and_the_voters_own_tabs() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let topics = [format!("room:{DESIGNERS}")];
    let mut davids_tab = Sync::connect(addr, &david.cookie_header(), &topics).await;
    davids_tab.welcome().await;
    let mut kevins_tab = Sync::connect(addr, &kevin.cookie_header(), &topics).await;
    kevins_tab.welcome().await;
    let poll_path = format!("/api/v1/rooms/{DESIGNERS}/polls/{OPEN_POLL}");

    let results: api::PollResults = ok(&david.send(get(&poll_path)).await);
    assert_eq!(results.my_option_ids, [1]);
    assert_eq!(
        counts(&results.poll),
        [(1, vec![DAVID]), (0, vec![]), (0, vec![])]
    );
    let results: api::PollResults = ok(&kevin.send(get(&poll_path)).await);
    assert!(results.my_option_ids.is_empty());

    // Kevin votes: the room hears the counts, and only Kevin's tabs his ballot.
    let results: api::PollResults = ok(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[2]).await);
    assert_eq!(results.my_option_ids, [2]);
    assert_eq!(results.poll.total_votes, 2);
    assert_eq!(
        counts(&results.poll),
        [(1, vec![DAVID]), (1, vec![KEVIN]), (0, vec![])]
    );
    let event = kevins_tab.until(poll_updated(OPEN_POLL), any_ballot).await;
    assert_eq!(event.topic, format!("room:{DESIGNERS}"));
    let api::SyncPayload::PollUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert_eq!((updated.room_id, updated.thread_id), (DESIGNERS, None));
    assert_eq!(updated.poll.total_votes, 2);
    assert!(updated.poll.as_of >= results.poll.as_of);
    let event = kevins_tab
        .until(ballot(OPEN_POLL, vec![2]), |_| false)
        .await;
    assert_eq!(event.topic, "user");
    let api::SyncPayload::PollBallot(sent) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        (sent.message_id, sent.room_id, sent.thread_id),
        (OPEN_POLL_MESSAGE, DESIGNERS, None)
    );
    davids_tab.until(poll_updated(OPEN_POLL), any_ballot).await;

    // A single-choice poll takes one of its own options.
    assert_eq!(
        fields(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[1, 2]).await),
        ["optionIds"]
    );
    assert_eq!(
        fields(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[4]).await),
        ["optionIds"]
    );
    let results: api::PollResults = ok(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[2, 2]).await);
    assert_eq!(results.my_option_ids, [2]);

    // David changes his vote: his tab hears his ballot, never Kevin's.
    let results: api::PollResults = ok(&vote(&mut david, DESIGNERS, OPEN_POLL, &[3]).await);
    assert_eq!(
        counts(&results.poll),
        [(0, vec![]), (1, vec![KEVIN]), (1, vec![DAVID])]
    );
    davids_tab
        .until(ballot(OPEN_POLL, vec![3]), |event| {
            matches!(&event.payload, api::SyncPayload::PollBallot(ballot) if ballot.my_option_ids != [3])
        })
        .await;

    // An empty ballot takes the vote back.
    let results: api::PollResults = ok(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[]).await);
    assert!(results.my_option_ids.is_empty());
    assert_eq!(results.poll.total_votes, 1);
    kevins_tab.until(ballot(OPEN_POLL, vec![]), |_| false).await;

    // Another room's path, or a room Kevin isn't in, is a 404.
    let reply = david
        .send(get(&format!("/api/v1/rooms/{HQ}/polls/{OPEN_POLL}")))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = vote(&mut kevin, ALL_PETS, OPEN_POLL, &[1]).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    // A closed poll takes no votes.
    assert_eq!(
        fields(&vote(&mut kevin, DESIGNERS, CLOSED_POLL, &[4]).await),
        ["poll"]
    );
    let closed: api::PollResults = ok(&kevin
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/polls/{CLOSED_POLL}"
        )))
        .await);
    assert!(closed.poll.closed && closed.poll.closed_at.is_some());

    // The closer closes the open one: the room hears it, and nobody gets a ballot.
    let now = a.booted.app.db.env().now();
    a.db()
        .write(move |tx| campfire_db::Poll::close_by_id(tx, OPEN_POLL, now))
        .await
        .unwrap();
    let event = davids_tab
        .until(
            |event| matches!(&event.payload, api::SyncPayload::PollUpdated(updated) if updated.poll.id == OPEN_POLL && updated.poll.closed),
            any_ballot,
        )
        .await;
    let api::SyncPayload::PollUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert!(updated.poll.closed_at.is_some());
    assert_eq!(
        fields(&vote(&mut kevin, DESIGNERS, OPEN_POLL, &[1]).await),
        ["poll"]
    );
    server.abort();
}

#[tokio::test]
async fn an_expired_poll_takes_no_votes() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut body = poll_body("expiring", &["Yes", "No"]);
    body["closesAt"] = json!("2026-03-09T12:00:00.000Z");
    let reply = send(
        &mut david,
        Method::POST,
        &format!("/api/v1/rooms/{HQ}/polls"),
        body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let poll = parse::<api::MessageDTO>(&reply).poll.expect("the poll");
    assert_eq!(poll.closes_at.as_deref(), Some("2026-03-09T12:00:00.000Z"));
    let option = poll.options[0].id;
    ok::<api::PollResults>(&vote(&mut david, HQ, poll.id, &[option]).await);

    // Its closing time passes before the closer gets to it.
    sql(
        &a,
        "UPDATE polls SET closes_at = '2026-03-01 12:00:00' WHERE id = ?",
        vec![poll.id],
    )
    .await;
    assert_eq!(fields(&vote(&mut david, HQ, poll.id, &[]).await), ["poll"]);
    let results: api::PollResults = ok(&david
        .send(get(&format!("/api/v1/rooms/{HQ}/polls/{}", poll.id)))
        .await);
    assert!(results.poll.closed && results.poll.closed_at.is_none());
    assert_eq!(results.my_option_ids, [option]);
}

#[tokio::test]
async fn anonymous_polls_hide_voters_but_tell_the_voter_their_ballot() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut davids_tab = Sync::connect(addr, &david.cookie_header(), &[]).await;
    davids_tab.welcome().await;
    let mut kevins_tab = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{HQ}")]).await;
    kevins_tab.welcome().await;
    let mut body = poll_body("secret", &["Red", "Blue", "Green"]);
    body["anonymous"] = json!(true);
    body["multiple"] = json!(true);
    let reply = send(
        &mut david,
        Method::POST,
        &format!("/api/v1/rooms/{HQ}/polls"),
        body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let poll = parse::<api::MessageDTO>(&reply).poll.expect("the poll");
    assert!(poll.anonymous && poll.multiple);
    let (red, blue) = (poll.options[0].id, poll.options[1].id);

    let results: api::PollResults = ok(&vote(&mut david, HQ, poll.id, &[blue, red]).await);
    assert_eq!(results.my_option_ids, [red, blue]);
    assert_eq!(results.poll.total_votes, 2);
    assert_eq!(
        counts(&results.poll),
        [(1, vec![]), (1, vec![]), (0, vec![])]
    );
    davids_tab
        .until(ballot(poll.id, vec![red, blue]), |_| false)
        .await;
    let event = kevins_tab.until(poll_updated(poll.id), any_ballot).await;
    let api::SyncPayload::PollUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        counts(&updated.poll),
        [(1, vec![]), (1, vec![]), (0, vec![])]
    );

    let page: api::MessagePage = ok(&david
        .send(get(&format!("/api/v1/rooms/{HQ}/messages")))
        .await);
    let listed = page
        .messages
        .iter()
        .find_map(|message| message.poll.as_ref().filter(|listed| listed.id == poll.id))
        .expect("the poll on the page");
    assert_eq!(counts(listed), [(1, vec![]), (1, vec![]), (0, vec![])]);
    server.abort();
}

#[tokio::test]
async fn a_classic_vote_tells_the_spa_tabs() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut kevin = a.sign_in(KEVIN).await;
    kevin.authenticity_token().await;
    let mut sync =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let reply = kevin
        .write(
            Req::new(
                Method::POST,
                &format!("/rooms/{DESIGNERS}/polls/{OPEN_POLL}/vote"),
            )
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(json!({"option_ids": [3]}).to_string()),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let event = sync.until(poll_updated(OPEN_POLL), any_ballot).await;
    let api::SyncPayload::PollUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert_eq!(updated.poll.total_votes, 2);
    sync.until(ballot(OPEN_POLL, vec![3]), |_| false).await;
    server.abort();
}

/// The classic frames a run of poll changes sends to a classic tab on Designers.




#[tokio::test]
async fn messages_carry_their_cards() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;

    let polled = message(&mut david, OPEN_POLL_MESSAGE).await;
    let poll = polled.poll.expect("the open poll");
    assert_eq!(
        (poll.id, poll.closed, poll.total_votes),
        (OPEN_POLL, false, 1)
    );
    assert_eq!(poll.as_of, polled.cards_as_of);
    let closed = message(&mut david, CLOSED_POLL_MESSAGE)
        .await
        .poll
        .expect("the closed poll");
    assert!(closed.closed);
    assert!(message(&mut david, PR_MESSAGE).await.poll.is_none());

    let cards = message(&mut david, PR_MESSAGE).await.cards;
    let [api::MessageCard::Github(github)] = cards.as_slice() else {
        panic!("{cards:?}")
    };
    // The repo is pinned through the URL: a bare quoted seed name here would make the frozen
    // seed inventory (parity/test_frozen_seeds.py) think this file boots that seed.
    assert_eq!(
        (
            github.pull_request_id,
            github.owner.as_str(),
            github.number,
            github.url.as_str(),
        ),
        (
            1,
            "smart-data-ohio",
            42,
            "https://github.com/smart-data-ohio/smartfire/pull/42"
        )
    );
    assert_eq!(
        github.url,
        format!(
            "https://github.com/{}/{}/pull/42",
            github.owner, github.repo
        )
    );
    let cards = message(&mut david, FIZZY_MESSAGE).await.cards;
    let [api::MessageCard::Fizzy(fizzy)] = cards.as_slice() else {
        panic!("{cards:?}")
    };
    assert_eq!(
        (fizzy.fizzy_card_id, fizzy.account_id.as_str(), fizzy.number),
        (1, "parity", 42)
    );
    assert!(fizzy.url.ends_with("/parity/cards/42"), "{}", fizzy.url);
    let cards = message(&mut david, LINK_MESSAGE).await.cards;
    let [api::MessageCard::Link(link)] = cards.as_slice() else {
        panic!("{cards:?}")
    };
    assert_eq!(link.url, "https://example.com/parity");
    assert_eq!(link.title.as_deref(), Some("Smartfire release notes"));
    let cards = message(&mut david, LINKEDIN_MESSAGE).await.cards;
    let [api::MessageCard::Linkedin(linkedin)] = cards.as_slice() else {
        panic!("{cards:?}")
    };
    assert_eq!(linkedin.title.as_deref(), Some("Smartfire launch"));
    // A `/posts/` URL names no activity URN, so classic shows no player either.
    assert_eq!(linkedin.embed_url, None);
    // The quoted message is in All Pets: everyone here sees this message, not necessarily that
    // one, so the preview is fetched per viewer.
    let cards = message(&mut david, QUOTE_MESSAGE).await.cards;
    assert_eq!(
        cards,
        [api::MessageCard::Quote(api::QuoteCard {
            reference_id: 1,
            preview: None
        })]
    );
    let cards = message(&mut david, X_MESSAGE).await.cards;
    let [api::MessageCard::X(post)] = cards.as_slice() else {
        panic!("{cards:?}")
    };
    assert_eq!(
        (post.fetch, post.post_id.as_str()),
        (api::CardFetch::Loaded, "20")
    );

    // Suppressed previews drop the page cards only.
    sql(
        &a,
        "UPDATE messages SET embeds_suppressed = 1 WHERE id IN (?, ?, ?)",
        vec![LINK_MESSAGE, LINKEDIN_MESSAGE, PR_MESSAGE],
    )
    .await;
    assert!(message(&mut david, LINK_MESSAGE).await.cards.is_empty());
    assert!(message(&mut david, LINKEDIN_MESSAGE).await.cards.is_empty());
    assert_eq!(message(&mut david, PR_MESSAGE).await.cards.len(), 1);

    // A Drive file, the event and a same-room quote join the pull request, in the classic slot
    // order.
    let source = OPEN_POLL_MESSAGE;
    a.db()
        .write(move |tx| {
            let conn = tx.conn();
            conn.execute(
                "INSERT INTO drive_attachments (created_at, file_id, message_id) VALUES ('2026-03-02 16:00:00', 'drive-file', ?)",
                [PR_MESSAGE],
            )?;
            conn.execute(
                "INSERT INTO event_references (created_at, updated_at, event_id, message_id) VALUES ('2026-03-02 16:00:00', '2026-03-02 16:00:00', ?, ?)",
                [LAUNCH_PARTY, PR_MESSAGE],
            )?;
            // An event in another room never shows here.
            conn.execute(
                "INSERT INTO event_references (created_at, updated_at, event_id, message_id) VALUES ('2026-03-02 16:00:00', '2026-03-02 16:00:00', ?, ?)",
                [WATERCOOLER_IN_ALL_TALK, PR_MESSAGE],
            )?;
            conn.execute(
                "INSERT INTO message_references (created_at, updated_at, message_id, referenced_message_id) VALUES ('2026-03-02 16:00:00', '2026-03-02 16:00:00', ?, ?)",
                [PR_MESSAGE, source],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let cards = message(&mut david, PR_MESSAGE).await.cards;
    let kinds: Vec<&str> = cards
        .iter()
        .map(|card| match card {
            api::MessageCard::Drive(_) => "drive",
            api::MessageCard::Github(_) => "github",
            api::MessageCard::Event(_) => "event",
            api::MessageCard::Quote(_) => "quote",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["drive", "github", "event", "quote"]);
    assert_eq!(
        cards[0],
        api::MessageCard::Drive(api::DriveFileCard {
            file_id: "drive-file".into(),
            url: "https://drive.google.com/open?id=drive-file".into(),
        })
    );
    let api::MessageCard::Event(event) = &cards[2] else {
        unreachable!()
    };
    assert_eq!(
        (
            event.event_id,
            event.room_id,
            event.title.as_str(),
            event.cancelled
        ),
        (LAUNCH_PARTY, DESIGNERS, "Launch party planning", false)
    );
    let api::MessageCard::Quote(quote) = &cards[3] else {
        unreachable!()
    };
    let preview = quote.preview.as_ref().expect("a same-room preview");
    assert_eq!(
        (
            preview.message_id,
            preview.room_id,
            preview.room_label.as_str()
        ),
        (source, DESIGNERS, "Designers")
    );
    assert!(!preview.excerpt.is_empty() && preview.excerpt.chars().count() <= 200);
}

#[tokio::test]
async fn card_updates_tell_the_conversation() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let mut sync =
        Sync::connect(addr, &david.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let cards_of = |message_id: i64| move |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::MessageCards(cards) if cards.message_id == message_id);

    // A fetch stored a new title.
    a.db()
        .write(|tx| {
            let embed_id: i64 = tx.conn().query_row(
                "SELECT link_embed_id FROM link_embed_references WHERE message_id = ?",
                [LINK_MESSAGE],
                |row| row.get(0),
            )?;
            tx.conn().execute(
                "UPDATE link_embeds SET title = 'Release notes, refreshed' WHERE id = ?",
                [embed_id],
            )?;
            tx.emit_after_commit(campfire_db::Event::broadcast(
                &campfire_app::integrations::link_embed::store::CardUpdate { embed_id },
            ));
            Ok(())
        })
        .await
        .unwrap();
    let event = sync.until(cards_of(LINK_MESSAGE), |_| false).await;
    assert_eq!(event.topic, format!("room:{DESIGNERS}"));
    let api::SyncPayload::MessageCards(cards) = event.payload else {
        unreachable!()
    };
    assert_eq!((cards.room_id, cards.thread_id), (DESIGNERS, None));
    let [api::MessageCard::Link(link)] = cards.cards.as_slice() else {
        panic!("{cards:?}")
    };
    assert_eq!(link.title.as_deref(), Some("Release notes, refreshed"));

    // An event the message links to was cancelled.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO event_references (created_at, updated_at, event_id, message_id) VALUES ('2026-03-02 16:00:00', '2026-03-02 16:00:00', ?, ?)",
                [LAUNCH_PARTY, FIZZY_MESSAGE],
            )?;
            tx.conn().execute(
                "UPDATE events SET cancelled_at = '2026-03-02 16:00:00' WHERE id = ?",
                [LAUNCH_PARTY],
            )?;
            tx.emit_after_commit(campfire_db::Event::broadcast(
                &campfire_db::models::calendar_event::CardUpdate {
                    event_id: LAUNCH_PARTY,
                },
            ));
            Ok(())
        })
        .await
        .unwrap();
    let event = sync.until(cards_of(FIZZY_MESSAGE), |_| false).await;
    let api::SyncPayload::MessageCards(cards) = event.payload else {
        unreachable!()
    };
    // Events come before Fizzy cards, in the classic slot order.
    let [api::MessageCard::Event(event), api::MessageCard::Fizzy(_)] = cards.cards.as_slice()
    else {
        panic!("{cards:?}")
    };
    assert!(event.cancelled);
    server.abort();
}

#[tokio::test]
async fn previews_are_per_viewer() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;

    let github = format!("/api/v1/rooms/{DESIGNERS}/github/pull_requests/1/card");
    let card: api::GithubPullRequestCard = ok(&david
        .send(get(&format!("{github}?messageId={PR_MESSAGE}")))
        .await);
    let api::GithubPullRequestCard::Loaded(pr) = card else {
        panic!("{card:?}")
    };
    assert_eq!(
        (pr.title.as_str(), pr.number, pr.status),
        (
            "Port the launch checklist",
            42,
            api::GithubPullRequestStatus::Open
        )
    );
    assert!(pr.files.is_none());
    for query in [
        String::new(),
        format!("?messageId={FIZZY_MESSAGE}"),
        "?threadId=1".into(),
        "?messageId=x".into(),
    ] {
        let reply = david.send(get(&format!("{github}{query}"))).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{query}: {}",
            reply.text()
        );
    }
    let reply = david
        .send(get(&format!(
            "/api/v1/rooms/{HQ}/github/pull_requests/1/card?messageId={PR_MESSAGE}"
        )))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    let fizzy = format!("/api/v1/rooms/{DESIGNERS}/fizzy/cards/1/card");
    // David connected the Parity workspace and has its card cached; Kevin hasn't connected.
    let preview: api::FizzyCardPreview = ok(&david
        .send(get(&format!("{fizzy}?messageId={FIZZY_MESSAGE}")))
        .await);
    let api::FizzyCardPreview::Loaded(card) = preview else {
        panic!("{preview:?}")
    };
    assert_eq!(
        (
            card.title.as_str(),
            card.url.as_str(),
            card.board_name.as_deref()
        ),
        (
            "Release checklist",
            "https://app.fizzy.do/parity/cards/42",
            Some("Release")
        )
    );
    let preview: api::FizzyCardPreview = ok(&kevin
        .send(get(&format!("{fizzy}?messageId={FIZZY_MESSAGE}")))
        .await);
    assert_eq!(preview, api::FizzyCardPreview::NotConnected);
    for query in [String::new(), format!("?messageId={PR_MESSAGE}")] {
        let reply = david.send(get(&format!("{fizzy}{query}"))).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{query}: {}",
            reply.text()
        );
    }

    let quote = format!("/api/v1/rooms/{DESIGNERS}/message_links/1/card");
    let result: api::QuotePreviewResult = ok(&david.send(get(&quote)).await);
    let api::QuotePreviewResult::Loaded(preview) = result else {
        panic!("{result:?}")
    };
    assert_eq!(preview.room_id, ALL_PETS);
    assert_eq!(preview.room_label, "All Pets");
    let result: api::QuotePreviewResult = ok(&kevin.send(get(&quote)).await);
    assert_eq!(result, api::QuotePreviewResult::Hidden);
    let reply = david
        .send(get(&format!("/api/v1/rooms/{HQ}/message_links/1/card")))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn attendance_answers_and_takes_responses() {
    let Some(a) = app(true).await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/events/{LAUNCH_PARTY}/attendance");
    let before: api::EventAttendance = ok(&kevin.send(get(&path)).await);
    assert_eq!(before.event_id, LAUNCH_PARTY);
    assert_eq!(before.response, None);
    assert!(before.respondable && !before.can_apply_to_future);

    let reply = send(
        &mut kevin,
        Method::PUT,
        &path,
        json!({"response": "going", "applyToFuture": false}),
    )
    .await;
    let after: api::EventAttendance = ok(&reply);
    assert_eq!(after.response, Some(api::AttendanceResponse::Going));
    assert_eq!(after.going_count, before.going_count + 1);
    let reply = send(
        &mut kevin,
        Method::PUT,
        &path,
        json!({"response": "declined", "applyToFuture": false}),
    )
    .await;
    let changed: api::EventAttendance = ok(&reply);
    assert_eq!(changed.response, Some(api::AttendanceResponse::Declined));
    assert_eq!(
        (changed.going_count, changed.declined_count),
        (before.going_count, before.declined_count + 1)
    );
    let reply = send(
        &mut kevin,
        Method::PUT,
        &path,
        json!({"response": "perhaps", "applyToFuture": false}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);

    // A cancelled event takes no responses.
    let cancelled = format!("/api/v1/rooms/{DESIGNERS}/events/{SPRINT_RETRO_CANCELLED}/attendance");
    let shown: api::EventAttendance = ok(&kevin.send(get(&cancelled)).await);
    assert!(!shown.respondable);
    let reply = send(
        &mut kevin,
        Method::PUT,
        &cancelled,
        json!({"response": "going", "applyToFuture": false}),
    )
    .await;
    assert_eq!(fields(&reply), ["response"]);

    // Like the classic event controller, every attendance action requires an active human.
    sql(&a, "UPDATE users SET role = 2 WHERE id = ?", vec![KEVIN]).await;
    let shown = kevin.send(get(&path)).await;
    assert_eq!(shown.status, StatusCode::FORBIDDEN);
    let reply = send(
        &mut kevin,
        Method::PUT,
        &path,
        json!({"response": "going", "applyToFuture": false}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    sql(&a, "UPDATE users SET role = 0 WHERE id = ?", vec![KEVIN]).await;

    // Another room's event, through this room, is a 404.
    let reply = kevin
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/events/{WATERCOOLER_IN_ALL_TALK}/attendance"
        )))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

/// Use the classic thread-page observation: stop workers, then inspect the durable PR job
/// and its claim, rather than relying on a fetch completing over the network.
async fn stale_pr(a: &TestApp) {
    a.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetched_at='2026-01-01 00:00:00', fetch_requested_at=NULL WHERE id=1")?;
        Ok(())
    }).await.unwrap();
}

async fn assert_pr_requested(a: &TestApp) {
    let (requested, jobs) = a.db().read(|conn| {
        let requested: bool = conn.query_row("SELECT fetch_requested_at IS NOT NULL FROM github_pull_requests WHERE id=1", [], |row| row.get(0))?;
        let jobs: i64 = conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' AND json_extract(arguments, '$.pull_request_id')=1", [], |row| row.get(0))?;
        Ok((requested, jobs))
    }).await.unwrap();
    assert!(requested, "the render must claim the stale PR refresh");
    assert_eq!(jobs, 1, "one durable refresh job for this PR");
}

#[tokio::test]
async fn card_followup_thread_parent_requests_a_stale_pr_without_a_room_page() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    sql(
        &a,
        "UPDATE channel_threads SET parent_message_id=? WHERE id=8",
        vec![PR_MESSAGE],
    )
    .await;
    sql(
        &a,
        "UPDATE messages SET created_at='2020-01-01 00:00:00' WHERE id=?",
        vec![PR_MESSAGE],
    )
    .await;
    a.db().read(|conn| {
        let replies: i64 = conn.query_row("SELECT COUNT(*) FROM github_pull_request_references r JOIN messages m ON m.id=r.message_id WHERE r.github_pull_request_id=1 AND m.id != ?", [PR_MESSAGE], |row| row.get(0))?;
        assert_eq!(replies, 0, "the PR is referenced only by the old starter");
        Ok(())
    }).await.unwrap();
    // Keep the discussion header fresh and distinct: only rendering the starter can request
    // PR 1, so the header cannot mask dropped starter fetches.
    a.db().write(|tx| {
        tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,private,fetched_at,created_at,updated_at) VALUES (2,'smart-data-ohio','smartfire',43,0,?,?,?)", (tx.now(), tx.now(), tx.now()))?;
        tx.conn().execute("UPDATE github_pull_request_threads SET github_pull_request_id=2 WHERE channel_thread_id=8", [])?;
        Ok(())
    }).await.unwrap();
    stale_pr(&a).await;
    let mut david = a.sign_in(DAVID).await;
    for _ in 0..2 {
        let detail: api::ThreadDetail = ok(&david.send(get("/api/v1/threads/8")).await);
        assert_eq!(detail.parent_message.unwrap().id, PR_MESSAGE);
        assert_pr_requested(&a).await;
    }
}

#[tokio::test]
async fn card_followup_thread_header_requests_a_stale_pr_without_a_parent() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    sql(
        &a,
        "UPDATE channel_threads SET parent_message_id=NULL WHERE id=8",
        vec![],
    )
    .await;
    stale_pr(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let detail: api::ThreadDetail = ok(&david.send(get("/api/v1/threads/8")).await);
    assert!(detail.parent_message.is_none());
    assert_pr_requested(&a).await;
}

#[tokio::test]
async fn card_followup_message_read_requests_a_stale_pr() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    stale_pr(&a).await;
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(message(&mut david, PR_MESSAGE).await.id, PR_MESSAGE);
    assert_pr_requested(&a).await;
}

#[tokio::test]
async fn card_followup_search_requests_a_stale_pr() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    stale_pr(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let results: api::SearchResults = ok(&david.send(get("/api/v1/search?q=smartfire")).await);
    assert!(results.messages.iter().any(|m| m.id == PR_MESSAGE));
    assert_pr_requested(&a).await;
}

#[tokio::test]
async fn card_followup_close_racing_a_vote_rechecks_in_the_writer() {
    const POLL: i64 = 980000001;
    let Some(a) = app(true).await else { return };
    let option = a.db().write(|tx| {
        tx.conn().execute("INSERT INTO polls (id,message_id,created_at,updated_at) VALUES (?,?,?,?)", (POLL, PR_MESSAGE, tx.now(), tx.now()))?;
        tx.conn().execute("INSERT INTO poll_options (poll_id,label,position,created_at,updated_at) VALUES (?,'Yes',0,?,?)", (POLL, tx.now(), tx.now()))?;
        Ok(tx.conn().last_insert_rowid())
    }).await.unwrap();
    let hold = campfire_api::test_hooks::hold_before_poll_vote_write(POLL);
    let mut kevin = a.sign_in(KEVIN).await;
    let options = [option];
    let (reply, ()) = tokio::join!(vote(&mut kevin, DESIGNERS, POLL, &options), async {
        hold.reached.wait().await;
        a.db()
            .write(|tx| campfire_db::Poll::close_by_id(tx, POLL, tx.now()))
            .await
            .unwrap();
        hold.release.wait().await;
    });
    assert_eq!(fields(&reply), ["poll"]);
    let votes: i64 = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM poll_votes WHERE poll_id=? AND user_id=?",
                [POLL, KEVIN],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(votes, 0);
}

#[tokio::test]
async fn card_followup_cancellation_racing_an_rsvp_rechecks_in_the_writer() {
    const EVENT: i64 = 980000002;
    let Some(a) = app(true).await else { return };
    sql(&a, "INSERT INTO events (id,created_at,updated_at,organizer_id,room_id,starts_at,time_zone,title) SELECT ?,created_at,updated_at,organizer_id,room_id,starts_at,time_zone,title FROM events WHERE id=?", vec![EVENT, LAUNCH_PARTY]).await;
    let hold = campfire_api::test_hooks::hold_before_attendance_write(EVENT);
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/events/{EVENT}/attendance");
    let (reply, ()) = tokio::join!(
        send(
            &mut kevin,
            Method::PUT,
            &path,
            json!({"response":"going", "applyToFuture":false})
        ),
        async {
            hold.reached.wait().await;
            sql(
                &a,
                "UPDATE events SET cancelled_at='2026-03-02 16:00:00' WHERE id=?",
                vec![EVENT],
            )
            .await;
            hold.release.wait().await;
        }
    );
    assert_eq!(fields(&reply), ["response"]);
    let rows: i64 = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM event_attendances WHERE event_id=? AND user_id=?",
                [EVENT, KEVIN],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn card_followup_simultaneous_duplicate_polls_create_once() {
    let Some(a) = app(true).await else { return };
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    let client_id = "card-followup-simultaneous-poll";
    campfire_api::test_hooks::hold_after_duplicate_check(client_id, 2);
    let path = format!("/api/v1/rooms/{HQ}/polls");
    let (first, second) = tokio::join!(
        send(
            &mut first,
            Method::POST,
            &path,
            poll_body(client_id, &["Yes", "No"])
        ),
        send(
            &mut second,
            Method::POST,
            &path,
            poll_body(client_id, &["Yes", "No"])
        )
    );
    let mut statuses = [first.status, second.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CREATED],
        "{} / {}",
        first.text(),
        second.text()
    );
    let first: api::MessageDTO = parse(&first);
    let second: api::MessageDTO = parse(&second);
    assert_eq!(first.id, second.id);
    a.db().read(move |conn| {
        let messages: i64 = conn.query_row("SELECT COUNT(*) FROM messages WHERE room_id=? AND creator_id=? AND client_message_id=?", (HQ, DAVID, client_id), |row| row.get(0))?;
        let polls: i64 = conn.query_row("SELECT COUNT(*) FROM polls WHERE message_id=?", [first.id], |row| row.get(0))?;
        assert_eq!((messages, polls), (1, 1));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn card_followup_private_github_previews_check_each_viewers_repository_access() {
    use crate::integrations::github::accounts::{Account, AccountInput};
    use crate::integrations::test_support::Route;
    let (github, network) = crate::integrations::github::tests::fake(vec![
        Route::new(
            "GET",
            "api.github.com",
            "/repos/smart-data-ohio/smartfire",
            200,
        )
        .body("{}"),
    ])
    .await;
    let Some(a) = TestApp::boot_with_network_clock_and_env(
        network,
        crate::controllers::presenters::test_support::seed_clock(),
        &[("SPA_ENABLED", "1")],
    )
    .await
    else {
        return;
    };
    let a = a.without_job_runner().await;
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE github_pull_requests SET private=1,title='Private followup title' WHERE id=1", [])?;
        for (user, login, token) in [(DAVID, "followup-david", "fixture-david-token"), (KEVIN, "followup-kevin", "fixture-kevin-token")] {
            Account::create(tx, &crypto, &AccountInput {user_id:user, github_login:login, access_token:token, refresh_token:None, token_expires_at:None, token_source:"pat"})?;
        }
        Ok(())
    }).await.unwrap();
    let path =
        format!("/api/v1/rooms/{DESIGNERS}/github/pull_requests/1/card?messageId={PR_MESSAGE}");
    let mut david = a.sign_in(DAVID).await;
    let reply = david.send(get(&path)).await;
    let card: api::GithubPullRequestCard = ok(&reply);
    let api::GithubPullRequestCard::Loaded(card) = card else {
        panic!("{card:?}")
    };
    assert_eq!(card.title, "Private followup title");
    github.replace_routes(vec![
        Route::new(
            "GET",
            "api.github.com",
            "/repos/smart-data-ohio/smartfire",
            404,
        )
        .body("{}"),
    ]);
    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin.send(get(&path)).await;
    assert_eq!(
        ok::<api::GithubPullRequestCard>(&reply),
        api::GithubPullRequestCard::Hidden
    );
    assert!(!reply.text().contains("Private followup title"));
    assert_eq!(
        github.received.lock().unwrap().len(),
        2,
        "each connected viewer's own repository check"
    );
}

#[tokio::test]
async fn card_followup_connected_fizzy_previews_keep_the_viewers_payloads_separate() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    a.db().write(|tx| {
        tx.conn().execute("INSERT INTO fizzy_connected_accounts (access_token,created_at,fizzy_account_id,updated_at,user_id) SELECT access_token,created_at,fizzy_account_id,updated_at,? FROM fizzy_connected_accounts WHERE user_id=?", [KEVIN, DAVID])?;
        for (viewer, prefix) in [(DAVID, "DavidOnly"), (KEVIN, "KevinOnly")] {
            let payload = json!({"title":format!("{prefix} title"), "url":format!("https://app.fizzy.do/{prefix}/cards/42"), "board":{"name":format!("{prefix} board")}, "column":{"name":format!("{prefix} column")}, "assignees":[{"name":format!("{prefix} assignee"), "avatar_url":format!("https://example.org/{prefix}.png")}], "tags":[format!("{prefix} tag")], "steps":[{"completed":true}, {"completed":false}]}).to_string();
            tx.conn().execute("INSERT INTO fizzy_card_caches (created_at,updated_at,fizzy_card_id,user_id,payload,fetched_at) VALUES (?,?,1,?,?,?) ON CONFLICT(fizzy_card_id,user_id) DO UPDATE SET payload=excluded.payload,fetched_at=excluded.fetched_at", (tx.now(), tx.now(), viewer, payload, tx.now()))?;
        }
        Ok(())
    }).await.unwrap();
    let path = format!("/api/v1/rooms/{DESIGNERS}/fizzy/cards/1/card?messageId={FIZZY_MESSAGE}");
    for (viewer, own, other) in [
        (DAVID, "DavidOnly", "KevinOnly"),
        (KEVIN, "KevinOnly", "DavidOnly"),
    ] {
        let mut browser = a.sign_in(viewer).await;
        let reply = browser.send(get(&path)).await;
        let preview: api::FizzyCardPreview = ok(&reply);
        let api::FizzyCardPreview::Loaded(card) = preview else {
            panic!("{preview:?}")
        };
        assert_eq!(card.title, format!("{own} title"));
        assert_eq!(card.board_name, Some(format!("{own} board")));
        assert_eq!(card.column_name, Some(format!("{own} column")));
        assert_eq!(card.assignees[0].name, format!("{own} assignee"));
        assert_eq!(
            card.assignees[0].avatar_url,
            Some(format!("https://example.org/{own}.png"))
        );
        assert_eq!(card.tags, [format!("{own} tag")]);
        assert_eq!((card.steps_total, card.steps_completed), (2, 1));
        assert_eq!(card.url, format!("https://app.fizzy.do/{own}/cards/42"));
        assert!(!reply.text().contains(other), "{}", reply.text());
    }
}

async fn card_conversation(a: &TestApp, message_id: i64, threaded: bool) -> (String, Option<i64>) {
    if !threaded {
        return (format!("room:{DESIGNERS}"), None);
    }
    let thread_id = a
        .db()
        .write(move |tx| {
            let thread = campfire_db::ChannelThread::create(
                tx,
                campfire_db::NewChannelThread {
                    room_id: DESIGNERS,
                    creator_id: DAVID,
                    name: Some("Card followup".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET thread_id=?,room_id=? WHERE id=?",
                (thread.id, DESIGNERS, message_id),
            )?;
            Ok(thread.id)
        })
        .await
        .unwrap();
    (format!("thread:{thread_id}"), Some(thread_id))
}

async fn changed_cards(
    sync: &mut Sync,
    topic: &str,
    message_id: i64,
    thread_id: Option<i64>,
) -> api::MessageCards {
    let event = sync.until(move |event| matches!(&event.payload, api::SyncPayload::MessageCards(cards) if cards.message_id == message_id), |_| false).await;
    assert_eq!(event.topic, topic);
    let api::SyncPayload::MessageCards(cards) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        (cards.message_id, cards.room_id, cards.thread_id),
        (message_id, DESIGNERS, thread_id)
    );
    cards
}

#[tokio::test]
async fn card_followup_github_updates_reach_room_and_thread_tabs() {
    for threaded in [false, true] {
        let Some(a) = app(true).await else { return };
        let a = a.without_job_runner().await;
        let (topic, thread_id) = card_conversation(&a, PR_MESSAGE, threaded).await;
        let (addr, server) = serve(&a).await;
        let david = a.sign_in(DAVID).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
        sync.welcome().await;
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE github_pull_requests SET owner='followup-org',number=73 WHERE id=1",
                    [],
                )?;
                tx.emit_after_commit(campfire_db::Event::broadcast(
                    &campfire_app::integrations::github::pull_requests::CardUpdated {
                        pull_request_id: 1,
                    },
                ));
                Ok(())
            })
            .await
            .unwrap();
        let cards = changed_cards(&mut sync, &topic, PR_MESSAGE, thread_id).await;
        let [api::MessageCard::Github(pr)] = cards.cards.as_slice() else {
            panic!("{cards:?}")
        };
        assert_eq!(
            (pr.owner.as_str(), pr.number, pr.url.as_str()),
            (
                "followup-org",
                73,
                "https://github.com/followup-org/smartfire/pull/73"
            )
        );
        server.abort();
    }
}

#[tokio::test]
async fn card_followup_fizzy_refreshes_reach_room_and_thread_tabs() {
    for threaded in [false, true] {
        let Some(a) = app(true).await else { return };
        let a = a.without_job_runner().await;
        let (topic, thread_id) = card_conversation(&a, FIZZY_MESSAGE, threaded).await;
        let (addr, server) = serve(&a).await;
        let mut david = a.sign_in(DAVID).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
        sync.welcome().await;
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE fizzy_cards SET account_id='followup-workspace',number=73 WHERE id=1",
                    [],
                )?;
                let card = campfire_app::integrations::fizzy::cards::Card::find(tx.conn(), 1)?;
                let cache = campfire_app::integrations::fizzy::cards::Cache::for_viewer(tx, &card, DAVID)?;
                cache.save(tx, Some(&json!({"title":"Fizzy refreshed privately","board":{"name":"Refreshed board"}})), Some(tx.now()), None)?;
                card.broadcast_updates(tx);
                Ok(())
            })
            .await
            .unwrap();
        let cards = changed_cards(&mut sync, &topic, FIZZY_MESSAGE, thread_id).await;
        let [api::MessageCard::Fizzy(card)] = cards.cards.as_slice() else {
            panic!("{cards:?}")
        };
        assert_eq!(
            (card.account_id.as_str(), card.number),
            ("followup-workspace", 73)
        );
        assert!(card.url.ends_with("/followup-workspace/cards/73"));
        // Shared twins carry references. The refreshed viewer payload stays in the private
        // endpoint, just as the classic replacement points to a private lazy frame.
        let preview: api::FizzyCardPreview = ok(&david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/fizzy/cards/1/card?messageId={FIZZY_MESSAGE}"
            )))
            .await);
        let api::FizzyCardPreview::Loaded(preview) = preview else {
            panic!("{preview:?}")
        };
        assert_eq!(preview.title, "Fizzy refreshed privately");
        assert_eq!(preview.board_name.as_deref(), Some("Refreshed board"));
        assert!(
            !serde_json::to_string(&cards)
                .unwrap()
                .contains("Fizzy refreshed privately")
        );
        server.abort();
    }
}

#[tokio::test]
async fn card_followup_x_updates_reach_room_and_thread_tabs() {
    for threaded in [false, true] {
        let Some(a) = app(true).await else { return };
        let a = a.without_job_runner().await;
        sql(
            &a,
            "UPDATE messages SET room_id=? WHERE id=?",
            vec![DESIGNERS, X_MESSAGE],
        )
        .await;
        let (topic, thread_id) = card_conversation(&a, X_MESSAGE, threaded).await;
        let (addr, server) = serve(&a).await;
        let david = a.sign_in(DAVID).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
        sync.welcome().await;
        a.db()
            .write(|tx| {
                let post_id: i64 = tx.conn().query_row(
                    "SELECT twitter_post_id FROM twitter_post_references WHERE message_id=?",
                    [X_MESSAGE],
                    |row| row.get(0),
                )?;
                tx.conn().execute(
                    "UPDATE twitter_posts SET text='Followup refreshed X post',likes=73 WHERE id=?",
                    [post_id],
                )?;
                tx.emit_after_commit(campfire_db::Event::broadcast(
                    &campfire_app::integrations::twitter::post::CardUpdate { post_id },
                ));
                Ok(())
            })
            .await
            .unwrap();
        let cards = changed_cards(&mut sync, &topic, X_MESSAGE, thread_id).await;
        let [api::MessageCard::X(post)] = cards.cards.as_slice() else {
            panic!("{cards:?}")
        };
        assert_eq!(post.text.as_deref(), Some("Followup refreshed X post"));
        assert_eq!(post.likes, Some(73));
        server.abort();
    }
}

#[tokio::test]
async fn card_followup_quote_updates_reach_room_and_thread_tabs() {
    for threaded in [false, true] {
        let Some(a) = app(true).await else { return };
        let a = a.without_job_runner().await;
        let (topic, thread_id) = card_conversation(&a, QUOTE_MESSAGE, threaded).await;
        let source_id = a
            .db()
            .write(|tx| {
                let source = campfire_db::Message::create(
                    tx,
                    campfire_db::NewMessage {
                        room_id: DESIGNERS,
                        creator_id: DAVID,
                        markdown_source: Some("Quote before refresh".into()),
                        ..Default::default()
                    },
                )?;
                tx.conn().execute(
                    "UPDATE message_references SET referenced_message_id=? WHERE message_id=?",
                    [source.id, QUOTE_MESSAGE],
                )?;
                Ok(source.id)
            })
            .await
            .unwrap();
        let (addr, server) = serve(&a).await;
        let david = a.sign_in(DAVID).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
        sync.welcome().await;
        a.db()
            .write(move |tx| {
                let mut source = campfire_db::Message::find(tx.conn(), source_id)?;
                source.update_body(tx, "Quote after refresh")?;
                assert_eq!(
                    campfire_db::models::message_reference::refresh_quote_cards(
                        tx, source_id, 200, 100
                    )?,
                    1
                );
                Ok(())
            })
            .await
            .unwrap();
        let cards = changed_cards(&mut sync, &topic, QUOTE_MESSAGE, thread_id).await;
        let [api::MessageCard::Quote(quote)] = cards.cards.as_slice() else {
            panic!("{cards:?}")
        };
        let preview = quote.preview.as_ref().unwrap();
        assert_eq!(preview.message_id, source_id);
        assert_eq!(preview.excerpt, "Quote after refresh");
        server.abort();
    }
}

#[tokio::test]
async fn github_thread_header_updates_arrive_after_the_parent_is_deleted() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    a.db().write(|tx| {
        tx.conn().execute("UPDATE channel_threads SET parent_message_id=? WHERE id=8", [PR_MESSAGE])?;
        campfire_db::Message::find(tx.conn(), PR_MESSAGE)?.destroy(tx)?;
        assert_eq!(campfire_db::ChannelThread::find(tx.conn(), 8)?.parent_message_id, None);
        Ok(())
    }).await.unwrap();
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut owner = Sync::connect(addr, &david.cookie_header(), &["thread:8".into()]).await;
    let mut member = Sync::connect(addr, &kevin.cookie_header(), &["thread:8".into()]).await;
    owner.welcome().await;
    member.welcome().await;
    a.db().write(|tx| {
        tx.emit_after_commit(campfire_db::Event::broadcast(&campfire_app::integrations::github::pull_requests::CardUpdated { pull_request_id: 1 }));
        Ok(())
    }).await.unwrap();
    for socket in [&mut owner, &mut member] {
        let event = socket.until(|event| matches!(&event.payload, api::SyncPayload::ThreadGithubUpdated(change) if change.thread_id == 8), |_| false).await;
        assert_eq!(event.topic, "thread:8");
        assert_eq!(event.payload, api::SyncPayload::ThreadGithubUpdated(api::ThreadGithubUpdated { room_id: DESIGNERS, thread_id: 8, pull_request_id: 1 }));
    }
    server.abort();
}

#[tokio::test]
async fn link_and_linkedin_refreshes_reach_room_and_thread_tabs_without_html() {
    for (message_id, linkedin) in [(LINK_MESSAGE, false), (LINKEDIN_MESSAGE, true)] {
        for threaded in [false, true] {
            let Some(a) = app(true).await else { return };
            let a = a.without_job_runner().await;
            let (topic, thread_id) = card_conversation(&a, message_id, threaded).await;
            let (addr, server) = serve(&a).await;
            let david = a.sign_in(DAVID).await;
            let mut sync = Sync::connect(addr, &david.cookie_header(), std::slice::from_ref(&topic)).await;
            sync.welcome().await;
            for standalone in [false, true] {
                let title = if standalone { "Standalone refreshed card" } else { "Batched refreshed card" };
                let _embed_id = a.db().write(move |tx| {
                    let embed_id: i64 = tx.conn().query_row("SELECT link_embed_id FROM link_embed_references WHERE message_id=? LIMIT 1", [message_id], |row| row.get(0))?;
                    tx.conn().execute("UPDATE link_embeds SET title=? WHERE id=?", (title, embed_id))?;
                    if !standalone {
                        tx.emit_after_commit(campfire_db::Event::broadcast(&campfire_app::integrations::link_embed::store::CardUpdate { embed_id }));
                    }
                    Ok(embed_id)
                }).await.unwrap();
                if standalone {
                    let state = a.booted.app.clone();
                    a.db().read(move |conn| {
                        let message = campfire_db::Message::find(conn, message_id)?;
                        crate::controllers::presenters::link_embeds::broadcast_message(&state, conn, &message, linkedin)
                    }).await.unwrap();
                }
                let cards = changed_cards(&mut sync, &topic, message_id, thread_id).await;
                let actual_title = match cards.cards.as_slice() {
                    [api::MessageCard::Link(card)] if !linkedin => &card.title,
                    [api::MessageCard::Linkedin(card)] if linkedin => &card.title,
                    other => panic!("unexpected {other:?}"),
                };
                assert_eq!(actual_title.as_deref(), Some(title));
            }
            server.abort();
        }
    }
}

#[tokio::test]
async fn wide_calendar_years_reach_the_message_socket_without_a_turbo_renderer() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let event_id = a.db().write(|tx| {
        let event = campfire_db::CalendarEvent::create(tx, campfire_db::NewCalendarEvent {
            room_id: DESIGNERS, organizer_id: DAVID, title: "Wide calendar".into(),
            starts_at: campfire_db::Timestamp::parse_db("60310-02-02 20:30:00"),
            ends_at: campfire_db::Timestamp::parse_db("60310-02-02 21:30:00"),
            time_zone: "America/New_York".into(), ..Default::default()
        })?;
        Ok(event.id)
    }).await.unwrap();
    let response = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/messages"), &json!({
        "markdownSource": format!("/rooms/{DESIGNERS}/events/{event_id}"),
        "clientMessageId": "wide-calendar-sync", "replyToMessageId": null, "replyNotifyAuthor": null,
    }))).await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let message_id = parse::<api::MessageDTO>(&response).id;
    let event = sync.until(move |event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == message_id), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = event.payload else { unreachable!() };
    let [api::MessageCard::Event(card)] = message.cards.as_slice() else { panic!("{:?}", message.cards) };
    assert_eq!(card.starts_at, "+060310-02-02T20:30:00.000Z");
    assert_eq!(card.ends_at.as_deref(), Some("+060310-02-02T21:30:00.000Z"));
    server.abort();
}


#[tokio::test]
async fn poll_media_images_and_emoji_round_trip_and_reject_invalid_choices() {
    let Some(a) = app(true).await else { return };
    let signed = super::admin_tests::upload(&a, "poll.png").await;
    let mut b = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &b.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let mut body = poll_body("poll-media", &["Image", "Emoji"]);
    body["optionMedia"] = json!([{ "signedId": signed }, { "emoji": "🌮" }]);
    let reply = send(&mut b, Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/polls"), body.clone()).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: Value = parse(&reply);
    assert_eq!(posted["poll"]["options"][0]["media"]["kind"], "image");
    assert_eq!(posted["poll"]["options"][1]["media"]["content"], "🌮");
    let url = posted["poll"]["options"][0]["media"]["url"].as_str().unwrap();
    let image = b.send(get(url)).await;
    assert!(image.status.is_success() || image.status.is_redirection());
    let mut outsider = a.sign_in(KEVIN).await;
    sql(&a, "DELETE FROM memberships WHERE room_id = ? AND user_id = ?", vec![DESIGNERS, KEVIN]).await;
    assert_eq!(outsider.send(get(url)).await.status, StatusCode::NOT_FOUND);
    let uploaded_url = format!("/rails/active_storage/blobs/redirect/{signed}/poll.png");
    assert_eq!(outsider.send(get(&uploaded_url)).await.status, StatusCode::NOT_FOUND);
    let poll_id = posted["poll"]["id"].as_i64().unwrap();
    let option_id = posted["poll"]["options"][0]["id"].as_i64().unwrap();
    ok::<api::PollResults>(&vote(&mut b, DESIGNERS, poll_id, &[option_id]).await);
    let event = sync.until(poll_updated(poll_id), any_ballot).await;
    let api::SyncPayload::PollUpdated(updated) = event.payload else { panic!("expected poll update") };
    let updated = serde_json::to_value(updated.poll).unwrap();
    assert_eq!(updated["options"][0]["media"], posted["poll"]["options"][0]["media"]);
    assert_eq!(updated["options"][1]["media"], posted["poll"]["options"][1]["media"]);
    let read: Value = ok(&b.send(get(&format!("/api/v1/rooms/{DESIGNERS}/polls/{poll_id}"))).await);
    assert_eq!(read["poll"]["options"], updated["options"]);
    let both = json!([{ "signedId": signed, "emoji": "🌮" }, null]);
    let unknown = json!([{ "emoji": ":missing_poll_emoji:" }, null]);
    let remote = json!([{ "signedId": "https://example.com/image.png" }, null]);
    for (index, media) in [both, unknown, remote].into_iter().enumerate() {
        body["clientMessageId"] = json!(format!("poll-media-invalid-{index}"));
        body["optionMedia"] = media;
        assert_eq!(fields(&send(&mut b, Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/polls"), body.clone()).await), ["optionMedia"]);
    }
    server.abort();
}

#[tokio::test]
async fn poll_media_rejects_non_images_and_oversize_uploads() {
    let Some(a) = app(true).await else { return };
    let signed = super::admin_tests::upload(&a, "poll.png").await;
    let blob_id = campfire_storage::paths::verify_signed_blob_id(&*a.booted.app.storage.verifier, &signed, a.booted.app.clock.now()).unwrap();
    let mut b = a.sign_in(DAVID).await;
    for (index, sql_text) in [
        "UPDATE active_storage_blobs SET content_type = 'text/plain' WHERE id = ?",
        "UPDATE active_storage_blobs SET content_type = 'image/png', byte_size = 104857601 WHERE id = ?",
    ].into_iter().enumerate() {
        sql(&a, sql_text, vec![blob_id]).await;
        let mut body = poll_body(&format!("poll-media-reject-{index}"), &["A", "B"]);
        body["optionMedia"] = json!([{ "signedId": signed }, null]);
        assert_eq!(fields(&send(&mut b, Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/polls"), body).await), ["optionMedia"]);
    }
}


#[tokio::test]
async fn poll_media_stills_disk_urls_and_serializers_follow_room_access_and_purge() {
    let Some(a) = app(true).await else { return };
    let staged = a.booted.app.storage.stage_bytes(
        include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
        campfire_storage::Filename::new("poll.gif"), Some("image/gif"),
    ).unwrap();
    let uploaded = a.db().write(move |tx| crate::controllers::messages::save_staged(tx, staged)).await.unwrap();
    let signed = campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, uploaded.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut other = a.sign_in(KEVIN).await;
    let mut body = poll_body("poll-media-animated", &["Moving", "Still"]);
    body["optionMedia"] = json!([{ "signedId": signed }, { "emoji": "👩🏽‍💻" }]);
    let reply = send(&mut author, Method::POST, &format!("/api/v1/rooms/{ALL_PETS}/polls"), body).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let message: Value = parse(&reply);
    let poll_id = message["poll"]["id"].as_i64().unwrap();
    let message_id = message["id"].as_i64().unwrap();
    let original = message["poll"]["options"][0]["media"]["url"].as_str().unwrap();
    let still = message["poll"]["options"][0]["media"]["stillUrl"].as_str().unwrap();
    for url in [original, still] {
        let permitted = author.send(get(url)).await;
        assert_eq!(permitted.header("cache-control"), Some("private, no-store"));
        let disk = permitted.location().unwrap();
        assert_eq!(other.send(get(url)).await.status, StatusCode::NOT_FOUND);
        assert_eq!(other.send(get(disk)).await.status, StatusCode::NOT_FOUND);
        let bytes = author.send(get(disk)).await;
        assert_eq!(bytes.status, StatusCode::OK);
        assert_eq!(bytes.header("cache-control"), Some("private, no-store"));
        if url == still { assert_eq!(&bytes.body[..8], b"\x89PNG\r\n\x1a\n"); }
        let proxy = url.replace("/redirect/", "/proxy/");
        assert_eq!(other.send(get(&proxy)).await.status, StatusCode::NOT_FOUND);
        assert_eq!(author.send(get(&proxy)).await.header("cache-control"), Some("private, no-store"));
    }
    assert_eq!(other.send(get(&format!("/api/v1/rooms/{ALL_PETS}/polls/{poll_id}"))).await.status, StatusCode::NOT_FOUND);
    let blob_id = a.db().read(move |conn| {
        Ok(conn.query_row("SELECT a.blob_id FROM active_storage_attachments a JOIN poll_options o ON o.id=a.record_id WHERE a.record_type='PollOption' AND o.poll_id=? ORDER BY o.position LIMIT 1", [poll_id], |r| r.get::<_, i64>(0))?)
    }).await.unwrap();
    let variant = a.db().read(move |conn| {
        Ok(conn.query_row("SELECT a.blob_id FROM active_storage_attachments a JOIN active_storage_variant_records v ON v.id=a.record_id WHERE a.record_type='ActiveStorage::VariantRecord' AND v.blob_id=?", [blob_id], |r| r.get::<_, i64>(0))?)
    }).await.unwrap();
    let variant_blob = a.db().read(move |conn| Ok(campfire_storage::Blob::find(conn, variant).unwrap().unwrap())).await.unwrap();
    let variant_url = campfire_storage::paths::blob_redirect_path(&*a.booted.app.storage.verifier, &variant_blob, None);
    let variant_response = author.send(get(&variant_url)).await;
    assert_eq!(variant_response.header("cache-control"), Some("private, no-store"));
    let variant_disk = variant_response.location().unwrap().to_owned();
    assert_eq!(other.send(get(&variant_url)).await.status, StatusCode::NOT_FOUND);
    let app = a.booted.app.clone();
    let payload = a.db().read(move |conn| {
        campfire_db::Poll::find(conn, poll_id)?.results_payload(conn, &*app.db.env().rich_text, app.db.env().now(), Some(DAVID), &*app.storage.verifier)
    }).await.unwrap();
    assert_eq!(payload["options"][0]["media"], message["poll"]["options"][0]["media"]);
    assert_eq!(payload["options"][1]["media"]["content"], "👩🏽‍💻");
    a.db().write(move |tx| campfire_db::Message::find(tx.conn(), message_id)?.destroy(tx)).await.unwrap();
    assert_eq!(author.send(get(original)).await.status, StatusCode::NOT_FOUND);
    crate::active_storage::purge(&a.booted.app, blob_id).await.unwrap();
    // The still's purge job can run later than its original's job, so drive the still's purge here too.
    // purge is a no-op when the queued job already removed the still.
    crate::active_storage::purge(&a.booted.app, variant).await.unwrap();
    // A queued job that already removed the row may still be deleting files; wait for that bounded time.
    for _ in 0..50 {
        if author.send(get(&variant_disk)).await.status == StatusCode::NOT_FOUND {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(author.send(get(&variant_url)).await.status, StatusCode::NOT_FOUND, "still URL still resolves after its purge");
    assert_eq!(author.send(get(&variant_disk)).await.status, StatusCode::NOT_FOUND, "still disk URL still resolves after its purge");
    let exists = a.db().read(move |conn| Ok(campfire_storage::Blob::find(conn, blob_id).unwrap().is_some())).await.unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn poll_media_accepts_workspace_emoji_by_name_and_preserves_field_order() {
    let Some(a) = app(true).await else { return };
    a.db().write(|tx| {
        tx.conn().execute("INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('poll_party','Party',?1,?2,?2)", rusqlite::params![DAVID, tx.now()])?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let mut body = poll_body("poll-custom-media", &["Party", "Quiet"]);
    body["optionMedia"] = json!([{ "emoji": "poll_party" }, null]);
    let reply = send(&mut b, Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/polls"), body).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let message: Value = parse(&reply);
    let option = &message["poll"]["options"][0];
    assert_eq!(option["media"], json!({ "kind": "emoji", "content": ":poll_party:" }));
    assert_eq!(option.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["id", "label", "votes", "voterIds", "media"]);
}

#[tokio::test]
async fn poll_media_bot_reply_tokens_cannot_cross_rooms() {
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, encode};
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    a.db().write(|tx| {
        tx.conn().execute("INSERT OR IGNORE INTO memberships (room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![ALL_PETS, BENDER, tx.now(), tx.now()])?;
        Ok(())
    }).await.unwrap();
    let staged = a.booted.app.storage.stage_bytes(
        include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
        campfire_storage::Filename::new("poll.gif"), Some("image/gif"),
    ).unwrap();
    let blob = a.db().write(move |tx| crate::controllers::messages::save_staged(tx, staged)).await.unwrap();
    let signed = campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut body = poll_body("poll-reply-token-media", &["Moving", "Quiet"]);
    body["optionMedia"] = json!([{ "signedId": signed }, null]);
    let posted = send(&mut author, Method::POST, &format!("/api/v1/rooms/{ALL_PETS}/polls"), body).await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let posted: Value = parse(&posted);
    let now = a.booted.app.clock.now();
    let wrong_room = encode(&rails_compat::verifiers::bot_reply::token_for(&a.booted.app.secrets, BENDER, ALL_TALK, now));
    let right_room = encode(&rails_compat::verifiers::bot_reply::token_for(&a.booted.app.secrets, BENDER, ALL_PETS, now));
    for field in ["url", "stillUrl"] {
        let url = posted["poll"]["options"][0]["media"][field].as_str().unwrap();
        let disk = author.send(get(url)).await.location().unwrap().to_owned();
        let proxy = url.replace("/redirect/", "/proxy/");
        for path in [url, &disk, &proxy] {
            let denied = format!("{path}?bot_key={wrong_room}&room_id={ALL_TALK}");
            assert_eq!(a.anonymous().send(get(&denied)).await.status, StatusCode::NOT_FOUND, "{field}: {path}");
            let permitted = format!("{path}?bot_key={right_room}&room_id={ALL_PETS}");
            let response = a.anonymous().send(get(&permitted)).await;
            assert!(response.status.is_success() || response.status.is_redirection(), "{field}: {}", response.text());
        }
    }
}

async fn follow_media_redirects(browser: &mut Browser<'_>, request: Req) -> Reply {
    let mut response = browser.send(request).await;
    for _ in 0..5 {
        if !response.status.is_redirection() { return response; }
        let location = response.location().unwrap().to_owned();
        response = browser.send(get(&location)).await;
    }
    panic!("poll media redirected more than five times");
}

#[tokio::test]
async fn poll_media_agent_downloads_follow_poll_access() {
    use crate::controllers::agent_http_tests::{AGENT, SECRET, initialize};
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER_KEY};
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    initialize(&a).await;
    let staged = a.booted.app.storage.stage_bytes(
        include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
        campfire_storage::Filename::new("poll.gif"), Some("image/gif"),
    ).unwrap();
    let blob = a.db().write(move |tx| crate::controllers::messages::save_staged(tx, staged)).await.unwrap();
    let signed = campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut body = poll_body("poll-agent-media", &["Moving", "Quiet"]);
    body["optionMedia"] = json!([{ "signedId": signed }, null]);
    let reply = send(&mut author, Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/polls"), body).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: Value = parse(&reply);
    let poll_id = posted["poll"]["id"].as_i64().unwrap();
    for field in ["url", "stillUrl"] {
        let url = posted["poll"]["options"][0]["media"][field].as_str().unwrap();
        let response = follow_media_redirects(&mut a.anonymous(), get(&format!("{url}?bot_key={BENDER_KEY}"))).await;
        assert_eq!(response.status, StatusCode::OK, "bot {field}: {}", response.text());
        assert_eq!(response.header("cache-control"), Some("private, no-store"));
        if field == "stillUrl" { assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n"); }
    }
    let request = |url: &str| get(url).header("authorization", &format!("Bearer {SECRET}"));
    let poll_url = format!("/rooms/{ALL_TALK}/agents/polls/{poll_id}");
    let read = a.anonymous().send(request(&poll_url)).await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let payload: Value = parse(&read);
    let media = &payload["options"][0]["media"];
    assert_eq!(media, &posted["poll"]["options"][0]["media"]);
    let mut urls = Vec::new();
    for field in ["url", "stillUrl"] {
        let url = media[field].as_str().unwrap().to_owned();
        let downloaded = follow_media_redirects(&mut a.anonymous(), request(&url)).await;
        assert_eq!(downloaded.status, StatusCode::OK, "{field}: {}", downloaded.text());
        assert_eq!(downloaded.header("cache-control"), Some("private, no-store"));
        if field == "stillUrl" { assert_eq!(&downloaded.body[..8], b"\x89PNG\r\n\x1a\n"); }
        let disk = author.send(get(&url)).await.location().unwrap().to_owned();
        let proxy = url.replace("/redirect/", "/proxy/");
        for path in [&disk, &proxy] {
            let response = a.anonymous().send(request(path)).await;
            assert_eq!(response.status, StatusCode::OK, "{path}: {}", response.text());
            assert_eq!(response.header("cache-control"), Some("private, no-store"));
            if field == "stillUrl" { assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n"); }
        }
        urls.extend([url, disk, proxy]);
    }
    a.db().write(move |tx| {
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES (?,'post_messages',?,?,?,?)", rusqlite::params![AGENT, ALL_PETS, DAVID, tx.now(), tx.now()])?;
        Ok(())
    }).await.unwrap();
    assert_eq!(a.anonymous().send(request(&poll_url)).await.status, StatusCode::FORBIDDEN);
    for url in &urls {
        assert_eq!(a.anonymous().send(request(url)).await.status, StatusCode::NOT_FOUND, "{url}");
    }
    let bot_key = a.db().write(move |tx| {
        use sha2::{Digest, Sha256};
        tx.conn().execute("UPDATE agent_grants SET room_id=? WHERE agent_id=?", [ALL_TALK, AGENT])?;
        let user = campfire_db::Agent::find(tx.conn(), AGENT)?.unwrap().user_id;
        tx.conn().execute("UPDATE users SET bot_token_digest=? WHERE id=?", rusqlite::params![format!("{:x}", Sha256::digest("poll-media-bot-key")), user])?;
        Ok(format!("{user}-poll-media-bot-key"))
    }).await.unwrap();
    assert_eq!(a.anonymous().send(request(&poll_url)).await.status, StatusCode::OK);
    for url in &urls {
        let bot_url = format!("{url}{}bot_key={bot_key}", if url.contains('?') { "&" } else { "?" });
        let response = follow_media_redirects(&mut a.anonymous(), get(&bot_url)).await;
        assert_eq!(response.status, StatusCode::OK, "{url}: {}", response.text());
        assert_eq!(response.header("cache-control"), Some("private, no-store"));
    }
    a.db().write(move |tx| {
        let user = campfire_db::Agent::find(tx.conn(), AGENT)?.unwrap().user_id;
        tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?", [ALL_TALK, user])?;
        Ok(())
    }).await.unwrap();
    assert_eq!(a.anonymous().send(request(&poll_url)).await.status, StatusCode::NOT_FOUND);
    for url in urls {
        assert_eq!(a.anonymous().send(request(&url)).await.status, StatusCode::NOT_FOUND, "{url}");
    }
}

#[tokio::test]
async fn poll_media_bot_key_still_honours_byte_ranges() {
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER_KEY};
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let staged = a.booted.app.storage.stage_bytes(
        include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
        campfire_storage::Filename::new("poll.gif"), Some("image/gif"),
    ).unwrap();
    let blob = a.db().write(move |tx| crate::controllers::messages::save_staged(tx, staged)).await.unwrap();
    let signed = campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut body = poll_body("poll-bot-still-range", &["Moving", "Quiet"]);
    body["optionMedia"] = json!([{ "signedId": signed }, null]);
    let posted = send(&mut author, Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/polls"), body).await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let posted: Value = parse(&posted);
    let still = posted["poll"]["options"][0]["media"]["stillUrl"].as_str().unwrap();
    let url = format!("{still}?bot_key={BENDER_KEY}");
    let full = a.anonymous().send(get(&url)).await;
    assert_eq!(full.status, StatusCode::OK, "{}", full.text());
    let length = full.body.len();
    let ranged = a.anonymous().send(get(&url).header("range", "bytes=0-7")).await;
    assert_eq!(ranged.status, StatusCode::PARTIAL_CONTENT, "{}", ranged.text());
    assert_eq!(ranged.body.len(), 8);
    assert_eq!(&ranged.body[..], &full.body[..8]);
    assert_eq!(ranged.header("content-range"), Some(format!("bytes 0-7/{length}").as_str()));
    assert_eq!(ranged.header("content-length"), Some("8"));
    assert_eq!(ranged.header("cache-control"), Some("private, no-store"));
}

#[tokio::test]
async fn poll_media_bot_key_ranged_original_keeps_attachment_disposition() {
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER_KEY};
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let staged = a.booted.app.storage.stage_bytes(
        include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
        campfire_storage::Filename::new("poll.gif"), Some("image/gif"),
    ).unwrap();
    let blob = a.db().write(move |tx| crate::controllers::messages::save_staged(tx, staged)).await.unwrap();
    let signed = campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut body = poll_body("poll-bot-original-range", &["Moving", "Quiet"]);
    body["optionMedia"] = json!([{ "signedId": signed }, null]);
    let posted = send(&mut author, Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/polls"), body).await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let posted: Value = parse(&posted);
    let original = posted["poll"]["options"][0]["media"]["url"].as_str().unwrap();
    let url = format!("{original}?disposition=attachment&bot_key={BENDER_KEY}");
    let full = a.anonymous().send(get(&url)).await;
    assert_eq!(full.status, StatusCode::OK, "{}", full.text());
    let full_disposition = full.header("content-disposition").unwrap().to_owned();
    assert!(full_disposition.starts_with("attachment") && full_disposition.contains("poll.gif"), "{full_disposition}");
    let ranged = a.anonymous().send(get(&url).header("range", "bytes=0-7")).await;
    assert_eq!(ranged.status, StatusCode::PARTIAL_CONTENT, "{}", ranged.text());
    assert_eq!(ranged.body.len(), 8);
    let disposition = ranged.header("content-disposition").unwrap();
    assert!(disposition.starts_with("attachment") && disposition.contains("poll.gif"), "{disposition}");
    assert_eq!(disposition, full_disposition);
}

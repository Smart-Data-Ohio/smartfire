//! S3 polls and cards on `/api/v1` (`campfire_api::cards`): posting and voting on polls with
//! their `poll.updated` and `poll.ballot` twins, event responses, the cards a message carries
//! with their `message.cards` twin, and the per-viewer previews.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve};
use crate::controllers::presenters::test_support::{
    Browser, DAVID, HQ, KEVIN, Reply, Req, TestApp,
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
async fn poll_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let designers = a
        .db()
        .read(|conn| campfire_db::Room::find(conn, DESIGNERS))
        .await
        .unwrap();
    let gid = campfire_app::cable::room_gid(&designers).to_param();
    let signed =
        rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&gid, "messages"]);
    client
        .confirm(&crate::channels::tests::support::identifier(
            json!({"channel": "RoomMessagesChannel", "signed_stream_name": signed}),
        ))
        .await;
    let mut kevin = a.sign_in(KEVIN).await;
    kevin.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let mut sync =
            Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    assert_eq!(a.booted.app.cable.sync_wanted(), spa);
    let capture = a.publications();
    capture.take();
    for option_ids in [json!([2]), json!([]), json!([3])] {
        let reply = kevin
            .write(
                Req::new(
                    Method::POST,
                    &format!("/rooms/{DESIGNERS}/polls/{OPEN_POLL}/vote"),
                )
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(json!({ "option_ids": option_ids }).to_string()),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }
    let now = a.booted.app.db.env().now();
    a.db()
        .write(move |tx| campfire_db::Poll::close_by_id(tx, OPEN_POLL, now))
        .await
        .unwrap();
    // Some frames go out after the response (the after-commit sink): wait for them to settle.
    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    cable.abort();
    if let Some(server) = server {
        server.abort();
    }
    Some(frames)
}

#[tokio::test]
async fn classic_poll_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (poll_frames(false).await, poll_frames(true).await) else {
        return;
    };
    let card = format!(r#"target=\"card_poll_{OPEN_POLL}\""#);
    assert_eq!(
        off.iter()
            .filter(|(_, frame)| frame.contains(&card))
            .count(),
        4,
        "{off:#?}"
    );
    assert_eq!(off, on);
}

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
    assert_eq!(
        cards,
        [api::MessageCard::Github(api::GithubCardRef {
            pull_request_id: 1,
            owner: "smart-data-ohio".into(),
            repo: "smartfire".into(),
            number: 42,
            url: "https://github.com/smart-data-ohio/smartfire/pull/42".into(),
        })]
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

    // A bot sees the counts but can't respond.
    sql(&a, "UPDATE users SET role = 2 WHERE id = ?", vec![KEVIN]).await;
    let shown: api::EventAttendance = ok(&kevin.send(get(&path)).await);
    assert!(!shown.respondable);
    assert_eq!(shown.going_count, before.going_count);
    let reply = send(
        &mut kevin,
        Method::PUT,
        &path,
        json!({"response": "going", "applyToFuture": false}),
    )
    .await;
    assert_eq!(fields(&reply), ["response"]);
    sql(&a, "UPDATE users SET role = 0 WHERE id = ?", vec![KEVIN]).await;

    // Another room's event, through this room, is a 404.
    let reply = kevin
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/events/{WATERCOOLER_IN_ALL_TALK}/attendance"
        )))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

//! `test/models/poll_test.rb`, plus the claim `close!` makes across processes and the poll's
//! part in `Message#destroy`.

use super::channel_thread_test::{create_thread, frozen, markdown};
use super::*;
use crate::broadcasts::{Broadcast, Partial};
use crate::{Error, Message, NewMessage, NewPoll, Poll, PollOption};

fn labels(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|l| l.to_string()).collect()
}

fn message(t: &TestDb, text: &str) -> Message {
    let attributes = markdown("watercooler", "david", text);
    t.write(move |tx| Message::create(tx, attributes))
}

fn create_poll(t: &TestDb, question: &str, options: NewPoll) -> Poll {
    let message = message(t, question);
    let options = NewPoll { labels: labels(&["Tacos", "Pizza"]), ..options };
    t.write(move |tx| Poll::create_for_message(tx, &message, options))
}

fn try_create(t: &TestDb, message: &Message, poll: NewPoll) -> Result<Poll> {
    let message = message.clone();
    t.try_write(move |tx| Poll::create_for_message(tx, &message, poll))
}

fn base_error(result: Result<impl std::fmt::Debug>) -> String {
    match result.unwrap_err() {
        Error::RecordInvalid(errors) => errors.full_messages().join(", "),
        other => panic!("{other:?}"),
    }
}

fn vote(t: &TestDb, poll: &Poll, voter: &str, option_ids: Vec<i64>) -> Result<Poll> {
    let (mut poll, voter) = (poll.clone(), id(voter));
    t.try_write(move |tx| {
        poll.cast_vote(tx, voter, &option_ids)?;
        Ok(poll)
    })
}

fn options(t: &TestDb, poll: &Poll) -> Vec<PollOption> {
    t.read(|c| poll.options(c))
}

fn david_votes(t: &TestDb, poll: &Poll) -> Vec<i64> {
    t.read(|c| poll.votes(c)).into_iter().filter(|v| v.user_id == id("david")).map(|v| v.poll_option_id).collect()
}

#[test]
fn creates_a_poll_with_options_on_a_message() {
    let t = frozen();
    let message = message(&t, "Lunch?");
    let poll = try_create(&t, &message, NewPoll { labels: labels(&["Tacos", "Pizza"]), ..Default::default() }).unwrap();
    assert_eq!(poll.message_id, message.id);
    assert_eq!(options(&t, &poll).iter().map(|o| o.label.as_str()).collect::<Vec<_>>(), vec!["Tacos", "Pizza"]);
    assert_eq!(options(&t, &poll).iter().map(|o| o.position).collect::<Vec<_>>(), vec![0, 1]);
    assert!(poll.single());
    assert!(!poll.anonymous);
    assert!(poll.open(t.now()));
}

#[test]
fn a_streaming_message_cannot_carry_a_poll() {
    let t = frozen();
    let attributes = NewMessage { streaming: true, ..markdown("watercooler", "david", "Drafting…") };
    let message = t.write(move |tx| Message::create(tx, attributes));
    let error = base_error(try_create(&t, &message, NewPoll { labels: labels(&["Tacos", "Pizza"]), ..Default::default() }));
    assert!(error.contains("cannot carry a poll"), "{error}");
    assert!(t.read(|c| Poll::find_by_message(c, message.id)).is_none());
}

#[test]
fn requires_between_two_and_ten_options() {
    let t = frozen();
    let message = message(&t, "Lunch?");
    let error = base_error(try_create(&t, &message, NewPoll { labels: labels(&["Only"]), ..Default::default() }));
    assert!(error.contains("between 2 and 10"), "{error}");
    assert!(t.read(|c| Poll::find_by_message(c, message.id)).is_none());
    let eleven: Vec<String> = (1..=11).map(|n| format!("Option {n}")).collect();
    let error = base_error(try_create(&t, &message, NewPoll { labels: eleven, ..Default::default() }));
    assert!(error.contains("between 2 and 10"), "{error}");
}

#[test]
fn blank_labels_are_dropped_before_counting() {
    let t = frozen();
    let message = message(&t, "Lunch?");
    let poll = try_create(&t, &message, NewPoll { labels: labels(&["Tacos", "", "  ", " Pizza "]), ..Default::default() }).unwrap();
    assert_eq!(options(&t, &poll).iter().map(|o| o.label.as_str()).collect::<Vec<_>>(), vec!["Tacos", "Pizza"]);
}

#[test]
fn one_message_carries_at_most_one_poll() {
    let t = frozen();
    let message = message(&t, "Lunch?");
    try_create(&t, &message, NewPoll { labels: labels(&["Tacos", "Pizza"]), ..Default::default() }).unwrap();
    let Error::RecordInvalid(errors) = try_create(&t, &message, NewPoll { labels: labels(&["Sushi", "Ramen"]), ..Default::default() }).unwrap_err() else {
        panic!()
    };
    assert_eq!(errors.on("message_id"), vec!["has already been taken"]);
}

#[test]
fn single_choice_voters_replace_their_ballot() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll::default());
    let options = options(&t, &poll);
    vote(&t, &poll, "david", vec![options[0].id]).unwrap();
    assert_eq!(david_votes(&t, &poll), vec![options[0].id]);
    vote(&t, &poll, "david", vec![options[1].id]).unwrap();
    assert_eq!(david_votes(&t, &poll), vec![options[1].id]);
    assert_eq!(t.read(|c| poll.votes(c)).len(), 1);
}

#[test]
fn single_choice_rejects_several_options() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll::default());
    let ids = options(&t, &poll).iter().map(|o| o.id).collect();
    assert!(base_error(vote(&t, &poll, "david", ids)).contains("only one option"));
}

#[test]
fn multiple_choice_voters_hold_several_options() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll { multiple: true, ..Default::default() });
    let ids = options(&t, &poll).iter().map(|o| o.id).collect();
    vote(&t, &poll, "david", ids).unwrap();
    assert_eq!(david_votes(&t, &poll).len(), 2);
}

#[test]
fn empty_ballots_retract_the_vote_and_touch_the_poll() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll::default());
    let first = options(&t, &poll)[0].id;
    let before = vote(&t, &poll, "david", vec![first]).unwrap().updated_at;
    t.travel(1);
    let after = vote(&t, &poll, "david", vec![]).unwrap();
    assert!(david_votes(&t, &poll).is_empty());
    assert!(t.read(|c| Poll::find(c, poll.id)).updated_at > before);
    assert_eq!(after.updated_at, t.now());
}

#[test]
fn votes_reject_foreign_options() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll::default());
    let other = create_poll(&t, "Dinner?", NewPoll::default());
    let foreign = options(&t, &other)[0].id;
    assert!(base_error(vote(&t, &poll, "david", vec![foreign])).contains("not part of this poll"));
}

#[test]
fn closed_polls_reject_votes() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll { closes_at: Some(t.now().since(jiff::SignedDuration::from_hours(1))), ..Default::default() });
    t.travel(2 * 3600);
    assert!(poll.closed(t.now()));
    let first = options(&t, &poll)[0].id;
    assert!(base_error(vote(&t, &poll, "david", vec![first])).contains("closed"));
}

#[test]
fn close_due_stamps_and_broadcasts_due_polls_once() {
    let t = frozen();
    let open_poll = create_poll(&t, "Lunch?", NewPoll { closes_at: Some(t.now().since(jiff::SignedDuration::from_hours(2))), ..Default::default() });
    let due_poll = create_poll(&t, "Due?", NewPoll { closes_at: Some(t.now().since(jiff::SignedDuration::from_hours(1))), ..Default::default() });
    t.travel(90 * 60);
    let from = t.events().len();
    let now = t.now();
    assert_eq!(t.write(move |tx| Poll::close_due(tx, now)), vec![due_poll.id]);
    assert!(!t.read(|c| Poll::find(c, open_poll.id)).closed(t.now()));
    let due = t.read(|c| Poll::find(c, due_poll.id));
    assert!(due.closed(t.now()));
    assert_eq!(due.closed_at, Some(t.now()));
    assert_eq!(t.write(move |tx| Poll::close_due(tx, now)), Vec::<i64>::new(), "once");

    let cards: Vec<_> = t.events()[from..].iter().filter_map(|e| e.as_broadcast()).collect();
    assert_eq!(cards.len(), 1);
    let Broadcast::Turbo(card) = &cards[0] else { panic!() };
    assert_eq!(card.target, format!("card_poll_{}", due_poll.id));
    assert_eq!(card.partial, Some(Partial::Poll { poll_id: due_poll.id }));
    assert!(card.maintain_scroll);
    assert_eq!(cards[0].stream_name(), format!("{}:messages", rails_compat::global_id::GlobalId::new("Rooms::Closed", id("watercooler")).to_param()));
}

#[test]
fn results_payload_carries_counts_and_voters_unless_anonymous() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll::default());
    let options = options(&t, &poll);
    vote(&t, &poll, "david", vec![options[0].id]).unwrap();
    vote(&t, &poll, "jason", vec![options[0].id]).unwrap();
    let payload = t.read(|c| poll.results_payload(c, &BasicRichText, t.now(), Some(id("david"))));
    assert_eq!(payload["question"], "Lunch?");
    assert_eq!(payload["total_votes"], 2);
    assert_eq!(payload["options"][0]["voters"], serde_json::json!(["David", "Jason"]));
    assert_eq!(payload["options"][0]["voted"], true);
    assert_eq!(payload["options"][1]["voted"], false);
    assert_eq!(payload["room_id"], id("watercooler"));
    assert_eq!(payload["closes_at"], serde_json::Value::Null);
    assert_eq!(payload["closed"], false);
}

#[test]
fn anonymous_payloads_carry_counts_only() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll { anonymous: true, ..Default::default() });
    let first = options(&t, &poll)[0].id;
    vote(&t, &poll, "david", vec![first]).unwrap();
    let payload = t.read(|c| poll.results_payload(c, &BasicRichText, t.now(), Some(id("jason"))));
    assert_eq!(payload["options"][0]["votes"], 1);
    assert!(payload["options"][0].get("voters").is_none());
    assert_eq!(payload["options"][0]["voted"], false);
}

#[test]
fn question_follows_the_message_text() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll { closes_at: Some(t.now().since(jiff::SignedDuration::from_hours(1))), ..Default::default() });
    let mut message = t.read(|c| Message::find(c, poll.message_id));
    t.write(move |tx| message.update(tx, crate::MessageChanges { markdown_source: Some("Dinner?".into()), ..Default::default() }));
    let payload = t.read(|c| poll.results_payload(c, &BasicRichText, t.now(), None));
    assert_eq!(payload["question"], "Dinner?");
    assert_eq!(payload["closes_at"], crate::models::poll::json_time(poll.closes_at.unwrap()));
    assert!(payload["closes_at"].as_str().unwrap().ends_with('Z'));
}

// Beyond poll_test.rb.

#[test]
fn closes_at_must_be_in_the_future_and_labels_are_limited() {
    let t = frozen();
    let message = message(&t, "Lunch?");
    let past = NewPoll { labels: labels(&["Tacos", "Pizza"]), closes_at: Some(t.now()), ..Default::default() };
    let Error::RecordInvalid(errors) = try_create(&t, &message, past).unwrap_err() else { panic!() };
    assert_eq!(errors.on("closes_at"), vec!["must be in the future"]);
    let long = NewPoll { labels: vec!["Tacos".into(), "x".repeat(201)], ..Default::default() };
    let Error::RecordInvalid(errors) = try_create(&t, &message, long).unwrap_err() else { panic!() };
    assert_eq!(errors.on("label"), vec!["is too long (maximum is 200 characters)"]);
    assert!(t.read(|c| Poll::find_by_message(c, message.id)).is_none(), "the poll rolls back with its options");
}

#[test]
fn a_thread_polls_card_broadcasts_to_the_thread() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Polls"));
    let thread_id = thread.id;
    let message = t.write(move |tx| {
        crate::ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, id("david"), NewMessage { markdown_source: Some("Q?".into()), ..Default::default() })
    });
    let poll = t.write(move |tx| Poll::create_for_message(tx, &message, NewPoll { labels: labels(&["A", "B"]), ..Default::default() }));
    let from = t.events().len();
    let first = options(&t, &poll)[0].id;
    vote(&t, &poll, "jason", vec![first]).unwrap();
    let card = t.events()[from..].iter().find_map(|e| e.as_broadcast()).unwrap();
    assert_eq!(card.stream_name(), format!("{}:messages", rails_compat::global_id::GlobalId::new("ChannelThread", thread_id).to_param()));
}

#[test]
fn destroying_the_message_destroys_its_poll_options_and_votes() {
    let t = frozen();
    let poll = create_poll(&t, "Lunch?", NewPoll { multiple: true, ..Default::default() });
    let ids = options(&t, &poll).iter().map(|o| o.id).collect();
    vote(&t, &poll, "david", ids).unwrap();
    let message = t.read(|c| Message::find(c, poll.message_id));
    t.write(move |tx| message.destroy(tx));
    for table in ["polls", "poll_options", "poll_votes"] {
        assert_eq!(t.read(|c| crate::sql::count(c, &format!("SELECT COUNT(*) FROM {table}"), [])), 0, "{table}");
    }
}

/// `close!`'s `update_all` claim holds across processes: several closers racing over one due
/// poll on their own connections close it exactly once, with one card broadcast.
#[test]
fn concurrent_closers_close_a_due_poll_once() {
    let t = frozen();
    let poll = create_poll(&t, "Race?", NewPoll { closes_at: Some(t.now().since(jiff::SignedDuration::from_mins(1))), ..Default::default() });
    t.travel(120);
    let from = t.events().len();
    let now = t.now();
    let handles: Vec<_> = (0..4).map(|_| t.another_process()).collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(handles.len()));
    let wins: Vec<bool> = std::thread::scope(|scope| {
        let threads: Vec<_> = handles
            .iter()
            .map(|db| {
                let barrier = barrier.clone();
                let poll_id = poll.id;
                scope.spawn(move || {
                    barrier.wait();
                    db.write_blocking(move |tx| Poll::close_by_id(tx, poll_id, now)).unwrap()
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    assert_eq!(wins.iter().filter(|w| **w).count(), 1, "{wins:?}");
    assert_eq!(t.events()[from..].iter().filter(|e| e.as_broadcast().is_some()).count(), 1);
}

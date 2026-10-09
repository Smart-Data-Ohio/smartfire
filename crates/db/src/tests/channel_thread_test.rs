//! `test/models/channel_thread_messages_count_test.rb`, `message_conversations_test.rb`,
//! `thread_tag_test.rb`, and the core `ChannelThread` rules they lean on (validations, the
//! lifecycle, `receive`, the stale sweep, thread memberships).

use super::*;
use crate::broadcasts::{Broadcast, Partial, TurboAction};
use crate::models::channel_thread::{PushMessageJob, thread_base_push};
use crate::{
    ChannelThread, Error, Involvement, Membership, Message, NewChannelThread, NewMessage, Room, RoomType, ThreadInvolvement,
    ThreadMembership, ThreadStatus, ThreadTag, Timeline,
};

/// `TestDb::new()` with time frozen, so timestamps compare exactly.
pub(super) fn frozen() -> TestDb {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    t
}

pub(super) fn markdown(room: &str, creator: &str, text: &str) -> NewMessage {
    NewMessage { room_id: id(room), creator_id: id(creator), markdown_source: Some(text.into()), ..Default::default() }
}

pub(super) fn post_root(t: &TestDb, room: &str, creator: &str, text: &str) -> Message {
    let attributes = markdown(room, creator, text);
    t.write(move |tx| Message::create(tx, attributes))
}

pub(super) fn create_thread(t: &TestDb, room: &str, creator: &str, parent_message_id: Option<i64>, name: Option<&str>) -> ChannelThread {
    let attributes = NewChannelThread {
        room_id: id(room),
        creator_id: id(creator),
        parent_message_id,
        name: name.map(Into::into),
        ..Default::default()
    };
    t.write(move |tx| ChannelThread::create(tx, attributes))
}

pub(super) fn post_reply(t: &TestDb, thread_id: i64, creator: &str, text: &str) -> Message {
    let creator_id = id(creator);
    let attributes = NewMessage { markdown_source: Some(text.into()), ..Default::default() };
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, creator_id, attributes))
}

fn thread(t: &TestDb, id: i64) -> ChannelThread {
    t.read(|c| ChannelThread::find(c, id))
}

fn message(t: &TestDb, id: i64) -> Message {
    t.read(|c| Message::find(c, id))
}

/// The thread indicator replaces broadcast since `from` (an index into the recorded events).
fn indicators(t: &TestDb, from: usize) -> Vec<(String, i64)> {
    t.events()[from..]
        .iter()
        .filter_map(|event| match event.as_broadcast()? {
            Broadcast::Turbo(stream) if stream.action == TurboAction::Replace => match stream.partial {
                Some(Partial::ThreadIndicator { reply_count, .. }) => {
                    assert!(stream.maintain_scroll);
                    Some((stream.target.clone(), reply_count))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

struct Counted {
    t: TestDb,
    parent: Message,
    thread: ChannelThread,
}

/// `setup`: a thread on `messages(:third)` in designers.
fn counted() -> Counted {
    let t = frozen();
    let parent = message(&t, id("third"));
    let thread = create_thread(&t, "designers", "jz", Some(parent.id), Some("Counted"));
    Counted { t, parent, thread }
}

fn indicator_target(parent: &Message) -> String {
    format!("thread_indicator_message_{}", parent.client_message_id)
}

// channel_thread_messages_count_test.rb

#[test]
fn a_new_thread_starts_at_zero() {
    let c = counted();
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 0);
}

#[test]
fn posting_a_reply_counts_it_and_bumps_the_parent_message() {
    let c = counted();
    let stamp = message(&c.t, c.parent.id).updated_at;
    c.t.travel(60);
    post_reply(&c.t, c.thread.id, "jz", "First");
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
    assert!(message(&c.t, c.parent.id).updated_at > stamp);
}

#[test]
fn the_counter_refresh_leaves_the_threads_own_updated_at_alone() {
    let c = counted();
    let reply = post_reply(&c.t, c.thread.id, "jz", "First");
    let stamp = thread(&c.t, c.thread.id).updated_at;
    c.t.travel(60);
    c.t.write(move |tx| reply.destroy(tx));
    let after = thread(&c.t, c.thread.id);
    assert_eq!(after.messages_count, 0);
    assert_eq!(after.updated_at, stamp);
}

#[test]
fn deleting_an_older_reply_lowers_the_count() {
    let c = counted();
    let older = post_reply(&c.t, c.thread.id, "jz", "Older");
    post_reply(&c.t, c.thread.id, "jz", "Newer");
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 2);
    c.t.write(move |tx| older.destroy(tx));
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
}

#[test]
fn system_notes_never_count() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "Real");
    let attributes = NewMessage { thread_id: Some(c.thread.id), system_note: true, ..markdown("designers", "jz", "pinned a message") };
    c.t.write(move |tx| Message::create(tx, attributes));
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
}

fn start_stream(c: &Counted, client_message_id: &str) -> Message {
    let bender = id("bender");
    let room_id = id("designers");
    c.t.write(move |tx| Room::find(tx.conn(), room_id)?.grant_to(tx, &[bender]));
    let attributes = NewMessage {
        client_message_id: Some(client_message_id.into()),
        streaming: true,
        markdown_source: Some("Thinking".into()),
        ..Default::default()
    };
    let thread_id = c.thread.id;
    c.t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, bender, attributes))
}

/// "a streaming reply counts only once it finalizes" and "a quietly finalized stream counts
/// too": both go through `claim_stream_finalized!`, which is what counts it.
#[test]
fn a_streaming_reply_counts_only_once_its_finalize_claim_lands() {
    let c = counted();
    let mut stream = start_stream(&c, "counter-stream");
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 0);
    assert!(c.t.write(move |tx| stream.claim_stream_finalized(tx)));
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
}

#[test]
fn a_second_finalize_claim_loses() {
    let c = counted();
    let stream = start_stream(&c, "counter-stream-twice");
    let mut first = stream.clone();
    assert!(c.t.write(move |tx| first.claim_stream_finalized(tx)));
    let mut second = stream;
    assert!(!c.t.write(move |tx| second.claim_stream_finalized(tx)));
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
}

#[test]
fn deleting_a_streaming_reply_leaves_the_count_alone() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "Done");
    let stream = start_stream(&c, "counter-dropped-stream");
    c.t.write(move |tx| stream.destroy(tx));
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 1);
}

#[test]
fn the_count_heals_from_the_rows_after_a_missed_refresh() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "One");
    let thread_id = c.thread.id;
    c.t.write(move |tx| Ok(tx.conn().execute("UPDATE channel_threads SET messages_count = 42 WHERE id = ?", [thread_id])?));
    post_reply(&c.t, c.thread.id, "jz", "Two");
    assert_eq!(thread(&c.t, c.thread.id).messages_count, 2);
}

#[test]
fn destroying_the_thread_with_its_replies_succeeds_and_stamps_the_parent() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "One");
    post_reply(&c.t, c.thread.id, "jz", "Two");
    let stamp = message(&c.t, c.parent.id).updated_at;
    c.t.travel(60);
    let doomed = thread(&c.t, c.thread.id);
    c.t.write(move |tx| doomed.destroy(tx));
    assert!(c.t.read(|conn| ChannelThread::find_by_id(conn, c.thread.id)).is_none());
    assert!(message(&c.t, c.parent.id).updated_at > stamp);
    assert_eq!(c.t.read(|conn| crate::sql::count(conn, "SELECT COUNT(*) FROM messages WHERE thread_id IS NOT NULL", [])), 0);
}

#[test]
fn a_posted_reply_broadcasts_the_parents_indicator_with_the_new_count() {
    let c = counted();
    let from = c.t.events().len();
    post_reply(&c.t, c.thread.id, "jz", "First");
    assert_eq!(indicators(&c.t, from), vec![(indicator_target(&c.parent), 1)]);
    let broadcast = c.t.events()[from..].iter().find_map(|e| e.as_broadcast().filter(|b| b.target().is_some())).unwrap();
    assert_eq!(broadcast.stream_name(), format!("{}:messages", rails_compat::global_id::GlobalId::new("Rooms::Closed", id("designers")).to_param()));
}

#[test]
fn a_deleted_reply_broadcasts_the_lowered_count() {
    let c = counted();
    let older = post_reply(&c.t, c.thread.id, "jz", "Older");
    post_reply(&c.t, c.thread.id, "jz", "Newer");
    let from = c.t.events().len();
    c.t.write(move |tx| older.destroy(tx));
    assert_eq!(indicators(&c.t, from), vec![(indicator_target(&c.parent), 1)]);
}

#[test]
fn deleting_the_last_reply_broadcasts_a_hidden_indicator() {
    let c = counted();
    let reply = post_reply(&c.t, c.thread.id, "jz", "Only");
    let from = c.t.events().len();
    c.t.write(move |tx| reply.destroy(tx));
    assert_eq!(indicators(&c.t, from), vec![(indicator_target(&c.parent), 0)]);
}

#[test]
fn a_finalized_thread_stream_broadcasts_the_parents_indicator() {
    let c = counted();
    let mut stream = start_stream(&c, "counter-broadcast-stream");
    let from = c.t.events().len();
    c.t.write(move |tx| stream.claim_stream_finalized(tx));
    assert_eq!(indicators(&c.t, from), vec![(indicator_target(&c.parent), 1)]);
}

#[test]
fn deleting_the_thread_broadcasts_a_hidden_indicator() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "One");
    let from = c.t.events().len();
    let doomed = thread(&c.t, c.thread.id);
    c.t.write(move |tx| doomed.destroy(tx));
    assert_eq!(indicators(&c.t, from), vec![(indicator_target(&c.parent), 0)]);
}

#[test]
fn a_system_note_in_the_thread_broadcasts_nothing() {
    let c = counted();
    post_reply(&c.t, c.thread.id, "jz", "Real");
    let from = c.t.events().len();
    let attributes = NewMessage { thread_id: Some(c.thread.id), system_note: true, ..markdown("designers", "jz", "note") };
    c.t.write(move |tx| Message::create(tx, attributes));
    assert_eq!(indicators(&c.t, from), vec![]);
}

// message_conversations_test.rb

#[test]
fn deleting_a_replied_to_message_leaves_a_body_less_tombstone() {
    let t = frozen();
    let source = post_root(&t, "designers", "jason", "Original body");
    let reply_attributes = NewMessage { reply_to_message_id: Some(source.id), ..markdown("designers", "jz", "Reply body") };
    let reply = t.write(move |tx| Message::create(tx, reply_attributes));
    t.write(move |tx| source.destroy(tx));
    let reply = message(&t, reply.id);
    assert!(reply.reply());
    assert_eq!(reply.reply_to_message_id, None);
    assert!(reply.reply_target_deleted_at.is_some());
    assert_eq!(t.read(|c| reply.plain_text_body(c, &BasicRichText)), "Reply body");
}

#[test]
fn replies_must_remain_in_their_root_stream_or_thread_except_a_thread_starter() {
    let t = frozen();
    let starter = post_root(&t, "designers", "jason", "Starter");
    let other_root = post_root(&t, "designers", "jason", "Elsewhere");
    let thread = create_thread(&t, "designers", "jz", Some(starter.id), Some("Discussion"));
    let thread_id = thread.id;
    t.write(move |tx| ThreadMembership::join(tx, thread_id, id("jz")));

    let to_starter = NewMessage { thread_id: Some(thread.id), reply_to_message_id: Some(starter.id), ..markdown("designers", "jz", "Allowed") };
    assert!(t.read(|c| Message::validate(c, &to_starter)).is_empty());

    let to_other = NewMessage { thread_id: Some(thread.id), reply_to_message_id: Some(other_root.id), ..markdown("designers", "jz", "Rejected") };
    let errors = t.read(|c| Message::validate(c, &to_other));
    assert_eq!(errors.on("reply_to_message"), vec!["must be in the same conversation"]);

    // And a root reply can't point into a thread.
    let in_thread = post_reply(&t, thread.id, "jz", "Threaded");
    let root_to_thread = NewMessage { reply_to_message_id: Some(in_thread.id), ..markdown("designers", "jz", "Out") };
    assert_eq!(t.read(|c| Message::validate(c, &root_to_thread)).on("reply_to_message"), vec!["must be in the same conversation"]);
}

#[test]
fn thread_push_notification_opens_the_parent_room_at_the_thread_message() {
    let t = frozen();
    let thread = create_thread(&t, "designers", "jz", None, Some("Push target"));
    let thread_id = thread.id;
    t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, id("jz"))?;
        ThreadMembership::join(tx, thread_id, id("jason"))?.update_involvement(tx, ThreadInvolvement::Everything)?;
        ThreadMembership::join(tx, thread_id, id("kevin"))?.update_involvement(tx, ThreadInvolvement::Everything)?;
        let mut kevin = Membership::find_by_room_and_user(tx.conn(), id("designers"), id("kevin"))?.unwrap();
        kevin.update_involvement(tx, Involvement::Invisible)
    });
    let attributes = NewMessage { client_message_id: Some("thread-push-target".into()), markdown_source: Some("Open here".into()), ..Default::default() };
    let message = t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, id("jz"), attributes));

    let pushes = t.read(|c| ChannelThread::push_recipients(c, &BasicRichText, thread_id, message.id, &thread_base_push));
    assert_eq!(pushes.len(), 1);
    let push = &pushes[0];
    assert_eq!(push.user_id, id("jason"));
    assert_eq!(push.payload.path, format!("/rooms/{}?message_id={}&thread={}", id("designers"), message.id, thread_id));
    assert_eq!(push.tag, format!("room-{}", id("designers")));
    assert_eq!(push.payload.title, "Push target");
    assert_eq!(push.payload.body, "JZ: Open here");
    assert!(push.subscriptions.iter().all(|s| s.user_id == id("jason")));
}

// thread_tag_test.rb

fn board_post(t: &TestDb, name: &'static str) -> ChannelThread {
    let david = id("david");
    t.write(move |tx| {
        let room = match Room::for_user_of_type(tx.conn(), david, RoomType::Board)?.into_iter().next() {
            Some(room) => room,
            None => Room::create_for(tx, RoomType::Board, Some("Launch"), david, &[david])?,
        };
        ChannelThread::create(
            tx,
            NewChannelThread { room_id: room.id, creator_id: david, name: Some(name.into()), work_status: Some("planned".into()), ..Default::default() },
        )
    })
}

#[test]
fn tag_name_is_required_lowercase_and_limited_in_length() {
    let t = frozen();
    let post = board_post(&t, "Tagged post");
    let errors = |name: String| t.read(|c| ThreadTag::validate(c, post.id, &name));
    assert!(errors("".into()).on("name").contains(&"can't be blank"));
    assert!(!errors("Needs Work!".into()).is_empty());
    assert!(!errors("a".repeat(31)).is_empty());
    assert!(errors("api-v2".into()).is_empty());
}

#[test]
fn tag_names_are_unique_per_post_but_reusable_across_posts() {
    let t = frozen();
    let post = board_post(&t, "Tagged post");
    t.write(move |tx| ThreadTag::create(tx, post.id, "bug"));
    let other = board_post(&t, "Other post");
    assert!(!t.read(|c| ThreadTag::validate(c, post.id, "bug")).is_empty());
    assert!(t.read(|c| ThreadTag::validate(c, other.id, "bug")).is_empty());
}

// The core rules the tests above lean on.

#[test]
fn threads_take_a_default_name_from_the_parents_first_line() {
    let t = frozen();
    let long = format!("{}\nsecond line", "x".repeat(120));
    let parent = post_root(&t, "designers", "jason", &long);
    let named = create_thread(&t, "designers", "jz", Some(parent.id), None);
    assert_eq!(named.name, format!("{}…", "x".repeat(99)));
    assert_eq!(named.name.chars().count(), 100);
    let unnamed = create_thread(&t, "designers", "jz", None, None);
    assert_eq!(unnamed.name, "New thread");
    assert_eq!(unnamed.auto_archive_after_minutes, 4320);
    assert_eq!(unnamed.last_activity_at, t.now());
}

#[test]
fn threads_validate_their_room_parent_and_board_state() {
    let t = frozen();
    let direct = NewChannelThread { room_id: id("david_and_kevin"), creator_id: id("david"), name: Some("DM".into()), ..Default::default() };
    let error = t.try_write(move |tx| ChannelThread::create(tx, direct)).unwrap_err();
    let Error::RecordInvalid(errors) = error else { panic!("{error:?}") };
    assert_eq!(errors.on("room"), vec!["can't be a direct room"]);

    let elsewhere = post_root(&t, "pets", "david", "Elsewhere");
    let wrong_parent = NewChannelThread { room_id: id("designers"), creator_id: id("david"), parent_message_id: Some(elsewhere.id), ..Default::default() };
    let Error::RecordInvalid(errors) = t.try_write(move |tx| ChannelThread::create(tx, wrong_parent)).unwrap_err() else { panic!() };
    assert_eq!(errors.on("parent_message"), vec!["must be a root message in the parent room"]);

    let bad_archive = NewChannelThread { room_id: id("designers"), creator_id: id("david"), name: Some("x".into()), auto_archive_after_minutes: Some(5), ..Default::default() };
    let Error::RecordInvalid(errors) = t.try_write(move |tx| ChannelThread::create(tx, bad_archive)).unwrap_err() else { panic!() };
    assert_eq!(errors.on("auto_archive_after_minutes"), vec!["is not included in the list"]);

    let david = id("david");
    let board = t.write(move |tx| Room::create_for(tx, RoomType::Board, Some("Work"), david, &[david]));
    let untracked = NewChannelThread { room_id: board.id, creator_id: david, name: Some("Post".into()), tag_names: Some(vec!["A".into(), "b c".into(), " a ".into()]), ..Default::default() };
    let Error::RecordInvalid(errors) = t.try_write(move |tx| ChannelThread::create(tx, untracked)).unwrap_err() else { panic!() };
    assert_eq!(errors.on("work_status"), vec!["must be tracked in a board"]);
    assert_eq!(errors.on("tags"), vec!["use lowercase letters, digits, and hyphens"]);

    let tagged = NewChannelThread {
        room_id: board.id,
        creator_id: david,
        name: Some("Post".into()),
        work_status: Some("planned".into()),
        tag_names: Some(vec!["Bug".into(), "bug".into(), " ".into(), "ui".into()]),
        ..Default::default()
    };
    let post = t.write(move |tx| ChannelThread::create(tx, tagged));
    assert_eq!(t.read(|c| post.tag_names(c)), vec!["bug", "ui"]);
    assert_eq!(post.work_status_changed_at, Some(t.now()));
}

#[test]
fn threads_go_stale_and_the_next_thread_write_closes_them() {
    let t = frozen();
    let stale = create_thread(&t, "designers", "jz", None, Some("Quiet"));
    let busy = create_thread(&t, "designers", "jz", None, Some("Busy"));
    assert_eq!(t.read(|c| stale.status(c, t.now())), ThreadStatus::Active);

    t.travel(4320 * 60);
    // Reads report it closed before anything persists it.
    assert_eq!(t.read(|c| stale.status(c, t.now())), ThreadStatus::Closed);
    assert_eq!(thread(&t, stale.id).closed_at, None);

    // A post to a sibling thread runs the room's sweep after commit.
    post_reply(&t, busy.id, "jz", "Still here");
    assert_eq!(thread(&t, stale.id).closed_at, Some(t.now()));
    assert_eq!(thread(&t, busy.id).closed_at, None);
    assert_eq!(thread(&t, busy.id).last_activity_at, t.now());
}

#[test]
fn boards_never_go_stale() {
    let t = frozen();
    let post = board_post(&t, "Forever");
    t.travel(10_080 * 60 * 2);
    assert!(!t.read(|c| post.stale(c, t.now())));
    assert_eq!(t.write(|tx| ChannelThread::close_stale_in(tx, None)), 0);
}

#[test]
fn posting_reopens_a_closed_thread_and_refuses_a_locked_one() {
    let t = frozen();
    let created = create_thread(&t, "designers", "jz", None, Some("Lifecycle"));
    let thread_id = created.id;
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.close(tx));
    assert_eq!(t.read(|c| thread(&t, thread_id).status(c, t.now())), ThreadStatus::Closed);
    post_reply(&t, thread_id, "jz", "Back");
    assert_eq!(thread(&t, thread_id).closed_at, None);

    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.lock_conversation(tx));
    let locked = thread(&t, thread_id);
    assert_eq!(t.read(|c| locked.status(c, t.now())), ThreadStatus::Locked);
    assert!(locked.closed_at.is_some());
    let attributes = NewMessage { markdown_source: Some("Nope".into()), ..Default::default() };
    let error = t.try_write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, id("jz"), attributes)).unwrap_err();
    assert_eq!(error.to_string(), "This thread is locked");

    // Closing a locked thread changes nothing; unlocking reopens it.
    let before = thread(&t, thread_id);
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.close(tx));
    assert_eq!(thread(&t, thread_id), before);
    t.travel(4320 * 60);
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.unlock_conversation(tx));
    let unlocked = thread(&t, thread_id);
    assert_eq!((unlocked.locked_at, unlocked.closed_at), (None, None));
    assert_eq!(unlocked.last_activity_at, t.now(), "a stale unlock gets fresh activity");
}

#[test]
fn reopen_revives_a_stale_thread() {
    let t = frozen();
    let created = create_thread(&t, "designers", "jz", None, Some("Stale"));
    t.travel(60 * 60 * 24 * 4);
    let thread_id = created.id;
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.reopen(tx));
    let reopened = thread(&t, thread_id);
    assert_eq!(t.read(|c| reopened.status(c, t.now())), ThreadStatus::Active);
    assert_eq!(reopened.last_activity_at, t.now());
}

#[test]
fn posting_requires_room_membership_and_joins_the_thread() {
    let t = frozen();
    let created = create_thread(&t, "designers", "jz", None, Some("Members"));
    let thread_id = created.id;
    // Bender isn't in designers.
    let attributes = NewMessage { markdown_source: Some("Hi".into()), ..Default::default() };
    let error = t.try_write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, id("bender"), attributes)).unwrap_err();
    assert!(matches!(error, Error::RecordNotFound("Membership")), "{error:?}");
    assert!(t.try_write(move |tx| ThreadMembership::join(tx, thread_id, id("bender"))).is_err());

    post_reply(&t, thread_id, "kevin", "Hello");
    let membership = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("kevin"))).unwrap();
    assert_eq!(membership.involvement, ThreadInvolvement::Mentions);
    assert_eq!(membership.joined_at, t.now());
}

#[test]
fn receiving_marks_other_members_unread_broadcasts_and_enqueues_the_push() {
    let t = frozen();
    let created = create_thread(&t, "designers", "jz", None, Some("Unread"));
    let thread_id = created.id;
    t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, id("jason"))?;
        ThreadMembership::join(tx, thread_id, id("kevin"))
    });
    // Kevin leaves the room: his thread membership goes with it.
    t.write(move |tx| Membership::find_by_room_and_user(tx.conn(), id("designers"), id("kevin"))?.unwrap().destroy(tx));
    assert!(t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("kevin"))).is_none());

    let from = t.events().len();
    t.travel(5);
    let reply = post_reply(&t, thread_id, "jz", "News");
    let jason = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    assert_eq!(jason.unread_at, Some(reply.created_at));
    let jz = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jz"))).unwrap();
    assert_eq!(jz.unread_at, None);

    let events = t.events()[from..].to_vec();
    let cable: Vec<_> = events
        .iter()
        .filter_map(|e| match e.as_broadcast()? {
            Broadcast::Cable { stream, payload } => Some((stream.clone(), payload.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        cable,
        vec![(format!("user_{}_unread_threads", id("jason")), serde_json::json!({ "threadId": thread_id, "roomId": id("designers") }))]
    );
    let jobs: Vec<PushMessageJob> = events.iter().filter_map(|e| e.as_job::<PushMessageJob>()).collect();
    assert_eq!(jobs, vec![PushMessageJob { thread_id, message_id: reply.id }]);
    assert!(!events.iter().any(|e| matches!(e, Event::PushMessage { .. })), "a thread reply doesn't push the room");

    let mut jason = jason;
    t.write(move |tx| jason.read(tx));
    assert!(!t.read(|c| ChannelThread::find(c, thread_id)?.unread_for(c, id("jason"))));
}

/// Reading a thread records the newest message it was read through, and a no-op read (already
/// read) leaves it.
#[test]
fn reading_a_thread_records_how_far_it_was_read() {
    let t = frozen();
    let thread_id = create_thread(&t, "designers", "jz", None, Some("Read through")).id;
    let joined = t.write(move |tx| ThreadMembership::join(tx, thread_id, id("jason")));
    assert_eq!(joined.last_read_message_id, None);

    t.travel(5);
    let first = post_reply(&t, thread_id, "jz", "One");
    let mut jason = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    t.write(move |tx| jason.read(tx).map(|()| jason));
    let jason = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    assert_eq!((jason.unread_at, jason.last_read_message_id), (None, Some(first.id)));

    t.travel(5);
    post_root(&t, "designers", "jz", "Elsewhere");
    let mut again = jason.clone();
    t.write(move |tx| again.read(tx).map(|()| again));
    let unchanged = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    assert_eq!(unchanged.last_read_message_id, Some(first.id), "an already-read thread's read is a no-op");

    t.travel(5);
    let second = post_reply(&t, thread_id, "jz", "Two");
    let mut unread = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    assert!(unread.unread());
    t.write(move |tx| unread.read(tx).map(|()| unread));
    let read = t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread_id, id("jason"))).unwrap();
    assert_eq!(read.last_read_message_id, Some(second.id));
}

#[test]
fn the_room_timeline_leaves_thread_messages_out() {
    let t = frozen();
    let root = post_root(&t, "designers", "jz", "Root");
    let created = create_thread(&t, "designers", "jz", Some(root.id), None);
    let reply = post_reply(&t, created.id, "jz", "Threaded");
    let room_page = t.read(|c| Message::last_page(c, Timeline::Room(id("designers"))));
    assert!(room_page.iter().any(|m| m.id == root.id));
    assert!(!room_page.iter().any(|m| m.id == reply.id));
    let thread_page = t.read(|c| Message::last_page(c, Timeline::Thread(created.id)));
    assert_eq!(thread_page.iter().map(|m| m.id).collect::<Vec<_>>(), vec![reply.id]);
}

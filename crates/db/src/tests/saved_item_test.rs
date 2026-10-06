//! `test/models/saved_item_test.rb`, `test/models/saved_item/reminder_dispatcher_test.rb` and
//! `reminder_pusher_test.rb`, plus the dispatcher's claim across processes.

use jiff::SignedDuration;

use super::channel_thread_test::{create_thread, frozen, post_reply};
use super::*;
use crate::broadcasts::Broadcast;
use crate::models::saved_item::ReminderPushJob;
use crate::{ActivityItem, Error, Membership, Message, NewSavedItem, Room, SavedItem, SavedItemChanges, Timestamp, User};

fn minutes(n: i64) -> SignedDuration {
    SignedDuration::from_mins(n)
}

fn save(t: &TestDb, user: &str, message: &str, remind_at: Option<Timestamp>) -> SavedItem {
    try_save(t, user, message, remind_at).unwrap()
}

fn try_save(t: &TestDb, user: &str, message: &str, remind_at: Option<Timestamp>) -> Result<SavedItem> {
    let attributes = NewSavedItem { user_id: id(user), message_id: id(message), remind_at, status: None };
    t.try_write(move |tx| SavedItem::create(tx, attributes))
}

fn reload(t: &TestDb, item: &SavedItem) -> SavedItem {
    t.read(|c| SavedItem::find(c, item.id))
}

fn accessible_ids(t: &TestDb, user: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = t.read(|c| SavedItem::accessible_to(c, id(user))).into_iter().map(|i| i.id).collect();
    ids.sort();
    ids
}

fn activity_count(t: &TestDb) -> i64 {
    t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "activity_items""#, []))
}

fn reminder_item(t: &TestDb, user: &str, item: &SavedItem) -> Option<ActivityItem> {
    t.read(|c| ActivityItem::find_by_user_and_source(c, id(user), "SavedItem", item.id))
}

fn remove_membership(t: &TestDb, name: &str) {
    let membership_id = id(name);
    t.write(move |tx| Membership::find(tx.conn(), membership_id)?.destroy(tx));
}

fn dispatch_due(t: &TestDb) -> Vec<i64> {
    t.write(|tx| {
        let now = tx.now();
        SavedItem::dispatch_due(tx, now)
    })
}

fn push_jobs(t: &TestDb, from: usize) -> Vec<ReminderPushJob> {
    t.events()[from..].iter().filter_map(|e| e.as_job::<ReminderPushJob>()).collect()
}

fn mark_read(t: &TestDb, item: &ActivityItem) {
    let id = item.id;
    t.write(move |tx| {
        tx.conn().execute(r#"UPDATE "activity_items" SET "read_at" = ? WHERE "id" = ?"#, rusqlite::params![tx.now(), id])?;
        Ok(())
    });
}

// SavedItemTest

#[test]
fn saving_defaults_to_in_progress_without_a_reminder() {
    let t = frozen();
    let item = save(&t, "david", "first", None);
    assert!(item.in_progress());
    assert_eq!(item.remind_at, None);
    assert!(!item.reminder_pending());
}

#[test]
fn one_saved_item_per_user_per_message() {
    let t = frozen();
    save(&t, "david", "first", None);
    let Error::RecordInvalid(errors) = try_save(&t, "david", "first", None).unwrap_err() else { panic!() };
    assert_eq!(errors.on("message_id"), vec!["has already been taken"]);
    assert!(try_save(&t, "jason", "first", None).is_ok());
}

#[test]
fn reminders_must_be_in_the_future() {
    let t = frozen();
    let Error::RecordInvalid(errors) = try_save(&t, "david", "first", Some(t.now().ago(minutes(1)))).unwrap_err() else { panic!() };
    assert_eq!(errors.on("remind_at"), vec!["must be in the future"]);
    assert!(try_save(&t, "david", "second", Some(t.now().since(minutes(1)))).is_ok());
}

#[test]
fn statuses_are_in_progress_or_done() {
    let t = frozen();
    let mut item = save(&t, "david", "first", None);
    let mut bad = item.clone();
    let Error::RecordInvalid(errors) =
        t.try_write(move |tx| bad.update(tx, SavedItemChanges { status: Some("later".into()), ..Default::default() })).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(errors.on("status"), vec!["is not included in the list"]);
    t.travel(60);
    item = t.write(move |tx| {
        item.update(tx, SavedItemChanges { status: Some("done".into()), ..Default::default() })?;
        Ok(item)
    });
    assert_eq!((item.status.as_str(), item.updated_at), ("done", t.now()));
}

#[test]
fn accessible_items_hide_rooms_the_user_can_no_longer_see() {
    let t = frozen();
    let item = save(&t, "david", "first", None);
    let other_room_item = save(&t, "david", "fourth", None);
    let mut both = vec![item.id, other_room_item.id];
    both.sort();
    assert_eq!(accessible_ids(&t, "david"), both);

    remove_membership(&t, "david_designers");
    assert_eq!(accessible_ids(&t, "david"), vec![other_room_item.id]);

    t.write(|tx| Room::find(tx.conn(), id("designers"))?.grant_to(tx, &[id("david")]));
    assert!(accessible_ids(&t, "david").contains(&item.id));
}

#[test]
fn accessible_items_hide_other_users_bots_and_deleted_rooms() {
    let t = frozen();
    save(&t, "david", "first", None);
    save(&t, "jason", "first", None);
    let messages: Vec<i64> = t.read(|c| SavedItem::accessible_to(c, id("david"))).into_iter().map(|i| i.message_id).collect();
    assert_eq!(messages, vec![id("first")]);
    assert!(t.read(|c| SavedItem::accessible_to(c, id("bender"))).is_empty());

    t.write(|tx| {
        tx.conn().execute(r#"UPDATE "rooms" SET "deleted_at" = ? WHERE "id" = ?"#, rusqlite::params![tx.now(), id("designers")])?;
        Ok(())
    });
    assert!(t.read(|c| SavedItem::accessible_to(c, id("david"))).is_empty());
}

#[test]
fn due_reminders_are_pending_items_whose_time_has_come() {
    let t = frozen();
    let due = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    let due_ids = |t: &TestDb, at: Timestamp| t.read(|c| SavedItem::due_reminders(c, at)).into_iter().map(|i| i.id).collect::<Vec<_>>();
    assert!(due_ids(&t, t.now().since(minutes(61))).contains(&due.id));
    assert!(!due_ids(&t, t.now()).contains(&due.id));
    let plain = save(&t, "david", "second", None);
    assert!(!due_ids(&t, t.now().since(minutes(61))).contains(&plain.id));

    let fired = save(&t, "david", "third", Some(t.now().since(minutes(60))));
    t.travel(61 * 60);
    t.write(move |tx| {
        tx.conn().execute(r#"UPDATE "saved_items" SET "reminded_at" = ? WHERE "id" = ?"#, rusqlite::params![tx.now(), fired.id])?;
        Ok(())
    });
    assert!(!due_ids(&t, t.now()).contains(&fired.id));
}

#[test]
fn a_firing_reminder_creates_a_separate_inbox_item_and_leaves_mentions_alone() {
    let t = frozen();
    let mention = t.write(|tx| ActivityItem::refresh_unread(tx, id("david"), "Message", id("first"), "mention"));
    mark_read(&t, &mention);
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    let before = activity_count(&t);
    let reminder = t.write({
        let item = item.clone();
        move |tx| item.create_reminder_item(tx)
    });
    assert_eq!(activity_count(&t), before + 1);
    assert_eq!(reminder.event_type, "message_reminder");
    assert_eq!((reminder.source_type.as_str(), reminder.source_id), ("SavedItem", item.id));
    assert!(reminder.unread());

    let mention = t.read(|c| ActivityItem::find(c, mention.id));
    assert_eq!(mention.event_type, "mention");
    assert!(mention.read_at.is_some());
}

#[test]
fn re_firing_a_reminder_refreshes_its_own_inbox_item_in_place() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    let create = |t: &TestDb| {
        let item = item.clone();
        t.write(move |tx| item.create_reminder_item(tx))
    };
    let first = create(&t);
    mark_read(&t, &first);
    let before = activity_count(&t);
    let from = t.events().len();
    let second = create(&t);
    assert_eq!(activity_count(&t), before);
    assert_eq!(second.id, first.id);
    assert_eq!(second.event_type, "message_reminder");
    assert!(second.unread());
    // `broadcast_updated`: read_at changed, so the inbox hears of it.
    let activity = t.events()[from..].iter().filter_map(|e| e.as_broadcast()).collect::<Vec<_>>();
    assert_eq!(
        activity,
        vec![Broadcast::Cable { stream: format!("user_{}_activity", id("david")), payload: serde_json::json!({ "activityItemId": first.id }) }]
    );
    // An unread item refreshed again changes nothing, so it's quiet.
    let from = t.events().len();
    create(&t);
    assert!(t.events()[from..].iter().all(|e| e.as_broadcast().is_none()));
}

#[test]
fn destroying_the_message_destroys_its_saved_items() {
    let t = frozen();
    save(&t, "david", "first", None);
    let message = t.read(|c| Message::find(c, id("first")));
    t.write(move |tx| message.destroy(tx));
    assert_eq!(t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "saved_items""#, [])), 0);
}

#[test]
fn destroying_a_saved_item_destroys_its_reminder_item() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    let reminder = t.write({
        let item = item.clone();
        move |tx| item.create_reminder_item(tx)
    });
    let before = activity_count(&t);
    t.write(move |tx| item.destroy(tx));
    assert_eq!(activity_count(&t), before - 1);
    assert!(t.read(|c| ActivityItem::find_by_user_and_source(c, id("david"), "SavedItem", reminder.source_id)).is_none());
}

#[test]
fn setting_a_new_reminder_time_after_a_firing_re_arms_the_reminder() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    t.travel(120);
    dispatch_due(&t);
    assert!(reload(&t, &item).reminded_at.is_some());

    let later = t.now().since(minutes(60));
    let item = t.write(move |tx| SavedItem::save_for(tx, id("david"), id("first"), Some(later)));
    assert_eq!(item.reminded_at, None);
    assert!(item.reminder_pending());
    assert!(t.read(|c| SavedItem::due_reminders(c, later.since(minutes(1)))).iter().any(|i| i.id == item.id));
}

#[test]
fn re_saving_with_the_same_reminder_writes_nothing() {
    let t = frozen();
    let at = t.now().since(minutes(5));
    let item = save(&t, "david", "first", Some(at));
    t.travel(60);
    let again = t.write(move |tx| SavedItem::save_for(tx, id("david"), id("first"), Some(at)));
    assert_eq!(again, item);
}

// SavedItem::ReminderDispatcherTest

#[test]
fn a_due_reminder_notifies_once_and_enqueues_push() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    t.travel(61 * 60);
    let from = t.events().len();
    assert_eq!(dispatch_due(&t), vec![item.id]);
    assert_eq!(push_jobs(&t, from), vec![ReminderPushJob { saved_item_id: item.id }]);
    assert_eq!(reload(&t, &item).reminded_at, Some(t.now()));
    let inbox = reminder_item(&t, "david", &item).unwrap();
    assert_eq!(inbox.event_type, "message_reminder");
    assert!(inbox.unread());
}

#[test]
fn a_reminder_fires_only_once_across_runs() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    t.travel(120);
    dispatch_due(&t);
    let fired_at = reload(&t, &item).reminded_at;
    t.travel(1);
    let from = t.events().len();
    assert!(dispatch_due(&t).is_empty());
    assert!(push_jobs(&t, from).is_empty());
    assert_eq!(reload(&t, &item).reminded_at, fired_at);
    let count = t.read(|c| {
        crate::sql::count(c, r#"SELECT COUNT(*) FROM "activity_items" WHERE "source_type" = 'SavedItem' AND "source_id" = ? AND "event_type" = 'message_reminder'"#, [item.id])
    });
    assert_eq!(count, 1);
}

#[test]
fn a_reminder_rescheduled_past_now_before_the_claim_does_not_fire() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    t.travel(120);
    // Selected while due, then moved before the claim: the re-check inside the claim sees it.
    let selected = t.read(|c| SavedItem::due_reminder_ids(c, t.now()));
    assert_eq!(selected, vec![item.id]);
    let later = t.now().since(minutes(60));
    t.write(move |tx| SavedItem::save_for(tx, id("david"), id("first"), Some(later)));
    let before = activity_count(&t);
    let from = t.events().len();
    assert!(!t.write(move |tx| {
        let now = tx.now();
        SavedItem::dispatch_reminder(tx, item.id, now)
    }));
    assert!(push_jobs(&t, from).is_empty());
    assert_eq!(activity_count(&t), before);
    assert!(reload(&t, &item).reminder_pending());
}

#[test]
fn future_reminders_and_items_without_reminders_do_not_fire() {
    let t = frozen();
    let future = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    let plain = save(&t, "david", "second", None);
    let from = t.events().len();
    assert!(dispatch_due(&t).is_empty());
    assert!(push_jobs(&t, from).is_empty());
    assert_eq!(reload(&t, &future).reminded_at, None);
    assert_eq!(reload(&t, &plain).reminded_at, None);
    assert_eq!(activity_count(&t), 0);
}

#[test]
fn a_reminder_scheduled_in_another_time_zone_fires_at_the_same_instant() {
    let t = TestDb::new();
    let noon: Timestamp = Timestamp::from_jiff("2026-09-24T12:00:00Z".parse().unwrap());
    t.clock.travel_to(noon);
    // 9am in New York is 1pm UTC (September daylight time).
    let remind_at = Timestamp::from_jiff(
        jiff::civil::date(2026, 9, 24).at(9, 0, 0, 0).in_tz("America/New_York").unwrap().timestamp(),
    );
    assert_eq!(remind_at.jiff().to_string(), "2026-09-24T13:00:00Z");
    let item = save(&t, "david", "first", Some(remind_at));

    t.clock.travel_to(Timestamp::from_jiff("2026-09-24T12:59:00Z".parse().unwrap()));
    dispatch_due(&t);
    assert_eq!(reload(&t, &item).reminded_at, None);

    t.clock.travel_to(Timestamp::from_jiff("2026-09-24T13:01:00Z".parse().unwrap()));
    dispatch_due(&t);
    assert!(reload(&t, &item).reminded_at.is_some());
    assert_eq!(reminder_item(&t, "david", &item).unwrap().event_type, "message_reminder");
}

#[test]
fn a_saver_who_lost_room_access_is_claimed_without_notifying() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    remove_membership(&t, "david_designers");
    t.travel(120);
    let from = t.events().len();
    assert!(dispatch_due(&t).is_empty());
    assert!(push_jobs(&t, from).is_empty());
    assert!(reload(&t, &item).reminded_at.is_some());
    assert!(reminder_item(&t, "david", &item).is_none());
}

#[test]
fn every_due_reminder_dispatches_in_one_run() {
    let t = frozen();
    let david_item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    let jason_item = save(&t, "jason", "second", Some(t.now().since(minutes(1))));
    t.travel(120);
    let from = t.events().len();
    dispatch_due(&t);
    assert_eq!(push_jobs(&t, from).len(), 2);
    assert_eq!(reminder_item(&t, "david", &david_item).unwrap().event_type, "message_reminder");
    assert_eq!(reminder_item(&t, "jason", &jason_item).unwrap().event_type, "message_reminder");
}

#[test]
fn deactivated_savers_and_deleted_rooms_are_not_due() {
    let t = frozen();
    save(&t, "david", "first", Some(t.now().since(minutes(1))));
    let kevin = save(&t, "kevin", "fourth", Some(t.now().since(minutes(1))));
    t.travel(120);
    t.write(|tx| User::find(tx.conn(), id("david"))?.deactivate(tx));
    assert_eq!(t.read(|c| SavedItem::due_reminder_ids(c, t.now())), vec![kevin.id]);
    t.write(|tx| {
        tx.conn().execute(r#"UPDATE "rooms" SET "deleted_at" = ? WHERE "id" = ?"#, rusqlite::params![tx.now(), id("watercooler")])?;
        Ok(())
    });
    assert!(t.read(|c| SavedItem::due_reminder_ids(c, t.now())).is_empty());
}

#[test]
fn concurrent_dispatchers_fire_a_reminder_once() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(1))));
    t.travel(120);
    let now = t.now();
    let from = t.events().len();
    let handles: Vec<_> = (0..4).map(|_| t.another_process()).collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(handles.len()));
    let wins: Vec<bool> = std::thread::scope(|scope| {
        let threads: Vec<_> = handles
            .iter()
            .map(|db| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let ids = db.read_blocking(|c| SavedItem::due_reminder_ids(c, now)).unwrap();
                    barrier.wait();
                    ids.into_iter().any(|id| db.write_blocking(move |tx| SavedItem::dispatch_reminder(tx, id, now)).unwrap())
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    assert_eq!(wins.iter().filter(|w| **w).count(), 1, "{wins:?}");
    assert_eq!(push_jobs(&t, from), vec![ReminderPushJob { saved_item_id: item.id }]);
    assert_eq!(activity_count(&t), 1);
}

// SavedItem::ReminderPusherTest

fn push(t: &TestDb, item: &SavedItem, policy: &dyn Fn(&User) -> bool) -> Option<crate::models::saved_item::ReminderPush> {
    t.read(|c| SavedItem::reminder_push(c, &BasicRichText, item.id, policy))
}

#[test]
fn pushes_the_reminder_to_the_saver_with_a_message_link() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    let push = push(&t, &item, &|_| true).unwrap();
    let message = t.read(|c| Message::find(c, id("first")));
    let text = t.read(|c| message.plain_text_body(c, &BasicRichText));
    assert_eq!(push.payload.title, "Designers");
    assert_eq!(push.payload.body, format!("Reminder: {text}"));
    assert_eq!(push.payload.path, format!("/rooms/{}/@{}", id("designers"), id("first")));
    assert_eq!(push.tag, format!("saved-{}", item.id));
    assert!(!push.subscriptions.is_empty());
    assert!(push.subscriptions.iter().all(|s| s.user_id == id("david")));
}

#[test]
fn the_reminder_body_truncates_at_140_characters() {
    let t = frozen();
    let long = super::channel_thread_test::post_root(&t, "designers", "jason", &"a".repeat(200));
    let item = t.write(move |tx| SavedItem::create(tx, NewSavedItem { user_id: id("david"), message_id: long.id, ..Default::default() }));
    let push = push(&t, &item, &|_| true).unwrap();
    assert_eq!(push.payload.body, format!("Reminder: {}...", "a".repeat(137)));
}

#[test]
fn no_push_goes_out_after_the_saver_loses_room_access() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    remove_membership(&t, "david_designers");
    assert!(push(&t, &item, &|_| true).is_none());
}

/// "no push goes out while the saver is in Do Not Disturb": DND is `Notifications::Policy`'s
/// (WS17), given here as the policy; this checks the pusher asks it about the saver.
#[test]
fn no_push_goes_out_when_the_policy_refuses_the_saver() {
    let t = frozen();
    let item = save(&t, "david", "first", Some(t.now().since(minutes(60))));
    let asked = std::cell::Cell::new(0);
    assert!(push(&t, &item, &|user| {
        asked.set(user.id);
        false
    })
    .is_none());
    assert_eq!(asked.get(), id("david"));
}

#[test]
fn a_thread_message_reminder_links_into_its_thread() {
    let t = frozen();
    let thread = create_thread(&t, "designers", "jason", None, Some("Deep dive"));
    let reply = post_reply(&t, thread.id, "jason", "Threaded thought");
    let item = t.write(move |tx| {
        SavedItem::create(tx, NewSavedItem { user_id: id("david"), message_id: reply.id, remind_at: Some(tx.now().since(minutes(60))), status: None })
    });
    let push = push(&t, &item, &|_| true).unwrap();
    assert_eq!(push.payload.path, format!("/rooms/{}?message_id={}&thread={}", id("designers"), reply.id, thread.id));
}

#[test]
fn stale_reminder_update_preserves_a_concurrent_status_change() {
    let t = frozen();
    let mut stale = save(&t,"david","first",None);
    let item_id = stale.id;
    t.write(move |tx| {
        let mut fresh = SavedItem::find(tx.conn(),item_id)?;
        fresh.update(tx,SavedItemChanges {status:Some("done".into()),..Default::default()})
    });
    let stale = t.write(move |tx| {
        let remind_at = tx.now().since(minutes(60));
        stale.update(tx,SavedItemChanges {remind_at:Some(Some(remind_at)),..Default::default()})?;
        Ok(stale)
    });
    let item = reload(&t,&stale);
    assert_eq!(item.status,"done");
    assert!(item.remind_at.is_some());
}

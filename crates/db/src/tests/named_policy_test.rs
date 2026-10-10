//! Exact named PolicyTest sequences observed while running the pin's original test bodies.
use super::*;
use crate::models::notification_policy::dnd_exceptions_for;
use crate::{
    DndAllowedUser, Involvement, NotificationKind, NotificationPolicy, ThreadInvolvement,
    Timestamp, UserStatusSettings,
};
use rusqlite::{
    params,
    trace::{TraceEvent, TraceEventCodes},
    types::Value as SqlValue,
};
use serde_json::{Value, json};
use std::cell::Cell;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/ws17_named_policy.json")).unwrap()
}
fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}
thread_local! { static QUERIES: Cell<usize> = const { Cell::new(0) }; }
fn trace(e: TraceEvent<'_>) {
    if let TraceEvent::Stmt(_, sql) = e
        && sql.starts_with("SELECT")
    {
        QUERIES.with(|n| n.set(n.get() + 1));
    }
}
fn replay(name: &str) {
    let golden = vectors();
    let scenario = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["test"] == name)
        .expect("exact pinned title");
    assert!(
        !scenario["calls"].as_array().unwrap().is_empty(),
        "non-policy helper gets a separate real SQL test"
    );
    for call in scenario["calls"].as_array().unwrap() {
        assert!(
            call.get("error").is_none(),
            "unknown kind remains deferred at the dynamic input boundary"
        );
        let now = stamp(call["now"].as_str().unwrap());
        let t = TestDb::with_clock(TestClock::frozen_at(now), 4);
        let recipient = call["recipient_id"].as_i64();
        let sender = call["sender_id"].as_i64();
        let setup = call.clone();
        t.write(move|tx|{
   if let Some(id)=recipient {
    for(key,value)in setup["attrs"].as_object().unwrap() {
     let value=match value {
      Value::Null=>SqlValue::Null,Value::Bool(b)=>SqlValue::Integer(i64::from(*b)),Value::Number(n)=>SqlValue::Integer(n.as_i64().unwrap()),
      Value::String(s) if key=="status"=>SqlValue::Integer(crate::Status::from_name(s).unwrap() as i64),
      Value::String(s) if key=="role"=>SqlValue::Integer(crate::Role::from_name(s).unwrap() as i64),
      Value::String(s) if key.ends_with("_at")||key.ends_with("_until")=>SqlValue::Text(stamp(s).to_db()),Value::String(s)=>SqlValue::Text(s.clone()),_=>panic!("unexpected {value}")
     };
     tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),params![value,id])?;
    }
    tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,?,?,?,?)",params![id,setup["busy"].to_string(),setup["ooo"].to_string(),now,now])?;
    for sender in setup["allowed_sender_ids"].as_array().unwrap(){DndAllowedUser::create(tx,id,sender.as_i64().unwrap())?;}
   }
   Ok(())
  });
        let user = recipient.map(|id| t.read(|conn| UserStatusSettings::find(conn, id)));
        let exception = match call["args"]["dnd_exception"].as_bool() {
            Some(preloaded) => preloaded,
            None => recipient.is_some_and(|id| {
                t.read(|conn| dnd_exceptions_for(conn, &[id], sender))
                    .contains(&id)
            }),
        };
        let policy = NotificationPolicy {
            room_id: None,
            recipient: user.as_ref(),
            kind: call["args"]["kind"]
                .as_str()
                .unwrap()
                .parse::<NotificationKind>()
                .unwrap(),
            room_involvement: call["room_present"]
                .as_bool()
                .unwrap()
                .then(|| call["room"].as_str().and_then(Involvement::from_name)),
            thread_involvement: call["thread"]
                .as_str()
                .and_then(ThreadInvolvement::from_name),
            mentioned: call["args"]["mentioned"].as_bool().unwrap_or(false),
            reply_to_recipient: call["args"]["reply_to_recipient"]
                .as_bool()
                .unwrap_or(false),
            keyword_matched: call["args"]["keyword_matched"].as_bool().unwrap_or(false),
            dnd_exception: exception,
            now,
        };
        // The preloaded pure policy can neither reload the user nor query an allowance/cache.
        t.read(|conn|{conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT,Some(trace));QUERIES.with(|n|n.set(0));let actual=json!({"inbox":policy.inbox_event_type(),"push":policy.push(),"sound":policy.sound()});assert_eq!(actual,call["expected"],"{name}: {call}");assert_eq!(QUERIES.with(Cell::get),0);Ok(())});
    }
}
#[test]
fn ws17_named_policy_dnd_exceptions_load_for_batch_in_one_query() {
    let t = TestDb::new();
    t.write(|tx| DndAllowedUser::create(tx, id("david"), id("jason")));
    t.read(|conn| {
        conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(trace));
        QUERIES.with(|n| n.set(0));
        assert_eq!(
            dnd_exceptions_for(conn, &[id("david"), id("kevin")], Some(id("jason")))?,
            std::collections::HashSet::from([id("david")])
        );
        assert_eq!(QUERIES.with(Cell::get), 1);
        QUERIES.with(|n| n.set(0));
        assert!(dnd_exceptions_for(conn, &[id("david")], None)?.is_empty());
        assert_eq!(QUERIES.with(Cell::get), 0);
        Ok(())
    });
}
macro_rules! named_policy {
    ($name:ident,$title:literal) => {
        #[test]
        fn $name() {
            replay($title);
        }
    };
}
named_policy!(
    ws17_named_policy_dnd_silences_a_muted_room_mention_push_but_keeps_the_inbox_item,
    "DND silences a muted room mention push but keeps the inbox item"
);
named_policy!(
    ws17_named_policy_a_followed_thread_records_activity_and_pushes,
    "a followed thread records activity and pushes"
);
named_policy!(
    ws17_named_policy_a_missing_recipient_pushes_nothing,
    "a missing recipient pushes nothing"
);
named_policy!(
    ws17_named_policy_a_muted_board_post_stays_silent_for_members_outside_the_thread,
    "a muted board post stays silent for members outside the thread"
);
named_policy!(
    ws17_named_policy_a_muted_room_keyword_match_records_without_pushing,
    "a muted room keyword match records without pushing"
);
named_policy!(
    ws17_named_policy_a_muted_room_mention_records_and_pushes,
    "a muted room mention records and pushes"
);
named_policy!(
    ws17_named_policy_a_muted_room_reply_stays_silent,
    "a muted room reply stays silent"
);
named_policy!(
    ws17_named_policy_a_muted_thread_gets_nothing_not_even_mentions_or_keywords,
    "a muted thread gets nothing, not even mentions or keywords"
);
named_policy!(
    ws17_named_policy_a_non_member_of_the_thread_gets_nothing,
    "a non-member of the thread gets nothing"
);
named_policy!(
    ws17_named_policy_a_plain_room_message_does_nothing_for_mentions_members,
    "a plain room message does nothing for mentions members"
);
named_policy!(
    ws17_named_policy_a_plain_room_message_pushes_everything_followers_without_an_inbox_item,
    "a plain room message pushes everything followers without an inbox item"
);
named_policy!(
    ws17_named_policy_a_preloaded_dnd_exception_decides_without_another_lookup,
    "a preloaded DND exception decides without another lookup"
);
named_policy!(ws17_named_policy_a_room_keyword_match_records_without_pushing_for_mentions_and_notifications_off_members,"a room keyword match records without pushing for mentions and notifications-off members");
named_policy!(
    ws17_named_policy_a_room_mention_records_and_pushes_for_a_mentions_member,
    "a room mention records and pushes for a mentions member"
);
named_policy!(
    ws17_named_policy_a_room_mention_still_records_with_notifications_off_but_sends_no_push,
    "a room mention still records with notifications off but sends no push"
);
named_policy!(ws17_named_policy_a_room_message_without_a_membership_records_nothing_not_even_mentions_or_keywords,"a room message without a membership records nothing, not even mentions or keywords");
named_policy!(
    ws17_named_policy_a_room_reply_records_and_pushes_for_mentions_and_everything_members,
    "a room reply records and pushes for mentions and everything members"
);
named_policy!(
    ws17_named_policy_a_room_reply_stays_silent_with_notifications_off,
    "a room reply stays silent with notifications off"
);
named_policy!(
    ws17_named_policy_a_starred_sender_still_pushes_through_dnd,
    "a starred sender still pushes through DND"
);
named_policy!(
    ws17_named_policy_a_starred_sender_still_pushes_through_out_of_office,
    "a starred sender still pushes through out of office"
);
named_policy!(
    ws17_named_policy_a_starred_sender_still_pushes_through_quiet_during_meetings,
    "a starred sender still pushes through quiet-during-meetings"
);
named_policy!(
    ws17_named_policy_a_starred_sender_still_pushes_through_quiet_hours,
    "a starred sender still pushes through quiet hours"
);
named_policy!(
    ws17_named_policy_a_thread_keyword_match_records_for_unfollowed_members_without_pushing,
    "a thread keyword match records for unfollowed members without pushing"
);
named_policy!(
    ws17_named_policy_a_thread_mention_records_and_pushes_for_mentions_and_everything_members,
    "a thread mention records and pushes for mentions and everything members"
);
named_policy!(
    ws17_named_policy_a_thread_reply_records_and_pushes_for_followers_only,
    "a thread reply records and pushes for followers only"
);
named_policy!(
    ws17_named_policy_an_everything_member_s_keyword_match_still_pushes_as_a_broadcast,
    "an everything member's keyword match still pushes as a broadcast"
);
named_policy!(
    ws17_named_policy_an_expired_out_of_office_pushes_again,
    "an expired out of office pushes again"
);
named_policy!(
    ws17_named_policy_an_invisible_room_membership_gets_nothing_at_all,
    "an invisible room membership gets nothing at all"
);
named_policy!(
    ws17_named_policy_an_unfollowed_thread_stays_silent_for_plain_messages,
    "an unfollowed thread stays silent for plain messages"
);
named_policy!(
    ws17_named_policy_bots_and_deactivated_recipients_record_nothing,
    "bots and deactivated recipients record nothing"
);
named_policy!(
    ws17_named_policy_calendar_out_of_office_quiets_like_a_manual_one,
    "calendar out of office quiets like a manual one"
);
named_policy!(
    ws17_named_policy_huddle_invitations_push_unless_dnd_is_on_without_a_starred_caller,
    "huddle invitations push unless DND is on without a starred caller"
);
named_policy!(
    ws17_named_policy_huddle_join_notices_honor_dnd_with_a_starred_caller_exception,
    "huddle join notices honor DND with a starred-caller exception"
);
named_policy!(
    ws17_named_policy_huddle_join_notices_push_for_live_memberships_and_record_no_inbox_item,
    "huddle join notices push for live memberships and record no inbox item"
);
named_policy!(
    ws17_named_policy_huddle_join_notices_stay_silent_during_meetings_and_out_of_office,
    "huddle join notices stay silent during meetings and out of office"
);
named_policy!(
    ws17_named_policy_huddle_join_notices_stay_silent_when_muted_off_hidden_or_no_membership,
    "huddle join notices stay silent when muted, off, hidden, or no membership"
);
named_policy!(
    ws17_named_policy_manual_dnd_suppresses_push_and_sound_but_still_records_the_inbox_item,
    "manual DND suppresses push and sound but still records the inbox item"
);
named_policy!(
    ws17_named_policy_mention_beats_reply_beats_keyword_for_one_room_message,
    "mention beats reply beats keyword for one room message"
);
named_policy!(
    ws17_named_policy_muted_room_thread_activity_stays_silent_for_followers,
    "muted room thread activity stays silent for followers"
);
named_policy!(
    ws17_named_policy_out_of_office_pushes_when_the_member_keeps_notifications_on,
    "out of office pushes when the member keeps notifications on"
);
named_policy!(
    ws17_named_policy_out_of_office_suppresses_push_and_sound_but_still_records_the_inbox_item,
    "out of office suppresses push and sound but still records the inbox item"
);
named_policy!(
    ws17_named_policy_quiet_during_meetings_needs_meeting_status_on,
    "quiet-during-meetings needs meeting status on"
);
named_policy!(
    ws17_named_policy_quiet_during_meetings_pushes_outside_busy_intervals,
    "quiet-during-meetings pushes outside busy intervals"
);
named_policy!(ws17_named_policy_quiet_during_meetings_suppresses_push_and_sound_but_still_records_the_inbox_item,"quiet-during-meetings suppresses push and sound but still records the inbox item");
named_policy!(
    ws17_named_policy_quiet_hours_follow_the_recipient_s_time_zone,
    "quiet hours follow the recipient's time zone"
);
named_policy!(
    ws17_named_policy_quiet_hours_suppress_push_inside_the_window_only,
    "quiet hours suppress push inside the window only"
);
named_policy!(
    ws17_named_policy_reminders_and_huddles_stay_silent_during_meetings_with_no_sender_exception,
    "reminders and huddles stay silent during meetings with no sender exception"
);
named_policy!(
    ws17_named_policy_reminders_and_huddles_stay_silent_during_out_of_office,
    "reminders and huddles stay silent during out of office"
);
named_policy!(
    ws17_named_policy_reminders_push_unless_dnd_is_on_and_carry_no_sender_exception,
    "reminders push unless DND is on, and carry no sender exception"
);
named_policy!(
    ws17_named_policy_room_notifications_off_suppresses_thread_activity_but_not_keywords,
    "room notifications off suppresses thread activity but not keywords"
);

#[test]
fn ws17_named_policy_an_unknown_kind_raises() {
    let v = vectors();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["test"] == "an unknown kind raises")
        .unwrap();
    let call = &row["calls"][0];
    let error = call["args"]["kind"]
        .as_str()
        .unwrap()
        .parse::<NotificationKind>()
        .unwrap_err();
    assert_eq!(error.to_string(), call["message"].as_str().unwrap());
}

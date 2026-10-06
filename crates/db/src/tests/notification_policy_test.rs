use super::*;
use crate::models::workspace_presence_lease::Presence;
use crate::{
    Involvement, NotificationKind, NotificationPolicy, ThreadInvolvement, Timestamp,
    UserStatusSettings,
};
use serde_json::{Value, json};

fn vectors() -> Value {
    serde_json::from_str(include_str!("ws17_vectors.json")).unwrap()
}
fn instant(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}

#[test]
fn ws17_policy_matches_rails_combinations() {
    let t = TestDb::new();
    let mut user = t.read(|conn| {
        Ok(UserStatusSettings::for_ids(conn, &[id("david")])?
            .remove(&id("david"))
            .unwrap())
    });
    let vectors = vectors();
    for row in vectors["policies"].as_array().unwrap() {
        user.dnd_enabled = row["quiet"].as_bool().unwrap();
        let policy = NotificationPolicy {
            recipient: Some(&user),
            kind: match row["kind"].as_str().unwrap() {
                "room_message" => NotificationKind::RoomMessage,
                "thread_message" => NotificationKind::ThreadMessage,
                "reminder" => NotificationKind::Reminder,
                "huddle" => NotificationKind::Huddle,
                "huddle_join" => NotificationKind::HuddleJoin,
                other => panic!("unexpected kind {other}"),
            },
            room_involvement: if row["room"] == "absent" {
                None
            } else {
                Some(row["room"].as_str().and_then(Involvement::from_name))
            },
            thread_involvement: row["thread"]
                .as_str()
                .and_then(ThreadInvolvement::from_name),
            mentioned: row["mentioned"].as_bool().unwrap(),
            reply_to_recipient: row["reply"].as_bool().unwrap(),
            keyword_matched: row["keyword"].as_bool().unwrap(),
            dnd_exception: false,
            now: instant(vectors["now"].as_str().unwrap()),
        };
        assert_eq!(
            json!({"push": policy.push(), "sound": policy.sound(), "inbox": policy.inbox_event_type()}),
            json!({"push": row["push"], "sound": row["sound"], "inbox": row["inbox"]}),
            "{row}"
        );
    }
    assert_eq!(vectors["policies"].as_array().unwrap().len(), 2240);
}

#[test]
fn ws17_status_readers_match_rails_times_zones_and_dst() {
    let t = TestDb::new();
    let vectors = vectors();
    for row in vectors["statuses"].as_array().unwrap() {
        let attrs = row["attrs"].as_object().unwrap().clone();
        let busy = row["busy"].to_string();
        let ooo = row["ooo"].to_string();
        t.write(move |tx| {
            tx.conn().execute("UPDATE users SET presence_setting='auto', custom_status_emoji=NULL, custom_status_text=NULL, custom_status_expires_at=NULL, dnd_enabled=0, dnd_until=NULL, quiet_hours_enabled=0, quiet_hours_start_minute=NULL, quiet_hours_end_minute=NULL, time_zone=NULL, meeting_status_enabled=0, meeting_dnd_enabled=0, ooo_until=NULL, ooo_note=NULL, ooo_calendar_enabled=0, ooo_notify_enabled=0 WHERE id=?", [id("david")])?;
            for (key, value) in attrs {
                let value = match value {
                    Value::Bool(b) => rusqlite::types::Value::Integer(i64::from(b)),
                    Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                    Value::String(s) if key.ends_with("_until") || key.ends_with("_expires_at") => rusqlite::types::Value::Text(instant(&s).to_db()),
                    Value::String(s) => rusqlite::types::Value::Text(s),
                    _ => panic!("unexpected vector attribute"),
                };
                tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"), rusqlite::params![value, id("david")])?;
            }
            tx.conn().execute("INSERT INTO calendar_meeting_caches (user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET busy_intervals=excluded.busy_intervals,ooo_intervals=excluded.ooo_intervals", rusqlite::params![id("david"),busy,ooo,tx.now(),tx.now()])?;
            Ok(())
        });
        let user = t.read(|conn| {
            Ok(UserStatusSettings::for_ids(conn, &[id("david")])?
                .remove(&id("david"))
                .unwrap())
        });
        let now = instant(row["now"].as_str().unwrap());
        let policy = NotificationPolicy {
            recipient: Some(&user),
            kind: NotificationKind::Reminder,
            room_involvement: None,
            thread_involvement: None,
            mentioned: false,
            reply_to_recipient: false,
            keyword_matched: false,
            dnd_exception: false,
            now,
        };
        assert_eq!(
            json!({"dnd":user.dnd_active(now), "meeting":user.in_meeting(now), "out_of_office":user.out_of_office(now), "custom":user.custom_status_display(now), "status":user.status_text_display(now), "push":policy.push(),
            "presence":([Presence::Online,Presence::Idle,Presence::Offline].map(|p|user.effective_presence(p)))}),
            json!({"dnd":row["dnd"], "meeting":row["meeting"], "out_of_office":row["out_of_office"], "custom":row["custom"], "status":row["status"], "push":row["push"], "presence":row["presence"]}),
            "{row}"
        );
    }
    assert_eq!(vectors["statuses"].as_array().unwrap().len(), 161);
}

#[test]
fn ws17_allowed_sender_bypasses_dnd_but_stars_alone_do_not() {
    use crate::models::notification_policy::dnd_exceptions_for;
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("jason")])?;
        tx.conn().execute("INSERT INTO user_stars (user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![id("jason"),id("david"),tx.now(),tx.now()])?;
        Ok(())
    });
    assert!(
        t.read(|c| dnd_exceptions_for(c, &[id("jason")], Some(id("david"))))
            .is_empty()
    );
    t.write(|tx| {
        tx.conn().execute("INSERT INTO dnd_allowed_users (user_id,allowed_user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![id("jason"),id("david"),tx.now(),tx.now()])?;
        Ok(())
    });
    assert!(
        t.read(|c| dnd_exceptions_for(c, &[id("jason")], None))
            .is_empty()
    );
    assert!(
        t.read(|c| dnd_exceptions_for(c, &[id("jason")], Some(id("david"))))
            .contains(&id("jason"))
    );
    let user = t.read(|c| {
        Ok(UserStatusSettings::for_ids(c, &[id("jason")])?
            .remove(&id("jason"))
            .unwrap())
    });
    let policy = |u| NotificationPolicy {
        recipient: u,
        kind: NotificationKind::Huddle,
        room_involvement: None,
        thread_involvement: None,
        mentioned: false,
        reply_to_recipient: false,
        keyword_matched: false,
        dnd_exception: true,
        now: t.now(),
    };
    assert!(policy(Some(&user)).push());
    assert!(!policy(None).push());
    let mut bot = user.clone();
    bot.user.role = crate::Role::Bot;
    let mut p = policy(Some(&bot));
    p.kind = NotificationKind::RoomMessage;
    p.mentioned = true;
    p.room_involvement = Some(Some(Involvement::Everything));
    assert!(p.push()); // Rails gates inbox activity, not push, by active human.
    assert_eq!(p.inbox_event_type(), None);
}

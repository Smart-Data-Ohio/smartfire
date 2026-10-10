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
fn a9_room_choices_advance_revision_even_when_only_inheritance_changes() {
    use crate::models::notification_policy::NotificationPreferences;
    use crate::models::user::profile_settings::{self, Changes};
    let t = TestDb::new();
    let room = t.read(|conn| crate::Membership::find(conn, id("david_watercooler"))).room_id;
    t.write(move |tx| profile_settings::update(tx, id("david"), Changes {
        inbox_preferences: Some(json!({"default_notification_level":"nothing", "room_notification_levels":{room.to_string():null}})),
        ..Default::default()
    }));
    let mut previous = t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision;
    for level in [Involvement::Everything, Involvement::Mentions, Involvement::Nothing, Involvement::Muted, Involvement::Invisible] {
        t.write(move |tx| crate::Membership::find(tx.conn(), id("david_watercooler"))?.update_involvement(tx, level));
        let preferences = t.read(|conn| NotificationPreferences::load(conn, id("david")));
        assert!(preferences.settings_revision > previous, "{level:?} did not advance settings revision");
        assert!(!preferences.room_notification_levels.contains_key(&room));
        previous = preferences.settings_revision;
    }
    t.write(|tx| crate::Membership::find(tx.conn(), id("david_watercooler"))?.update_involvement(tx, Involvement::Invisible));
    assert_eq!(t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision, previous);
}

#[test]
fn a9_notification_writers_advance_revision_outside_the_settings_api() {
    use crate::models::notification_policy::NotificationPreferences;
    let t = TestDb::new();
    let revision = |t: &TestDb| t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision;
    let mut previous = revision(&t);
    t.write(|tx| {
        let mut status = UserStatusSettings::find(tx.conn(), id("david"))?;
        status.dnd_enabled = !status.dnd_enabled;
        status.save(tx)
    });
    assert!(revision(&t) > previous, "status save did not advance revision");
    previous = revision(&t);
    t.write(|tx| crate::slash_commands::user_settings::update(tx, id("david"), json!({"presence_setting":"dnd"})));
    assert!(revision(&t) > previous, "slash setting did not advance revision");
    previous = revision(&t);
    t.write(|tx| {
        crate::models::user_status_settings::replace_keyword_alerts(tx, id("david"), &["revision witness".into()])
    });
    assert!(revision(&t) > previous, "keywords did not advance revision");
    previous = revision(&t);
    t.write(|tx| {
        crate::DndAllowedUser::remove(tx, id("david"), id("kevin"))?;
        crate::DndAllowedUser::create(tx, id("david"), id("kevin"))?;
        Ok(())
    });
    assert!(revision(&t) > previous, "DND allowance did not advance revision");
    previous = revision(&t);
    t.write(|tx| {
        crate::DndAllowedUser::remove(tx, id("david"), id("kevin"))?;
        Ok(())
    });
    assert!(revision(&t) > previous, "DND allowance removal did not advance revision");
}

#[test]
fn a9_direct_keyword_writers_advance_revision() {
    use crate::models::notification_policy::NotificationPreferences;
    let t = TestDb::new();
    let revision = |t: &TestDb| t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision;
    let mut previous = revision(&t);
    let keyword = t.write(|tx| crate::KeywordAlert::create(tx, id("david"), "revision witness"));
    assert!(revision(&t) > previous, "keyword creation did not advance revision");
    previous = revision(&t);
    let keyword = t.write(move |tx| {
        let mut keyword = keyword;
        keyword.update(tx, "revision changed")?;
        Ok(keyword)
    });
    assert!(revision(&t) > previous, "keyword update did not advance revision");
    previous = revision(&t);
    t.write(move |tx| keyword.destroy(tx));
    assert!(revision(&t) > previous, "keyword removal did not advance revision");
}

#[test]
fn a9_calendar_notification_state_advances_revision() {
    use crate::models::notification_policy::NotificationPreferences;
    let t = TestDb::new();
    let before = t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision;
    t.write(|tx| crate::models::google_meeting_cache::complete(tx, id("david"), Some(json!([])), Some(json!([])), Some("revision witness".into()), tx.now()));
    let after = t.read(|conn| NotificationPreferences::load(conn, id("david"))).settings_revision;
    assert!(after > before, "calendar notification state did not advance revision");
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
            room_id: None,
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
            room_id: None,
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
        room_id: None,
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

#[test]
fn a9_notification_preferences_extend_and_preserve_legacy_keys() {
    use crate::models::user::profile_settings::{self, Changes};
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![json!({"agent_work":"0","custom_legacy":true}).to_string(), id("david")])?;
        profile_settings::update(tx, id("david"), Changes {
            inbox_preferences: Some(json!({"default_notification_level":"mentions", "room_notification_levels":{"1":null}, "room_mute_until":{"1":"2030-01-01T00:00:00Z"}})),
            ..Default::default()
        })
    });
    let raw: String = t.read(|c| Ok(c.query_row("SELECT inbox_preferences FROM users WHERE id=?", [id("david")], |r| r.get(0))?));
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value, json!({"settings_revision":1,"agent_work":"0","custom_legacy":true,"default_notification_level":"mentions","room_notification_levels":{"1":null},"room_mute_until":{"1":"2030-01-01T00:00:00Z"}}));
}

#[test]
fn a9_preference_writes_advance_settings_and_activity_revisions() {
    use crate::models::user::profile_settings::{self, Changes};
    let t = TestDb::new();
    let before = t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()));
    for revision in 1..=2 {
        t.write(move |tx| profile_settings::update(tx, id("david"), Changes {
            inbox_preferences: Some(json!({"default_notification_level":"nothing", "settings_revision":0})),
            ..Default::default()
        }));
        let value: Value = t.read(|c| {
            let raw: String = c.query_row("SELECT inbox_preferences FROM users WHERE id=?", [id("david")], |r| r.get(0))?;
            Ok(serde_json::from_str(&raw).unwrap())
        });
        assert_eq!(value["settings_revision"], revision);
        let after = t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()));
        assert!(after.revision > before.revision);
    }
}

#[test]
fn a9_clearing_preferences_keeps_the_settings_revision_monotonic() {
    use crate::models::user::profile_settings::{self, Changes};
    let t = TestDb::new();
    for preferences in [json!({"default_notification_level":"nothing"}), Value::Null] {
        t.write(move |tx| profile_settings::update(tx, id("david"), Changes {
            inbox_preferences: Some(preferences),
            ..Default::default()
        }));
    }
    let revision: i64 = t.read(|c| Ok(c.query_row(
        "SELECT json_extract(inbox_preferences, '$.settings_revision') FROM users WHERE id=?",
        [id("david")], |r| r.get(0)
    )?));
    assert_eq!(revision, 2);
}

#[test]
fn a9_muted_rooms_do_not_contribute_to_push_badges() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute("UPDATE memberships SET unread_at=? WHERE user_id=?", rusqlite::params![tx.now(), id("david")])?;
        let rooms: Vec<i64> = crate::sql::query_all(tx.conn(), "SELECT room_id FROM memberships WHERE user_id=?", [id("david")], |r| r.get(0))?;
        let mutes: serde_json::Map<String,Value> = rooms.into_iter().map(|id| (id.to_string(), Value::Null)).collect();
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![json!({"room_mute_until":mutes}).to_string(), id("david")])?;
        Ok(())
    });
    assert_eq!(t.read(|c| crate::Membership::unread_count(c, id("david"), t.now())), 0);
}

#[test]
fn a9_inheritance_override_and_mute_expiry_resolve_at_delivery() {
    use crate::models::notification_policy::{NotificationLevel, NotificationPreferences};
    let t = TestDb::new();
    let mut user = t.read(|c| UserStatusSettings::find(c, id("david")));
    let now = instant("2026-10-10T12:00:00Z");
    let end = now.since(jiff::SignedDuration::from_secs(900));
    let mut preferences = NotificationPreferences::parse(Some(r#"{"agent_work":"0","default_notification_level":"mentions","room_notification_levels":{"1":null,"2":"everything"},"room_mute_until":{"1":"2026-10-10T12:15:00Z","3":null}}"#));
    assert_eq!(preferences.involvement(1, Some(Involvement::Everything)), Some(Involvement::Mentions));
    assert_eq!(preferences.involvement(2, Some(Involvement::Nothing)), Some(Involvement::Everything));
    preferences.default_notification_level = NotificationLevel::Everything;
    user.notification_preferences = preferences;
    let mut policy = NotificationPolicy {
        room_id: Some(1), recipient: Some(&user), kind: NotificationKind::RoomMessage,
        room_involvement: Some(Some(Involvement::Mentions)), thread_involvement: None,
        mentioned: true, reply_to_recipient: true, keyword_matched: true, dnd_exception: true, now,
    };
    assert!(!policy.push());
    assert!(!policy.sound());
    assert_eq!(policy.inbox_event_type(), None);
    policy.now = end;
    assert!(policy.push());
    assert_eq!(policy.inbox_event_type(), Some("mention"));
    policy.mentioned = false;
    policy.reply_to_recipient = false;
    assert!(policy.push(), "inherited all-messages default takes effect after expiry");
    policy.room_id = Some(3);
    assert!(!policy.push(), "indefinite mute has no deadline");
    assert_eq!(serde_json::to_value(&user.notification_preferences).unwrap()["agent_work"], "0");
}


#[test]
fn a9_existing_activity_badges_follow_mute_at_read_time() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        let message = crate::Message::find(tx.conn(), id("first"))?;
        crate::ActivityItem::refresh_unread(tx, id("david"), "Message", message.id, "mention")?;
        Ok(message.room_id)
    });
    let before = t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()));
    assert!(before.count > 0);
    for (until, muted) in [(json!(null), true), (json!("2030-01-01T00:00:00Z"), true), (json!("2020-01-01T00:00:00Z"), false)] {
        t.write(move |tx| {
            tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![json!({"room_mute_until": {room.to_string(): until}}).to_string(), id("david")])?;
            Ok(())
        });
        let after = t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()));
        assert_eq!(after.count, if muted {before.count - 1} else {before.count});
        assert_eq!(after.revision, before.revision);
    }
}

#[test]
fn a9_activity_badges_expire_at_the_injected_clock() {
    let t = TestDb::new();
    t.clock.travel_to(instant("2035-01-01T12:00:00Z"));
    let room = t.write(|tx| {
        tx.conn()
            .execute("DELETE FROM activity_items WHERE user_id=?", [id("david")])?;
        let message = crate::Message::find(tx.conn(), id("first"))?;
        crate::ActivityItem::refresh_unread(tx, id("david"), "Message", message.id, "mention")?;
        let until = tx.now().since(jiff::SignedDuration::from_secs(900)).jiff();
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            rusqlite::params![
                json!({"room_mute_until": {message.room_id.to_string(): until}}).to_string(),
                id("david")
            ],
        )?;
        Ok(message.room_id)
    });
    assert!(
        t.read(|c| UserStatusSettings::find(c, id("david")))
            .notification_preferences
            .muted(room, t.now())
    );
    assert_eq!(
        t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()))
            .count,
        0
    );
    t.travel(900);
    assert!(
        !t.read(|c| UserStatusSettings::find(c, id("david")))
            .notification_preferences
            .muted(room, t.now())
    );
    assert_eq!(
        t.read(|c| crate::ActivityItem::unread_snapshot(c, id("david"), t.now()))
            .count,
        1
    );
    assert_eq!(
        t.read(|c| crate::ActivityItem::unread_count(
            c,
            &crate::User::find(c, id("david"))?,
            t.now()
        )),
        1
    );
}

#[test]
fn a9_push_badges_expire_at_the_injected_clock() {
    let t = TestDb::new();
    t.clock.travel_to(instant("2035-01-01T12:00:00Z"));
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL WHERE user_id=?",
            [id("david")],
        )?;
        let room = crate::Message::find(tx.conn(), id("first"))?.room_id;
        tx.conn().execute(
            "UPDATE memberships SET unread_at=? WHERE user_id=? AND room_id=?",
            rusqlite::params![tx.now(), id("david"), room],
        )?;
        let until = tx.now().since(jiff::SignedDuration::from_secs(900)).jiff();
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            rusqlite::params![
                json!({"room_mute_until": {room.to_string(): until}}).to_string(),
                id("david")
            ],
        )?;
        Ok(())
    });
    assert_eq!(
        t.read(|c| crate::Membership::unread_count(c, id("david"), t.now())),
        0
    );
    t.travel(900);
    assert_eq!(
        t.read(|c| crate::Membership::unread_count(c, id("david"), t.now())),
        1
    );
}

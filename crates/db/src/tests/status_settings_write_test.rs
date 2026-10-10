use super::*;
use crate::models::user_status_settings::{clock_time_to_minutes, minutes_to_clock_time};
use crate::{DndAllowedUser, Error, KeywordAlert, Timestamp, UserStatusSettings as Settings};
use jiff::SignedDuration;
use serde_json::{Value, json};

fn settings(t: &TestDb) -> Settings {
    t.read(|c| Settings::find(c, id("david")))
}
fn vectors() -> Value {
    serde_json::from_str(include_str!("ws17_settings_vectors.json")).unwrap()
}
fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}

#[test]
fn ws17_status_broadcast_emission_order_matches_actual_rails_calls() {
    use crate::Broadcast;
    use crate::models::user_status_settings::updates::{StatusBadgeBroadcast};
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws17_status_requests.json"
    ))
    .unwrap();
    for name in ["manual_on", "both_off", "calendar_off_and_note"] {
        let row = golden["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        let t = TestDb::new();
        let now = stamp(golden["now"].as_str().unwrap());
        t.clock.travel_to(now);
        if name != "manual_on" {
            t.write(move |tx|{
            tx.conn().execute("UPDATE users SET meeting_status_enabled=?,ooo_calendar_enabled=1,ooo_until=?,ooo_note=? WHERE id=?",rusqlite::params![name=="both_off",(name=="calendar_off_and_note").then(||now.since(SignedDuration::from_hours(24))),(name=="calendar_off_and_note").then_some("Back soon"),id("david")])?;
            tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,'[]','[[\"2026-03-02T15:55:00Z\",\"2026-03-04T16:00:00Z\"]]',?,?)",rusqlite::params![id("david"),now,now])?;
            Ok(())
        });
        }
        let mut user = settings(&t);
        if name == "manual_on" {
            user.ooo_until = user.ooo_preset_until("tomorrow", None, now).unwrap();
            user.ooo_note = Some("Back <soon> & safe".into());
        } else {
            user.meeting_status_enabled = false;
            user.ooo_calendar_enabled = false;
            if name == "calendar_off_and_note" {
                user.ooo_note = Some("Changed".into());
            }
        }
        let before = t.events().len();
        t.write(move |tx| user.save_status(tx));
        let streams = t
            .events()
            .into_iter()
            .skip(before)
            .map(|event| match event {
                Event::Broadcast(b) if b.kind == StatusBadgeBroadcast::KIND => "status",
                other => panic!("unexpected {other:?}"),
            })
            .collect::<Vec<_>>();
        let expected = row["frames"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| { let stream = f["stream"].as_str().unwrap().rsplit(':').next().unwrap(); (stream == "status").then_some(stream) })
            .collect::<Vec<_>>();
        assert_eq!(streams, expected, "{name}");
    }
}

#[test]
fn ws17_manual_ooo_claims_match_actual_rails_conditional_updates() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws17_status_requests.json"
    ))
    .unwrap();
    let t = TestDb::new();
    let now = stamp(golden["now"].as_str().unwrap());
    t.clock.travel_to(now);
    for row in golden["claims"].as_array().unwrap() {
        let row = row.clone();
        t.write(move |tx| {
            let stored = row["stored"].as_bool();
            let until = row["until_time"].as_str().map(stamp);
            tx.conn().execute(
                "UPDATE users SET ooo_broadcast=?,ooo_until=?,ooo_note='Keep or clear' WHERE id=?",
                rusqlite::params![stored, until, id("david")],
            )?;
            let user = Settings::find(tx.conn(), id("david"))?;
            let won = user.claim_ooo_broadcast(tx, row["active"].as_bool().unwrap(), now)?;
            let after = Settings::find(tx.conn(), id("david"))?;
            assert_eq!(won, row["won"].as_bool().unwrap(), "{row}");
            assert_eq!(
                after.ooo_broadcast,
                row["after"]["broadcast"].as_bool(),
                "{row}"
            );
            assert_eq!(
                after.ooo_until,
                row["after"]["until_time"].as_str().map(stamp),
                "{row}"
            );
            assert_eq!(
                after.ooo_note.as_deref(),
                row["after"]["note"].as_str(),
                "{row}"
            );
            // Re-running the same claim cannot win; failed false claims preserve future ends.
            assert!(!user.claim_ooo_broadcast(tx, row["active"].as_bool().unwrap(), now)?);
            Ok(())
        });
    }
}

#[test]
fn ws17_stale_ooo_end_claim_does_not_clear_a_new_manual_end() {
    let t = TestDb::new();
    let now = t.now();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET ooo_broadcast=1,ooo_until=? WHERE id=?",
            rusqlite::params![now.ago(SignedDuration::from_mins(1)), id("david")],
        )?;
        let stale = Settings::find(tx.conn(), id("david"))?;
        let future = now.since(SignedDuration::from_hours(24));
        tx.conn().execute(
            "UPDATE users SET ooo_until=?,ooo_note='New end' WHERE id=?",
            rusqlite::params![future, id("david")],
        )?;
        assert!(!stale.claim_ooo_broadcast(tx, false, now)?);
        let fresh = Settings::find(tx.conn(), id("david"))?;
        assert_eq!(fresh.ooo_until, Some(future));
        assert_eq!(fresh.ooo_note.as_deref(), Some("New end"));
        assert_eq!(fresh.ooo_broadcast, Some(true));
        Ok(())
    });
}

#[test]
fn defaults_to_automatic_presence_no_dnd_and_system_theme() {
    let t = TestDb::new();
    let user = settings(&t);
    assert_eq!(
        (&*user.presence_setting, &*user.theme, &*user.text_size),
        ("auto", "system", "default")
    );
    assert!(!user.dnd_enabled && !user.quiet_hours_enabled);
    assert_eq!(user.time_zone, None);
}

#[test]
fn the_dnd_presence_silences_like_the_dnd_switch_and_only_allows_the_allowed_sender() {
    let t = TestDb::new();
    let mut user = settings(&t);
    user.presence_setting = "dnd".into();
    user = t.write(move |tx| {
        user.save(tx)?;
        Ok(user)
    });
    let now = t.now();
    assert!(t.read(move |c| user.notifications_muted(c, Some(id("jason")), now)));
    t.write(|tx| DndAllowedUser::create(tx, id("david"), id("jason")));
    let user = settings(&t);
    t.read(move |c| {
        assert!(!user.notifications_muted(c, Some(id("jason")), now)?);
        assert!(user.notifications_muted(c, Some(id("kevin")), now)?);
        assert!(user.notifications_muted(c, None, now)?);
        Ok(())
    });
}

#[test]
fn deactivating_clears_the_manual_ooo_columns() {
    let t = TestDb::new();
    let mut user = settings(&t);
    user.ooo_until = Some(t.now().since(SignedDuration::from_hours(24)));
    user.ooo_note = Some("Vacation".into());
    t.write(move |tx| {
        user.save(tx)?;
        tx.conn()
            .execute("UPDATE users SET ooo_broadcast=1 WHERE id=?", [id("david")])?;
        user.user.deactivate(tx)
    });
    let user = settings(&t);
    assert_eq!(user.ooo_until, None);
    assert_eq!(user.ooo_note, None);
    assert_eq!(user.ooo_broadcast, None);
}

#[test]
fn ws17_custom_status_and_ooo_presets_match_rails_zones_and_dst() {
    let t = TestDb::new();
    for row in vectors()["rows"].as_array().unwrap() {
        let mut user = settings(&t);
        user.time_zone = Some(row["zone"].as_str().unwrap().into());
        let now = stamp(row["now"].as_str().unwrap());
        let actual = if row["kind"] == "custom" {
            user.set_custom_status_expires_in(row["preset"].as_str().unwrap(), now)
                .unwrap();
            user.custom_status_expires_at
        } else {
            user.ooo_preset_until(row["preset"].as_str().unwrap(), row["custom"].as_str(), now)
                .unwrap()
        };
        assert_eq!(actual, row["value"].as_str().map(stamp), "{row}");
    }
}

#[test]
fn ws17_quiet_hours_virtual_setters_match_rails() {
    for row in vectors()["clocks"].as_array().unwrap() {
        let actual = clock_time_to_minutes(row["input"].as_str().unwrap_or(""));
        assert_eq!(actual, row["minute"].as_i64(), "{row}");
        assert_eq!(
            minutes_to_clock_time(actual),
            row["display"].as_str().map(str::to_owned),
            "{row}"
        );
    }
}

#[test]
fn ws17_status_settings_validations_match_rails_and_leave_rows_unchanged() {
    let t = TestDb::new();
    let golden = vectors();
    t.clock.travel_to(stamp(golden["now"].as_str().unwrap()));
    for row in golden["validations"].as_array().unwrap() {
        let mut user = settings(&t);
        for (key, value) in row["attrs"].as_object().unwrap() {
            match key.as_str() {
                "presence_setting" => user.presence_setting = value.as_str().unwrap().into(),
                "theme" => user.theme = value.as_str().unwrap().into(),
                "text_size" => user.text_size = value.as_str().unwrap().into(),
                "time_zone" => user.time_zone = Some(value.as_str().unwrap().into()),
                "custom_status_emoji" => {
                    user.custom_status_emoji = Some(value.as_str().unwrap().into())
                }
                "custom_status_text" => {
                    user.custom_status_text = Some(value.as_str().unwrap().into())
                }
                "ooo_note" => user.ooo_note = Some(value.as_str().unwrap().into()),
                "ooo_until" => user.ooo_until = Some(stamp(value.as_str().unwrap())),
                "quiet_hours_enabled" => user.quiet_hours_enabled = value.as_bool().unwrap(),
                "quiet_hours_start_minute" => user.quiet_hours_start_minute = value.as_i64(),
                "quiet_hours_end_minute" => user.quiet_hours_end_minute = value.as_i64(),
                "quiet_hours_start" => {
                    user.quiet_hours_start_minute = clock_time_to_minutes(value.as_str().unwrap())
                }
                "quiet_hours_end" => {
                    user.quiet_hours_end_minute = clock_time_to_minutes(value.as_str().unwrap())
                }
                other => panic!("unexpected oracle attr {other}"),
            }
        }
        let actual = t.try_write(move |tx| user.save(tx));
        let errors = match actual {
            Ok(()) => vec![],
            Err(Error::RecordInvalid(errors)) => errors.0,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(json!(errors), row["errors"], "{row}");
        if !errors.is_empty() {
            assert_eq!(settings(&t).presence_setting, "auto");
            assert!(!settings(&t).quiet_hours_enabled);
        }
    }
}

#[test]
fn changed_settings_touch_updated_at_and_an_unchanged_save_does_not() {
    let t = TestDb::new();
    let mut user = settings(&t);
    let before = user.user.updated_at;
    t.clock.travel(SignedDuration::from_mins(1));
    user.presence_setting = "invisible".into();
    user = t.write(move |tx| {
        user.save(tx)?;
        Ok(user)
    });
    assert!(user.user.updated_at > before);
    assert_eq!(settings(&t).presence_setting, "invisible");
    let updated = user.user.updated_at;
    t.clock.travel(SignedDuration::from_mins(1));
    t.write(move |tx| user.save(tx));
    assert_eq!(settings(&t).user.updated_at, updated);
}

/// A core change persisted ahead of the frozen clock (a later writer's clock), then the clock
/// returns: touches at the frozen time must keep that later revision.
fn later_core_revision(t: &TestDb) -> crate::User {
    let frozen = t.now();
    t.clock.travel(SignedDuration::from_mins(1));
    let user = t.write(|tx| {
        let mut user = crate::User::find(tx.conn(), id("david"))?;
        for name in ["First core change", "Latest core change"] {
            user.update(
                tx,
                crate::UserChanges {
                    name: Some(name.into()),
                    ..Default::default()
                },
            )?;
        }
        Ok(user)
    });
    t.clock.travel_to(frozen);
    assert!(user.updated_at > t.now());
    user
}

#[test]
fn slash_settings_and_ooo_touches_preserve_later_core_revisions() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let core = later_core_revision(&t);
    let updated = t.write(|tx| {
        crate::slash_commands::user_settings::update(
            tx,
            id("david"),
            json!({"custom_status_text":"Slash status"}),
        )?;
        crate::slash_commands::dispatch(
            tx,
            &crate::slash_commands::Context {
                user_id: id("david"),
                room_id: id("watercooler"),
                thread_id: None,
                huddles_configured: false,
            },
            "/ooo 1h",
        )?;
        Settings::find(tx.conn(), id("david"))
    });
    assert_eq!(updated.user.updated_at, core.updated_at);
    assert_eq!(updated.user.name, core.name);
    assert_eq!(updated.custom_status_text.as_deref(), Some("Slash status"));
    assert_eq!(updated.ooo_broadcast, Some(true));
    assert!(updated.ooo_until.is_some_and(|until| until > t.now()));
}

#[test]
fn settings_touches_preserve_core_revisions_and_refresh_stale_snapshots() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let mut first = settings(&t);
    let mut stale = first.clone();
    let core = later_core_revision(&t);
    first.custom_status_text = Some("Current text".into());
    let first = t.write(move |tx| {
        first.save(tx)?;
        Ok(first)
    });
    stale.custom_status_emoji = Some("🚀".into());
    let second = t.write(move |tx| {
        stale.save(tx)?;
        Ok(stale)
    });
    assert_eq!(first.user.updated_at, core.updated_at);
    assert_eq!(second.user.updated_at, core.updated_at);
    assert_eq!(second.user.name, core.name);
    assert_eq!(second.custom_status_text.as_deref(), Some("Current text"));
    assert_eq!(second.custom_status_emoji.as_deref(), Some("🚀"));
    let persisted = settings(&t);
    assert_eq!(second.user, persisted.user);
    assert_eq!(second.custom_status_text, persisted.custom_status_text);
    assert_eq!(second.custom_status_emoji, persisted.custom_status_emoji);
    let revision = second.user.updated_at;
    t.write(move |tx| {
        let mut second = second;
        second.save(tx)
    });
    assert_eq!(settings(&t).user.updated_at, revision);
}

#[test]
fn ooo_claims_preserve_core_revisions_from_the_persisted_row() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let now = t.now();
    let mut first = settings(&t);
    let stale_claimant = first.clone();
    let core = later_core_revision(&t);
    first.custom_status_text = Some("Current status".into());
    let saved = t.write(move |tx| {
        first.save(tx)?;
        Ok(first)
    });
    let claimed = t.write(move |tx| {
        assert!(stale_claimant.claim_ooo_broadcast(tx, true, now)?);
        Settings::find(tx.conn(), id("david"))
    });
    assert_eq!(saved.user.updated_at, core.updated_at);
    assert_eq!(claimed.user.updated_at, core.updated_at);
    let claimed_revision = claimed.user.updated_at;
    let cleared = t.write(move |tx| {
        assert!(claimed.claim_ooo_broadcast(tx, false, now)?);
        Settings::find(tx.conn(), id("david"))
    });
    assert_eq!(cleared.user.updated_at, claimed_revision);
    assert_eq!(cleared.ooo_broadcast, Some(false));
    assert_eq!(
        cleared.custom_status_text.as_deref(),
        Some("Current status")
    );
}

#[test]
fn concurrent_settings_instances_only_write_the_fields_each_changed() {
    let t = TestDb::new();
    let mut first = settings(&t);
    let mut second = settings(&t);
    first.dnd_enabled = true;
    first.presence_setting = "invisible".into();
    t.write(move |tx| first.save(tx));
    second.custom_status_text = Some("Concurrent edit".into());
    t.write(move |tx| second.save(tx));
    let actual = settings(&t);
    assert_eq!(
        serde_json::json!({"dnd_enabled":actual.dnd_enabled,"presence_setting":actual.presence_setting,"custom_status_text":actual.custom_status_text}),
        vectors()["dirty_write"]
    );
}

#[test]
fn an_unchanged_expired_ooo_end_still_saves_but_replacing_it_with_a_past_end_is_invalid() {
    let t = TestDb::new();
    let old = t.now().ago(SignedDuration::from_hours(1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET ooo_until=? WHERE id=?",
            rusqlite::params![old, id("david")],
        )?;
        Ok(())
    });
    let mut user = settings(&t);
    user.custom_status_text = Some("Working".into());
    user = t.write(move |tx| {
        user.save(tx)?;
        Ok(user)
    });
    user.ooo_until = Some(old.ago(SignedDuration::from_hours(1)));
    assert!(matches!(
        t.try_write(move |tx| user.save(tx)),
        Err(Error::RecordInvalid(_))
    ));
    assert_eq!(settings(&t).ooo_until, Some(old));
}

#[test]
fn blank_time_zone_normalizes_to_nil_and_setting_it_is_persisted() {
    let t = TestDb::new();
    let mut user = settings(&t);
    user.time_zone = Some("Pacific Time (US & Canada)".into());
    user.time_zone_explicit = true;
    user = t.write(move |tx| {
        user.save(tx)?;
        Ok(user)
    });
    assert_eq!(settings(&t).time_zone, user.time_zone);
    user.time_zone = Some(" ".into());
    t.write(move |tx| user.save(tx));
    assert_eq!(settings(&t).time_zone, None);
    assert!(settings(&t).time_zone_explicit);
}

#[test]
fn unknown_expiry_raises_and_blank_preserves_the_old_expiry() {
    let t = TestDb::new();
    let mut user = settings(&t);
    let end = t.now().since(SignedDuration::from_hours(1));
    user.custom_status_expires_at = Some(end);
    user.set_custom_status_expires_in("", t.now()).unwrap();
    assert_eq!(user.custom_status_expires_at, Some(end));
    assert!(
        user.set_custom_status_expires_in("fortnight", t.now())
            .is_err()
    );
    assert_eq!(user.custom_status_expires_at, Some(end));
    assert!(user.ooo_preset_until("fortnight", None, t.now()).is_err());
}

#[test]
fn replacing_keywords_strips_dedupes_and_caps_the_list() {
    let t = TestDb::new();
    t.write(|tx| KeywordAlert::create(tx, id("david"), "stale"));
    t.write(|tx| {
        crate::models::user_status_settings::replace_keyword_alerts(
            tx,
            id("david"),
            &["Deploy\n  deploy  \n\n Launch\n\u{a0}".into()],
        )
    });
    assert_eq!(
        t.read(|c| KeywordAlert::for_user(c, id("david")))
            .iter()
            .map(|a| a.phrase.as_str())
            .collect::<Vec<_>>(),
        ["Deploy", "Launch"]
    );
    let lines = (0..25).map(|i| format!("phrase {i}")).collect::<Vec<_>>();
    assert!(matches!(
        t.try_write(
            move |tx| crate::models::user_status_settings::replace_keyword_alerts(
                tx,
                id("david"),
                &lines
            )
        ),
        Err(Error::RecordInvalid(_))
    ));
    assert_eq!(t.read(|c| KeywordAlert::for_user(c, id("david"))).len(), 2);
}

#[test]
fn an_invalid_settings_save_keeps_the_previous_keyword_list_atomically() {
    let t = TestDb::new();
    t.write(|tx| KeywordAlert::create(tx, id("david"), "deploy"));
    let mut user = settings(&t);
    user.quiet_hours_enabled = true;
    user.dnd_enabled = true;
    // Rescue inside the writer: the savepoint must still undo the replacement.
    t.write(move |tx| {
        assert!(
            user.save_with_keywords(tx, Some(&["launch".into()]))
                .is_err()
        );
        Ok(())
    });
    assert_eq!(
        t.read(|c| KeywordAlert::for_user(c, id("david")))[0].phrase,
        "deploy"
    );
    assert!(!settings(&t).dnd_enabled && !settings(&t).quiet_hours_enabled);
}

#[test]
fn disabling_dnd_clears_keywords_and_the_running_timer() {
    let t = TestDb::new();
    let mut user = settings(&t);
    user.dnd_enabled = true;
    user.dnd_until = Some(t.now().since(SignedDuration::from_hours(1)));
    user = t.write(move |tx| {
        user.save_with_keywords(tx, Some(&["deploy".into()]))?;
        Ok(user)
    });
    user.dnd_enabled = false;
    user.reconcile_dnd_timer(t.now());
    t.write(move |tx| user.save_with_keywords(tx, Some(&[String::new()])));
    let user = settings(&t);
    assert_eq!(user.dnd_until, None);
    assert!(!user.dnd_enabled);
    assert!(
        t.read(|c| KeywordAlert::for_user(c, id("david")))
            .is_empty()
    );
}

#[test]
fn enabling_dnd_clears_an_expired_timer_and_preserves_a_live_one() {
    let t = TestDb::new();
    let mut user = settings(&t);
    user.dnd_enabled = true;
    user.dnd_until = Some(t.now().ago(SignedDuration::from_hours(1)));
    user.reconcile_dnd_timer(t.now());
    assert_eq!(user.dnd_until, None);
    let end = t.now().since(SignedDuration::from_hours(1));
    user.dnd_until = Some(end);
    user.reconcile_dnd_timer(t.now());
    assert_eq!(user.dnd_until, Some(end));
}

#[test]
fn allows_another_active_person_but_rejects_duplicates_self_bots_and_missing_people() {
    let t = TestDb::new();
    t.write(|tx| DndAllowedUser::create(tx, id("david"), id("jason")));
    for target in [id("jason"), id("david"), id("bender"), -1] {
        assert!(matches!(
            t.try_write(move |tx| DndAllowedUser::create(tx, id("david"), target)),
            Err(Error::RecordInvalid(_))
        ));
    }
    assert!(
        t.read(|c| DndAllowedUser::find(c, id("david"), id("jason")))
            .is_some()
    );
    assert!(
        t.read(|c| DndAllowedUser::find(c, id("david"), id("kevin")))
            .is_none()
    );
}

#[test]
fn finding_or_creating_an_allowance_is_idempotent_and_removal_is_scoped() {
    let t = TestDb::new();
    let first = t.write(|tx| DndAllowedUser::find_or_create(tx, id("david"), id("jason")));
    assert_eq!(
        t.write(|tx| DndAllowedUser::find_or_create(tx, id("david"), id("jason"))),
        first
    );
    t.write(|tx| DndAllowedUser::find_or_create(tx, id("kevin"), id("jason")));
    t.write(|tx| DndAllowedUser::remove(tx, id("david"), id("jason")));
    assert!(
        t.read(|c| DndAllowedUser::find(c, id("david"), id("jason")))
            .is_none()
    );
    assert!(
        t.read(|c| DndAllowedUser::find(c, id("kevin"), id("jason")))
            .is_some()
    );
}

// Remaining named StatusSettings sequences use a real writer and an advancing fixture clock.
#[test]
fn ws17_dnd_is_manual_only_outside_quiet_hours() {
    let t = TestDb::new();
    assert!(!settings(&t).dnd_active(t.now()));
    t.write(|tx| {
        let mut user = Settings::find(tx.conn(), id("david"))?;
        user.dnd_enabled = true;
        user.save(tx)
    });
    assert!(settings(&t).dnd_active(t.now()));
}
#[test]
fn ws17_quiet_hours_cover_an_overnight_window_in_the_users_time_zone() {
    let t = TestDb::new();
    t.write(|tx| {
        let mut user = Settings::find(tx.conn(), id("david"))?;
        user.time_zone = Some("Pacific Time (US & Canada)".into());
        user.quiet_hours_enabled = true;
        user.quiet_hours_start_minute = clock_time_to_minutes("22:00");
        user.quiet_hours_end_minute = clock_time_to_minutes("07:00");
        user.save(tx)
    });
    t.clock.travel_to(stamp("2026-09-23T06:30:00Z"));
    assert!(settings(&t).quiet_hours_active(t.now()));
    assert!(settings(&t).dnd_active(t.now()));
    t.clock.travel_to(stamp("2026-09-23T16:00:00Z"));
    assert!(!settings(&t).quiet_hours_active(t.now()));
    assert!(!settings(&t).dnd_active(t.now()));
}
#[test]
fn ws17_an_expired_custom_status_reads_as_blank() {
    let t = TestDb::new();
    t.write(|tx| {
        let mut user = Settings::find(tx.conn(), id("david"))?;
        user.custom_status_emoji = Some("🚂".into());
        user.custom_status_text = Some("On a train".into());
        user.custom_status_expires_at = Some(tx.now().since(SignedDuration::from_hours(1)));
        user.save(tx)
    });
    assert_eq!(
        settings(&t).custom_status_display(t.now()).as_deref(),
        Some("🚂 On a train")
    );
    t.travel(2 * 60 * 60);
    assert!(settings(&t).custom_status_display(t.now()).is_none());
}
#[test]
fn ws17_effective_presence_folds_the_manual_setting_over_the_lease_state() {
    use crate::models::workspace_presence_lease::Presence::*;
    let t = TestDb::new();
    for lease in [Online, Idle, Offline] {
        assert_eq!(settings(&t).effective_presence(lease), lease);
    }
    t.write(|tx| {
        let mut user = Settings::find(tx.conn(), id("david"))?;
        user.presence_setting = "dnd".into();
        user.save(tx)
    });
    for (lease, expected) in [(Online, Dnd), (Idle, Dnd), (Offline, Offline)] {
        assert_eq!(settings(&t).effective_presence(lease), expected);
    }
    t.write(|tx| {
        let mut user = Settings::find(tx.conn(), id("david"))?;
        user.presence_setting = "invisible".into();
        user.save(tx)
    });
    assert_eq!(settings(&t).effective_presence(Online), Offline);
}

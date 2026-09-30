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

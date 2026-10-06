//! Original OOO/meeting test bodies observed in Rails, replayed as ordered operations.
use super::*;
use crate::{Error, Timestamp, UserStatusSettings as Settings};
use rusqlite::{params, types::Value as SqlValue};
use serde_json::{Value, json};

fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}
fn time(value: Option<Timestamp>) -> Value {
    json!(value.map(|s| format!("{:.6}", s.jiff())))
}
fn snapshot(u: &Settings) -> Value {
    json!({"status":u.user.status.name(),"presence_setting":u.presence_setting,
        "custom_status_emoji":u.custom_status_emoji,"custom_status_text":u.custom_status_text,
        "custom_status_expires_at":time(u.custom_status_expires_at),"dnd_enabled":u.dnd_enabled,
        "dnd_until":time(u.dnd_until),"quiet_hours_enabled":u.quiet_hours_enabled,
        "quiet_hours_start_minute":u.quiet_hours_start_minute,"quiet_hours_end_minute":u.quiet_hours_end_minute,
        "time_zone":u.time_zone,"meeting_status_enabled":u.meeting_status_enabled,"meeting_dnd_enabled":u.meeting_dnd_enabled,
        "ooo_until":time(u.ooo_until),"ooo_note":u.ooo_note,"ooo_calendar_enabled":u.ooo_calendar_enabled,
        "ooo_notify_enabled":u.ooo_notify_enabled,"ooo_broadcast":u.ooo_broadcast})
}
fn raw_attrs(tx: &mut Tx<'_>, attrs: &Value) -> Result<()> {
    for (key, value) in attrs.as_object().unwrap() {
        let value = match value {
            Value::Null => SqlValue::Null,
            Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
            Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap()),
            Value::String(s) if key == "status" => {
                SqlValue::Integer(crate::Status::from_name(s).unwrap() as i64)
            }
            Value::String(s) if key.ends_with("_at") || key.ends_with("_until") => {
                SqlValue::Text(stamp(s).to_db())
            }
            Value::String(s) => SqlValue::Text(s.clone()),
            _ => panic!("unexpected fixture value {value}"),
        };
        tx.conn().execute(
            &format!("UPDATE users SET {key}=? WHERE id=?"),
            params![value, id("david")],
        )?;
    }
    Ok(())
}
fn assign(u: &mut Settings, attrs: &Value) {
    for (key, value) in attrs.as_object().unwrap() {
        let text = || value.as_str().map(str::to_owned);
        let timestamp = || value.as_str().map(stamp);
        match key.as_str() {
            "presence_setting" => u.presence_setting = text().unwrap(),
            "time_zone" => u.time_zone = text(),
            "custom_status_emoji" => u.custom_status_emoji = text(),
            "custom_status_text" => u.custom_status_text = text(),
            "custom_status_expires_at" => u.custom_status_expires_at = timestamp(),
            "dnd_enabled" => u.dnd_enabled = value.as_bool().unwrap(),
            "dnd_until" => u.dnd_until = timestamp(),
            "quiet_hours_enabled" => u.quiet_hours_enabled = value.as_bool().unwrap(),
            "quiet_hours_start_minute" => u.quiet_hours_start_minute = value.as_i64(),
            "quiet_hours_end_minute" => u.quiet_hours_end_minute = value.as_i64(),
            "meeting_status_enabled" => u.meeting_status_enabled = value.as_bool().unwrap(),
            "meeting_dnd_enabled" => u.meeting_dnd_enabled = value.as_bool().unwrap(),
            "ooo_until" => u.ooo_until = timestamp(),
            "ooo_note" => u.ooo_note = text(),
            "ooo_calendar_enabled" => u.ooo_calendar_enabled = value.as_bool().unwrap(),
            "ooo_notify_enabled" => u.ooo_notify_enabled = value.as_bool().unwrap(),
            other => panic!("unexpected submitted attribute {other}"),
        }
    }
}
fn replay(file: &str, title: &str) {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws17_named_calendar_status.json"
    ))
    .unwrap();
    let row = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["file"] == file && r["test"] == title)
        .unwrap();
    let steps = row["steps"].as_array().unwrap();
    let t = TestDb::with_clock(
        TestClock::frozen_at(stamp(steps[0]["now"].as_str().unwrap())),
        4,
    );
    let initial = steps
        .iter()
        .find_map(|step| step.get("stored_before"))
        .unwrap()
        .clone();
    if matches!(
        title,
        "out of office defaults off" | "meeting status and quiet-during-meetings default off"
    ) {
        assert_eq!(
            snapshot(&t.read(|c| Settings::find(c, id("david")))),
            initial,
            "{title}: untouched fixture defaults"
        );
    } else {
        t.write(move |tx| {
            raw_attrs(tx, &initial)?;
            Ok(())
        });
    }
    let mut user = t.read(|c| Settings::find(c, id("david")));
    for step in steps {
        let now = stamp(step["now"].as_str().unwrap());
        t.clock.travel_to(now);
        let op = step["operation"].as_str().unwrap();
        if op == "cache_create" {
            let attrs = step["attrs"].clone();
            t.write(move |tx| {
                tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,created_at,updated_at) VALUES (?,?,?,?,?,?)",
                    params![id("david"),attrs["busy_intervals"].to_string(),attrs["ooo_intervals"].to_string(),attrs["fetched_at"].as_str().map(stamp),now,now])?;
                Ok(())
            });
            user.meeting_cache = t.read(|c| Settings::find(c, id("david"))).meeting_cache;
            continue;
        }
        assert_eq!(
            snapshot(&user),
            step["loaded_before"],
            "{title}: before {op}"
        );
        assert_eq!(
            snapshot(&t.read(|c| Settings::find(c, id("david")))),
            step["stored_before"],
            "{title}: stored before {op}"
        );
        let result = match op {
            "update" | "update!" => {
                let attrs = if step["args"].as_array().unwrap().is_empty() {
                    &step["kwargs"]
                } else {
                    &step["args"][0]
                };
                assign(&mut user, attrs);
                let next = user.clone();
                let (next, ok, errors) = t.write(move |tx| {
                    let mut next = next;
                    let (ok, errors) = match next.save(tx) {
                        Ok(()) => (true, json!([])),
                        Err(Error::RecordInvalid(errors)) => (false, json!(errors.0)),
                        Err(error) => return Err(error),
                    };
                    Ok((next, ok, errors))
                });
                assert_eq!(errors, step["errors"], "{title}: {op} validation");
                user = next;
                json!(ok)
            }
            "update_columns" => {
                let attrs = if step["args"].as_array().unwrap().is_empty() {
                    step["kwargs"].clone()
                } else {
                    step["args"][0].clone()
                };
                t.write(move |tx| raw_attrs(tx, &attrs));
                user = t.read(|c| Settings::find(c, id("david")));
                json!(true)
            }
            "reload" => {
                user = t.read(|c| Settings::find(c, id("david")));
                Value::Null
            }
            "deactivate" => {
                let mut next = user.clone();
                user = t.write(move |tx| {
                    next.deactivate(tx)?;
                    Ok(next)
                });
                Value::Null
            }
            "claim_ooo_broadcast!" => {
                let u = user.clone();
                let active = step["args"][0].as_bool().unwrap();
                json!(t.write(move |tx| u.claim_ooo_broadcast(tx, active, now)))
            }
            "ooo_preset_until" => {
                let result = user.ooo_preset_until(
                    step["args"][0].as_str().unwrap(),
                    step["args"][1].as_str(),
                    now,
                );
                if step.get("error").is_some() {
                    assert!(result.is_err(), "{title}: {op} must raise");
                }
                match result {
                    Ok(end) => time(end),
                    Err(error) => {
                        assert_eq!(json!(error.to_string()), step["error"], "{title}");
                        Value::Null
                    }
                }
            }
            "manual_ooo_active?" => json!(user.manual_ooo_active(now)),
            "calendar_ooo_active?" => json!(user.calendar_ooo_active(now)),
            "out_of_office?" => json!(user.out_of_office(now)),
            "ooo_until_effective" => time(user.ooo_until_effective(now)),
            "ooo_until_date" => json!(user.ooo_until_date(now)),
            "ooo_status_visible?" => json!(user.ooo_status_visible(now)),
            "ooo_status_text" => json!(user.ooo_status_text(now)),
            "status_text_display" => json!(user.status_text_display(now)),
            "in_meeting?" => json!(user.in_meeting(now)),
            "meeting_status_visible?" => json!(user.meeting_status_visible(now)),
            "meeting_dnd_active?" => json!(user.meeting_dnd_active(now)),
            "ooo_dnd_active?" => json!(user.ooo_dnd_active(now)),
            "quiet_hours_active?" => json!(user.quiet_hours_active(now)),
            "ooo_until" => time(user.ooo_until),
            "ooo_note" => json!(user.ooo_note),
            "ooo_broadcast" => json!(user.ooo_broadcast),
            "ooo_calendar_enabled?" => json!(user.ooo_calendar_enabled),
            "ooo_notify_enabled?" => json!(user.ooo_notify_enabled),
            "meeting_status_enabled?" => json!(user.meeting_status_enabled),
            "meeting_dnd_enabled?" => json!(user.meeting_dnd_enabled),
            other => panic!("unexpected operation {other}"),
        };
        if step.get("error").is_none() {
            assert_eq!(result, step["result"], "{title}: {op}");
        }
        assert_eq!(snapshot(&user), step["loaded_after"], "{title}: after {op}");
        assert_eq!(
            snapshot(&t.read(|c| Settings::find(c, id("david")))),
            step["stored_after"],
            "{title}: stored after {op}"
        );
    }
}
macro_rules! named_status {
    ($name:ident, $file:literal, $title:literal) => {
        #[test]
        fn $name() {
            replay($file, $title);
        }
    };
}

named_status!(
    ws17_named_out_of_office_out_of_office_defaults_off,
    "test/models/user/out_of_office_test.rb",
    "out of office defaults off"
);

named_status!(
    ws17_named_out_of_office_a_manual_ooo_is_active_until_its_end_then_reads_as_off,
    "test/models/user/out_of_office_test.rb",
    "a manual OOO is active until its end, then reads as off"
);

named_status!(ws17_named_out_of_office_setting_an_ooo_end_in_the_past_is_invalid_but_an_expired_end_left_behind_still_saves, "test/models/user/out_of_office_test.rb", "setting an OOO end in the past is invalid, but an expired end left behind still saves");

named_status!(
    ws17_named_out_of_office_a_note_longer_than_140_characters_is_invalid,
    "test/models/user/out_of_office_test.rb",
    "a note longer than 140 characters is invalid"
);

named_status!(
    ws17_named_out_of_office_the_status_line_names_the_return_date_and_the_note,
    "test/models/user/out_of_office_test.rb",
    "the status line names the return date and the note"
);

named_status!(
    ws17_named_out_of_office_the_return_date_renders_in_the_ooo_member_s_own_zone,
    "test/models/user/out_of_office_test.rb",
    "the return date renders in the OOO member's own zone"
);

named_status!(
    ws17_named_out_of_office_ooo_wins_over_a_custom_status_dnd_and_the_meeting_label,
    "test/models/user/out_of_office_test.rb",
    "OOO wins over a custom status, DND, and the meeting label"
);

named_status!(
    ws17_named_out_of_office_invisible_hides_the_ooo_label_but_ooo_still_reads_as_active,
    "test/models/user/out_of_office_test.rb",
    "invisible hides the OOO label but OOO still reads as active"
);

named_status!(
    ws17_named_out_of_office_ooo_quiet_never_suppresses_the_ooo_label,
    "test/models/user/out_of_office_test.rb",
    "OOO quiet never suppresses the OOO label"
);

named_status!(
    ws17_named_out_of_office_calendar_ooo_needs_the_opt_in_and_a_covering_interval,
    "test/models/user/out_of_office_test.rb",
    "calendar OOO needs the opt-in and a covering interval"
);

named_status!(
    ws17_named_out_of_office_a_calendar_ooo_outside_its_intervals_reads_as_off,
    "test/models/user/out_of_office_test.rb",
    "a calendar OOO outside its intervals reads as off"
);

named_status!(
    ws17_named_out_of_office_overlapping_manual_and_calendar_ooo_show_the_later_end,
    "test/models/user/out_of_office_test.rb",
    "overlapping manual and calendar OOO show the later end"
);

named_status!(
    ws17_named_out_of_office_the_note_shows_only_while_the_manual_ooo_is_active,
    "test/models/user/out_of_office_test.rb",
    "the note shows only while the manual OOO is active"
);

named_status!(
    ws17_named_out_of_office_ooo_presets_run_to_the_end_of_the_day_in_the_member_s_zone,
    "test/models/user/out_of_office_test.rb",
    "OOO presets run to the end of the day in the member's zone"
);

named_status!(
    ws17_named_out_of_office_the_monday_preset_is_a_week_out_on_mondays,
    "test/models/user/out_of_office_test.rb",
    "the Monday preset is a week out on Mondays"
);

named_status!(
    ws17_named_out_of_office_the_custom_preset_parses_a_datetime_local_value_in_the_member_s_zone,
    "test/models/user/out_of_office_test.rb",
    "the custom preset parses a datetime-local value in the member's zone"
);

named_status!(
    ws17_named_out_of_office_an_unknown_preset_raises,
    "test/models/user/out_of_office_test.rb",
    "an unknown preset raises"
);

named_status!(ws17_named_out_of_office_claim_ooo_broadcast_wins_the_first_claim_and_each_flip_and_loses_re_runs, "test/models/user/out_of_office_test.rb", "claim_ooo_broadcast! wins the first claim and each flip, and loses re-runs");

named_status!(
    ws17_named_out_of_office_claiming_an_end_clears_the_expired_manual_columns,
    "test/models/user/out_of_office_test.rb",
    "claiming an end clears the expired manual columns"
);

named_status!(
    ws17_named_out_of_office_claiming_an_end_keeps_a_manual_ooo_set_racing_the_sweep,
    "test/models/user/out_of_office_test.rb",
    "claiming an end keeps a manual OOO set racing the sweep"
);

named_status!(
    ws17_named_out_of_office_ooo_quiets_notifications_unless_the_member_keeps_them_on,
    "test/models/user/out_of_office_test.rb",
    "OOO quiets notifications unless the member keeps them on"
);

named_status!(
    ws17_named_out_of_office_deactivating_clears_the_manual_ooo_columns,
    "test/models/user/out_of_office_test.rb",
    "deactivating clears the manual OOO columns"
);

named_status!(
    ws17_named_meeting_status_meeting_status_and_quiet_during_meetings_default_off,
    "test/models/user/meeting_status_test.rb",
    "meeting status and quiet-during-meetings default off"
);

named_status!(
    ws17_named_meeting_status_in_meeting_needs_the_opt_in_and_a_covering_interval,
    "test/models/user/meeting_status_test.rb",
    "in_meeting? needs the opt-in and a covering interval"
);

named_status!(
    ws17_named_meeting_status_in_meeting_is_false_without_a_cache_row,
    "test/models/user/meeting_status_test.rb",
    "in_meeting? is false without a cache row"
);

named_status!(
    ws17_named_meeting_status_the_meeting_label_shows_while_in_a_meeting,
    "test/models/user/meeting_status_test.rb",
    "the meeting label shows while in a meeting"
);

named_status!(
    ws17_named_meeting_status_a_custom_status_wins_over_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "a custom status wins over the meeting label"
);

named_status!(
    ws17_named_meeting_status_an_expired_custom_status_yields_to_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "an expired custom status yields to the meeting label"
);

named_status!(
    ws17_named_meeting_status_manual_dnd_wins_over_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "manual DND wins over the meeting label"
);

named_status!(
    ws17_named_meeting_status_the_dnd_presence_wins_over_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "the DND presence wins over the meeting label"
);

named_status!(
    ws17_named_meeting_status_quiet_hours_win_over_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "quiet hours win over the meeting label"
);

named_status!(
    ws17_named_meeting_status_invisible_hides_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "invisible hides the meeting label"
);

named_status!(
    ws17_named_meeting_status_quiet_during_meetings_never_suppresses_the_meeting_label,
    "test/models/user/meeting_status_test.rb",
    "quiet-during-meetings never suppresses the meeting label"
);

named_status!(
    ws17_named_meeting_status_quiet_during_meetings_only_works_while_meeting_status_is_on,
    "test/models/user/meeting_status_test.rb",
    "quiet-during-meetings only works while meeting status is on"
);

named_status!(
    ws17_named_meeting_status_quiet_during_meetings_applies_through_a_custom_status,
    "test/models/user/meeting_status_test.rb",
    "quiet-during-meetings applies through a custom status"
);

named_status!(
    ws17_named_meeting_status_quiet_during_meetings_is_off_outside_busy_intervals,
    "test/models/user/meeting_status_test.rb",
    "quiet-during-meetings is off outside busy intervals"
);

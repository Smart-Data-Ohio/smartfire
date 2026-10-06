//! Membership#sync_removed_room_calendar_entries, including final transaction state.
use super::*;
use serde_json::{Value, json};

#[test]
fn membership_calendar_callback_matches_pinned_rails_final_state() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../models/calendar_event/membership_calendar.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let t = frozen();
        let name = row["name"].as_str().unwrap().to_owned();
        let scenario = name.clone();
        let user = id("david");
        let room = id("designers");
        let event = id("launch_party");
        t.write(move |tx| {
            if !matches!(scenario.as_str(), "no_entries" | "added_later") {
                crate::models::google_entry::reserve(
                    tx,
                    if scenario == "other_room" {
                        id("watercooler_sync")
                    } else {
                        event
                    },
                    if scenario == "other_user" {
                        id("jason")
                    } else {
                        user
                    },
                )?;
            }
            Ok(())
        });
        t.sink.take();
        let scenario = name.clone();
        let sink = t.sink.clone();
        let result = t.try_write(move |tx| {
            crate::Membership::find_by_room_and_user(tx.conn(), room, user)?
                .unwrap()
                .destroy(tx)?;
            if scenario == "removed_later" {
                let entry = crate::models::google_entry::find(tx.conn(), event, user)?.unwrap();
                crate::models::google_entry::delete(tx, &entry)?;
            }
            if scenario == "added_later" {
                crate::models::google_entry::reserve(tx, event, user)?;
            }
            assert!(
                sink.take()
                    .iter()
                    .all(|e| !matches!(e, Event::Job(j) if j.class == "Calendar::SyncEntryJob")),
                "not published before commit"
            );
            if scenario == "rollback" {
                return Err(Error::Other("outer rollback".into()));
            }
            Ok(())
        });
        assert_eq!(result.is_err(), name == "rollback", "{name}");
        let jobs: Vec<_> = t
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Job(j) if j.class == "Calendar::SyncEntryJob" => Some(json!([
                    j.class,
                    [j.arguments["event_id"], j.arguments["user_id"]]
                ])),
                _ => None,
            })
            .collect();
        assert_eq!(json!(jobs), row["jobs"], "{name}");
        assert_eq!(
            t.read(|c| Ok(crate::Membership::find_by_room_and_user(c, room, user)?.is_some())),
            row["membership_exists"].as_bool().unwrap(),
            "{name}"
        );
        assert_eq!(row["before_commit"], 0);
    }
}

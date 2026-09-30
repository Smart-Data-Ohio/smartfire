//! Both zone writers must reject the same identifiers as pinned ActiveSupport/TZInfo.
use super::{time_parser, user_settings};
use crate::{Error, User, UserChanges, fixtures::identify, tests::TestDb};
use serde_json::{Value, json};

fn vectors() -> Value {
    serde_json::from_str(include_str!("../rails_time_zones.json")).unwrap()
}

#[test]
fn known_zones_match_all_pinned_rails_identifiers_aliases_and_case_probes() {
    for (name, accepted) in vectors()["probes"].as_object().unwrap() {
        assert_eq!(
            time_parser::known_zone(name).is_some(),
            accepted.as_bool().unwrap(),
            "{name:?}"
        );
    }
}

fn writer_case(slash: bool) {
    let t = TestDb::new();
    let id = identify("david");
    // Put the reported lowercase bug first; also cover every accepted identifier and alias.
    let snapshot = vectors();
    let names = [
        "america/new_york",
        "AMERICA/NEW_YORK",
        "utc",
        "Mars/Olympus",
        " America/New_York",
        "America/New_York ",
        "",
        " \t",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(
        snapshot["identifiers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned()),
    )
    .chain(snapshot["mapping"].as_object().unwrap().keys().cloned());
    for name in names {
        t.db.write_blocking(move |tx| {
            tx.conn().execute(
                "UPDATE users SET time_zone='UTC',time_zone_explicit=0 WHERE id=?",
                [id],
            )?;
            Ok(())
        })
        .unwrap();
        let before = t.db.read_blocking(move |c| User::find(c, id)).unwrap();
        let zone = name.clone();
        let result = t.db.write_blocking(move |tx| {
            if slash {
                user_settings::update(tx, id, json!({"time_zone":zone}))
            } else {
                let mut user = User::find(tx.conn(), id)?;
                user.update(
                    tx,
                    UserChanges {
                        name: Some("must roll back".into()),
                        time_zone: Some(Some(zone)),
                        time_zone_explicit: Some(true),
                        ..Default::default()
                    },
                )
            }
        });
        let blank = campfire_richtext::ruby::is_blank(&name);
        let accepted = blank || snapshot["probes"][&name].as_bool().unwrap();
        assert_eq!(
            result.is_ok(),
            accepted,
            "writer slash={slash}, {name:?}: {result:?}"
        );
        let (stored, explicit): (Option<String>, bool) =
            t.db.read_blocking(move |c| {
                Ok(c.query_row(
                    "SELECT time_zone,time_zone_explicit FROM users WHERE id=?",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .unwrap();
        if accepted {
            assert_eq!(stored, (!blank).then_some(name));
            assert_eq!(explicit, !slash);
        } else {
            let Error::RecordInvalid(errors) = result.unwrap_err() else {
                panic!("expected validation error")
            };
            assert_eq!(errors.on("time_zone"), ["is not a valid time zone"]);
            assert_eq!(stored.as_deref(), Some("UTC"));
            assert!(!explicit);
            assert_eq!(
                t.db.read_blocking(move |c| User::find(c, id)).unwrap(),
                before
            );
        }
    }
}
#[test]
fn user_update_validates_zones_before_writing_any_change_or_marker() {
    writer_case(false);
}
#[test]
fn slash_settings_writer_uses_the_same_exact_zones_and_blank_normalization() {
    writer_case(true);
}

#[test]
fn persisted_invalid_zone_refuses_an_update_without_a_zone_assignment() {
    let t = TestDb::new();
    let id = identify("david");
    t.db.write_blocking(move |tx| {
        tx.conn().execute(
            "UPDATE users SET time_zone='america/new_york',time_zone_explicit=0 WHERE id=?",
            [id],
        )?;
        Ok(())
    })
    .unwrap();
    let before = t.db.read_blocking(move |c| User::find(c, id)).unwrap();
    let result = t.db.write_blocking(move |tx| {
        let mut user = User::find(tx.conn(), id)?;
        user.update(
            tx,
            UserChanges {
                name: Some("must roll back".into()),
                time_zone_explicit: Some(true),
                ..Default::default()
            },
        )
    });
    assert!(
        result.is_err(),
        "Rails validates an existing zone on every save: {result:?}"
    );
    let Error::RecordInvalid(errors) = result.unwrap_err() else {
        panic!("expected validation error")
    };
    assert_eq!(errors.on("time_zone"), ["is not a valid time zone"]);
    assert_eq!(
        t.db.read_blocking(move |c| User::find(c, id)).unwrap(),
        before
    );
    let explicit: bool =
        t.db.read_blocking(move |c| {
            Ok(c.query_row(
                "SELECT time_zone_explicit FROM users WHERE id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert!(!explicit);
}

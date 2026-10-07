//! The UI choice kept in `users.inbox_preferences`: stored beside the inbox keys without
//! disturbing them, and invisible to the profile's validation and inbox switches.
use super::*;
use crate::models::user::profile_settings;
use crate::models::user::ui_preference::{self, UiPreference};
use serde_json::{Value, json};

fn preferences(t: &TestDb, user: i64) -> Value {
    t.read(move |conn| {
        let raw: Option<String> = conn.query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |r| r.get(0),
        )?;
        Ok(raw
            .map(|raw| serde_json::from_str(&raw).unwrap())
            .unwrap_or(Value::Null))
    })
}

#[test]
fn no_choice_until_one_is_stored() {
    let t = TestDb::new();
    let david = id("david");
    assert_eq!(t.read(move |conn| ui_preference::stored(conn, david)), None);
    assert_eq!(
        t.read(|conn| ui_preference::stored(conn, 999_999)),
        None,
        "a missing user"
    );
    t.write(|tx| ui_preference::store(tx, 999_999, UiPreference::Next));

    t.write(move |tx| ui_preference::store(tx, david, UiPreference::Next));
    assert_eq!(
        t.read(move |conn| ui_preference::stored(conn, david)),
        Some(UiPreference::Next)
    );
    t.write(move |tx| ui_preference::store(tx, david, UiPreference::Classic));
    assert_eq!(
        t.read(move |conn| ui_preference::stored(conn, david)),
        Some(UiPreference::Classic)
    );
    assert_eq!(preferences(&t, david)[ui_preference::KEY], json!(false));
}

#[test]
fn the_deployment_default_decides_for_people_who_have_not_chosen() {
    use UiPreference::*;
    assert_eq!(UiPreference::effective(None, false), Classic);
    assert_eq!(UiPreference::effective(None, true), Next);
    assert_eq!(UiPreference::effective(Some(Classic), true), Classic);
    assert_eq!(UiPreference::effective(Some(Next), false), Next);
    assert_eq!(UiPreference::parse("next"), Some(Next));
    assert_eq!(UiPreference::parse("classic"), Some(Classic));
    assert_eq!(UiPreference::parse("Next"), None);
    assert_eq!(Next.as_str(), "next");
}

#[test]
fn storing_keeps_the_inbox_keys_and_the_row_untouched() {
    let t = TestDb::new();
    let david = id("david");
    let before = t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET inbox_preferences='{\"agent_work\":false,\"event_reminders\":\"0\"}' WHERE id=?",
            [david],
        )?;
        Ok(tx.conn().query_row("SELECT updated_at FROM users WHERE id=?", [david], |r| r.get::<_, String>(0))?)
    });
    let events = t.events().len();
    t.write(move |tx| ui_preference::store(tx, david, UiPreference::Next));
    assert_eq!(
        preferences(&t, david),
        json!({"agent_work": false, "event_reminders": "0", "next_ui": true})
    );
    let after = t.read(move |conn| {
        Ok(
            conn.query_row("SELECT updated_at FROM users WHERE id=?", [david], |r| {
                r.get::<_, String>(0)
            })?,
        )
    });
    assert_eq!(after, before, "no updated_at bump, so the avatar URL stays");
    assert_eq!(t.events().len(), events, "no broadcast or job");
}

#[test]
fn preferences_that_are_not_an_object_start_over() {
    let t = TestDb::new();
    let david = id("david");
    for raw in [None, Some("null"), Some("[]"), Some("not json")] {
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET inbox_preferences=? WHERE id=?",
                rusqlite::params![raw, david],
            )?;
            Ok(())
        });
        assert_eq!(
            t.read(move |conn| ui_preference::stored(conn, david)),
            None,
            "{raw:?}"
        );
        t.write(move |tx| ui_preference::store(tx, david, UiPreference::Next));
        assert_eq!(preferences(&t, david), json!({"next_ui": true}), "{raw:?}");
    }
}

#[test]
fn the_profile_form_saves_around_the_choice() {
    let t = TestDb::new();
    let david = id("david");
    t.write(move |tx| ui_preference::store(tx, david, UiPreference::Next));
    // The classic profile form submits the inbox switches (and the profile's validation reads
    // every key of the column); the choice survives it.
    t.write(move |tx| {
        profile_settings::update(
            tx,
            david,
            profile_settings::Changes {
                theme: Some("dark".into()),
                inbox_preferences: Some(json!({"agent_work": "0", "next_ui": false})),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        preferences(&t, david),
        json!({"agent_work": "0", "next_ui": true})
    );
    assert_eq!(
        t.read(move |conn| ui_preference::stored(conn, david)),
        Some(UiPreference::Next)
    );
}

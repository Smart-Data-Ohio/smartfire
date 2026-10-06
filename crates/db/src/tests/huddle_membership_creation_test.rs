use crate::tests::{TestDb, huddle_notices_test, huddle_revocation_test};
use crate::{Membership, Timestamp};
use serde_json::{Value, json};

fn run(name: &str) {
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_membership_creation_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 6);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let input = case["input"].clone();
    db.write(move |tx| huddle_notices_test::load(tx, &input));
    db.sink.take();
    let room = case["room_id"].as_i64().unwrap();
    let user = case["user_id"].as_i64().unwrap();
    let before = db.read(Membership::count);
    let result = db.try_write(move |tx| Membership::create_default(tx, room, user));
    match result {
        Ok(member) => {
            assert!(
                case["error"].is_null(),
                "{name} accepted invalid coordinates"
            );
            let mut actual = db
                .read(|conn| crate::tests::huddle_invitations_test::snapshot(conn, "memberships"))
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == member.id)
                .unwrap()
                .clone();
            let mut expected = case["row"].clone();
            huddle_revocation_test::normalize(&mut actual);
            huddle_revocation_test::normalize(&mut expected);
            assert_eq!(actual, expected, "{name} row");
            assert_eq!(db.read(Membership::count), before + 1);
        }
        Err(crate::Error::RecordInvalid(errors)) => {
            let mut fields = serde_json::Map::new();
            for (key, message) in errors.0 {
                fields
                    .entry(key)
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(json!(message));
            }
            assert_eq!(Value::Object(fields), case["error"], "{name} errors");
            assert_eq!(
                db.read(Membership::count),
                before,
                "{name} invalid row persisted"
            );
            assert!(
                db.events().is_empty(),
                "{name} invalid creation emitted callbacks"
            );
        }
        Err(error) => panic!("{name}: {error}"),
    }
    assert_eq!(
        json!(db.read(|conn| Ok(crate::Room::find(conn, 9001)?.direct_member_key))),
        case["direct_member_key"],
        "{name} commit callback"
    );
}
macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            run(stringify!($name));
        }
    };
}
case!(stage);
case!(voice);
case!(direct);
case!(missing_room);
case!(missing_user);
case!(both_missing);

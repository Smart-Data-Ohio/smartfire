//! `bot_avatars`: a bot's picked avatar, validated, set, replaced and cleared.
use super::*;
use crate::{BotAvatar, Error, User, bot_avatar::normalize_color};

fn spec(user_id: i64) -> BotAvatar {
    BotAvatar {
        user_id,
        shape: "clover".into(),
        color: "#7aa2ff".into(),
        face: "mouth".into(),
        accessory: Some("beret".into()),
    }
}

fn updated_at(t: &TestDb, id: i64) -> crate::Timestamp {
    t.read(|c| User::find(c, id)).updated_at
}

#[test]
fn colours_are_six_digit_hex() {
    assert_eq!(normalize_color("#7AA2FF").as_deref(), Some("#7aa2ff"));
    assert_eq!(normalize_color("7aa2ff"), None);
    assert_eq!(normalize_color("#7af"), None);
    assert_eq!(normalize_color("#7aa2fg"), None);
    assert_eq!(normalize_color("red"), None);
}

#[test]
fn a_bot_keeps_the_spec_it_was_given() {
    let t = TestDb::new();
    let bot = id("bender");
    let before = updated_at(&t, bot);
    t.travel(5);
    t.write(move |tx| BotAvatar::set(tx, bot, Some(&spec(bot))));
    assert_eq!(t.read(|c| BotAvatar::for_user(c, bot)), Some(spec(bot)));
    assert!(updated_at(&t, bot) > before, "the bot is touched");

    let replaced = BotAvatar {
        shape: "cat".into(),
        accessory: None,
        ..spec(bot)
    };
    let stored = replaced.clone();
    t.write(move |tx| BotAvatar::set(tx, bot, Some(&stored)));
    assert_eq!(t.read(|c| BotAvatar::for_user(c, bot)), Some(replaced));

    t.write(move |tx| BotAvatar::set(tx, bot, None));
    assert_eq!(t.read(|c| BotAvatar::for_user(c, bot)), None);
}

#[test]
fn an_unchanged_spec_leaves_the_bot_alone() {
    let t = TestDb::new();
    let bot = id("bender");
    t.write(move |tx| BotAvatar::set(tx, bot, Some(&spec(bot))));
    let before = updated_at(&t, bot);
    t.travel(5);
    t.write(move |tx| BotAvatar::set(tx, bot, Some(&spec(bot))));
    assert_eq!(updated_at(&t, bot), before);
}

#[test]
fn unknown_values_and_people_are_refused() {
    let t = TestDb::new();
    let bot = id("bender");
    for bad in [
        BotAvatar {
            shape: "dragon".into(),
            ..spec(bot)
        },
        BotAvatar {
            color: "blue".into(),
            ..spec(bot)
        },
        BotAvatar {
            color: "#7AA2FF".into(),
            ..spec(bot)
        },
        BotAvatar {
            face: "nose".into(),
            ..spec(bot)
        },
        BotAvatar {
            accessory: Some("monocle".into()),
            ..spec(bot)
        },
    ] {
        let outcome = t.try_write(move |tx| BotAvatar::set(tx, bot, Some(&bad)));
        assert!(
            matches!(outcome, Err(Error::RecordInvalid(_))),
            "{outcome:?}"
        );
    }
    let david = id("david");
    let outcome = t.try_write(move |tx| BotAvatar::set(tx, david, Some(&spec(david))));
    assert!(
        matches!(outcome, Err(Error::RecordInvalid(_))),
        "{outcome:?}"
    );
    assert_eq!(t.read(|c| BotAvatar::for_ids(c, &[bot, david])).len(), 0);
}

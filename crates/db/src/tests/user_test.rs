//! `test/models/user_test.rb`, `user/bot_test.rb`, `user/role_test.rb`, plus Bannable.

use super::*;
use crate::{
    Ban, Membership, Message, NewUser, PasswordDigest, PushSubscription, Role, Room, RoomType, Search, Session,
    Status, User, UserChanges, Webhook,
};
use crate::models::user::{BOT_KEY_PLACEHOLDER, digest_bot_token, secure_compare};

fn user(t: &TestDb, label: &str) -> User {
    let user_id = id(label);
    t.read(|c| User::find(c, user_id))
}

fn create_new_user(t: &TestDb) -> User {
    t.write(|tx| {
        User::create(
            tx,
            NewUser {
                name: "User".into(),
                email_address: Some("user@example.com".into()),
                password_digest: Some(PasswordDigest::create("secret123456", 4).unwrap()),
                ..Default::default()
            },
        )
    })
}

#[test]
fn profile_identity_normalizes_counts_unicode_and_clears_to_the_account_name() {
    let t = TestDb::new();
    let original = user(&t, "david");
    let mut changed = original.clone();
    let changed = t.write(move |tx| {
        changed.update(tx, UserChanges {
            pronouns: Some(Some(format!("  {}  ", "é".repeat(40)))),
            nickname: Some(Some(format!("  {}  ", "名".repeat(32)))),
            ..Default::default()
        })?;
        Ok(changed)
    });
    assert_eq!(changed.pronouns.as_deref(), Some("é".repeat(40).as_str()));
    assert_eq!(changed.display_name(), "名".repeat(32));
    assert_eq!(changed.name, original.name);
    assert_eq!(changed, user(&t, "david"));
    let cleared = t.write(move |tx| {
        let mut changed = changed;
        changed.update(tx, UserChanges {
            pronouns: Some(Some("  ".into())), nickname: Some(None), ..Default::default()
        })?;
        Ok(changed)
    });
    assert_eq!(cleared.pronouns, None);
    assert_eq!(cleared.nickname, None);
    assert_eq!(cleared.display_name(), original.name);
}

#[test]
fn profile_identity_rejects_controls_and_long_values_without_partial_writes() {
    let t = TestDb::new();
    let before = user(&t, "david");
    for (pronouns, nickname) in [
        (Some("é".repeat(41)), None), (None, Some("名".repeat(33))),
        (Some("they\nthem".into()), None), (None, Some("\tNick".into())),
        (None, Some("Nick\u{7f}".into())), (Some("\u{85}".into()), None),
    ] {
        let mut changed = before.clone();
        let error = t.try_write(move |tx| changed.update(tx, UserChanges {
            name: Some("should not save".into()),
            pronouns: pronouns.map(Some), nickname: nickname.map(Some), ..Default::default()
        })).unwrap_err();
        assert!(matches!(error, crate::Error::RecordInvalid(_)));
        assert_eq!(user(&t, "david"), before);
    }
}

#[test]
fn profile_identity_creation_uses_the_same_validation() {
    let t = TestDb::new();
    let created = t.write(|tx| User::create(tx, NewUser {
        name: "Account name".into(), pronouns: Some("  she/her  ".into()),
        nickname: Some("  Nick  ".into()), ..Default::default()
    }));
    assert_eq!(created.display_name(), "Nick");
    assert_eq!(created.pronouns.as_deref(), Some("she/her"));
    assert!(matches!(t.try_write(|tx| User::create(tx, NewUser {
        name: "Account name".into(), nickname: Some("bad\nname".into()), ..Default::default()
    })), Err(crate::Error::RecordInvalid(_))));
}

#[test]
fn profile_identity_is_used_in_push_titles_and_mention_picker_queries() {
    let t = TestDb::new();
    let mut author = user(&t, "david");
    let author = t.write(move |tx| {
        author.update(tx, UserChanges {
            nickname: Some(Some("Nickname".into())), ..Default::default()
        })?;
        Ok(author)
    });
    t.read(|conn| {
        let message = Message::by_creator(conn, author.id)?.remove(0);
        let mut room = Room::find(conn, message.room_id)?;
        let text = crate::rich_text::BasicRichText;
        let shared = PushSubscription::payload_for(conn, &text, &room, &message)?;
        assert!(shared.body.starts_with("Nickname: "));
        room.room_type = RoomType::Direct;
        let direct = PushSubscription::payload_for(conn, &text, &room, &message)?;
        assert_eq!(direct.title, "Nickname");
        let (matches, _) = crate::autocomplete_users::page(conn, None, Some("Nickname"), 0, 20)?;
        assert_eq!(matches.iter().map(|user| user.id).collect::<Vec<_>>(), [author.id]);
        assert_eq!(crate::autocomplete_users::count(conn, None, Some("Nickname"))?, 1);
        Ok(())
    });
}

#[test]
fn user_revisions_keep_rails_stamps_for_same_clock_ban_unban_with_stale_snapshots() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let original = user(&t, "david");
    let mut stale_unban = original.clone();
    let mut first_ban = original.clone();
    let banned = t.write(move |tx| {
        first_ban.ban(tx)?;
        Ok(first_ban)
    });
    assert_eq!(banned.status, Status::Banned);
    assert_eq!(banned.updated_at, t.now());

    let active = t.write(move |tx| {
        stale_unban.unban(tx)?;
        Ok(stale_unban)
    });
    assert_eq!(active.status, Status::Active);
    assert_eq!(active.updated_at, banned.updated_at);
    assert_eq!(active, user(&t, "david"));

    let mut stale_ban = banned;
    let banned_again = t.write(move |tx| {
        stale_ban.ban(tx)?;
        Ok(stale_ban)
    });
    assert_eq!(banned_again.status, Status::Banned);
    assert_eq!(banned_again.updated_at, active.updated_at);
    assert_eq!(banned_again, user(&t, "david"));
}

#[test]
fn stale_user_updates_return_the_persisted_state_with_the_new_revision() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let mut first = user(&t, "david");
    let mut stale = first.clone();
    let renamed = t.write(move |tx| {
        first.update(
            tx,
            UserChanges {
                name: Some("Changed name".into()),
                ..Default::default()
            },
        )?;
        Ok(first)
    });
    let updated = t.write(move |tx| {
        stale.update(
            tx,
            UserChanges {
                bio: Some(Some("Changed bio".into())),
                ..Default::default()
            },
        )?;
        Ok(stale)
    });
    assert_eq!(updated.name, "Changed name");
    assert_eq!(updated.bio.as_deref(), Some("Changed bio"));
    assert_eq!(updated.updated_at, renamed.updated_at);
    assert_eq!(updated, user(&t, "david"));
}

#[test]
fn user_does_not_prevent_very_long_passwords() {
    let t = TestDb::new();
    let mut david = user(&t, "david");
    t.write(move |tx| {
        david.update(
            tx,
            UserChanges {
                password_digest: Some(PasswordDigest::create(&"secret".repeat(50), 4).unwrap()),
                ..Default::default()
            },
        )
    });
    assert!(user(&t, "david").authenticate(&"secret".repeat(50)));
}

#[test]
fn creating_users_grants_membership_to_the_open_rooms() {
    let t = TestDb::new();
    let before = t.read(Membership::count);
    let open_rooms = t.read(|c| Room::count_of_type(c, RoomType::Open));
    let user = create_new_user(&t);
    assert_eq!(t.read(Membership::count), before + open_rooms);
    // grant_membership_to_open_rooms leaves involvement to the column default.
    assert!(
        t.read(|c| user.memberships(c))
            .iter()
            .all(|m| m.involved_in(crate::Involvement::Mentions))
    );
}

#[test]
fn creating_subsequent_users_makes_them_members() {
    let t = TestDb::new();
    let user = create_new_user(&t);
    assert!(user.is_member());
    assert!(user.is_active());
    assert!(
        user.password_digest
            .as_deref()
            .unwrap()
            .starts_with("$2a$04$")
    );
}

#[test]
fn deactivating_a_user_deletes_push_subscriptions_searches_memberships_for_non_direct_rooms_and_changes_their_email_address()
 {
    let t = TestDb::new();
    let david = id("david");
    let memberships = t.read(Membership::count);
    let without_directs = t.read(|c| Membership::count_without_direct_rooms(c, david));
    let subscriptions = t.read(PushSubscription::count);
    let davids_subscriptions = t.read(|c| PushSubscription::for_user(c, david)).len() as i64;
    let searches = t.read(Search::count);
    let davids_searches = t.read(|c| Search::count_for_user(c, david));

    let mut user = user(&t, "david");
    t.write(move |tx| user.deactivate(tx));

    assert_eq!(t.read(Membership::count), memberships - without_directs);
    assert_eq!(
        t.read(PushSubscription::count),
        subscriptions - davids_subscriptions
    );
    assert_eq!(t.read(Search::count), searches - davids_searches);

    let reloaded = t.read(|c| User::find(c, david));
    let email = reloaded.email_address.unwrap();
    assert!(
        email.starts_with("david-deactivated-") && email.ends_with("@37signals.com"),
        "{email}"
    );
    assert_eq!(
        email.len(),
        "david-deactivated-2e7de450-cf04-4fa8-9b02-ff5ab2d733e7@37signals.com".len()
    );
    assert_eq!(reloaded.status, Status::Deactivated);
    // David's agent is suspended with him: the single-page app's `agent.status`.
    assert_eq!(
        t.events(),
        vec![
            Event::DisconnectUser {
                user_id: david,
                reconnect: false
            },
            Event::broadcast(&crate::models::agent::AgentSyncChange {
                agent_id: 773018776
            }),
        ]
    );
}

#[test]
fn deactivating_a_user_deletes_their_sessions() {
    let t = TestDb::new();
    assert_eq!(t.read(|c| Session::count_for_user(c, id("david"))), 1);
    let mut david = user(&t, "david");
    t.write(move |tx| david.deactivate(tx));
    assert_eq!(t.read(|c| Session::count_for_user(c, id("david"))), 0);
}

#[test]
fn initials_and_title() {
    let t = TestDb::new();
    let mut jz = user(&t, "jz");
    assert_eq!(jz.initials(), "J");
    assert_eq!(jz.title(), "JZ – Designer");
    assert_eq!(user(&t, "bender").initials(), "BB");
    jz.name = "Émile Zola".into();
    assert_eq!(
        jz.initials(),
        "Z",
        "Ruby's \\b sees É as a word character, \\w doesn't"
    );
    jz.bio = Some("  ".into());
    assert_eq!(jz.title(), "Émile Zola");
}

// User::Bot

/// `User::Bot` "creation writes the digest alone and stored bots hide their key".
#[test]
fn create_bot() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Bender", None));
    let token = bot.plain_bot_token.clone().unwrap();
    assert_eq!(token.len(), 12);
    assert_eq!(bot.bot_key(), format!("{}-{token}", bot.id));
    assert_eq!(bot.role, Role::Bot);
    assert!(bot.password_digest.is_none());

    let stored = t.read(|c| User::find(c, bot.id));
    assert_eq!(stored.bot_token_digest, Some(digest_bot_token(&token)));
    let plaintext: Option<String> = t.read(|c| {
        Ok(c.query_row("SELECT bot_token FROM users WHERE id = ?", [bot.id], |r| r.get(0))?)
    });
    assert_eq!(plaintext, None, "the plaintext column is never written");
    assert_eq!(stored.plain_bot_key(), None);
    assert_eq!(stored.bot_key(), BOT_KEY_PLACEHOLDER);
    assert_eq!(
        t.read(|c| User::authenticate_bot(c, &format!("{}-{token}", bot.id))).map(|u| u.id),
        Some(bot.id)
    );
}

/// `User.digest_bot_token` against vectors from our Rails.
#[test]
fn bot_token_digest_matches_rails() {
    assert_eq!(digest_bot_token("BenderToken1"), BENDER_TOKEN_DIGEST);
    assert_eq!(
        digest_bot_token("5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"),
        "aaf402e02932d4f28f1f1115b44fd22b6a17b658f58402d28db3bfa02b404ae9"
    );
}

/// The bender fixture's key authenticates by its digest alone.
#[test]
fn fixture_bot_authenticates_by_digest() {
    let t = TestDb::new();
    let key = format!("{}-BenderToken1", id("bender"));
    assert_eq!(t.read(|c| User::authenticate_bot(c, &key)).map(|u| u.id), Some(id("bender")));
}

/// A tampered key is refused: every changed character of the token, a truncated or extended
/// token, another bot's id, and a leftover plaintext token ("leftover plaintext is never
/// consulted").
#[test]
fn tampered_bot_keys_are_refused() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Bender", None));
    let token = bot.plain_bot_token.clone().unwrap();
    let key = bot.bot_key();
    assert!(t.read(|c| User::authenticate_bot(c, &key)).is_some());

    let mut tampered = Vec::new();
    for i in 0..token.len() {
        let mut bytes = token.clone().into_bytes();
        bytes[i] = if bytes[i] == b'x' { b'y' } else { b'x' };
        tampered.push(format!("{}-{}", bot.id, String::from_utf8(bytes).unwrap()));
    }
    tampered.push(format!("{}-{}", bot.id, &token[..11]));
    tampered.push(format!("{}-{token}x", bot.id));
    tampered.push(format!("{}-{}", bot.id, token.to_uppercase()));
    tampered.push(format!("{}-{token}", id("bender")));
    tampered.push(format!("{}-{}", bot.id, digest_bot_token(&token)));
    for key in &tampered {
        assert!(t.read(|c| User::authenticate_bot(c, key)).is_none(), "{key}");
    }

    let bot_id = bot.id;
    t.write(move |tx| {
        tx.conn().execute("UPDATE users SET bot_token = 'DecoyToken12' WHERE id = ?", [bot_id])?;
        Ok(())
    });
    assert!(t.read(|c| User::authenticate_bot(c, &format!("{bot_id}-DecoyToken12"))).is_none());
    assert!(t.read(|c| User::authenticate_bot(c, &key)).is_some());
}

/// "a bot without a digest cannot authenticate until its key is reset"
#[test]
fn a_bot_without_a_digest_cannot_authenticate_until_its_key_is_reset() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Legacy", None));
    let bot_id = bot.id;
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET bot_token = 'OldRelease12', bot_token_digest = NULL WHERE id = ?",
            [bot_id],
        )?;
        Ok(())
    });
    assert!(t.read(|c| User::authenticate_bot(c, &format!("{bot_id}-OldRelease12"))).is_none());
    let new_key = t.write(move |tx| User::find(tx.conn(), bot_id)?.reset_bot_key(tx));
    assert_eq!(t.read(|c| User::authenticate_bot(c, &new_key)).map(|u| u.id), Some(bot_id));
}

/// `secure_compare`: equal only for identical bytes of the same length.
#[test]
fn secure_compare_needs_identical_bytes() {
    assert!(secure_compare(b"abc", b"abc"));
    assert!(!secure_compare(b"abc", b"abd"));
    assert!(!secure_compare(b"abc", b"ab"));
    assert!(!secure_compare(b"", b"a"));
    assert!(secure_compare(b"", b""));
}

#[test]
fn create_bot_with_webhook() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Bot", Some("http://x")));
    assert_eq!(t.read(|c| bot.webhook_url(c)).as_deref(), Some("http://x"));

    let mut b = bot.clone();
    t.write(move |tx| {
        b.update_bot(
            tx,
            UserChanges {
                name: Some("Bot2".into()),
                ..Default::default()
            },
            Some(""),
        )
    });
    assert!(t.read(|c| Webhook::find_by_user(c, bot.id)).is_none());
    assert_eq!(t.read(|c| User::find(c, bot.id)).name, "Bot2");
}

/// "reset stores the digest alone, clears leftover plaintext, and retires the old key"
#[test]
fn reset_bot_key() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Bender", None));
    let bot_id = bot.id;
    t.write(move |tx| {
        tx.conn().execute("UPDATE users SET bot_token = 'PreRetire123' WHERE id = ?", [bot_id])?;
        Ok(())
    });
    let first = bot.bot_key();
    let mut b = t.read(|c| User::find(c, bot_id));
    assert_eq!(b.bot_key(), BOT_KEY_PLACEHOLDER);
    let second = t.write(move |tx| {
        let key = b.reset_bot_key(tx)?;
        assert_eq!(b.bot_key(), key);
        Ok(key)
    });
    assert_ne!(first, second);
    let stored = t.read(|c| User::find(c, bot_id));
    let (_, token) = second.split_once('-').unwrap();
    assert_eq!(stored.bot_token_digest, Some(digest_bot_token(token)));
    let plaintext: Option<String> = t.read(|c| {
        Ok(c.query_row("SELECT bot_token FROM users WHERE id = ?", [bot_id], |r| r.get(0))?)
    });
    assert_eq!(plaintext, None);
    assert!(t.read(|c| User::authenticate_bot(c, &first)).is_none());
    assert!(t.read(|c| User::authenticate_bot(c, &second)).is_some());
}

#[test]
fn authenticate_bot() {
    let t = TestDb::new();
    let bot = t.write(|tx| User::create_bot(tx, "Bender", None));
    assert_eq!(
        t.read(|c| User::authenticate_bot(c, &bot.bot_key()))
            .unwrap()
            .id,
        bot.id
    );
    let token = bot.plain_bot_token.clone().unwrap();
    // `split("-", 2)`, then a blank or non-numeric id, or a blank token, finds nothing.
    for key in [
        "nonsense".to_string(),
        String::new(),
        format!("{}-", bot.id),
        format!("{}-   ", bot.id),
        format!("-{token}"),
        format!(" {}-{token}", bot.id),
        format!("{}x-{token}", bot.id),
        format!("+{}-{token}", bot.id),
        format!("99999999999999999999999-{token}"),
    ] {
        assert!(t.read(|c| User::authenticate_bot(c, &key)).is_none(), "{key:?}");
    }
    // `find_by(id: "007")` casts the id.
    assert!(t.read(|c| User::authenticate_bot(c, &format!("00{}-{token}", bot.id))).is_some());
    // A deactivated bot doesn't authenticate.
    let mut deactivated = bot.clone();
    t.write(move |tx| deactivated.deactivate(tx));
    assert!(t.read(|c| User::authenticate_bot(c, &bot.bot_key())).is_none());
}

#[test]
fn deliver_message_by_webhook() {
    let t = TestDb::new();
    let bender = user(&t, "bender");
    t.write(move |tx| bender.deliver_webhook_later(tx, id("first")));
    assert_eq!(
        t.events(),
        vec![Event::DeliverWebhook {
            bot_id: id("bender"),
            message_id: id("first")
        }]
    );

    // No webhook, no job.
    let jz = user(&t, "jz");
    t.write(move |tx| jz.deliver_webhook_later(tx, id("first")));
    assert_eq!(t.events().len(), 1);
}

#[test]
fn webhook_payload() {
    let t = TestDb::new();
    let message = t.read(|c| Message::find(c, id("first")));
    let webhook = t.read(|c| Ok(Webhook::find_by_user(c, id("bender"))?.unwrap()));
    let payload = t.read(|c| {
        webhook.payload(
            c,
            &BasicRichText,
            &message,
            "/rooms/1/bot/key/messages",
            "/rooms/1/@2",
        )
    });
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(
        json["user"],
        serde_json::json!({ "id": id("jason"), "name": "Jason" })
    );
    assert_eq!(
        json["room"],
        serde_json::json!({ "id": id("designers"), "name": "Designers", "path": "/rooms/1/bot/key/messages" })
    );
    assert_eq!(
        json["message"]["body"],
        serde_json::json!({ "html": "First post!", "plain": "First post!" })
    );
    assert!(payload.starts_with(r#"{"user":{"id":"#));
}

// User::Role

#[test]
fn can_administer() {
    let t = TestDb::new();
    let mut admin = user(&t, "david");
    assert!(admin.can_administer(None, false));
    admin.role = Role::Member;
    assert!(!admin.can_administer(None, false));

    let member = user(&t, "kevin");
    assert!(member.can_administer(Some(member.id), false), "creator");
    assert!(member.can_administer(Some(id("jz")), true), "new record");
    let designers = t.read(|c| Room::find(c, id("designers")));
    assert!(!member.can_administer(Some(designers.creator_id), false));
}

// User::Bannable

#[test]
fn ban_creates_bans_from_session_ips_and_removes_sessions() {
    let t = TestDb::new();
    let kevin = id("kevin");
    t.write(move |tx| {
        Session::start(tx, kevin, Some("ua"), Some("8.8.8.8"))?;
        Session::start(tx, kevin, Some("ua"), Some("8.8.8.8"))?;
        Session::start(tx, kevin, Some("ua"), Some(""))?;
        Ok(())
    });
    t.sink.take();

    let mut user = user(&t, "kevin");
    t.write(move |tx| user.ban(tx));

    assert_eq!(
        t.read(|c| Ban::for_user(c, kevin))
            .iter()
            .map(|b| b.ip_address.clone())
            .collect::<Vec<_>>(),
        ["8.8.8.8"]
    );
    assert!(t.read(|c| Ban::banned(c, "8.8.8.8")));
    assert_eq!(t.read(|c| Session::count_for_user(c, kevin)), 0);
    assert_eq!(t.read(|c| User::find(c, kevin)).status, Status::Banned);
    assert_eq!(
        t.events(),
        vec![
            Event::DisconnectUser {
                user_id: kevin,
                reconnect: false
            },
            Event::RemoveBannedContent { user_id: kevin }
        ]
    );

    let mut user = t.read(|c| User::find(c, kevin));
    t.write(move |tx| user.unban(tx));
    assert!(!t.read(|c| Ban::banned(c, "8.8.8.8")));
    assert_eq!(t.read(|c| User::find(c, kevin)).status, Status::Active);
}

#[test]
fn ban_rejects_private_session_ips() {
    let t = TestDb::new();
    let kevin = id("kevin");
    t.write(move |tx| Session::start(tx, kevin, None, Some("192.168.1.1")).map(|_| ()));
    let mut user = user(&t, "kevin");
    assert!(t.try_write(move |tx| user.ban(tx)).is_err());
    assert_eq!(
        t.read(|c| User::find(c, kevin)).status,
        Status::Active,
        "rolled back"
    );
}

#[test]
fn remove_banned_content() {
    let t = TestDb::new();
    let jz = user(&t, "jz");
    let removed = t.write(move |tx| jz.remove_banned_content(tx));
    assert_eq!(removed.len(), 5);
    assert!(t.read(|c| Message::by_creator(c, id("jz"))).is_empty());
}

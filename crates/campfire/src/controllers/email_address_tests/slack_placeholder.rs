use super::super::password_reset_tests::{app_with_smtp, start_mail_runner};
use super::*;
use crate::integrations::slack::{undoer::tests::undo, users};
use campfire_db::models::slack::SlackWorkspace;
use campfire_db::models::slack_import::{Kind, Mode, NewImport, SlackImport};
use campfire_db::{
    Involvement, Membership, Message, NewMessage, NewSession, NewUser, Room, RoomType, Session,
};
use rusqlite::params;
use std::time::Duration;

const SLACK_EMAIL: &str = "slack-owner@example.test";

async fn prior_import(a: &TestApp, status: &'static str) -> (SlackImport, User) {
    let crypto = a.booted.app.ar_encryption.clone();
    a.db()
        .write(move |tx| {
            // Make the frozen fixture's pre-existing orphan valid in this test database.
            tx.conn().execute(
                "UPDATE messages SET creator_id=? WHERE id=935962042
                 AND NOT EXISTS (SELECT 1 FROM users WHERE id=messages.creator_id)",
                [KEVIN],
            )?;
            let workspace = SlackWorkspace::create(tx, &crypto, "client", "secret", Some(KEVIN))?;
            let run = SlackImport::create(
                tx,
                NewImport {
                    workspace_id: workspace.id,
                    connection_id: None,
                    user_id: KEVIN,
                    kind: Kind::Workspace,
                    mode: Mode::Import,
                    options: json!({}),
                },
            )?;
            tx.conn().execute(
                "UPDATE slack_imports SET status=? WHERE id=?",
                params![status, run.id],
            )?;
            let placeholder = User::create_slack_placeholder(
                tx,
                NewUser {
                    name: "Slack Owner".into(),
                    email_address: Some(SLACK_EMAIL.into()),
                    ..Default::default()
                },
                true,
                None,
                false,
            )?;
            users::record(tx, &run, "user", "UJAY", "User", placeholder.id, true)?;
            Ok((run, placeholder))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn confirmed_slack_placeholder_moves_history_and_undo_preserves_the_account() {
    let (a, _, mut mail, smtp) = app_with_smtp().await;
    let (run, placeholder) = prior_import(&a, "completed").await;
    let placeholder = placeholder.id;
    let run_id = run.id;
    let (open, closed, dm, other_dm, existing, message, boost, pin, thread) = a.db().write(move |tx| {
        let open = Room::create_for(tx, RoomType::Open, Some("Imported open"), KEVIN, &[DAVID, placeholder])?;
        let closed = Room::create_for(tx, RoomType::Closed, Some("Imported closed"), KEVIN, &[placeholder])?;
        let dm = Room::find_or_create_direct_for(tx, &[placeholder, KEVIN], KEVIN)?;
        let other_dm = Room::find_or_create_direct_for(tx, &[placeholder, KEVIN + 1], KEVIN)?;
        let existing = Room::find_or_create_direct_for(tx, &[DAVID, KEVIN], DAVID)?;
        let mut david = Membership::find_by_room_and_user(tx.conn(), open.id, DAVID)?.unwrap();
        david.update_involvement(tx, Involvement::Invisible)?;
        let mut member = Membership::find_by_room_and_user(tx.conn(), open.id, placeholder)?.unwrap();
        member.update_involvement(tx, Involvement::Everything)?;
        users::record(tx, &run, "membership", &format!("COPEN:user-{placeholder}"), "Membership", member.id, true)?;
        let member = Membership::find_by_room_and_user(tx.conn(), closed.id, placeholder)?.unwrap();
        users::record(tx, &run, "membership", &format!("CCLOSED:user-{placeholder}"), "Membership", member.id, true)?;
        users::record(tx, &run, "conversation", "DPEER", "Room", dm.id, true)?;

        let message = Message::create_imported(
            tx,
            NewMessage {
                room_id: dm.id,
                creator_id: placeholder,
                markdown_source: Some("historical mention @[Slack Owner]".into()),
                ..Default::default()
            },
            tx.now(), None,
        )?;
        users::record(tx, &run, "message", "DPEER:1", "Message", message.id, true)?;
        let boost = tx.conn().query_row(
            "INSERT INTO boosts (message_id,booster_id,content,created_at,updated_at) VALUES (?,?,'star',?,?) RETURNING id",
            params![message.id,placeholder,tx.now(),tx.now()], |r|r.get::<_,i64>(0)
        )?;
        users::record(tx, &run, "reaction", "DPEER:1:star:UJAY", "Boost", boost, true)?;
        let duplicate_boost = tx.conn().query_row(
            "INSERT INTO boosts (message_id,booster_id,content,created_at,updated_at) VALUES (?,?,'heart',?,?), (?,?,'heart',?,?) RETURNING id",
            params![message.id,placeholder,tx.now(),tx.now(),message.id,DAVID,tx.now(),tx.now()],
            |r|r.get::<_,i64>(0)
        )?;
        users::record(tx, &run, "reaction", "DPEER:1:heart:UJAY", "Boost", duplicate_boost, true)?;
        let pin = tx.conn().query_row(
            "INSERT INTO message_pins (message_id,room_id,pinner_id,created_at,updated_at) VALUES (?,?,?,?,?) RETURNING id",
            params![message.id,dm.id,placeholder,tx.now(),tx.now()], |r|r.get::<_,i64>(0)
        )?;
        users::record(tx, &run, "pin", "DPEER:1", "MessagePin", pin, true)?;
        let thread = tx.conn().query_row(
            "INSERT INTO channel_threads (room_id,creator_id,parent_message_id,created_at,updated_at,last_activity_at,name) VALUES (?,?,?,?,?,?,'Imported thread') RETURNING id",
            params![dm.id,placeholder,message.id,tx.now(),tx.now(),tx.now()], |r|r.get::<_,i64>(0)
        )?;
        users::record(tx, &run, "thread", "DPEER:1", "ChannelThread", thread, true)?;
        tx.conn().execute(
            "INSERT INTO thread_access_memberships (thread_id,user_id,added_by_id,created_at)
             VALUES (?1,?2,?2,?4), (?1,?3,?2,?4)",
            params![thread,placeholder,DAVID,tx.now()],
        )?;
        tx.conn().execute(
            "INSERT INTO user_blocks (blocker_id,blocked_id,created_at) VALUES (?,?,?)",
            params![DAVID,placeholder,tx.now()],
        )?;
        Ok((open.id, closed.id, dm.id, other_dm.id, existing.id, message.id, boost, pin, thread))
    }).await.unwrap();

    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let reply = owner.write(add(SLACK_EMAIL)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        reply.json(),
        json!({"kind":"confirmationSent","emailAddress":SLACK_EMAIL})
    );
    let runner = start_mail_runner(&a);
    let raw = tokio::time::timeout(Duration::from_secs(5), mail.recv())
        .await
        .expect("confirmation delivered over loopback SMTP")
        .expect("SMTP capture is running");
    let delivery = campfire_mail::parse::Email::parse(raw.as_bytes()).unwrap();
    assert_eq!(delivery.recipients, [SLACK_EMAIL]);
    let token = delivery
        .body
        .lines()
        .find_map(|line| line.strip_prefix("http://campfire.test/app/email_addresses/confirm/"))
        .expect("confirmation link");
    runner.shutdown(Duration::from_secs(1)).await;
    smtp.abort();
    assert!(smtp.await.unwrap_err().is_cancelled());

    let reply = owner.write(confirmation(Method::PUT, token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let adapter = a.db().env().rich_text.clone();
    a.db().read(move |conn| {
        assert!(User::find_by_id(conn, placeholder)?.is_none());
        assert_eq!(EmailAddress::by_address(conn, SLACK_EMAIL)?.unwrap().0, DAVID);
        assert_eq!(
            Membership::find_by_room_and_user(conn, open, DAVID)?.unwrap().involvement,
            Some(Involvement::Everything)
        );
        let closed_member = Membership::find_by_room_and_user(conn, closed, DAVID)?.unwrap().id;
        let workspace = SlackWorkspace::current(conn)?.unwrap().id;
        assert_eq!(users::mapped_id(conn, workspace, "membership", &format!("CCLOSED:user-{DAVID}"))?, Some(closed_member));
        assert!(Room::find_by_id(conn, dm)?.is_none());
        assert_eq!(Room::find_direct_for(conn, &[DAVID, KEVIN + 1])?.unwrap().id, other_dm);
        for (table, id, author) in [
            ("messages", message, "creator_id"), ("boosts", boost, "booster_id"),
            ("message_pins", pin, "pinner_id"), ("channel_threads", thread, "creator_id")
        ] {
            assert_eq!(conn.query_row(&format!("SELECT {author} FROM {table} WHERE id=?"), [id], |r|r.get::<_,i64>(0))?, DAVID);
            if table != "boosts" {
                assert_eq!(conn.query_row(&format!("SELECT room_id FROM {table} WHERE id=?"), [id], |r|r.get::<_,i64>(0))?, existing);
            }
        }
        assert_eq!(users::mapped_id(conn, workspace, "reaction", "DPEER:1:heart:UJAY")?, None);
        assert_eq!(conn.query_row("SELECT count(*) FROM boosts WHERE message_id=? AND booster_id=? AND content='heart'", [message,DAVID], |r|r.get::<_,i64>(0))?, 1);
        assert_eq!(conn.query_row("SELECT added_by_id FROM thread_access_memberships WHERE thread_id=? AND user_id=?", [thread,DAVID], |r|r.get::<_,i64>(0))?, DAVID);
        let body = Message::find(conn, message)?.body(conn)?.unwrap();
        assert_eq!(adapter.try_mentioned_user_ids(conn, body.body.as_deref().unwrap()).unwrap(), [DAVID]);
        assert_eq!(users::mapped_id(conn, workspace, "conversation", "DPEER")?, Some(existing));
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM slack_import_records WHERE slack_import_id=? AND slack_kind IN ('user','conversation') AND created_record=0",[run_id],|r|r.get::<_,i64>(0))?,2);
        assert_eq!(conn.query_row("SELECT count(*) FROM audit_logs WHERE action='user.slack_placeholder.claim' AND target_id=?", [DAVID], |r|r.get::<_,i64>(0))?, 1);
        assert!(conn.prepare("PRAGMA foreign_key_check")?.query([])?.next()?.is_none());
        Ok(())
    }).await.unwrap();

    undo(a.db(), run_id).await;
    a.db()
        .read(move |conn| {
            assert!(User::find_by_id(conn, DAVID)?.is_some());
            assert!(Room::find_by_id(conn, existing)?.is_some());
            assert!(Membership::find_by_room_and_user(conn, open, DAVID)?.is_some());
            assert!(
                conn.prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slack_placeholder_confirmation_waits_for_imports_and_enforces_limit() {
    let a = app(MAIL).await.without_job_runner().await;
    let (run, placeholder) = prior_import(&a, "running").await;
    let token = token(&a, DAVID, SLACK_EMAIL, SignedDuration::ZERO).await;
    let mut browser = a.anonymous();
    let reply = browser.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(
        reply.json()["fieldErrors"]["base"],
        json!(["A Slack import is in progress. Wait for it to finish, then confirm again."])
    );
    a.db()
        .write(move |tx| {
            assert!(User::find_by_id(tx.conn(), placeholder.id)?.is_some());
            tx.conn().execute(
                "UPDATE slack_imports SET status='completed' WHERE id=?",
                [run.id],
            )?;
            let owner = User::find(tx.conn(), DAVID)?;
            for index in 0..10 {
                assert_eq!(
                    EmailAddress::confirm(tx, &owner, &format!("extra-{index}@example.test"))?,
                    Confirmation::Added
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    let reply = browser.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(
        reply.json()["fieldErrors"]["base"],
        json!(["You can link up to 10 additional email addresses. Remove one and try again."])
    );
    let placeholder_id = placeholder.id;
    a.db()
        .write(move |tx| {
            assert!(User::find_by_id(tx.conn(), placeholder_id)?.is_some());
            let owned = EmailAddress::for_user(tx.conn(), DAVID)?;
            EmailAddress::remove(tx, DAVID, owned[0].id)?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = browser.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
}

#[tokio::test]
async fn confirmation_never_claims_a_slack_user_with_account_activity() {
    for condition in ["session", "google", "deactivated", "real"] {
        let a = app(MAIL).await.without_job_runner().await;
        let (run, placeholder) = prior_import(&a, "completed").await;
        a.db().write(move |tx| {
            let id = placeholder.id;
            match condition {
                "session" => { Session::start_with(tx, id, NewSession::default())?; }
                "google" => { tx.conn().execute(
                    "INSERT INTO google_identities (user_id,subject,email,created_at,updated_at) VALUES (?,'subject',?,?,?)",
                    params![id,SLACK_EMAIL,tx.now(),tx.now()]
                )?; }
                "deactivated" => { tx.conn().execute("UPDATE users SET status=1 WHERE id=?", [id])?; }
                "real" => { tx.conn().execute("DELETE FROM slack_import_records WHERE slack_import_id=?", [run.id])?; }
                _ => unreachable!(),
            }
            Ok(())
        }).await.unwrap();
        assert!(
            confirmation_message(&a, DAVID, SLACK_EMAIL, a.db().env().now())
                .await
                .is_none(),
            "{condition}"
        );
        let token = token(&a, DAVID, SLACK_EMAIL, SignedDuration::ZERO).await;
        let reply = a.anonymous().write(confirmation(Method::PUT, &token)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{condition}: {}",
            reply.text()
        );
        assert_eq!(
            reply.json()["fieldErrors"]["emailAddress"],
            json!(["This email address is already linked to another account."])
        );
    }
}

#[tokio::test]
async fn claiming_a_slack_placeholder_keeps_note_keys_private_and_undoable() {
    let a = app(MAIL).await.without_job_runner().await;
    let (run, placeholder) = prior_import(&a, "completed").await;
    let run_id = run.id;
    let workspace = run.slack_workspace_id;
    let (source, notes, moved, original) = a
        .db()
        .write(move |tx| {
            let source = Room::find_or_create_direct_for(tx, &[DAVID, placeholder.id], DAVID)?;
            let notes = Room::find_or_create_direct_for(tx, &[DAVID], DAVID)?;
            let existing_run = SlackImport::create(
                tx,
                NewImport {
                    workspace_id: workspace,
                    connection_id: None,
                    user_id: DAVID,
                    kind: Kind::Personal,
                    mode: Mode::Import,
                    options: json!({}),
                },
            )?;
            tx.conn().execute(
                "UPDATE slack_imports SET status='completed' WHERE id=?",
                [existing_run.id],
            )?;
            let old_key = format!("notes:{}:DSELF", placeholder.id);
            let key = format!("notes:{DAVID}:DSELF");
            let mut messages = Vec::new();
            for (room, author, key, import, created_room) in [
                (source.id, placeholder.id, old_key, &run, true),
                (notes.id, DAVID, key, &existing_run, false),
            ] {
                let message = Message::create_imported(
                    tx,
                    NewMessage {
                        room_id: room,
                        creator_id: author,
                        markdown_source: Some("saved Slack note".into()),
                        ..Default::default()
                    },
                    tx.now(),
                    None,
                )?;
                users::record(tx, import, "conversation", &key, "Room", room, created_room)?;
                users::record(
                    tx,
                    import,
                    "message",
                    &format!("{key}:1"),
                    "Message",
                    message.id,
                    true,
                )?;
                messages.push(message.id);
            }
            Ok((source.id, notes.id, messages[0], messages[1]))
        })
        .await
        .unwrap();
    let token = token(&a, DAVID, SLACK_EMAIL, SignedDuration::ZERO).await;
    let reply = a.anonymous().write(confirmation(Method::PUT, &token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    a.db().read(move |conn| {
        assert!(Room::find_by_id(conn, source)?.is_none());
        assert_eq!(Room::note_to_self_for(conn, DAVID)?.unwrap().id, notes);
        assert_eq!(Message::find(conn, moved)?.room_id, notes);
        let notes_key = format!("notes:{DAVID}:DSELF");
        assert_eq!(users::mapped_id(conn, workspace, "conversation", &notes_key)?, Some(notes));
        assert_eq!(users::mapped_id(conn, workspace, "message", &format!("{notes_key}:1"))?, Some(original));
        let (key, owned): (String, bool) = conn.query_row(
            "SELECT slack_key, created_record FROM slack_import_records WHERE slack_kind='message' AND record_id=?",
            [moved], |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert!(key.starts_with(&notes_key) && owned);
        assert!(conn.prepare("PRAGMA foreign_key_check")?.query([])?.next()?.is_none());
        Ok(())
    }).await.unwrap();

    undo(a.db(), run_id).await;
    a.db()
        .read(move |conn| {
            assert!(Message::find_by_id(conn, moved)?.is_none());
            assert_eq!(Message::find(conn, original)?.room_id, notes);
            assert_eq!(Room::note_to_self_for(conn, DAVID)?.unwrap().id, notes);
            Ok(())
        })
        .await
        .unwrap();
}

async fn confirm_claim(a: &TestApp) {
    let token = token(a, DAVID, SLACK_EMAIL, SignedDuration::ZERO).await;
    let reply = a.anonymous().write(confirmation(Method::PUT, &token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
}

#[tokio::test]
async fn slack_placeholder_first_notes_room_survives_import_undo() {
    let a = app(MAIL).await.without_job_runner().await;
    let (run, placeholder) = prior_import(&a, "completed").await;
    let run_id = run.id;
    let (notes, message, member) = a
        .db()
        .write(move |tx| {
            assert!(Room::note_to_self_for(tx.conn(), DAVID)?.is_none());
            let room = Room::find_or_create_direct_for(tx, &[DAVID, placeholder.id], DAVID)?;
            let message = Message::create_imported(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: placeholder.id,
                    markdown_source: Some("imported note".into()),
                    ..Default::default()
                },
                tx.now(),
                None,
            )?;
            users::record(tx, &run, "conversation", "DFIRST", "Room", room.id, true)?;
            users::record(tx, &run, "message", "DFIRST:1", "Message", message.id, true)?;
            let memberships = room.memberships(tx.conn())?;
            for member in &memberships {
                users::record(
                    tx,
                    &run,
                    "membership",
                    &format!("DFIRST:user-{}", member.user_id),
                    "Membership",
                    member.id,
                    true,
                )?;
            }
            let survivor = memberships.iter().find(|m| m.user_id == DAVID).unwrap().id;
            Ok((room.id, message.id, survivor))
        })
        .await
        .unwrap();

    confirm_claim(&a).await;
    a.db()
        .read(move |conn| {
            assert_eq!(Room::note_to_self_for(conn, DAVID)?.unwrap().id, notes);
            assert_eq!(Message::find(conn, message)?.room_id, notes);
            Ok(())
        })
        .await
        .unwrap();

    undo(a.db(), run_id).await;
    a.db()
        .read(move |conn| {
            assert!(Message::find_by_id(conn, message)?.is_none());
            assert_eq!(Room::note_to_self_for(conn, DAVID)?.unwrap().id, notes);
            assert_eq!(
                Membership::find_by_room_and_user(conn, notes, DAVID)?
                    .unwrap()
                    .id,
                member
            );
            assert!(
                conn.prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slack_placeholder_claim_preserves_both_named_group_dms() {
    use crate::controllers::presenters::test_support::JASON;

    const JZ: i64 = 773523953;
    let a = app(MAIL).await.without_job_runner().await;
    let (_, placeholder) = prior_import(&a, "completed").await;
    let groups = a
        .db()
        .write(move |tx| {
            let mut groups = Vec::new();
            for (name, member, peer, already_suffixed) in [
                ("Planning", placeholder.id, JZ, false),
                ("Launch", DAVID, JZ, false),
                ("Planning again", placeholder.id, JASON, false),
                ("Launch again", DAVID, JASON, true),
            ] {
                let members = [member, KEVIN, peer];
                let room = Room::create_for(tx, RoomType::Direct, Some(name), KEVIN, &members)?;
                let key = Room::direct_member_key_for(&members);
                assert_eq!(room.direct_member_key.as_deref(), Some(key.as_str()));
                if already_suffixed {
                    tx.conn().execute(
                        "UPDATE rooms SET direct_member_key=? WHERE id=?",
                        params![format!("{key}#{}", room.id), room.id],
                    )?;
                }
                let message = Message::create_markdown(
                    tx,
                    NewMessage {
                        room_id: room.id,
                        creator_id: member,
                        ..Default::default()
                    },
                    &format!("{name} history"),
                )?;
                groups.push((
                    room.id,
                    message.id,
                    name,
                    peer,
                    already_suffixed || member == placeholder.id,
                ));
            }
            Ok(groups)
        })
        .await
        .unwrap();

    confirm_claim(&a).await;
    a.db()
        .read(move |conn| {
            for (id, message, name, peer, suffixed) in groups {
                let mut members = [DAVID, KEVIN, peer];
                members.sort_unstable();
                let canonical = Room::direct_member_key_for(&members);
                let room = Room::find(conn, id)?;
                assert_eq!(room.name.as_deref(), Some(name));
                let mut actual = room.user_ids(conn)?;
                actual.sort_unstable();
                assert_eq!(actual, members);
                let expected = if suffixed {
                    format!("{canonical}#{id}")
                } else {
                    canonical
                };
                assert_eq!(room.direct_member_key.as_deref(), Some(expected.as_str()));
                let message = Message::find(conn, message)?;
                assert_eq!(message.room_id, id);
                assert_eq!(message.creator_id, DAVID);
                assert_eq!(
                    message.markdown_source.as_deref(),
                    Some(format!("{name} history").as_str())
                );
            }
            assert!(
                conn.prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slack_placeholder_claim_keeps_native_rooms_hidden() {
    let a = app(MAIL).await.without_job_runner().await;
    let native = a
        .db()
        .write(|tx| {
            let room = Room::create_for(tx, RoomType::Open, Some("Native open"), KEVIN, &[DAVID])?;
            let mut member = Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?.unwrap();
            member.update_involvement(tx, Involvement::Invisible)?;
            Ok(room.id)
        })
        .await
        .unwrap();
    let (run, placeholder) = prior_import(&a, "completed").await;
    let imported = a
        .db()
        .write(move |tx| {
            let conn = tx.conn();
            assert_eq!(
                Membership::find_by_room_and_user(conn, native, placeholder.id)?
                    .unwrap()
                    .involvement,
                Some(Involvement::Mentions)
            );
            let room = Room::create_for(
                tx,
                RoomType::Open,
                Some("Imported open"),
                KEVIN,
                &[DAVID, placeholder.id],
            )?;
            let mut member = Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?.unwrap();
            member.update_involvement(tx, Involvement::Invisible)?;
            let mut source =
                Membership::find_by_room_and_user(tx.conn(), room.id, placeholder.id)?.unwrap();
            source.update_involvement(tx, Involvement::Everything)?;
            users::record(
                tx,
                &run,
                "membership",
                &format!("CIMPORTED:user-{}", placeholder.id),
                "Membership",
                source.id,
                true,
            )?;
            Ok(room.id)
        })
        .await
        .unwrap();

    confirm_claim(&a).await;
    a.db()
        .read(move |conn| {
            for (room, involvement) in [
                (native, Involvement::Invisible),
                (imported, Involvement::Everything),
            ] {
                assert_eq!(
                    Membership::find_by_room_and_user(conn, room, DAVID)?
                        .unwrap()
                        .involvement,
                    Some(involvement)
                );
            }
            assert!(
                conn.prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slack_placeholder_claim_preserves_mention_activity_and_existing_state() {
    use campfire_db::ActivityItem;

    let a = app(MAIL).await.without_job_runner().await;
    let (_, placeholder) = prior_import(&a, "completed").await;
    let (transferred, duplicate, own, revision) = a
        .db()
        .write(move |tx| {
            let room = Room::create_for(
                tx,
                RoomType::Open,
                Some("Mentions"),
                KEVIN,
                &[DAVID, KEVIN, placeholder.id],
            )?;
            let mention = campfire_richtext::markdown::user_mention_token;
            let message = Message::create_markdown(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: KEVIN,
                    ..Default::default()
                },
                &format!("hi {}", mention(placeholder.id)),
            )?;
            let source = ActivityItem::find_by_user_and_source(
                tx.conn(),
                placeholder.id,
                "Message",
                message.id,
            )?
            .unwrap();
            assert_eq!(source.event_type, "mention");
            assert!(source.unread());
            let own = Message::create_markdown(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: DAVID,
                    ..Default::default()
                },
                &format!("hi {}", mention(placeholder.id)),
            )?;
            assert!(
                ActivityItem::find_by_user_and_source(
                    tx.conn(),
                    placeholder.id,
                    "Message",
                    own.id
                )?
                .is_some()
            );
            let reply = Message::create_markdown(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: KEVIN,
                    reply_to_message_id: Some(own.id),
                    ..Default::default()
                },
                &format!("hi {}", mention(placeholder.id)),
            )?;
            let mentioned = ActivityItem::find_by_user_and_source(
                tx.conn(),
                placeholder.id,
                "Message",
                reply.id,
            )?
            .unwrap();
            assert_eq!(mentioned.event_type, "mention");
            let state =
                ActivityItem::find_by_user_and_source(tx.conn(), DAVID, "Message", reply.id)?
                    .unwrap();
            assert_eq!(state.event_type, "reply");
            let state = state.mark_read(tx)?.mark_handled(tx)?;
            let revision = tx.conn().query_row(
                "SELECT activity_revision FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, i64>(0),
            )?;
            Ok((source, state, own.id, revision))
        })
        .await
        .unwrap();

    confirm_claim(&a).await;
    a.db()
        .read(move |conn| {
            let item = ActivityItem::find_by_user_and_source(
                conn,
                DAVID,
                "Message",
                transferred.source_id,
            )?
            .unwrap();
            assert_eq!(item.id, transferred.id);
            assert_eq!(item.event_type, "mention");
            assert!(item.unread());
            let existing =
                ActivityItem::find_by_user_and_source(conn, DAVID, "Message", duplicate.source_id)?
                    .unwrap();
            assert_eq!(
                (existing.id, existing.read_at, existing.handled_at),
                (duplicate.id, duplicate.read_at, duplicate.handled_at)
            );
            assert_eq!(existing.event_type, "reply");
            assert!(ActivityItem::find_by_user_and_source(conn, DAVID, "Message", own)?.is_none());
            let after = conn.query_row(
                "SELECT activity_revision FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, i64>(0),
            )?;
            assert!(after > revision);
            assert!(
                conn.prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn slack_placeholder_claim_preserves_markdown_mentions_on_edit() {
    use campfire_db::models::message::MessageChanges;
    use campfire_richtext::markdown::{mention_token, user_mention_token};

    let a = app(MAIL).await.without_job_runner().await;
    let (_, placeholder) = prior_import(&a, "completed").await;
    let (message, expected) = a.db().write(move |tx| {
        assert_ne!(placeholder.display_name(), User::find(tx.conn(), DAVID)?.display_name());
        let room = Room::create_for(tx, RoomType::Open, Some("Editable mentions"), KEVIN, &[DAVID, KEVIN, placeholder.id])?;
        let old_name = mention_token(placeholder.display_name()).unwrap();
        let old_id = user_mention_token(placeholder.id);
        let preserved = format!("`{old_id} {old_name}`\n\n```text\n{old_id} {old_name}\n```\n\n[{old_name}](/) and \\{old_name}");
        let source = format!("hi {old_name} and {old_id}\n\n{preserved}");
        let message = Message::create_markdown(tx, NewMessage {
            room_id: room.id, creator_id: KEVIN, ..Default::default()
        }, &source)?;
        let body = message.body(tx.conn())?.unwrap().body.unwrap();
        assert_eq!(tx.rich_text().try_mentioned_user_ids(tx.conn(), &body).unwrap(), [placeholder.id]);
        let new_id = user_mention_token(DAVID);
        Ok((message.id, format!("hi {new_id} and {new_id}\n\n{preserved}")))
    }).await.unwrap();

    confirm_claim(&a).await;
    a.db()
        .write(move |tx| {
            let mut message = Message::find(tx.conn(), message)?;
            let source = message.markdown_source.as_deref().unwrap();
            assert_eq!(source, expected);
            message.update(
                tx,
                MessageChanges {
                    markdown_source: Some(format!("{source}\n\nAn unrelated edit.")),
                    ..Default::default()
                },
            )?;
            let body = message.body(tx.conn())?.unwrap().body.unwrap();
            assert_eq!(
                tx.rich_text()
                    .try_mentioned_user_ids(tx.conn(), &body)
                    .unwrap(),
                [DAVID]
            );
            assert!(
                tx.conn()
                    .prepare("PRAGMA foreign_key_check")?
                    .query([])?
                    .next()?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

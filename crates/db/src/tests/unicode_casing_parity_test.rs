//! Ruby 3.4.10 expectations recorded by reference-tools/unicode_casing_parity.rb.
use super::*;
use crate::models::{
    audit_log, direct_room, huddle_grant::HuddleGrant, room_files, search_query::SearchQuery,
    user::profile_settings,
};
use crate::{
    Agent, AgentApproval, Blob, CalendarEvent, ChannelThread, Membership, NewAgent, NewApproval,
    NewCalendarEvent, NewChannelThread, NewMessage, Room, RoomType, Session, User,
};
use rusqlite::params;
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/unicode_casing_parity.json"
    ))
    .unwrap()
}

#[test]
fn unicode_parity_search_from_and_room_filters_use_ruby_downcase() {
    let t = TestDb::new();
    let message = t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET name='οσ' WHERE id=?", [id("david")])?;
        let room = Room::create_for(
            tx,
            RoomType::Closed,
            Some("οσ"),
            id("david"),
            &[id("david")],
        )?;
        crate::Message::create(
            tx,
            NewMessage {
                room_id: room.id,
                creator_id: id("david"),
                body: Some("Hello".into()),
                ..Default::default()
            },
        )
    });
    for query in ["in:ΟΣ", "from:ΟΣ", "from:ΟΣ in:ΟΣ"] {
        let rows = t.read(|c| SearchQuery::parse(query).messages_in_room(c, message.room_id));
        assert_eq!(
            rows.iter().map(|m| m.id).collect::<Vec<_>>(),
            [message.id],
            "{query}"
        );
    }
}

#[test]
fn unicode_parity_search_thread_names_and_event_text_use_ruby_downcase() {
    let t = TestDb::new();
    let (board, work, event) = t.write(|tx| {
        let room = Room::create_for(tx, RoomType::Board, Some("οσ"), id("david"), &[id("david")])?;
        let board = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("david"),
                name: Some("οσ".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        tx.conn()
            .execute("UPDATE rooms SET name='οσ' WHERE id=?", [id("designers")])?;
        let work = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("david"),
                name: Some("οσ".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        let event = CalendarEvent::create(
            tx,
            NewCalendarEvent {
                room_id: id("designers"),
                organizer_id: id("david"),
                title: "Unrelated title".into(),
                description: Some("οσ".into()),
                starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(1))),
                time_zone: "UTC".into(),
                ..Default::default()
            },
        )?;
        Ok((board.id, work.id, event.id))
    });
    for query in ["ΟΣ", "ΟΣ in:ΟΣ"] {
        let sections = t.read(|c| SearchQuery::parse(query).sections_for_user(c, id("david")));
        for (kind, expected) in [
            ("board-posts", board),
            ("work-threads", work),
            ("events", event),
        ] {
            let section = sections.iter().find(|s| s.kind == kind).unwrap();
            assert_eq!(
                section.records.iter().map(|r| r.id).collect::<Vec<_>>(),
                [expected],
                "{query} {kind}"
            );
        }
    }
}

#[test]
fn unicode_parity_filename_search_preserves_sql_like_escaping() {
    let t = TestDb::new();
    let (room, blob) = t.write(|tx| {
        let blob = Blob::create(
            tx,
            &Blob {
                id: 0,
                key: "unicode-filename-fixture".into(),
                filename: "οσ_%\\.txt".into(),
                content_type: Some("text/plain".into()),
                metadata: None,
                service_name: "local".into(),
                byte_size: 1,
                checksum: None,
                created_at: tx.now(),
            },
        )?;
        let message = crate::Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                attachment_blob_id: Some(blob.id),
                ..Default::default()
            },
        )?;
        Ok((message.room_id, blob.id))
    });
    for filename in ["ΟΣ", "ΟΣ_%\\"] {
        let rows = t.read(|c| room_files::uploads(c, room, "all", filename, 10));
        assert_eq!(rows.iter().map(|r| r.blob_id).collect::<Vec<_>>(), [blob]);
    }
    assert!(
        t.read(|c| room_files::uploads(c, room, "all", "ΟΣX", 10))
            .is_empty()
    );
}

#[test]
fn unicode_parity_failed_sign_in_deduplicates_ruby_downcased_labels() {
    let t = TestDb::new();
    t.write(|tx| {
        let ctx = audit_log::Context {
            ip_address: Some("192.0.2.73".into()),
            ..Default::default()
        };
        assert!(
            audit_log::AuditLog::record_sign_in_failure(tx, "οσ@example.com", "password", &ctx)?
                .is_some()
        );
        assert!(
            audit_log::AuditLog::record_sign_in_failure(tx, "ΟΣ@example.com", "password", &ctx)?
                .is_none()
        );
        Ok(())
    });
}

#[test]
fn unicode_parity_failed_sign_in_account_label_uses_ruby_downcase() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET email_address='οσ@local' WHERE id=?",
            [id("david")],
        )?;
        assert!(!audit_log::failure_email_shape("ΟΣ@local"));
        assert_eq!(
            audit_log::failure_actor_label(tx.conn(), Some(" ΟΣ@local "))?,
            "ΟΣ@local"
        );
        Ok(())
    });
}

#[test]
fn unicode_parity_preloaded_direct_names_use_ruby_sort_order() {
    let t = TestDb::new();
    let mut members = t.read(|c| {
        Ok(vec![
            User::find(c, id("david"))?,
            User::find(c, id("jason"))?,
        ])
    });
    members[0].name = "ΟΣ".into();
    members[1].name = "οςa".into();
    let expected = oracle()["sigma_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        direct_room::display_name(None, None, &members).unwrap(),
        expected
    );
}

#[test]
fn unicode_parity_huddle_participants_use_ruby_sort_order() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        let room = Room::create_for(tx, RoomType::Closed, Some("Casing"), id("david"), &[id("david"), id("jason")])?;
        for (user, name) in [(id("david"), "ΟΣ"), (id("jason"), "οςa")] {
            tx.conn().execute("UPDATE users SET name=? WHERE id=?", params![name, user])?;
            let member = Membership::find_by_room_and_user(tx.conn(), room.id, user)?.unwrap();
            let session = Session::start(tx, user, None, None)?;
            tx.conn().execute("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,last_seen_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)", params![format!("unicode-{user}"), "casing", session.id, user, member.id, room.id, tx.now(), tx.now(), tx.now()])?;
        }
        Ok(room.id)
    });
    let users = t.read(|c| HuddleGrant::participants_for(c, room, t.now()));
    assert_eq!(
        serde_json::json!(users.iter().map(|u| &u.name).collect::<Vec<_>>()),
        oracle()["sigma_names"]
    );
}

#[test]
fn unicode_parity_agent_directory_uses_ruby_sort_order() {
    let t = TestDb::new();
    let ids = t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET name='ΟΣ' WHERE id=?", [id("bender")])?;
        tx.conn().execute(
            "UPDATE users SET name='οςa',role=2 WHERE id=?",
            [id("jason")],
        )?;
        let other = Agent::create(
            tx,
            NewAgent {
                user_id: id("jason"),
                owner_id: Some(id("david")),
                ..Default::default()
            },
        )?;
        Ok([id("bender_agent"), other.id])
    });
    let names = t.read(|c| {
        Agent::for_directory(c)?
            .into_iter()
            .filter(|a| ids.contains(&a.id))
            .map(|a| User::find(c, a.user_id).map(|u| u.name))
            .collect::<Result<Vec<_>>>()
    });
    assert_eq!(serde_json::json!(names), oracle()["sigma_names"]);
}

#[test]
fn unicode_parity_manual_github_login_uses_downcase_without_folding() {
    let t = TestDb::new();
    for case in oracle()["casing"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap().to_owned();
        let stored = t.write(move |tx| {
            profile_settings::update(
                tx,
                id("david"),
                profile_settings::Changes {
                    github_login: Some(format!(" {input} ")),
                    ..Default::default()
                },
            )?;
            Ok(tx.conn().query_row(
                "SELECT github_login FROM users WHERE id=?",
                [id("david")],
                |r| r.get::<_, String>(0),
            )?)
        });
        assert_eq!(stored, case["downcase"].as_str().unwrap());
    }
}

#[test]
fn unicode_parity_github_approval_identity_uses_full_fold_without_normalization() {
    let t = TestDb::new();
    let mut approval = t.write(|tx| {
        AgentApproval::create(
            tx,
            NewApproval {
                agent_id: id("bender_agent"),
                action: "deploy".into(),
                summary: "Casing".into(),
                ..Default::default()
            },
        )
    });
    approval.github_account_id = Some(73);
    for case in oracle()["comparisons"].as_array().unwrap() {
        approval.github_login = Some(case["left"].as_str().unwrap().into());
        assert_eq!(
            approval.github_identity_matches(73, case["right"].as_str().unwrap()),
            case["equal"].as_bool().unwrap(),
            "{case}"
        );
        assert!(!approval.github_identity_matches(74, case["right"].as_str().unwrap()));
    }
    for blank in [None, Some(" \t".into())] {
        approval.github_login = blank;
        assert!(!approval.github_identity_matches(73, " \t"));
    }
}

//! The fixture loader against facts from a Ruby-loaded fixtures database, and (ignored
//! unless `CAMPFIRE_RUBY_FIXTURES_DB` is set) a row-for-row comparison with one.

use super::*;
use crate::{Membership, Message, Role, Room, RoomType, Session, User};

#[test]
fn ids_are_label_crcs() {
    let t = TestDb::new();
    assert_eq!(t.read(|c| User::find(c, 127326141)).name, "David");
    assert_eq!(
        t.read(|c| Room::find(c, 654632876)).name.as_deref(),
        Some("Designers")
    );
}

#[test]
fn associations_enums_and_defaults() {
    let t = TestDb::new();
    let pets = t.read(|c| Room::find(c, id("pets")));
    assert_eq!(
        (pets.room_type, pets.creator_id),
        (RoomType::Open, id("david"))
    );

    assert_eq!(
        t.read(|c| User::find(c, id("david"))).role,
        Role::Administrator
    );
    assert_eq!(t.read(|c| User::find(c, id("jz"))).role, Role::Member);
    let bender = t.read(|c| User::find(c, id("bender")));
    // `Digest::SHA256.hexdigest("BenderToken1")`, evaluated by the fixture's ERB.
    assert_eq!(
        (bender.role, bender.bot_token_digest.as_deref()),
        (Role::Bot, Some(BENDER_TOKEN_DIGEST))
    );

    let kevin_designers = t.read(|c| Membership::find(c, id("kevin_designers")));
    assert_eq!(
        kevin_designers.involvement,
        Some(crate::Involvement::Mentions),
        "column default"
    );
    assert_eq!(kevin_designers.connections, 0);

    let rich_text: (i64, String) = t.read(|c| {
        Ok(c.query_row(
            "SELECT record_id, record_type FROM action_text_rich_texts WHERE id = ?",
            [id("first")],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    });
    assert_eq!(rich_text, (id("first"), "Message".into()));
}

#[test]
fn erb_times_are_whole_seconds_and_default_timestamps_are_now() {
    let t = TestDb::new();
    let first = t.read(|c| Message::find(c, id("first")));
    assert_eq!(first.created_at.subsec_microsecond(), 0);
    assert!(first.updated_at > first.created_at);
    let session = t.read(|c| Session::find(c, id("david_safari")));
    assert_eq!(session.last_active_at.subsec_microsecond(), 0);
    assert!(
        (session.created_at.as_second() - session.last_active_at.as_second() - 7200).abs() <= 1
    );
}

#[test]
fn passwords_are_bcrypt_of_secret123456() {
    let t = TestDb::new();
    let david = t.read(|c| User::find(c, id("david")));
    assert!(david.authenticate("secret123456"));
    assert!(david.password_digest.unwrap().starts_with("$2a$"));
}

/// Every row of a table, sorted (some FTS shadow tables have no rowid), exactly, except the salted BCrypt digests: those are
/// their cost prefix (`$2a$12$`) and whether they verify `secret123456`, the fixtures' password.
fn dump(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM \"{table}\""))
        .unwrap();
    let names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let mut rows = stmt.query([]).unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let fields: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let value: rusqlite::types::Value = row.get(i).unwrap();
                let rendered = match (name.as_str(), value) {
                    ("password_digest", rusqlite::types::Value::Text(s)) => format!(
                        "bcrypt:{}:{}",
                        &s[..7],
                        bcrypt::verify("secret123456", &s).unwrap()
                    ),
                    (_, v) => format!("{v:?}"),
                };
                format!("{name}={rendered}")
            })
            .collect();
        out.push(fields.join(" "));
    }
    out.sort();
    out
}

fn tables(conn: &Connection) -> Vec<String> {
    crate::sql::query_all(
        conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name <> 'ar_internal_metadata' ORDER BY name",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

/// The fixture set names `db:fixtures:load` loads: every `.yml` under test/fixtures but `files/`.
fn fixture_sets() -> Vec<String> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path != root.join("files") {
                    walk(&path, root, out);
                }
            } else if path.extension().is_some_and(|e| e == "yml") {
                let relative = path.strip_prefix(root).unwrap().with_extension("");
                out.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    let root = crate::fixtures::reference_dir();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// Every table (fixture tables, the ones fixtures leave empty, FTS shadow tables and
/// `sqlite_sequence`; all but `ar_internal_metadata`, which `db:prepare` stamps) row for row
/// against the reference's `db:fixtures:load`, with both clocks frozen at
/// `CAMPFIRE_FIXTURES_NOW` so default timestamps and ERB times (`5.minutes.ago`) compare exactly.
#[test]
#[ignore = "needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW"]
fn fixtures_match_ruby_row_for_row() {
    let path = std::env::var("CAMPFIRE_RUBY_FIXTURES_DB").expect("CAMPFIRE_RUBY_FIXTURES_DB");
    let now = std::env::var("CAMPFIRE_FIXTURES_NOW").expect("CAMPFIRE_FIXTURES_NOW");
    let now = crate::Timestamp::parse_db(&now).expect("CAMPFIRE_FIXTURES_NOW like 2026-09-26 12:34:56.123456");
    let ruby = Connection::open(path).unwrap();
    // bcrypt-ruby's default cost, which the fixtures' `BCrypt::Password.create` uses.
    let t = TestDb::with_clock(crate::TestClock::frozen_at(now), 12);

    let ruby_tables = tables(&ruby);
    assert_eq!(t.read(|c| Ok(tables(c))), ruby_tables);
    let mut loaded = Vec::new();
    let mut mismatches = Vec::new();
    for table in &ruby_tables {
        let expected = dump(&ruby, table);
        let actual = t.read(|c| Ok(dump(c, table)));
        if !expected.is_empty() {
            loaded.push(table.clone());
        }
        if actual != expected {
            let only_ruby: Vec<_> = expected.iter().filter(|r| !actual.contains(r)).collect();
            let only_rust: Vec<_> = actual.iter().filter(|r| !expected.contains(r)).collect();
            mismatches.push(format!("{table}:\n  ruby only: {only_ruby:#?}\n  rust only: {only_rust:#?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));

    // Every fixture set's table was compared, and at least one row came from each non-empty set.
    let sets = fixture_sets();
    assert_eq!(sets.len(), 20, "{sets:?}");
    for set in &sets {
        let table = set.replace('/', "_");
        assert!(ruby_tables.contains(&table), "{set} -> {table}");
    }
    eprintln!("compared {} tables; rows in {}: {loaded:?}", ruby_tables.len(), loaded.len());
}

#[test]
#[ignore = "writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check"]
fn export_database_for_rails() {
    let path = std::env::var("CAMPFIRE_EXPORT_DB").expect("CAMPFIRE_EXPORT_DB");
    let _ = std::fs::remove_file(&path);
    // Frozen, so the messages written in one step below share a `created_at` and Rails has to
    // break the tie by id. At the instant Ruby loaded its fixtures, when differential.sh gives
    // one, so rollback.rb can tell every row Rust wrote from the fixtures by comparing with them.
    let start = std::env::var("CAMPFIRE_FIXTURES_NOW")
        .ok()
        .map(|now| crate::Timestamp::parse_db(&now).expect("CAMPFIRE_FIXTURES_NOW"))
        .unwrap_or_else(|| crate::Clock::now(&crate::SystemClock));
    let clock = crate::TestClock::frozen_at(start);
    let env = crate::Env {
        clock: std::sync::Arc::new(clock.clone()),
        bcrypt_cost: 4,
        ..Default::default()
    };
    let mut config = crate::Config::new(&path);
    config.environment = "test".into();
    let db = crate::Database::open(config, env).unwrap();
    db.write_blocking(|tx| {
        let options = crate::fixtures::Options {
            now: tx.now(),
            bcrypt_cost: 4,
        };
        crate::fixtures::load(tx.conn(), &crate::fixtures::reference_dir(), &options)?;
        Ok(())
    })
    .unwrap();
    db.write_blocking(|tx| {
        let message = Message::create(
            tx,
            crate::NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                client_message_id: Some("rust-1".into()),
                body: Some("Written by <b>Rust</b> hovercraft".into()),
                attachment_blob_id: None,
                ..Default::default()
            },
        )?;
        crate::Boost::create(tx, message.id, id("jason"), "🦀")?;
        let user = User::create(
            tx,
            crate::NewUser {
                name: "Rusty".into(),
                email_address: Some("rusty@example.com".into()),
                password_digest: Some(crate::PasswordDigest::create("secret123456", 4).unwrap()),
                ..Default::default()
            },
        )?;
        crate::Session::start(tx, user.id, Some("ua"), Some("8.8.8.8"))?;
        crate::Search::record(tx, user.id, "hovercraft")?;
        Room::create_for(
            tx,
            RoomType::Closed,
            Some("Rust Room"),
            user.id,
            &[user.id, id("david")],
        )?;
        let mut account = crate::Account::first(tx.conn())?.unwrap();
        account.update(
            tx,
            None,
            None,
            Some(&[("restrict_room_creation_to_administrators", "true")]),
        )?;
        Ok(())
    })
    .unwrap();
    clock.travel(jiff::SignedDuration::from_secs(1));
    let bot_key = db
        .write_blocking(|tx| {
            let rusty = User::find_by_email_address(tx.conn(), "rusty@example.com")?.unwrap();
            let members = [rusty.id, id("david")];
            Room::create_for(tx, RoomType::Voice, Some("Rust Voice"), rusty.id, &members)?;
            Room::create_for(tx, RoomType::Stage, Some("Rust Stage"), rusty.id, &members)?;
            Room::create_for(tx, RoomType::Board, Some("Rust Board"), rusty.id, &members)?;
            Room::find_or_create_direct_for(tx, &[rusty.id, id("kevin"), id("jason")], rusty.id)?;

            let rust_room_id: i64 = tx.conn().query_row(
                r#"SELECT "id" FROM "rooms" WHERE "name" = 'Rust Room'"#,
                [],
                |r| r.get(0),
            )?;
            let rust_room = Room::find(tx.conn(), rust_room_id)?;
            let mut membership =
                Membership::find_by_room_and_user(tx.conn(), rust_room.id, rusty.id)?.unwrap();
            membership.update_involvement(tx, crate::Involvement::Muted)?;

            crate::Session::start_with(
                tx,
                rusty.id,
                crate::NewSession {
                    user_agent: Some("ua"),
                    ip_address: Some("9.9.9.9"),
                    device_id: Some("rust-device"),
                    two_factor_verified: true,
                },
            )?;

            // Three root messages in one instant, then a system note and a thread reply.
            for body in ["tie one", "tie two", "tie three"] {
                Message::create(
                    tx,
                    crate::NewMessage {
                        room_id: rust_room.id,
                        creator_id: rusty.id,
                        client_message_id: Some(format!("rust-{body}")),
                        body: Some(body.into()),
                        ..Default::default()
                    },
                )?;
            }
            Message::create(
                tx,
                crate::NewMessage {
                    room_id: rust_room.id,
                    creator_id: rusty.id,
                    client_message_id: Some("rust-note".into()),
                    body: Some("Rusty renamed the room".into()),
                    system_note: true,
                    ..Default::default()
                },
            )?;
            let mut thread = crate::ChannelThread::create(
                tx,
                crate::NewChannelThread {
                    room_id: rust_room.id,
                    creator_id: rusty.id,
                    name: Some("Thread".into()),
                    ..Default::default()
                },
            )?;
            thread.post_message(
                tx,
                rusty.id,
                crate::NewMessage {
                    client_message_id: Some("rust-reply".into()),
                    body: Some("in the thread".into()),
                    ..Default::default()
                },
            )?;
            crate::ThreadMembership::join(tx, thread.id, id("david"))?
                .update_involvement(tx, crate::ThreadInvolvement::Everything)?;

            let mut bot = User::create_bot(tx, "Rust Bot", None)?;
            bot.reset_bot_key(tx)
        })
        .unwrap();
    // The plain key exists only in memory (Rails keeps only its digest): hand it to rollback.rb.
    std::fs::write(format!("{path}.bot_key"), bot_key).unwrap();

    // A board's root takes only system notes; its chat goes in post threads.
    db.write_blocking(|tx| {
        let rusty = User::find_by_email_address(tx.conn(), "rusty@example.com")?.unwrap();
        let board_id: i64 = tx.conn().query_row(
            r#"SELECT "id" FROM "rooms" WHERE "name" = 'Rust Board'"#,
            [],
            |r| r.get(0),
        )?;
        // Tracked, as a board post must be (`work_status_required_in_boards`), and tagged.
        let post = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: board_id,
                creator_id: rusty.id,
                name: Some("Thread".into()),
                work_status: Some("planned".into()),
                tag_names: Some(vec!["Rust-Tag".into(), "ui".into()]),
                ..Default::default()
            },
        )?
        .id;
        for (client_message_id, thread_id, system_note) in
            [("rust-board-note", None, true), ("rust-board-post", Some(post), false)]
        {
            Message::create(
                tx,
                crate::NewMessage {
                    room_id: board_id,
                    creator_id: rusty.id,
                    client_message_id: Some(client_message_id.into()),
                    body: Some(client_message_id.into()),
                    thread_id,
                    system_note,
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    })
    .unwrap();

    // A stream saved nine minutes after it started isn't overdue until ten minutes after that.
    let mut stream = db
        .write_blocking(|tx| {
            let rusty = User::find_by_email_address(tx.conn(), "rusty@example.com")?.unwrap();
            Message::create(
                tx,
                crate::NewMessage {
                    room_id: id("designers"),
                    creator_id: rusty.id,
                    client_message_id: Some("rust-stream".into()),
                    body: Some("thinking".into()),
                    streaming: true,
                    ..Default::default()
                },
            )
        })
        .unwrap();
    clock.travel(jiff::SignedDuration::from_mins(9));
    db.write_blocking(move |tx| stream.update_body(tx, "thinking harder"))
        .unwrap();

    // Destroying a message leaves a tombstone on its replies.
    db.write_blocking(|tx| {
        let doomed = Message::create(
            tx,
            crate::NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                client_message_id: Some("rust-doomed".into()),
                body: Some("rust-doomed".into()),
                ..Default::default()
            },
        )?;
        Message::create(
            tx,
            crate::NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                client_message_id: Some("rust-reply-to-doomed".into()),
                body: Some("rust-reply-to-doomed".into()),
                reply_to_message_id: Some(doomed.id),
                ..Default::default()
            },
        )?;
        doomed.destroy(tx)
    })
    .unwrap();
}

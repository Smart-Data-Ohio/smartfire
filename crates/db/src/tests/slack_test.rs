use rails_compat::{Secrets, ar_encryption::ArEncryption};
use rusqlite::params;
use serde_json::{Value, json};

use super::{TestDb, id};
use crate::models::slack::{NewConnection, SlackConnection, SlackWorkspace};
use crate::models::slack_import::{Kind, Mode, NewImport, SlackImport};

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/slack/crypto.json")).unwrap()
}
fn encryption() -> ArEncryption {
    ArEncryption::new(&Secrets::new(
        vectors()["secret_key_base"].as_str().unwrap(),
    ))
}

#[test]
fn slack_credentials_read_rails_encrypted_rows_and_reject_wrong_key() {
    let t = TestDb::new();
    let v = vectors();
    t.write(move |tx| {
        tx.conn().execute("INSERT INTO slack_workspaces (id, client_id, client_secret, created_at, updated_at) VALUES (1, 'fixture-client', ?, ?, ?)", params![v["workspace"]["ciphertext"].as_str().unwrap(), tx.now(), tx.now()])?;
        tx.conn().execute("INSERT INTO slack_connections (id, slack_workspace_id, user_id, slack_user_id, access_token, created_at, updated_at) VALUES (1, 1, ?, 'UCRYPTO', ?, ?, ?)", params![id("david"), v["connection"]["ciphertext"].as_str().unwrap(), tx.now(), tx.now()])?;
        Ok(())
    });
    let w = t.read(|c| Ok(SlackWorkspace::current(c)?.unwrap()));
    let c = t.read(|c| Ok(SlackConnection::for_user(c, id("david"))?.unwrap()));
    assert_eq!(
        w.client_secret(&encryption()).unwrap().unwrap(),
        vectors()["workspace"]["plaintext"]
    );
    assert_eq!(
        c.access_token(&encryption()).unwrap().unwrap(),
        vectors()["connection"]["plaintext"]
    );
    assert!(w.app_configured(&encryption()).unwrap());
    assert!(c.connected(&encryption()).unwrap());
    let wrong = ArEncryption::new(&Secrets::new(&"wrong-key".repeat(32)));
    assert!(w.client_secret(&wrong).is_err());
    assert!(c.access_token(&wrong).is_err());
}

#[test]
fn slack_credentials_create_encrypts_rows_and_exports_for_rails_readback() {
    let t = TestDb::new();
    let w = t.write(|tx| {
        SlackWorkspace::create(
            tx,
            &encryption(),
            "fixture-client",
            "fixture-slack-secret",
            Some(id("david")),
        )
    });
    let wid = w.id;
    let c = t.write(move |tx| {
        SlackConnection::create(
            tx,
            &encryption(),
            NewConnection {
                workspace_id: wid,
                user_id: id("david"),
                slack_user_id: "UCRYPTO",
                access_token: Some("fixture-slack-user-token"),
                scopes: Some("users:read"),
            },
        )
    });
    let cid = c.id;
    let raw = t.read(move |conn| {
        let secret: String = conn.query_row("SELECT client_secret FROM slack_workspaces WHERE id = ?", [wid], |r| r.get(0))?;
        let token: String = conn.query_row("SELECT access_token FROM slack_connections WHERE id = ?", [cid], |r| r.get(0))?;
        Ok(json!({"workspace": {"ciphertext": secret, "plaintext": "fixture-slack-secret"}, "connection": {"ciphertext": token, "plaintext": "fixture-slack-user-token"}}))
    });
    assert_ne!(raw["workspace"]["ciphertext"], "fixture-slack-secret");
    assert_ne!(raw["connection"]["ciphertext"], "fixture-slack-user-token");
    assert_eq!(
        w.client_secret(&encryption()).unwrap().as_deref(),
        Some("fixture-slack-secret")
    );
    assert_eq!(
        c.access_token(&encryption()).unwrap().as_deref(),
        Some("fixture-slack-user-token")
    );
    if let Some(path) = std::env::var_os("SLACK_RUST_CRYPTO_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&raw).unwrap()).unwrap();
    }
}

#[test]
fn slack_credentials_validate_and_unique_indexes_prevent_duplicate_connections() {
    let t = TestDb::new();
    assert!(
        t.try_write(|tx| SlackWorkspace::create(tx, &encryption(), " ", " ", None))
            .is_err()
    );
    let w = t.write(|tx| {
        SlackWorkspace::create(tx, &encryption(), "fixture-client", "fixture-secret", None)
    });
    let wid = w.id;
    assert!(
        t.try_write(move |tx| SlackConnection::create(
            tx,
            &encryption(),
            NewConnection {
                workspace_id: wid,
                user_id: id("david"),
                slack_user_id: " ",
                access_token: None,
                scopes: None
            }
        ))
        .is_err()
    );
    let c = t.write(move |tx| {
        SlackConnection::create(
            tx,
            &encryption(),
            NewConnection {
                workspace_id: wid,
                user_id: id("david"),
                slack_user_id: "U1",
                access_token: None,
                scopes: None,
            },
        )
    });
    assert!(!c.connected(&encryption()).unwrap());
    assert!(
        t.try_write(move |tx| SlackConnection::create(
            tx,
            &encryption(),
            NewConnection {
                workspace_id: wid,
                user_id: id("kevin"),
                slack_user_id: "U1",
                access_token: None,
                scopes: None
            }
        ))
        .err()
        .unwrap()
        .is_record_not_unique()
    );
}

#[test]
fn slack_credentials_destroy_connection_nullifies_runs() {
    let t = TestDb::new();
    let w = t.write(|tx| {
        SlackWorkspace::create(tx, &encryption(), "fixture-client", "fixture-secret", None)
    });
    let wid = w.id;
    let c = t.write(move |tx| {
        SlackConnection::create(
            tx,
            &encryption(),
            NewConnection {
                workspace_id: wid,
                user_id: id("david"),
                slack_user_id: "U1",
                access_token: None,
                scopes: None,
            },
        )
    });
    let cid = c.id;
    let run = t.write(move |tx| {
        SlackImport::create(
            tx,
            NewImport {
                workspace_id: wid,
                connection_id: Some(cid),
                user_id: id("david"),
                kind: Kind::Workspace,
                mode: Mode::Import,
                options: json!({}),
            },
        )
    });
    let rid = run.id;
    t.write(move |tx| SlackConnection::destroy(tx, cid));
    let after = t.read(move |c| Ok(SlackImport::find(c, rid)?.unwrap()));
    assert!(after.slack_connection_id.is_none());
    assert_eq!(after.updated_at, run.updated_at);
    assert!(t.read(move |c| SlackConnection::find(c, cid)).is_none());
}

fn huddle_snapshot(conn: &crate::Connection, selectors: &Value) -> crate::Result<Value> {
    use rusqlite::types::ValueRef;
    let mut snapshot = json!({});
    for (table, selector) in selectors.as_object().unwrap() {
        let mut query = conn.prepare(&format!(
            "SELECT * FROM {table} WHERE {} ORDER BY id",
            selector.as_str().unwrap()
        ))?;
        let columns = query
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let rows = query
            .query_map([], |row| {
                let mut value = json!({});
                for (index, column) in columns.iter().enumerate() {
                    if (table == "rooms" && column == "client_room_id")
                        || (table == "channel_threads" && column == "client_post_id")
                    {
                        assert!(
                            matches!(row.get_ref(index)?, ValueRef::Null),
                            "classic fixture has an API creation key"
                        );
                        continue;
                    }
                    // Port-only user preferences and activity counter that Rails doesn't have.
                    if table == "users"
                        && matches!(column.as_str(), "activity_revision" | "appearance_preferences" | "pronouns" | "nickname")
                    {
                        continue;
                    }
                    // Port-only read boundary for the sidebar's thread pings.
                    if table == "thread_memberships" && column == "last_read_message_id" {
                        continue;
                    }
                    value[column] = match row.get_ref(index)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(v) => json!(v),
                        ValueRef::Real(v) => json!(v),
                        ValueRef::Text(v) => json!(std::str::from_utf8(v).unwrap()),
                        ValueRef::Blob(_) => panic!("unexpected binary huddle fixture"),
                    };
                }
                Ok(value)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        snapshot[table] = json!(rows);
    }
    Ok(snapshot)
}
fn huddle_fixture() -> (TestDb, Value) {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/slack/undo_huddle.json")).unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    let before = oracle["before"].clone();
    db.write(move |tx| {
        for table in ["users", "rooms", "memberships", "huddle_grants", "streams"] {
            for row in before[table].as_array().unwrap() {
                super::huddle_notices_test::insert(tx, table, row)?;
            }
        }
        Ok(())
    });
    let selectors = oracle["selectors"].clone();
    assert_eq!(
        db.read(move |conn| huddle_snapshot(conn, &selectors)),
        oracle["before"]
    );
    db.sink.take();
    (db, oracle)
}
#[test]
fn slack_undo_huddle_user_callbacks_match_rails_rows_and_emit_stream_presence() {
    let (db, oracle) = huddle_fixture();
    db.write(|tx| crate::User::find(tx.conn(), 8800)?.destroy_for_slack_undo(tx));
    let selectors = oracle["selectors"].clone();
    assert_eq!(
        db.read(move |conn| huddle_snapshot(conn, &selectors)),
        oracle["after"]
    );
    let kinds = db
        .events()
        .into_iter()
        .filter_map(|event| match event {
            crate::Event::Broadcast(request) => Some(request.kind),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "Stream#broadcast_stream_changed")
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "HuddleGrant#broadcast_voice_presence")
            .count(),
        1
    );
    println!(
        "Slack undo huddle parity: six Rails affected tables matched every row and field; stream/presence callbacks emitted"
    );
}
#[test]
fn slack_undo_huddle_cleanup_failure_rolls_back_user_grants_streams_and_events() {
    let (db, oracle) = huddle_fixture();
    db.write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER ws16_huddle_failure BEFORE INSERT ON huddle_cleanups BEGIN SELECT RAISE(ABORT,'ws16 cleanup failure'); END;")?));
    assert!(
        db.try_write(|tx| crate::User::find(tx.conn(), 8800)?.destroy_for_slack_undo(tx))
            .is_err()
    );
    let selectors = oracle["selectors"].clone();
    assert_eq!(
        db.read(move |conn| huddle_snapshot(conn, &selectors)),
        oracle["before"]
    );
    assert!(db.events().is_empty());
}

//! Full row-set oracle from the actual Rails import/undo/reimport jobs and fixtures.
use super::client::tests::fake;
use super::jobs::tests::setup_sequence;
use super::store::tests::{import, routes};
use super::undoer::tests::undo;
use campfire_db::Database;
use campfire_db::models::slack_import::{Kind, Mode, NewImport, SlackImport};
use rusqlite::types::Value as SqlValue;
use serde_json::{Value, json};

fn input(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(v) => SqlValue::Integer(i64::from(*v)),
        Value::Number(v) => v
            .as_i64()
            .map_or_else(|| SqlValue::Real(v.as_f64().unwrap()), SqlValue::Integer),
        Value::String(v) => SqlValue::Text(v.clone()),
        _ => SqlValue::Text(value.to_string()),
    }
}
async fn restore_initial(db: &Database, initial: Value) {
    db.write(move |tx| {
        for table in [
            "slack_connections",
            "slack_workspaces",
            "users",
            "sqlite_sequence",
        ] {
            tx.conn().execute(&format!("DELETE FROM {table}"), [])?;
        }
        for table in [
            "users",
            "slack_workspaces",
            "slack_connections",
            "sqlite_sequence",
        ] {
            if table == "sqlite_sequence" {
                tx.conn().execute("DELETE FROM sqlite_sequence", [])?;
            }
            for row in initial[table].as_array().unwrap() {
                let columns: Vec<_> = row.as_object().unwrap().keys().cloned().collect();
                let sql = format!(
                    "INSERT INTO {table} ({}) VALUES ({})",
                    columns.join(","),
                    vec!["?"; columns.len()].join(",")
                );
                tx.conn().execute(
                    &sql,
                    rusqlite::params_from_iter(columns.iter().map(|column| input(&row[column]))),
                )?;
            }
        }
        Ok(())
    })
    .await
    .unwrap();
}
async fn snapshot(db: &Database, json_columns: Value) -> Value {
    db.read(move |c| {
        let mut result = json!({});
        for (table, json_columns) in json_columns.as_object().unwrap() {
            let select = if table == "message_search_index" {
                "rowid AS id,body"
            } else {
                "*"
            };
            let mut stmt = c.prepare(&format!("SELECT {select} FROM {table}"))?;
            let columns: Vec<_> = stmt.column_names().into_iter().map(str::to_owned).collect();
            let rows = stmt
                .query_map([], |row| {
                    let mut value = json!({});
                    for (i, column) in columns.iter().enumerate() {
                        // The API's per-attempt creation key is port-only; Slack imports leave it null.
                        if table == "rooms" && column == "client_room_id" {
                            assert!(
                                matches!(row.get::<_, SqlValue>(i)?, SqlValue::Null),
                                "Slack imports never set API creation keys"
                            );
                            continue;
                        }
                        // Port-only counter for the SPA activity badge that Rails doesn't have.
                        if table == "users" && column == "activity_revision" {
                            continue;
                        }
                        let v = match row.get::<_, SqlValue>(i)? {
                            SqlValue::Null => Value::Null,
                            SqlValue::Integer(v) => json!(v),
                            SqlValue::Real(v) => json!(v),
                            SqlValue::Text(v) => {
                                if json_columns.as_array().unwrap().contains(&json!(column)) {
                                    serde_json::from_str(&v).expect("JSON column")
                                } else {
                                    json!(v)
                                }
                            }
                            SqlValue::Blob(_) => {
                                panic!("unexpected binary field in Slack fixture: {table}.{column}")
                            }
                        };
                        value[column] = v;
                    }
                    Ok(value)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut rows = rows;
            rows.sort_by(|a, b| match (a["id"].as_i64(), b["id"].as_i64()) {
                (Some(a), Some(b)) => a.cmp(&b),
                _ => a["name"].as_str().cmp(&b["name"].as_str()),
            });
            result[table] = json!(rows);
        }
        Ok(result)
    })
    .await
    .unwrap()
}
fn first_difference(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for key in e.keys().chain(a.keys()) {
                if let Some(diff) =
                    first_difference(&expected[key], &actual[key], &format!("{path}.{key}"))
                {
                    return Some(diff);
                }
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                return Some(format!(
                    "{path}: expected {} rows, got {}",
                    e.len(),
                    a.len()
                ));
            }
            for (i, (e, a)) in e.iter().zip(a).enumerate() {
                if let Some(diff) = first_difference(e, a, &format!("{path}[{i}]")) {
                    return Some(diff);
                }
            }
        }
        _ => (),
    }
    Some(format!("{path}: expected {expected}, got {actual}"))
}
#[tokio::test]
async fn slack_sequence_matches_rails_import_undo_reimport_database_rows() {
    compare_sequence(
        false,
        include_str!("../../../../../vectors/slack/sequence.json"),
    )
    .await;
}
#[tokio::test]
async fn slack_sequence_personal_matches_rails_import_undo_reimport_database_rows() {
    compare_sequence(
        true,
        include_str!("../../../../../vectors/slack/sequence_personal.json"),
    )
    .await;
}
async fn compare_sequence(personal: bool, source: &str) {
    let oracle: Value = serde_json::from_str(source).unwrap();
    let (db, crypto, _dir) = setup_sequence().await;
    restore_initial(&db, oracle["initial"].clone()).await;
    assert_eq!(
        snapshot(&db, oracle["json_columns"].clone()).await,
        oracle["initial"]
    );
    let (_server, network) = fake(routes(personal)).await;
    let options = oracle["imported"]["slack_imports"][0]["options"].clone();
    let id = create(&db, personal, options).await;
    import(&db, crypto.clone(), id, network.clone()).await;
    let imported = snapshot(&db, oracle["json_columns"].clone()).await;
    if let Some(mutations) = oracle.get("mutations") {
        let mutations = mutations.clone();
        db.write(move |tx| {
            for mutation in mutations.as_array().unwrap() {
                tx.conn().execute(
                    mutation["sql"].as_str().unwrap(),
                    rusqlite::params_from_iter(
                        mutation["params"].as_array().unwrap().iter().map(input),
                    ),
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let changed = snapshot(&db, oracle["json_columns"].clone()).await;
        assert!(
            first_difference(&oracle["changed"], &changed, "common inputs").is_none(),
            "{}: {}",
            oracle["retained"],
            first_difference(&oracle["changed"], &changed, "common inputs").unwrap_or_default()
        );
    }
    undo(&db, id).await;
    let undone = snapshot(&db, oracle["json_columns"].clone()).await;
    let options = oracle["reimported"]["slack_imports"][1]["options"].clone();
    let again = create(&db, personal, options).await;
    import(&db, crypto, again, network).await;
    let reimported = snapshot(&db, oracle["json_columns"].clone()).await;
    if let Ok(path) = std::env::var("SLACK_SEQUENCE_OUTPUT") {
        let path = if personal {
            format!("{path}.personal")
        } else {
            path
        };
        std::fs::write(
            path,
            serde_json::to_string_pretty(
                &json!({"imported":imported,"undone":undone,"reimported":reimported}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let mut differences = Vec::new();
    for (phase, actual) in [
        ("imported", imported),
        ("undone", undone),
        ("reimported", reimported),
    ] {
        for table in oracle[phase].as_object().unwrap().keys() {
            if let Some(diff) = first_difference(
                &oracle[phase][table],
                &actual[table],
                &format!("{phase}.{table}"),
            ) {
                differences.push(diff);
            }
        }
    }
    assert!(
        differences.is_empty(),
        "Rails/Rust row mismatches:\n{}",
        differences.join("\n")
    );
    println!(
        "Slack DB differential ({}): import -> undo -> reimport; {} tables x 3 snapshots; every row and field matched",
        oracle["retained"]
            .as_str()
            .unwrap_or(if personal { "personal" } else { "workspace" }),
        oracle["json_columns"].as_object().unwrap().len()
    );
}

async fn create(db: &Database, personal: bool, options: Value) -> i64 {
    db.write(move |tx| {
        Ok(SlackImport::create(
            tx,
            NewImport {
                workspace_id: 1,
                connection_id: Some(1),
                user_id: 1,
                kind: if personal {
                    Kind::Personal
                } else {
                    Kind::Workspace
                },
                mode: Mode::Import,
                options,
            },
        )?
        .id)
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn slack_retained_undo_reimport_matches_rails_saved_poll_schedules_threads_and_claims() {
    for source in [
        include_str!("../../../../../vectors/slack/sequence_keep_saved_reply.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_poll_reply.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_pending_quoted_reply.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_sent_reply.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_foreign_thread.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_room_event_schedule.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_claimed_session.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_claimed_google_account.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_claimed_google_identity.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_claimed_password.json"),
        include_str!("../../../../../vectors/slack/sequence_keep_placeholder_authorship.json"),
    ] {
        compare_sequence(false, source).await;
    }
}

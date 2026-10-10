//! Actual persisted values for Rails differentials; only timestamp storage spelling is normalized.
use campfire_db::Timestamp;
use serde_json::{Value, json};

/// Repeat the same oracle with different physical row and subscription layouts.
/// IDs, fields, jobs, expected frames and deadlines stay unchanged.
pub(super) fn embed_groups(mut oracle: Value) -> Vec<Value> {
    use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
    let seed = embed_seed();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut groups = oracle["groups"].as_array_mut().unwrap().clone();
    groups.shuffle(&mut rng);
    for group in &mut groups {
        for rows in group["rows"].as_object_mut().unwrap().values_mut() {
            rows.as_array_mut().unwrap().shuffle(&mut rng);
        }
    }
    println!("WS8bm2 embed layout seed: {seed}");
    groups
}

pub(super) fn embed_seed() -> u64 {
    std::env::var("WS8BM2_EMBED_SEED")
        .map(|value| value.parse().expect("numeric embed layout seed"))
        .unwrap_or(0)
}

pub(super) async fn settle_jobs(app: &crate::controllers::presenters::test_support::TestApp) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let now = app.db().env().now();
            let pending = app.db().read(move |conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE status='running' OR (status='ready' AND run_at<=?)",
                [now], |row| row.get::<_, i64>(0))?)).await.unwrap();
            if pending == 0 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.expect("fetch jobs complete");
}

pub(super) fn row(
    conn: &campfire_db::Connection,
    table: &str,
    id: i64,
) -> campfire_db::Result<Value> {
    use rusqlite::OptionalExtension;
    let mut q = conn.prepare(&format!("SELECT * FROM {table} WHERE id=?"))?;
    let names = q
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Ok(q.query_row([id], |r| {
        let mut out = serde_json::Map::new();
        for (i, name) in names.iter().enumerate() {
            let value = match r.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Value::Null,
                rusqlite::types::ValueRef::Integer(v) => json!(v),
                rusqlite::types::ValueRef::Text(v) => json!(std::str::from_utf8(v).unwrap()),
                rusqlite::types::ValueRef::Real(v) => json!(v),
                _ => panic!("blob {name}"),
            };
            out.insert(name.clone(), value);
        }
        Ok(Value::Object(out))
    })
    .optional()?
    .unwrap_or(Value::Null))
}
pub(super) fn same_row(actual: &Value, expected: &Value, context: &str) {
    if expected.is_null() {
        assert!(actual.is_null(), "{context}: {actual}");
        return;
    }
    assert_eq!(
        actual.as_object().unwrap().len(),
        expected.as_object().unwrap().len(),
        "{context} columns"
    );
    for (key, want) in expected.as_object().unwrap() {
        let got = &actual[key];
        if key.ends_with("_at") && want.is_string() && got.is_string() {
            assert_eq!(
                Timestamp::parse_db(got.as_str().unwrap()).expect("actual persisted timestamp"),
                Timestamp::parse_db(want.as_str().unwrap()).expect("Rails persisted timestamp"),
                "{context}.{key}"
            );
        } else {
            assert_eq!(got, want, "{context}.{key}");
        }
    }
}

/// Expand only recorded Rails expectations. Candidate rows are always selected
/// independently in full; the encoding drops no row, column or deletion check.
pub(super) fn expected_tables(group: &Value, observation: &Value) -> serde_json::Map<String, Value> {
    let mut tables = group["baseline"].as_object().unwrap().clone();
    for (table, changes) in observation["state"].as_object().unwrap() {
        if group["state_encoding"] != "rows_by_id_v1" {
            tables.insert(table.clone(), changes.clone());
            continue;
        }
        let mut rows: std::collections::BTreeMap<_, _> = tables[table].as_array().unwrap().iter().map(|row| (row["id"].as_i64().unwrap(), row.clone())).collect();
        for id in changes["deleted_ids"].as_array().unwrap() { assert!(rows.remove(&id.as_i64().unwrap()).is_some(), "recorded Rails deletion exists"); }
        for row in changes["rows"].as_array().unwrap() { rows.insert(row["id"].as_i64().unwrap(), row.clone()); }
        tables.insert(table.clone(), json!(rows.into_values().collect::<Vec<_>>()));
    }
    tables
}

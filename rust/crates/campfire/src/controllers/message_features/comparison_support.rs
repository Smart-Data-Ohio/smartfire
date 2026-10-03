//! Actual persisted values for Rails differentials; only timestamp storage spelling is normalized.
use campfire_db::Timestamp;
use serde_json::{Value, json};
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

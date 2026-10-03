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

fn embed_seed() -> u64 {
    std::env::var("WS8BM2_EMBED_SEED")
        .map(|value| value.parse().expect("numeric embed layout seed"))
        .unwrap_or(0)
}

pub(super) async fn embed_streams(
    app: &crate::app::tests::TestApp,
    thread: i64,
) -> (crate::channels::tests::support::Client, tokio::task::JoinHandle<()>) {
    use crate::channels::tests::support::identifier;
    use crate::controllers::presenters::test_support::QUIET_CORNER;
    let (mut client, server) = super::quote_integration_tests::stream(app).await;
    let gid = campfire_views::helpers::gid_param("ChannelThread", thread);
    let thread = identifier(json!({"channel":"RoomMessagesChannel","signed_stream_name":
        rails_compat::turbo::signed_stream_name(&app.booted.app.secrets,&[&gid,"messages"])}));
    if embed_seed() % 2 == 1 {
        let room = app.db().read(|conn| campfire_db::Room::find(conn, QUIET_CORNER)).await.unwrap();
        let gid = crate::channels::room_gid(&room).to_param();
        let root = identifier(json!({"channel":"RoomMessagesChannel","signed_stream_name":
            rails_compat::turbo::signed_stream_name(&app.booted.app.secrets,&[&gid,"messages"])}));
        client.unsubscribe(&root).await;
        client.confirm(&thread).await;
        client.confirm(&root).await;
    } else {
        client.confirm(&thread).await;
    }
    (client, server)
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

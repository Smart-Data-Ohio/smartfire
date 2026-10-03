//! Original controller copy assertion and both real HTML/JSON body producers.
use super::presenters::{activity, test_support::*};
use campfire_db::{ActivityItem, Membership, Room, Session, User};
use serde_json::Value;

#[tokio::test]
async fn ws12_started_and_missed_huddles_assert_the_original_rails_copy() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws12_huddle_copy.json")).unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed")
        .without_job_runner()
        .await;
    let inputs = oracle["rows"].as_array().unwrap().clone();
    let rows = app.db().write(move |tx| {
        let mut rows = Vec::new();
        for (index, input) in inputs.into_iter().enumerate() {
            let caller = input["caller_id"].as_i64().unwrap();
            let room = input["room_id"].as_i64().unwrap();
            Room::find(tx.conn(), room)?.grant_to(tx, &[caller])?;
            let membership = Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap();
            let session = Session::start(tx, caller, None, None)?;
            let id: i64 = tx.conn().query_row("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,last_issued_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?) RETURNING id", rusqlite::params![format!("ws12-copy-{index}"),format!("ws12-copy-{index}"),session.id,caller,membership.id,room,tx.now(),tx.now(),tx.now()],|r|r.get(0))?;
            let item = ActivityItem::refresh_unread(tx, DAVID, "HuddleGrant", id, input["event_type"].as_str().unwrap())?;
            rows.push((item, input));
        }
        Ok(rows)
    }).await.unwrap();
    let state = app.booted.app.clone();
    let expected = rows.clone();
    app.db()
        .read(move |conn| {
            let viewer = User::find(conn, DAVID)?;
            for (row, expected) in expected {
                let sources = activity::Sources::load(conn, std::slice::from_ref(&row))?;
                let view = activity::item(conn, &state, &row, &viewer, &sources)?;
                assert_eq!(view.event_label, expected["label"]);
                assert_eq!(view.author.as_deref(), expected["author"].as_str());
                assert_eq!(view.body, expected["body"], "HTML missed/started copy");
                let payload = activity::payload(conn, &state, &row)?;
                assert_eq!(
                    serde_json::to_value(payload).unwrap()["source"]["body"],
                    expected["body"],
                    "JSON missed/started copy"
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    let response = app
        .david()
        .send(Req::new(axum::http::Method::GET, "/activity"))
        .await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    let document = response.text();
    for (row, expected) in rows {
        let start = document
            .find(&format!("id=\"activity_item_{}\"", row.id))
            .expect("accessible item is rendered");
        let end = start
            + document[start..]
                .find("</article>")
                .expect("item article closes");
        let text = &document[start..end];
        assert!(
            text.contains(expected["label"].as_str().unwrap()),
            "original Rails label"
        );
        assert!(
            text.contains(expected["body"].as_str().unwrap()),
            "original Rails body: {text}"
        );
    }
    println!(
        "WS12_HUDDLE_COPY 2 persisted sources; HTML presenter, JSON producer and served inbox DOM agree with Rails"
    );
}

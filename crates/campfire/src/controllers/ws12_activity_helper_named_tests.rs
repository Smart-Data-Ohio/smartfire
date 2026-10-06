//! Exact Rails helper assertions, using the shared inbox presenter and persisted sources.
use super::presenters::{
    activity,
    test_support::{DAVID, TestApp},
};
use campfire_db::{ActivityItem, User};
use serde_json::{Value, json};

async fn run(key: &'static str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_activity_helpers_named.json"
    ))
    .unwrap();
    let expected = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap()["facts"]
        .clone();
    let app = TestApp::boot_frozen()
        .await
        .expect("pinned seed")
        .without_job_runner()
        .await;
    let setup = oracle["setup"].clone();
    let event_id = setup["events"][0]["id"].as_i64().unwrap();
    let credential_id = setup["two_factor_credentials"][0]["id"].as_i64().unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM two_factor_credentials WHERE user_id=?",
                [DAVID],
            )?;
            for table in ["rooms", "events", "two_factor_credentials"] {
                for row in setup[table].as_array().unwrap() {
                    let fields = row.as_object().unwrap();
                    let columns = fields.keys().cloned().collect::<Vec<_>>();
                    let values = fields
                        .values()
                        .map(|v| match v {
                            Value::Null => rusqlite::types::Value::Null,
                            Value::Number(n) => {
                                rusqlite::types::Value::Integer(n.as_i64().unwrap())
                            }
                            Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                            _ => panic!("persisted SQLite fixture field"),
                        })
                        .collect::<Vec<_>>();
                    tx.conn().execute(
                        &format!(
                            "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT(id) DO UPDATE SET {}",
                            columns.join(","),
                            vec!["?"; columns.len()].join(","),
                            columns
                                .iter()
                                .map(|c| format!("{c}=excluded.{c}"))
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                        rusqlite::params_from_iter(values),
                    )?;
                }
            }
            if key == "no_venue" {
                tx.conn().execute(
                    "UPDATE events SET venue_room_id=NULL WHERE id=?",
                    [event_id],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let state = app.booted.app.clone();
    let actual=app.db().read(move|conn|{
        let now=state.db.env().now();
        let (source_type,source_id,event_type)=match key {
            "venue"|"no_venue"=>("Event",event_id,"event_reminder"),
            "review"=>("TwoFactorCredential",credential_id,"pr_review_request"),
            _=>("TwoFactorCredential",credential_id,"two_factor_lockout")
        };
        let row=ActivityItem{id:0,user_id:DAVID,source_type:source_type.into(),source_id,event_type:event_type.into(),read_at:None,handled_at:None,created_at:now,updated_at:now};
        let sources=activity::Sources::load(conn,std::slice::from_ref(&row))?;
        let view=activity::item(conn,&state,&row,&User::find(conn,DAVID)?,&sources)?;
        Ok(match key {
            "review"=>json!({"label":view.event_label}),
            "venue"|"no_venue"=>json!({"body":view.body}),
            _=>json!({"label":view.event_label,"body":view.body,"title":view.title,"path":activity::destination(conn,&row)?})
        })
    }).await.unwrap();
    assert_eq!(actual, expected, "complete helper declaration {key}");
}
#[tokio::test]
async fn ws12_inbox_named_review_request_label() {
    run("review").await;
}
#[tokio::test]
async fn ws12_inbox_named_event_reminder_venue() {
    run("venue").await;
}
#[tokio::test]
async fn ws12_inbox_named_event_reminder_without_venue() {
    run("no_venue").await;
}
#[tokio::test]
async fn ws12_inbox_named_lockout_label_body_source_and_profile() {
    run("lockout").await;
}

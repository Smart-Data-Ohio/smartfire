use crate::channels::{
    tests::support::{Client, bind_listener, identifier},
    user_gid,
};
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp, david_cookie};
use axum::http::{Method, StatusCode};
use campfire_db::{Timestamp, UserStatusSettings};
use rusqlite::types::Value as SqlValue;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const PATH: &str = "/users/me/status";
async fn boot() -> TestApp {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the actual Rails parity seed");
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=NULL WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    app
}
async fn settings(app: &TestApp) -> UserStatusSettings {
    app.db()
        .read(|conn| UserStatusSettings::find(conn, DAVID))
        .await
        .unwrap()
}

// Real socket on the seeded HTTP app, so this checks sink registration and broadcast guards too.
struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn subscribe(app: &TestApp) -> (Server, Client) {
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    for suffix in ["status", "ooo_notice"] {
        let signed = rails_compat::turbo::signed_stream_name(
            &app.booted.app.secrets,
            &[&user_gid(DAVID).to_param(), suffix],
        );
        client
            .confirm(&identifier(
                json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}),
            ))
            .await;
    }
    (server, client)
}

async fn replay(names: &[&str]) {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/ws17_status_requests.json"
    ))
    .unwrap();
    let app = boot().await;
    let (_server, mut socket) = subscribe(&app).await;
    for name in names {
        let row = golden["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == *name)
            .unwrap()
            .clone();
        let setup = row.clone();
        app.db().write(move |tx|{
            tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
            tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",[])?;
            tx.conn().execute("UPDATE users SET presence_setting='auto',custom_status_emoji=NULL,custom_status_text=NULL,custom_status_expires_at=NULL,dnd_enabled=0,quiet_hours_enabled=0,meeting_status_enabled=0,meeting_dnd_enabled=0,ooo_until=NULL,ooo_note=NULL,ooo_calendar_enabled=0,ooo_notify_enabled=0,ooo_broadcast=NULL,time_zone='UTC' WHERE id=?",[DAVID])?;
            if let Some(attrs)=setup["attrs"].as_object(){for(key,value)in attrs{
                let value=match value {Value::Bool(b)=>SqlValue::Integer(i64::from(*b)),Value::String(s) if key.ends_with("_at")||key=="ooo_until"=>SqlValue::Text(Timestamp::from_jiff(s.parse().unwrap()).to_db()),Value::String(s)=>SqlValue::Text(s.clone()),Value::Null=>SqlValue::Null,_=>panic!("unexpected fixture {value}")};
                tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,DAVID])?;
            }}
            if setup.get("busy").is_some()||setup.get("ooo").is_some(){
                tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,created_at,updated_at) VALUES (?,?,?,?,?,?)",rusqlite::params![DAVID,setup.get("busy").unwrap_or(&json!([])).to_string(),setup.get("ooo").unwrap_or(&json!([])).to_string(),tx.now(),tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let mut browser = if row["anonymous"] == true {
            app.anonymous()
        } else {
            app.david()
        };
        let reply = browser
            .write(
                Req::new(Method::PATCH, PATH)
                    .header("content-type", "application/json")
                    .header("accept", "text/html")
                    .body(json!({"user":row["params"]}).to_string()),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            reply.text()
        );
        assert_eq!(reply.location(), row["location"].as_str(), "{name}");
        let current = settings(&app).await;
        let stored = json!({"presence_setting":current.presence_setting,"custom_status_emoji":current.custom_status_emoji,"custom_status_text":current.custom_status_text,"meeting_status_enabled":current.meeting_status_enabled,"ooo_calendar_enabled":current.ooo_calendar_enabled,"ooo_note":current.ooo_note,"ooo_broadcast":current.ooo_broadcast});
        for (key, value) in stored.as_object().unwrap() {
            assert_eq!(value, &row["stored"][key], "{name}/{key}");
        }
        for (key, time) in [
            ("custom_status_expires_at", current.custom_status_expires_at),
            ("ooo_until", current.ooo_until),
        ] {
            if let Some(expected) = row["stored"][key].as_str() {
                let expected = Timestamp::from_jiff(expected.parse().unwrap());
                let actual = time.unwrap();
                assert!(
                    (actual.as_microsecond() - expected.as_microsecond()).abs() < 2_000_000,
                    "{name}/{key}: {actual:?} != {expected:?}"
                );
            } else {
                assert!(time.is_none(), "{name}/{key}");
            }
        }
        let cache = current
            .meeting_cache
            .map(|c| json!({"busy_intervals":c.busy_intervals,"ooo_intervals":c.ooo_intervals}));
        assert_eq!(json!(cache), row["cache"], "{name}");
        let jobs:Vec<Value>=app.db().read(|conn|{
            let mut q=conn.prepare("SELECT arguments FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob' ORDER BY id")?;
            Ok(q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.iter().map(|s|json!({"class":"Calendar::MeetingRefreshJob","args":[serde_json::from_str::<Value>(s).unwrap()["user_id"]]})).collect())
        }).await.unwrap();
        assert_eq!(json!(jobs), row["jobs"], "{name}");
        // Rails dispatches each stream callback with worker_pool.async_invoke. Emission order
        // across independent channel identifiers is not socket delivery order. Preserve complete
        // bytes, exact streams/counts, and each stream's sequence; domain tests check emission order.
        let mut expected =
            std::collections::BTreeMap::<String, std::collections::VecDeque<&Value>>::new();
        for frame in row["frames"].as_array().unwrap() {
            let stream = frame["stream"]
                .as_str()
                .unwrap()
                .split(':')
                .collect::<Vec<_>>();
            expected
                .entry(rails_compat::turbo::signed_stream_name(
                    &app.booted.app.secrets,
                    &stream,
                ))
                .or_default()
                .push_back(frame);
        }
        for _ in row["frames"].as_array().unwrap() {
            let actual: Value = serde_json::from_str(&socket.next_text().await).unwrap();
            let channel: Value =
                serde_json::from_str(actual["identifier"].as_str().unwrap()).unwrap();
            assert_eq!(channel["channel"], "Turbo::StreamsChannel");
            let signed = channel["signed_stream_name"].as_str().unwrap();
            let frame = expected
                .get_mut(signed)
                .expect("only the exact Rails stream may deliver")
                .pop_front()
                .expect("no duplicate frame");
            assert_eq!(actual["message"], frame["html"], "{name} complete frame");
        }
        assert!(
            expected.values().all(|frames| frames.is_empty()),
            "{name} all exact frames delivered"
        );
        socket.assert_silent().await;
    }
}

macro_rules! scenario{($test:ident,$($name:literal),+)=>{#[tokio::test]async fn $test(){replay(&[$($name),+]).await;}};}
scenario!(
    ws17_updates_presence_and_the_custom_status_with_an_expiry,
    "updates_presence"
);
scenario!(ws17_clears_the_custom_status, "clears_custom");
scenario!(
    ws17_rejects_an_unknown_presence_with_errors,
    "invalid_presence"
);
scenario!(ws17_statuses_requires_sign_in, "requires_sign_in");
scenario!(
    ws17_opting_into_meeting_status_enqueues_a_first_refresh,
    "meeting_on"
);
scenario!(
    ws17_opting_out_of_meeting_status_drops_the_cached_intervals,
    "meeting_off"
);
scenario!(
    ws17_opting_out_while_in_a_meeting_broadcasts_the_cleared_badge,
    "meeting_off_badge"
);
scenario!(
    ws17_opting_out_without_cached_intervals_broadcasts_nothing,
    "meeting_off_no_cache"
);
scenario!(
    ws17_saving_other_status_settings_leaves_meeting_refreshes_alone,
    "unrelated_meeting"
);
scenario!(
    ws17_sets_out_of_office_with_a_preset_and_a_note_and_broadcasts_it,
    "manual_on"
);
scenario!(
    ws17_sets_out_of_office_with_a_custom_date_and_time_in_the_members_zone,
    "custom_zone"
);
scenario!(
    ws17_rejects_an_unknown_ooo_preset_without_saving_anything,
    "unknown_ooo"
);
scenario!(
    ws17_rejects_a_blank_or_past_custom_ooo_end_without_saving_anything,
    "blank_ooo",
    "past_ooo"
);
scenario!(ws17_rejects_an_ooo_note_over_140_characters, "long_note");
scenario!(
    ws17_clears_out_of_office_early_and_broadcasts_the_cleared_state,
    "manual_clear"
);
scenario!(
    ws17_clearing_early_ends_only_the_manual_ooo_while_calendar_ooo_covers,
    "manual_clear_calendar"
);
scenario!(ws17_edits_the_ooo_note_alone, "note_edit");
scenario!(
    ws17_opting_into_calendar_ooo_enqueues_a_first_refresh,
    "calendar_on"
);
scenario!(
    ws17_opting_out_of_calendar_ooo_clears_its_intervals_and_broadcasts,
    "calendar_off"
);
scenario!(
    ws17_opting_out_of_calendar_ooo_keeps_the_row_while_meeting_status_is_on,
    "calendar_off_keep"
);
scenario!(
    ws17_opting_out_of_meeting_status_keeps_the_row_while_calendar_ooo_is_on,
    "meeting_off_keep"
);
scenario!(
    ws17_saving_other_status_settings_leaves_calendar_ooo_refreshes_alone,
    "unrelated_calendar"
);
scenario!(
    ws17_combined_status_edits_match_rails_dirty_callbacks,
    "both_on",
    "both_off",
    "calendar_off_and_note",
    "blank_note_clears"
);
scenario!(
    ws17_invalid_status_setters_keep_submitted_edits_and_do_not_persist,
    "unknown_expiry",
    "invalid_lengths"
);
scenario!(
    ws17_clear_buttons_follow_rails_present_not_boolean_cast,
    "clear_zero_is_present"
);
scenario!(
    ws17_nil_calendar_boolean_fails_without_saving,
    "nil_boolean"
);
scenario!(
    ws17_status_parameter_scalar_and_compound_casts_match_rails,
    "boolean_status",
    "false_note",
    "hash_note",
    "unknown_compound_preset",
    "unpermitted_flags"
);

#[tokio::test]
async fn ws17_seeded_enabled_2fa_settings_errors_match_the_actual_rails_failure() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/ws17_status_requests.json"
    ))
    .unwrap();
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the actual Rails parity seed");
    let before = settings(&app).await;
    let mut browser = app.david();
    for (endpoint, fields) in [
        ("status", vec![("user[presence_setting]", "away")]),
        (
            "notification_settings",
            vec![
                ("user[quiet_hours_enabled]", "1"),
                ("user[quiet_hours_start]", ""),
                ("user[quiet_hours_end]", ""),
            ],
        ),
    ] {
        let reply = browser
            .write(Req::new(Method::PATCH, &format!("/users/me/{endpoint}")).form(&fields))
            .await;
        assert_eq!(
            reply.status.as_u16(),
            golden["seeded_failures"][endpoint].as_u64().unwrap() as u16
        );
        assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(settings(&app).await.presence_setting, "auto");
        assert_eq!(
            settings(&app).await.quiet_hours_enabled,
            before.quiet_hours_enabled
        );
    }
}

#[tokio::test]
async fn ws17_failed_status_refresh_enqueue_rolls_back_the_entire_http_write() {
    let app = boot().await;
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER ws17_refresh_failure BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::MeetingRefreshJob' BEGIN SELECT RAISE(ABORT,'ws17 deliberate enqueue failure'); END;")?;Ok(())}).await.unwrap();
    let (_server, mut socket) = subscribe(&app).await;
    let mut browser = app.david();
    let reply = browser
        .write(Req::new(Method::PATCH, PATH).form(&[
            ("user[meeting_status_enabled]", "1"),
            ("user[presence_setting]", "invisible"),
            ("user[ooo_preset]", "tomorrow"),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let u = settings(&app).await;
    assert!(!u.meeting_status_enabled);
    assert_eq!(u.presence_setting, "auto");
    assert!(u.ooo_until.is_none());
    assert!(u.ooo_broadcast.is_none());
    let jobs:i64=app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",[],|r|r.get(0))?)).await.unwrap();
    assert_eq!(jobs, 0);
    socket.assert_silent().await;
}

#[tokio::test]
async fn ws17_failed_calendar_cache_reconciliation_rolls_back_status_and_sends_no_frames() {
    let app = boot().await;
    app.db().write(|tx|{
        tx.conn().execute("UPDATE users SET meeting_status_enabled=1,ooo_calendar_enabled=1 WHERE id=?",[DAVID])?;
        tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,'[[\"2026-03-02T15:00:00Z\",\"2026-03-02T17:00:00Z\"]]','[]',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;
        tx.conn().execute_batch("CREATE TRIGGER ws17_cache_failure BEFORE UPDATE OF busy_intervals ON calendar_meeting_caches BEGIN SELECT RAISE(ABORT,'ws17 deliberate cache failure'); END;")?;
        Ok(())
    }).await.unwrap();
    let before = settings(&app).await;
    let (_server, mut socket) = subscribe(&app).await;
    let mut browser = app.david();
    let reply = browser
        .write(Req::new(Method::PATCH, PATH).form(&[
            ("user[meeting_status_enabled]", "0"),
            ("user[custom_status_text]", "Changed"),
            ("user[ooo_note]", "Changed"),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let u = settings(&app).await;
    assert!(u.meeting_status_enabled);
    assert_eq!(u.custom_status_text, before.custom_status_text);
    assert_eq!(u.ooo_note, before.ooo_note);
    assert_eq!(
        u.meeting_cache
            .unwrap()
            .busy_intervals
            .as_array()
            .unwrap()
            .len(),
        1
    );
    socket.assert_silent().await;
}

#[path = "calendar_tests.rs"]
mod calendar;

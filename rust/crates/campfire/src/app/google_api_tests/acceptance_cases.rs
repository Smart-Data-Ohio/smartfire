use super::*;
use crate::integrations::net::http::HttpError;
use std::io;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/google_client_acceptance.json"
    ))
    .unwrap()
}

#[test]
fn google_client_configuration_requires_each_credential() {
    for row in oracle()["configuration"].as_array().unwrap() {
        let configuration = Config {
            client_id: row["client_id"].as_str().unwrap().into(),
            client_secret: row["client_secret"].as_str().unwrap().into(),
            webhook_url: None,
        };
        assert_eq!(
            configuration.configured(),
            row["configured"].as_bool().unwrap(),
            "{row}"
        );
    }
}

#[tokio::test]
async fn google_client_errors_exchange_revoke_and_paging_match_pinned_rails() {
    let vectors = oracle();
    let now = Timestamp::from_second(vectors["now"].as_i64().unwrap());
    let a = TestApp::boot_without_periodic_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        now.jiff(),
    )))
    .await
    .expect("default seed required");
    for row in vectors["rows"].as_array().unwrap() {
        let spec = &row["spec"];
        let r = Recorded::new(
            spec["responses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    (
                        p[0].as_u64().unwrap() as u16,
                        p[1].as_str().unwrap().as_bytes().to_vec(),
                    )
                })
                .collect(),
        );
        if let Some(kind) = spec["transport"].as_str() {
            let failure = match kind {
                "Net::OpenTimeout" => HttpError::OpenTimeout,
                "Net::ReadTimeout" => HttpError::ReadTimeout,
                "EOFError" => HttpError::ConnectionClosed,
                "Errno::ECONNREFUSED" => {
                    HttpError::Io(io::Error::from_raw_os_error(libc::ECONNREFUSED))
                }
                "Errno::ECONNRESET" => {
                    HttpError::Io(io::Error::from_raw_os_error(libc::ECONNRESET))
                }
                _ => panic!("unrecorded transport {kind}"),
            };
            let call = &row["requests"][0];
            r.fail_for_error(
                call["method"].as_str().unwrap().parse().unwrap(),
                call["path"].as_str().unwrap(),
                failure.into(),
            );
        }
        grant(
            &a,
            DAVID,
            now.since(jiff::SignedDuration::from_secs(
                if spec["expired"] == true { -3600 } else { 3600 },
            )),
            true,
        )
        .await;
        let api = Api::new(config(), r.clone());
        let result = match spec["operation"].as_str().unwrap() {
            "delete" => {
                api.request(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    ApiRequest::calendar(
                        Method::DELETE,
                        &format!(
                            "{}/{}",
                            api::EVENTS,
                            spec["event_id"].as_str().unwrap_or("gone-id")
                        ),
                        None,
                    ),
                    now,
                )
                .await
            }
            "drive" => {
                api.drive_file(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    "1AbcDefGhIjKlMnOpQrSt",
                    now,
                )
                .await
            }
            "list" => {
                api.list_drive_files(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    spec["query"].as_str().unwrap(),
                    now,
                )
                .await
            }
            "pages" => {
                api.list_events(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    spec["time_min"]
                        .as_str()
                        .map(|at| Timestamp::from_jiff(at.parse().unwrap()))
                        .unwrap_or_else(|| now.ago(jiff::SignedDuration::from_hours(1))),
                    spec["time_max"]
                        .as_str()
                        .map(|at| Timestamp::from_jiff(at.parse().unwrap()))
                        .unwrap_or_else(|| now.since(jiff::SignedDuration::from_hours(1))),
                    now,
                )
                .await
            }
            "exchange" => {
                api.exchange_code(
                    spec["code"].as_str().unwrap_or("disposable-code"),
                    "http://test.host/google/callback",
                )
                .await
            }
            "revoke" => api
                .revoke_token(Some("disposable-revoke-token"))
                .await
                .map(Value::Bool),
            "calendar" => {
                api.request(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    ApiRequest::calendar(Method::POST, api::EVENTS, Some(&json!({}))),
                    now,
                )
                .await
            }
            operation => panic!("unrecorded operation {operation}"),
        };
        match result {
            Ok(value) => {
                assert!(row["error"].is_null(), "{spec}");
                assert_eq!(value, row["result"], "{spec}");
            }
            Err(error) => assert_eq!(
                json!({"class":error.class(),"message":error.to_string(),"unavailable":error.unavailable()}),
                row["error"],
                "{spec}"
            ),
        }
        let account = a
            .db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            json!({"access_token":account.access_token(&ArEncryption::new(&a.booted.app.secrets)).unwrap(),"expires_at":account.access_token_expires_at.map(|t|t.as_second()),"disconnected_reason":account.disconnected_reason}),
            row["credentials"],
            "{spec}"
        );
        assert_eq!(json!(*r.calls.lock().unwrap()), row["requests"], "{spec}");
        assert!(r.answers.lock().unwrap().is_empty(), "{spec}");
    }
}

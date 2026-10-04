//! Recorded Rails Google::Client exchanges over the real encrypted account model.
use crate::{
    controllers::presenters::test_support::{DAVID, TestApp},
    integrations::{
        google::{
            api::{self, Api, ApiRequest, Config},
            client::{Client, Unavailable},
        },
        net::BoxFuture,
    },
};
use campfire_db::{
    Timestamp,
    models::google_account::{ConnectionGrant, GoogleAccount},
};
use hyper::Method;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
type Responses = VecDeque<(u16, Vec<u8>)>;
mod acceptance_cases;
type TargetedResponses = BTreeMap<(String, String), VecDeque<Result<(u16, Vec<u8>), Unavailable>>>;
pub struct Recorded {
    pub answers: Mutex<Responses>,
    pub calls: Mutex<Vec<Value>>,
    targeted_answers: Mutex<TargetedResponses>,
    failures: std::sync::atomic::AtomicUsize,
}
impl Recorded {
    pub fn new(answers: Vec<(u16, Vec<u8>)>) -> Arc<Self> {
        Arc::new(Self {
            answers: Mutex::new(answers.into()),
            calls: Mutex::new(vec![]),
            targeted_answers: Mutex::new(BTreeMap::new()),
            failures: std::sync::atomic::AtomicUsize::new(0),
        })
    }
    pub fn fail_next(&self) {
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn answer(&self, status: u16, body: Value) {
        self.answers
            .lock()
            .unwrap()
            .push_back((status, serde_json::to_vec(&body).unwrap()));
    }
    /// Concurrent job consumers must not consume an OAuth endpoint's recorded response.
    pub fn answer_for(&self, method: Method, target: &str, status: u16, body: Value) {
        self.targeted_answers
            .lock()
            .unwrap()
            .entry((method.to_string(), target.into()))
            .or_default()
            .push_back(Ok((status, serde_json::to_vec(&body).unwrap())));
    }
    pub fn fail_for(&self, method: Method, target: &str) {
        self.fail_for_error(
            method,
            target,
            crate::integrations::net::http::HttpError::OpenTimeout.into(),
        );
    }
    pub fn fail_for_error(&self, method: Method, target: &str, error: Unavailable) {
        self.targeted_answers
            .lock()
            .unwrap()
            .entry((method.to_string(), target.into()))
            .or_default()
            .push_back(Err(error));
    }
}
impl Client for Recorded {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: Method,
        target: &'a str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            assert!(matches!(
                host,
                "www.googleapis.com" | "oauth2.googleapis.com"
            ));
            let header = |name: &str| {
                headers
                    .iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.clone())
            };
            self.calls.lock().unwrap().push(json!({"method":method.to_string(),"path":target,"body":String::from_utf8(body).unwrap(),"content_type":header("Content-Type"),"access_token":header("Authorization").map(|v|v.strip_prefix("Bearer ").unwrap().to_owned())}));
            if self
                .failures
                .fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |n| n.checked_sub(1),
                )
                .is_ok()
            {
                return Err(crate::integrations::net::http::HttpError::OpenTimeout.into());
            }
            if let Some(answer) = self
                .targeted_answers
                .lock()
                .unwrap()
                .get_mut(&(method.to_string(), target.into()))
                .and_then(VecDeque::pop_front)
            {
                return answer;
            }
            Ok(self.answers.lock().unwrap().pop_front().unwrap_or_else(|| {
                panic!("unrecorded Google call: {method} {host}{target}; network prohibited")
            }))
        })
    }
}
pub fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/google_api.json")).unwrap()
}
pub fn config() -> Config {
    Config {
        client_id: "test-client-id".into(),
        client_secret: "FAKE-google-client-secret".into(),
        webhook_url: None,
    }
}
pub async fn install(app: &TestApp, recorded: Arc<Recorded>) {
    app.booted
        .app
        .google
        .install_api(Api::new(config(), recorded));
}
pub async fn grant(app: &TestApp, user_id: i64, expires: Timestamp, drive: bool) {
    let enc = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            GoogleAccount::save_connection(
                tx,
                &enc,
                ConnectionGrant {
                    user_id,
                    email: "david@gmail.test".into(),
                    access_token: Some("access-token".into()),
                    refresh_token: Some("refresh-token".into()),
                    access_token_expires_at: Some(expires),
                    scopes: drive.then(|| {
                        format!(
                            "{} {}",
                            campfire_db::models::google_account::CALENDAR_SCOPE,
                            campfire_db::models::google_account::DRIVE_SCOPE
                        )
                    }),
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn google_api_matches_recorded_rails_requests_refreshes_and_errors() {
    let v = vectors();
    let now = Timestamp::from_second(v["now"].as_i64().unwrap());
    for scenario in v["scenarios"].as_array().unwrap() {
        let name = scenario["name"].as_str().unwrap();
        let a = TestApp::boot().await.expect("default seed required");
        let responses = scenario["responses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p[0].as_u64().unwrap() as u16,
                    p[1].as_str().unwrap().as_bytes().to_vec(),
                )
            })
            .collect();
        let r = Recorded::new(responses);
        install(&a, r.clone()).await;
        grant(
            &a,
            DAVID,
            now.since(jiff::SignedDuration::from_secs(
                if matches!(name, "expired" | "invalid_grant") {
                    -3600
                } else {
                    3600
                },
            )),
            true,
        )
        .await;
        let api = a.booted.app.google.api();
        let result = match name {
            "drive_file" | "drive_forbidden" => {
                api.drive_file(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    v["drive_file"]["id"].as_str().unwrap(),
                    now,
                )
                .await
            }
            "drive_search" => {
                api.list_drive_files(a.db(), &a.booted.app.secrets, DAVID, "bob's\\draft", now)
                    .await
            }
            "pages" => {
                api.list_events(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    now.ago(jiff::SignedDuration::from_hours(1)),
                    now.since(jiff::SignedDuration::from_hours(1)),
                    now,
                )
                .await
            }
            _ => {
                let payload = if matches!(name, "insert" | "expired") {
                    json!({"summary":"Party"})
                } else {
                    json!({})
                };
                api.request(
                    a.db(),
                    &a.booted.app.secrets,
                    DAVID,
                    ApiRequest::calendar(Method::POST, api::EVENTS, Some(&payload)),
                    now,
                )
                .await
            }
        };
        match result {
            Ok(value) => {
                assert!(scenario["error"].is_null(), "{name}");
                assert_eq!(value, scenario["result"], "{name}");
            }
            Err(error) => {
                assert_eq!(
                    error.class(),
                    scenario["error"]["class"].as_str().unwrap(),
                    "{name}"
                );
                assert_eq!(
                    error.to_string(),
                    scenario["error"]["message"].as_str().unwrap(),
                    "{name}"
                );
            }
        }
        assert_eq!(
            *r.calls.lock().unwrap(),
            scenario["requests"].as_array().unwrap().clone(),
            "{name}"
        );
        assert!(r.answers.lock().unwrap().is_empty());
        let account = a
            .db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap();
        if name == "invalid_grant" {
            assert_eq!(
                account.disconnected_reason.as_deref(),
                Some("Google rejected the connection")
            );
        }
        if matches!(name, "expired" | "401") {
            assert_eq!(
                account
                    .access_token(&ArEncryption::new(&a.booted.app.secrets))
                    .unwrap()
                    .as_deref(),
                Some("refreshed-access-token")
            );
            assert_eq!(
                account.access_token_expires_at,
                Some(now.since(jiff::SignedDuration::from_hours(1)))
            );
        }
    }
}
#[tokio::test]
async fn google_api_connection_urls_and_id_token_claims_match_rails() {
    let v = vectors();
    let api = Api::new(config(), Recorded::new(vec![]));
    for (i, drive) in [false, true].into_iter().enumerate() {
        assert_eq!(
            api.authorize_url("http://test.host/google/callback", "signed-state", drive),
            v["authorize"][i]
        );
    }
    for case in v["email_tokens"].as_array().unwrap() {
        let result = api.email_from_id_token(
            case["token"].as_str().unwrap(),
            Timestamp::from_second(v["now"].as_i64().unwrap()),
        );
        if let Some(email) = case["email"].as_str() {
            assert_eq!(result.unwrap(), email);
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                case["error"].as_str().unwrap()
            );
        }
    }
}
#[tokio::test]
async fn google_api_unreadable_access_disconnects_before_http() {
    let a = TestApp::boot().await.expect("default seed required");
    let r = Recorded::new(vec![]);
    install(&a, r.clone()).await;
    let now = Timestamp::from_jiff(a.booted.app.clock.now());
    grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET access_token='broken-envelope' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let result = a
        .booted
        .app
        .google
        .api()
        .drive_file(
            a.db(),
            &a.booted.app.secrets,
            DAVID,
            "1AbcDefGhIjKlMnOpQrSt",
            now,
        )
        .await;
    assert!(matches!(result, Err(api::Error::Unauthorized(_))));
    assert!(r.calls.lock().unwrap().is_empty());
    assert_eq!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .disconnected_reason
            .as_deref(),
        Some(campfire_db::models::google_account::UNREADABLE_TOKEN_REASON)
    );
}

use super::{
    google_api_tests::{self as support, Recorded},
    google_test_support::QueueDrain,
};
use crate::{
    controllers::presenters::test_support::TestApp,
    errors::{Options, Subscriber},
    integrations::google::api,
};
use campfire_db::{Timestamp, models::google_calendar::DisconnectCleanupJob};
use campfire_kit::{Clock, FrozenClock};
use hyper::Method;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
struct Capture {
    db: campfire_db::Database,
    reports: Mutex<Vec<Value>>,
    arrived: tokio::sync::Notify,
}
impl Subscriber for Capture {
    fn report(&self, error: &anyhow::Error, options: &Options) {
        let remaining=self.db.read_blocking(|c|Ok(c.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Calendar::DisconnectCleanupJob'",[],|r|r.get::<_,i64>(0))?)).unwrap();
        assert_eq!(
            remaining, 0,
            "reports publish only after the fenced terminal outcome commits"
        );
        let mut value = serde_json::to_value(options).unwrap();
        let description = format!("{error:?}");
        value["tokens_absent"] =
            json!(!description.contains("access-token") && !description.contains("refresh-token"));
        self.reports.lock().unwrap().push(value);
        self.arrived.notify_one();
    }
}
#[tokio::test]
async fn google_cleanup_reports_the_rails_account_context_only_after_eight_committed_attempts() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_cleanup_report.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let clock = Arc::new(FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
        let a = TestApp::boot_with_clock(clock.clone()).await.unwrap();
        let r = Recorded::new(vec![]);
        a.booted
            .app
            .google
            .install_api(api::Api::new(support::config(), r.clone()));
        let capture = Arc::new(Capture {
            db: a.db().clone(),
            reports: Mutex::new(vec![]),
            arrived: tokio::sync::Notify::new(),
        });
        a.booted.app.errors.subscribe(capture.clone());
        let mut drain = QueueDrain::install(&a).await;
        for call in row["calls"].as_array().unwrap() {
            r.answer_for(
                Method::DELETE,
                call["path"].as_str().unwrap(),
                429,
                json!({}),
            );
        }
        let account_id = row["account_id"].as_i64();
        let snapshot = rails_compat::calendar_credentials::encrypt(
            &a.booted.app.secrets,
            &rails_compat::calendar_credentials::Snapshot {
                access_token: Some("access-token".into()),
                refresh_token: Some("refresh-token".into()),
                access_token_expires_at: Some("2026-03-12T16:00:00Z".parse().unwrap()),
            },
            clock.now(),
        );
        let job = DisconnectCleanupJob((vec!["orphan-id".into()], json!(snapshot), account_id));
        let arguments = serde_json::to_value(&job).unwrap();
        a.db()
            .write(move |tx| {
                tx.emit_after_commit(campfire_db::Event::job(&job));
                Ok(())
            })
            .await
            .unwrap();
        let mut schedules = vec![];
        for attempt in 1..=8 {
            let next =
                tokio::time::timeout(Duration::from_secs(5), drain.cleanup_attempt(&a, attempt))
                    .await
                    .expect("committed cleanup attempt did not drain");
            schedules.push(if let Some(next) = next {
                assert!(
                    capture.reports.lock().unwrap().is_empty(),
                    "attempt {attempt} must not report before exhaustion"
                );
                assert!(next > Timestamp::from_jiff(clock.now()));
                let saved = a
                    .db()
                    .read(|c| campfire_jobs::inspect::with_status(c, campfire_jobs::READY))
                    .await
                    .unwrap();
                let retry = saved
                    .iter()
                    .find(|j| j.class == "Calendar::DisconnectCleanupJob")
                    .expect("committed retry missing");
                assert_eq!(retry.arguments, arguments);
                clock.set(next.jiff());
                a.booted
                    .app
                    .jobs
                    .queue
                    .wake("Calendar::DisconnectCleanupJob");
                vec!["Calendar::DisconnectCleanupJob"]
            } else {
                vec![]
            });
        }
        assert_eq!(json!(schedules), row["jobs"]);
        tokio::time::timeout(Duration::from_secs(5), capture.arrived.notified())
            .await
            .expect("exhaustion never reached the error subscriber");
        assert_eq!(json!(*capture.reports.lock().unwrap()), row["reports"]);
        let calls = r
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| json!({"method":c["method"],"path":c["path"]}))
            .collect::<Vec<_>>();
        assert_eq!(json!(calls), row["calls"]);
    }
    println!(
        "Pinned Rails cleanup reporting: 2 accounts; 16 real runner attempts; 2 after-commit reports"
    );
}

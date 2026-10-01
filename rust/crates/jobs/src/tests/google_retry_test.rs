//! Actual Rails Calendar rescue handlers supply these six mixed-error sequences.
use super::*;
use crate::{retry::State, runner::decision_with_metadata, store::Claimed};

const GOOGLE: &str = "[Google::Client::Unavailable]";
const INHERITED: &str = "[Timeout::Error, Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout, ActiveRecord::Deadlocked, ActiveRecord::StatementTimeout, SQLite3::BusyException]";
fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/google_calendar_retries.json"
    ))
    .unwrap()
}
fn result(kind: &str) -> JobResult {
    Err(match kind {
        "google" => JobError::retry_group(anyhow::anyhow!("Google outage"), GOOGLE, 8),
        "inherited" => JobError::retry_group(anyhow::anyhow!("database timeout"), INHERITED, 5),
        _ => JobError::fail(anyhow::anyhow!("permanent")),
    })
}

#[test]
fn google_retry_decisions_match_serialized_rails_handler_counters() {
    let v = vectors();
    for case in v["cases"].as_array().unwrap() {
        let mut job = Claimed {
            id: 1,
            class: case["class"].as_str().unwrap().into(),
            arguments: "{\"n\":123}".into(),
            version: 1,
            attempts: 0,
            run_at: at(T0),
            created_at: at(T0),
        };
        for step in case["steps"].as_array().unwrap() {
            job.attempts += 1;
            let (decision, args) = decision_with_metadata(
                &RetryPolicy::application_job().attempts(8),
                &job,
                &result(step["kind"].as_str().unwrap()),
                0.0,
            );
            match decision {
                Decision::Reschedule {
                    wait,
                    reset_attempts,
                    ..
                } => {
                    assert_eq!(wait.as_secs_f64(), step["wait"].as_f64().unwrap(), "{case}");
                    assert!(!reset_attempts);
                }
                Decision::Fail(_) => assert_eq!(step["retry"], false),
                other => panic!("unexpected decision: {other:?}"),
            }
            if let Some(args) = args {
                job.arguments = args;
            }
            let state = State::from_value(serde_json::from_str(&job.arguments).unwrap());
            assert_eq!(serde_json::to_value(state.counts).unwrap(), step["counts"]);
            assert_eq!(step["executions"], job.attempts);
            assert_eq!(
                crate::retry::arguments(serde_json::from_str(&job.arguments).unwrap()),
                serde_json::json!({"n":123})
            );
        }
    }
    println!("Pinned Rails Calendar retry sequences: 6 exercised; 0 skipped");
}

#[tokio::test]
async fn google_retry_groups_survive_runner_restarts_and_manual_retry_resets_them() {
    let v = vectors();
    for case in
        v["cases"].as_array().unwrap().iter().filter(|case| {
            case["class"] == "Calendar::InboundSyncJob" && case["name"] != "permanent"
        })
    {
        let steps = Arc::new(case["steps"].as_array().unwrap().clone());
        let position = Arc::new(AtomicUsize::new(0));
        let (performed, mut receiver) = mpsc::unbounded_channel();
        let new_registry = || {
            let mut registry = Registry::new();
            let for_job = steps.clone();
            let next_step = position.clone();
            let performed = performed.clone();
            registry.register(move |(), job: Echo, execution: Execution| {
                let steps = for_job.clone();
                let index = next_step.fetch_add(1, Ordering::SeqCst);
                let performed = performed.clone();
                async move {
                    performed.send((job.n, execution.executions)).unwrap();
                    if let Some(step) = steps.get(index) {
                        result(step["kind"].as_str().unwrap())
                    } else {
                        Ok(Outcome::Done)
                    }
                }
            });
            registry
        };
        let registry = new_registry();
        let h = harness(&registry, &config());
        let id = h.enqueue(Echo { n: 123 }).await;
        for (index, step) in steps.iter().enumerate() {
            let runner = start(h.db.clone(), h.queue.clone(), new_registry(), (), config());
            assert_eq!(next(&mut receiver).await, (123, (index + 1) as u32));
            h.wait_for("the retry group's durable outcome", |jobs| {
                jobs[0].status == if step["retry"] == true { READY } else { FAILED }
            })
            .await;
            runner.shutdown(Duration::from_secs(5)).await;
            let job = h.job(id).unwrap();
            assert_eq!(job.arguments, serde_json::json!({"n":123}));
            let raw: String =
                h.db.read_blocking(|c| {
                    Ok(c.query_row(
                        "SELECT arguments FROM background_jobs WHERE id=?",
                        [id],
                        |r| r.get(0),
                    )?)
                })
                .unwrap();
            assert_eq!(
                serde_json::to_value(State::from_value(serde_json::from_str(&raw).unwrap()).counts)
                    .unwrap(),
                step["counts"]
            );
            if step["retry"] == true {
                let seconds = (job.run_at.as_microsecond() - job.updated_at.as_microsecond())
                    as f64
                    / 1_000_000.0;
                let minimum = step["wait"].as_f64().unwrap();
                assert!(
                    seconds >= minimum && seconds < minimum + (minimum - 2.0) * 0.15,
                    "Rails polynomial jitter: {seconds}"
                );
                h.travel(3600);
            }
        }
        assert!(
            h.db.write(move |tx| inspect::retry_failed(tx, id))
                .await
                .unwrap()
        );
        let raw: String =
            h.db.read_blocking(|c| {
                Ok(c.query_row(
                    "SELECT arguments FROM background_jobs WHERE id=?",
                    [id],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(raw, "{\"n\":123}");
        let runner = start(h.db.clone(), h.queue.clone(), new_registry(), (), config());
        assert_eq!(next(&mut receiver).await, (123, 1));
        h.wait_for("manual retry completes", |jobs| jobs.is_empty())
            .await;
        runner.shutdown(Duration::from_secs(5)).await;
    }
}

#[test]
fn google_cleanup_exhaustion_and_interrupted_execution_have_separate_budgets() {
    let policy = RetryPolicy::application_job().attempts(8);
    let mut job = Claimed {
        id: 1,
        class: "Cleanup".into(),
        arguments: "[]".into(),
        version: 1,
        attempts: 0,
        run_at: at(T0),
        created_at: at(T0),
    };
    let result = Err(JobError::RetryGroup {
        error: anyhow::anyhow!("outage"),
        key: GOOGLE,
        attempts: 8,
        discard_exhausted: true,
    });
    for count in 1..=8 {
        job.attempts = count;
        let (decision, args) = decision_with_metadata(&policy, &job, &result, 0.0);
        job.arguments = args.unwrap();
        if count == 8 {
            assert_eq!(decision, Decision::Delete)
        } else {
            assert!(matches!(decision, Decision::Reschedule { .. }))
        }
    }
    let state = State::from_value(serde_json::from_str(&job.arguments).unwrap());
    assert_eq!(state.interrupted_executions(12), 4);
    let (_, args) = decision_with_metadata(
        &policy,
        &job,
        &Ok(Outcome::Again(Duration::from_secs(1))),
        0.0,
    );
    assert_eq!(args.as_deref(), Some("[]"));
}

#[tokio::test]
async fn google_retry_metadata_rolls_back_and_completion_retry_does_not_double_count() {
    let rejected = Arc::new(AtomicUsize::new(0));
    let observed = rejected.clone();
    let mut registry = Registry::new();
    registry.register(|(), _: Echo, _: Execution| async { result("google") });
    let h = harness(&registry, &config());
    h.db.write(move |tx| {
        tx.conn().create_scalar_function("observe_rejected_outcome",0,rusqlite::functions::FunctionFlags::SQLITE_UTF8,move |_|{observed.fetch_add(1,Ordering::SeqCst);Ok(1)})?;
        tx.conn().execute_batch("CREATE TRIGGER reject_retry_outcome BEFORE UPDATE OF status ON background_jobs WHEN NEW.status='ready' BEGIN SELECT observe_rejected_outcome(); SELECT RAISE(ABORT,'retry outcome rejected'); END")?;Ok(())
    }).await.unwrap();
    let id = h.enqueue(Echo { n: 123 }).await;
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    h.wait_for("a rejected completion write", |_| {
        rejected.load(Ordering::SeqCst) > 0
    })
    .await;
    let raw: String =
        h.db.read_blocking(|c| {
            Ok(c.query_row(
                "SELECT arguments FROM background_jobs WHERE id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        raw, "{\"n\":123}",
        "metadata update rolled back with the failed reschedule"
    );
    h.db.write(|tx| {
        tx.conn()
            .execute_batch("DROP TRIGGER reject_retry_outcome")?;
        Ok(())
    })
    .await
    .unwrap();
    h.wait_for("the same completion commits", |jobs| {
        jobs[0].status == READY
    })
    .await;
    runner.shutdown(Duration::from_secs(5)).await;
    let raw: String =
        h.db.read_blocking(|c| {
            Ok(c.query_row(
                "SELECT arguments FROM background_jobs WHERE id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        State::from_value(serde_json::from_str(&raw).unwrap()).counts[GOOGLE],
        1
    );
    assert_eq!(h.job(id).unwrap().attempts, 1);
}

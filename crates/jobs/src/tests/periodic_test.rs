use super::*;
use crate::periodic::{Periodic, Task, interval_from_env, parse_interval};

fn logging(name: &'static str, interval: u64) -> Task<Log> {
    Task::new(name, Duration::from_secs(interval), move |log: Log| async move {
        log.lock().unwrap().push(name.to_string());
        Ok(())
    })
}

fn secs(seconds: i64) -> jiff::SignedDuration {
    jiff::SignedDuration::from_secs(seconds)
}

/// Each tick runs the tasks whose interval has elapsed since their last run, in order.
#[tokio::test]
async fn tasks_run_on_their_intervals() {
    let log: Log = Default::default();
    let mut periodic = Periodic::new("Periodic");
    periodic.task(logging("scheduled_messages", 30)).task(logging("stuck_rooms", 300));
    assert_eq!(periodic.tick_interval(), Some(Duration::from_secs(30)));

    let t0 = at(T0);
    assert_eq!(periodic.tick(log.clone(), t0).await, ["scheduled_messages", "stuck_rooms"]);
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(29))).await, Vec::<&str>::new());
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(30))).await, ["scheduled_messages"]);
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(299))).await, ["scheduled_messages"]);
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(300))).await, ["stuck_rooms"]);
    assert_eq!(log.lock().unwrap().len(), 5);
}

/// A task that fails, or panics, doesn't stop the tasks after it or later ticks, and runs again
/// on the next tick, since only a success counts as a run.
#[tokio::test]
async fn a_failing_task_is_isolated_and_retried_next_tick() {
    let log: Log = Default::default();
    let failures = Arc::new(AtomicUsize::new(0));
    let mut periodic = Periodic::new("Periodic");
    let counted = failures.clone();
    periodic
        .task(Task::new("event_reminders", Duration::from_secs(30), move |_: Log| {
            let failures = counted.clone();
            async move {
                failures.fetch_add(1, Ordering::SeqCst);
                anyhow::bail!("database is locked")
            }
        }))
        .task(Task::new("poll_closing", Duration::from_secs(30), |_: Log| async { panic!("boom") }))
        .task(logging("streaming_messages", 30));

    let t0 = at(T0);
    assert_eq!(periodic.tick(log.clone(), t0).await, ["event_reminders", "poll_closing", "streaming_messages"]);
    assert_eq!(*log.lock().unwrap(), ["streaming_messages"]);
    // A second later, the failures run again; the success waits its interval.
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(1))).await, ["event_reminders", "poll_closing"]);
    assert_eq!(failures.load(Ordering::SeqCst), 2);
    assert_eq!(periodic.tick(log.clone(), t0.since(secs(30))).await, ["event_reminders", "poll_closing", "streaming_messages"]);
    assert_eq!(*log.lock().unwrap(), ["streaming_messages", "streaming_messages"]);
}

/// `clear_bot_tokens_once`: done once per process, retried until it succeeds.
#[tokio::test]
async fn a_once_task_does_its_work_once_it_succeeds() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let counted = attempts.clone();
    let mut periodic = Periodic::new("Periodic");
    periodic.task(Task::once("clear_plaintext_bot_tokens", Duration::from_secs(86_400), move |_: Log| {
        let attempts = counted.clone();
        async move {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                anyhow::bail!("database is locked");
            }
            Ok(())
        }
    }));
    let t0 = at(T0);
    periodic.tick(Log::default(), t0).await;
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    periodic.tick(Log::default(), t0.since(secs(1))).await;
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "failed, so it ran again");
    for days in 1..4 {
        assert_eq!(periodic.tick(Log::default(), t0.since(secs(86_400 * days + 1))).await, ["clear_plaintext_bot_tokens"]);
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "its work is done once");
}

#[test]
#[should_panic(expected = r#"periodic task "stuck_rooms" is registered twice"#)]
fn task_names_are_unique() {
    Periodic::new("Periodic").task(logging("stuck_rooms", 300)).task(logging("stuck_rooms", 30));
}

/// The loop ticks every shortest interval until it's stopped.
#[tokio::test]
async fn the_loop_ticks_until_stopped() {
    let log: Log = Default::default();
    let mut periodic = Periodic::new("Huddle reconciliation");
    periodic.task(Task::new("reconcile", Duration::from_millis(20), {
        let log = log.clone();
        move |_: ()| {
            let log = log.clone();
            async move {
                log.lock().unwrap().push("reconcile".into());
                Ok(())
            }
        }
    }));
    // A real clock: the loop's interval is real time.
    let (stop, stopping) = tokio::sync::watch::channel(false);
    let handle = periodic.spawn((), Arc::new(campfire_db::SystemClock), stopping);
    tokio::time::sleep(Duration::from_millis(200)).await;
    stop.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(1), handle).await.expect("stopped").unwrap();
    let ticks = log.lock().unwrap().len();
    assert!((3..=11).contains(&ticks), "{ticks} ticks");
}

#[test]
fn intervals_are_positive_finite_seconds() {
    assert_eq!(parse_interval("30", "X").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_interval(" 0.5 ", "X").unwrap(), Duration::from_millis(500));
    for bad in ["0", "-5", "", "soon", "inf", "NaN"] {
        assert_eq!(parse_interval(bad, "EVENT_REMINDERS_INTERVAL").unwrap_err().to_string(), "EVENT_REMINDERS_INTERVAL must be a positive finite number", "{bad:?}");
    }
    let env = |name: &str| (name == "RETENTION_PRUNE_INTERVAL").then(|| "3600".to_string());
    assert_eq!(interval_from_env(env, "RETENTION_PRUNE_INTERVAL", Duration::from_secs(86_400)).unwrap(), Duration::from_secs(3600));
    assert_eq!(interval_from_env(env, "EVENT_REMINDERS_INTERVAL", Duration::from_secs(30)).unwrap(), Duration::from_secs(30));
}

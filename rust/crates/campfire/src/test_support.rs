//! Bounded waits for integration tests running beside other work on a busy host.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::FutureExt;

pub const WAIT: Duration = Duration::from_secs(30);

pub async fn wait<T>(what: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(WAIT, future)
        .await
        .unwrap_or_else(|_| panic!("timed out after {WAIT:?} waiting for {what}"))
}

pub async fn eventually<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    wait(what, async {
        loop {
            if check().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
}

pub type ServerStartup = tokio::sync::oneshot::Receiver<Result<(), String>>;

/// Retain a startup reporter outside the server future: an early error or panic must not
/// disappear when the ready callback is dropped.
pub fn spawn_server<F, Fut>(serve: F) -> (tokio::task::JoinHandle<std::io::Result<()>>, ServerStartup)
where
    F: FnOnce(Box<dyn FnOnce() + Send>) -> Fut + Send + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static,
{
    let (ready, started) = tokio::sync::oneshot::channel();
    let ready = Arc::new(Mutex::new(Some(ready)));
    let on_ready = ready.clone();
    let server = tokio::spawn(async move {
        let outcome = std::panic::AssertUnwindSafe(async move {
            serve(Box::new(move || {
                let sender = on_ready.lock().unwrap().take();
                if let Some(sender) = sender {
                    let _ = sender.send(Ok(()));
                }
            }))
            .await
        })
        .catch_unwind()
        .await;
        let sender = ready.lock().unwrap().take();
        if let Some(sender) = sender {
            let message = match &outcome {
                Ok(Ok(())) => "server exited before reporting readiness".to_string(),
                Ok(Err(error)) => format!("server startup failed: {error} ({:?})", error.kind()),
                Err(panic) => {
                    let message = panic.downcast_ref::<&str>().copied()
                        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                        .unwrap_or("non-string panic");
                    format!("server panicked before reporting readiness: {message}")
                }
            };
            let _ = sender.send(Err(message));
        }
        match outcome {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    });
    (server, started)
}

#[tokio::test]
async fn server_startup_panics_reach_the_reporter_and_join_handle() {
    let (server, started) = spawn_server(|_| {
        std::future::poll_fn(|_| -> std::task::Poll<std::io::Result<()>> {
            panic!("intentional startup panic probe");
        })
    });
    let report = wait("startup panic report", started).await.unwrap().unwrap_err();
    assert_eq!(report, "server panicked before reporting readiness: intentional startup panic probe");
    assert!(wait("panicking server task to finish", server).await.unwrap_err().is_panic());
}

// Serialize listener selection within this process. Front fixtures transfer the owned
// listeners into their server, keeping those ports reserved throughout startup.
pub static LISTENER_BINDING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn bind_listener() -> tokio::net::TcpListener {
    let _binding = LISTENER_BINDING.lock().await;
    bind_listener_locked().await
}

pub async fn bind_listener_locked() -> tokio::net::TcpListener {
    let Ok(range) = std::env::var("CABLE_TEST_PORT_RANGE") else {
        return tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    };
    let (start, end) = range
        .split_once('-')
        .expect("CABLE_TEST_PORT_RANGE=start-end");
    let (start, end): (u16, u16) = (start.parse().unwrap(), end.parse().unwrap());
    assert!(start > 0 && start <= end, "invalid test port range");
    for port in start..=end {
        match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
            Ok(listener) => return listener,
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => continue,
            Err(error) => panic!("binding {port}: {error}"),
        }
    }
    panic!("no free listening port in {range}");
}

/// Keep parser CPU budgets independent of time spent descheduled by other tests/processes.
/// These synchronous parsing tests do all their measured work on the calling thread.
pub fn cpu_time() -> Duration {
    #[cfg(target_os = "linux")]
    {
        let mut time = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `time` is writable; the clock reads CPU usage of the calling thread only.
        assert_eq!(
            unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) },
            0
        );
        Duration::new(
            time.tv_sec.try_into().unwrap(),
            time.tv_nsec.try_into().unwrap(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

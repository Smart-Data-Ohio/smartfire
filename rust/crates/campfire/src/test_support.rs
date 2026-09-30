//! Bounded waits for integration tests running beside other work on a busy host.

use std::future::Future;
use std::time::Duration;

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

// The front server takes port numbers rather than already-bound listeners. Hold this lock
// from reserving its ports through its startup acknowledgement; other test listeners use it too.
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

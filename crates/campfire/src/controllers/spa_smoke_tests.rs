//! Real-server SPA smoke: the actual `campfire` binary with the built SPA embedded, driven
//! through a real Chromium like a person: password sign-in on the real forms (including the real
//! two-factor enrollment), the SPA shell, an open room, a sent message, and a reload proving it
//! persisted. No mocks, no stubbed network, no test-only controller.
//!
//! Selected by `ci/ignored-tests.json`'s `pwa` suite, which builds the binary with the same
//! `SPA_DIST` as the test.

use campfire_db::User;

use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, TestApp};

/// The browser-facing port. Only these two ports are ever used, so parallel workers coordinate
/// by taking different ones (this smoke defaults to 4320); anything else
/// fails loudly instead of colliding with another suite's server.
fn smoke_port() -> u16 {
    match std::env::var("SMARTFIRE_E2E_PORT").as_deref() {
        Err(_) | Ok("4320") => 4320,
        Ok("4321") => 4321,
        Ok(other) => panic!("SMARTFIRE_E2E_PORT must be 4320 or 4321, got {other:?}"),
    }
}

/// The `campfire` server binary this smoke boots: CI's (`SPA_SMOKE_BINARY`, built by
/// `ci/correctness.sh pwa` with the same `SPA_DIST`), or the checkout's debug build locally.
fn server_binary(root: &std::path::Path) -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("SPA_SMOKE_BINARY") {
        return std::path::PathBuf::from(path);
    }
    std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("debug/campfire")
}

/// A test server that cannot outlive the test.
struct ServerProcess(std::process::Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// `docker run --rm` leaves the container running when the CLI is killed.
struct Container(String);

impl Drop for Container {
    fn drop(&mut self) {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", &self.0])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// One plain HTTP/1.1 request against the front server: status plus headers, read with a
/// deadline. The smoke only needs statuses and lengths, never bodies.
async fn get_status_and_headers(port: u16, path: &str) -> Option<(u16, Vec<(String, String)>)> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let mut stream = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)),
    )
    .await
    .ok()?
    .ok()?;
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.write_all(request.as_bytes()),
    )
    .await
    .ok()?
    .ok()?;
    let mut raw = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let mut chunk = [0u8; 4096];
        let read = tokio::time::timeout_at(deadline, stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..read]);
        if raw.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if raw.len() > 65536 {
            return None;
        }
    }
    let head = std::str::from_utf8(&raw).ok()?;
    let (status_line, headers) = head.split_once("\r\n")?;
    let status: u16 = status_line.split_whitespace().nth(1)?.parse().ok()?;
    let headers = headers
        .split("\r\n")
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(name, value)| (name.to_ascii_lowercase(), value.to_string()))
        })
        .collect();
    Some((status, headers))
}

/// `KEY=value` lines, skipping blanks and comments (`parity/.env.reference`).
fn dotenv(path: &std::path::Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && line.contains('='))
        .map(|line| {
            line.split_once('=')
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .unwrap()
        })
        .collect()
}

/// The last lines of the server log, for the failure output (the run directory is deleted).
fn tail(log: &str) -> String {
    let lines: Vec<&str> = log.lines().collect();
    lines[lines.len().saturating_sub(60)..].join("\n")
}

#[tokio::test]
#[ignore = "requires production SPA dist and Chromium; run ci/correctness.sh pwa"]
async fn real_server_spa_smoke_sends_and_persists_a_message() {
    let port = smoke_port();
    assert!(
        campfire_spa::built(),
        "build frontend/dist before compiling this browser test (ci/correctness.sh pwa)"
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let binary = server_binary(&root);
    assert!(
        binary.is_file(),
        "build the real server first: {} is missing (ci/correctness.sh pwa builds it with the same SPA_DIST)",
        binary.display()
    );

    // An isolated private copy of the frozen seed, prepared through the models: David unenrolled
    // so the browser performs the real two-factor enrollment, like the PWA suite.
    let app = TestApp::boot_frozen_with_env(&[
        ("RAILS_ENV", "test"),
        ("SPA_ENABLED", "1"),
        ("SPA_DEFAULT", "next"),
    ])
    .await
    .expect("SPA smoke requires the restored default seed");
    app.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let email = app
        .db()
        .read(|conn| Ok(User::find(conn, DAVID)?.email_address.unwrap()))
        .await
        .unwrap();
    let labels: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("parity/.seed/default/labels.json")).unwrap(),
    )
    .unwrap();
    let frozen = labels["clock.now"]
        .as_str()
        .expect("the seed labels its frozen clock")
        .to_owned();
    // Hand the seed copy to the real server: its jobs stop and its connections close here.
    let (live, run) = app.stop_jobs().await;
    drop(live);
    let database = run.path().join("db/production.sqlite3");
    let files = run.path().join("files");
    let log_path = run.path().join("server.log");

    // Serialize port selection with the other listener fixtures, and hold the lease until the
    // server answers, so nothing else can take the port mid-startup.
    let _binding = crate::test_support::LISTENER_BINDING.lock().await;
    match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
        Ok(probe) => drop(probe),
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            panic!(
                "port {port} (SMARTFIRE_E2E_PORT) is already in use: stop the other server or rerun with the other smoke port (4320/4321): {error}"
            )
        }
        Err(error) => panic!("probing SMARTFIRE_E2E_PORT={port}: {error}"),
    }
    let log = std::fs::File::create(&log_path).unwrap();
    let mut command = std::process::Command::new(&binary);
    command
        .arg("server")
        .current_dir(&root)
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    for (key, value) in dotenv(&root.join("parity/.env.reference")) {
        command.env(key, value);
    }
    for (key, value) in labels.as_object().unwrap() {
        if let Some(name) = key.strip_prefix("reference_env.") {
            command.env(
                name,
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string()),
            );
        }
    }
    // TARGET_PORT is HTTP_PORT: the front serves everything on the one smoke port instead of
    // opening a second listener. No test-only route is ever enabled in this server.
    command
        .env("RAILS_ENV", "test")
        .env("CAMPFIRE_STORAGE_PATH", run.path())
        .env("CAMPFIRE_DATABASE_PATH", &database)
        .env("CAMPFIRE_FILES_PATH", &files)
        .env("CAMPFIRE_FROZEN_TIME", &frozen)
        .env("DISABLE_SSL", "1")
        .env("HTTP_PORT", port.to_string())
        .env("TARGET_PORT", port.to_string())
        .env("SPA_ENABLED", "1")
        .env("SPA_DEFAULT", "next")
        .env("RAILS_LOG_LEVEL", "warn")
        .env_remove("TLS_DOMAIN")
        .env_remove("WS11UI_LEDGER_HOST");
    let mut server = ServerProcess(command.spawn().expect("the real campfire server starts"));

    let target = format!("http://127.0.0.1:{port}");
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match server.0.try_wait() {
            Ok(Some(status)) => panic!(
                "the real server exited during startup ({status})\n{}",
                tail(&std::fs::read_to_string(&log_path).unwrap_or_default())
            ),
            Ok(None) => {}
            Err(error) => panic!("polling the real server: {error}"),
        }
        if get_status_and_headers(port, "/up")
            .await
            .is_some_and(|(status, _)| status == 200)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            println!(
                "SPA_SMOKE_SERVER_LOG_TAIL\n{}",
                tail(&std::fs::read_to_string(&log_path).unwrap_or_default())
            );
            panic!("the real server never answered /up within 30s (see the log tail above)");
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    // The binary serves the same production dist this harness compiled against: a stub build
    // 404s these content-hashed files, and a stale dist differs in length.
    for file in campfire_spa::files().iter().take(4) {
        let path = format!("/app/{}", file.path);
        let (status, headers) = get_status_and_headers(port, &path)
            .await
            .unwrap_or_else(|| panic!("{path}: the real server stopped answering"));
        assert_eq!(
            status, 200,
            "{path}: the binary serves the same production dist (a stub 404s)"
        );
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .unwrap_or_else(|| panic!("{path}: no content-length in {headers:?}"))
            .1
            .parse::<usize>()
            .unwrap();
        assert_eq!(
            length,
            file.identity.len(),
            "{path}: the binary's bytes match this harness's SPA_DIST"
        );
    }
    println!(
        "SPA_SMOKE_SERVER target={target} room={ALL_TALK} binary={} dist_files={}",
        binary.display(),
        campfire_spa::files().len()
    );
    drop(_binding);

    let local = std::env::var("SPA_SMOKE_LOCAL").as_deref() == Ok("1");
    // Killing a Docker CLI alone leaves its browser container running.
    let container = (!local).then(|| Container(format!("spa-smoke-{}", std::process::id())));
    let mut command = if local {
        tokio::process::Command::new("node")
    } else {
        let mut docker = tokio::process::Command::new("docker");
        docker
            .args([
                "run",
                "--rm",
                "--init",
                "--network",
                "host",
                "--ipc",
                "host",
                "--cpus",
                "1",
            ])
            .arg("--name")
            .arg(&container.as_ref().unwrap().0)
            .arg("--volume")
            .arg(format!("{}:{}:ro", root.display(), root.display()))
            .arg("--env")
            .arg(format!("SPA_SMOKE_TARGET={target}"))
            .arg("--env")
            .arg(format!("SPA_SMOKE_EMAIL={email}"))
            .arg("--env")
            .arg(format!("SPA_SMOKE_ROOM={ALL_TALK}"))
            .arg(
                std::env::var("PWA_PLAYWRIGHT_IMAGE")
                    .expect("ci/correctness.sh pwa supplies the pinned browser image"),
            )
            .arg("node");
        docker
    };
    let output = command
        .arg(root.join("test-support/spa_smoke_browser.mjs"))
        .env("SPA_SMOKE_TARGET", &target)
        .env("SPA_SMOKE_EMAIL", &email)
        .env("SPA_SMOKE_ROOM", ALL_TALK.to_string())
        .kill_on_drop(true)
        .output();
    // Every browser wait is bounded; this catches anything that still hangs.
    let output = tokio::time::timeout(std::time::Duration::from_secs(300), output)
        .await
        .expect("the SPA smoke run finishes within five minutes")
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{stdout}");
    if !output.status.success() {
        println!(
            "SPA_SMOKE_SERVER_LOG_TAIL\n{}",
            tail(&std::fs::read_to_string(&log_path).unwrap_or_default())
        );
    }
    assert!(
        output.status.success(),
        "real SPA smoke failed\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("SPA_SMOKE_RECEIPT "),
        "browser completed sign-in, send and reload persistence"
    );
}

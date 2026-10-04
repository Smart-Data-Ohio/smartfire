//! Original WS12 system declarations: production router/assets/forms/Cable in Chromium.
//! The same committed driver is replayed against Rails; the Unix forwarder keeps
//! each browser inside a network-isolated pinned image, including mutation runs.
//! Run explicitly with `parity/system/ws12`; the normal CI toolchain has no Node/browser.
use super::presenters::test_support::{Req, TestApp};
use serde_json::Value;
use sha2::{Digest, Sha256};
struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn compare(key: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_browser_remaining.json"
    ))
    .unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == key)
        .unwrap();
    let digest = Sha256::digest(
        ["Dockerfile.playwright", "package.json", "package-lock.json"]
            .into_iter()
            .flat_map(|f| std::fs::read(root.join("parity").join(f)).unwrap())
            .collect::<Vec<_>>(),
    );
    let image = format!("ws12-playwright:{:x}", digest)[..28].to_owned();
    let mut measured = Vec::new();
    for size in [10, 100] {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db().write(move|tx|{
            for q in setup.as_array().unwrap(){tx.conn().execute_batch(q.as_str().unwrap())?;}
            for i in 0..size {let uid=901898000+i;
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Quiet member',0,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,?,'mentions',?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
            }Ok(())
        }).await.unwrap();
        if key == "c225" {
            campfire_db::models::board_automations::dispatch_digests(
                app.db(),
                campfire_db::Timestamp::from_second(1772467200),
            )
            .await
            .unwrap();
        }
        let mut david = app.david();
        let path = match key {
            "c221" | "c222" => "/rooms/boards/699448332/automations",
            "c224" => "/threads/4/work/handoff/new",
            _ => "/rooms/699448332",
        };
        david.get(path).await;
        let queries = app.db().capture_queries();
        let response = david.send(Req::new(campfire_kit::Method::GET, path)).await;
        app.db().stop_capturing_queries();
        assert_eq!(
            response.status.as_u16(),
            200,
            "{key}: read probe before browser"
        );
        measured.push(queries.lock().unwrap().len());
        let cache = std::env::var_os("WS12_BROWSER_SCRATCH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache")
                    })
                    .join("rust-port/ws12")
            });
        std::fs::create_dir_all(&cache).unwrap();
        let network = tempfile::Builder::new()
            .prefix("browser-")
            .tempdir_in(cache)
            .unwrap();
        let listener = crate::test_support::bind_listener().await;
        let target = format!("http://{}", listener.local_addr().unwrap());
        let router = app.booted.router.clone();
        let _server = Server(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }));
        let socket = network.path().join("upstream.sock");
        let mut forward = tokio::process::Command::new("node")
            .arg(root.join("parity/capture/forward.ts"))
            .arg(&socket)
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        crate::test_support::eventually("WS12 private browser forwarder", || async {
            socket.exists()
        })
        .await;
        let output = tokio::process::Command::new("docker")
            .args([
                "run",
                "--rm",
                "--init",
                "--network",
                "none",
                "--ipc",
                "host",
                "--cpus",
                "1",
            ])
            .arg("--volume")
            .arg(format!("{}:{}:ro", root.display(), root.display()))
            .arg("--volume")
            .arg(format!(
                "{}:{}",
                network.path().display(),
                network.path().display()
            ))
            .arg("--env")
            .arg(format!("PARITY_UPSTREAM_SOCKET={}", socket.display()))
            .arg("--env")
            .arg(format!("WS12_BROWSER_TARGET={target}"))
            .arg("--env")
            .arg(format!("WS12_BROWSER_CASE={key}"))
            .arg("--env")
            .arg(format!(
                "WS12_BROWSER_LABELS={}",
                root.join("parity/.seed/default/labels.json").display()
            ))
            .arg(&image)
            .arg("node")
            .arg(root.join("reference-tools/users/ws12_browser_remaining.mjs"))
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        forward.kill().await.unwrap();
        forward.wait().await.unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{key}: original browser assertions failed\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!(
        "WS12_BROWSER_READS {key} members=10/100 SELECTs={}/{}",
        measured[0], measured[1]
    );
    assert_eq!(
        measured[0], measured[1],
        "{key}: page reads stay flat across room members"
    );
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c221_original_named_system_assertions() {
    compare("c221").await;
    compare_cutover("inbox").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c222_original_named_system_assertions() {
    compare("c222").await;
    compare_cutover("inbox-filter").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c223_original_named_system_assertions() {
    compare("c223").await;
    compare_cutover("work").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c224_original_named_system_assertions() {
    compare("c224").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c225_original_named_system_assertions() {
    compare("c225").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c226_original_named_system_assertions() {
    compare("c226").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and pinned Chromium; run parity/system/ws12"]
async fn ws12_browser_c227_original_named_system_assertions() {
    compare("c227").await;
}

async fn compare_cutover(scenario: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let test_host = std::env::current_exe().unwrap();
    let target = test_host.parent().unwrap().parent().unwrap();
    let binary = if let Some(path) = std::env::var_os("WS11UI_BROWSER_BINARY") {
        std::path::PathBuf::from(path)
    } else {
        // The ignored-test CI job invokes nextest directly; unlike the shell
        // entry point, it has only built the test harness. Build the real work host.
        if scenario == "work" {
            let output = tokio::process::Command::new("cargo")
                .args([
                    "build", "--locked", "-j", "4", "-p", "campfire", "--bin", "campfire",
                ])
                .arg("--manifest-path")
                .arg(root.join("rust/Cargo.toml"))
                .arg("--target-dir")
                .arg(target.parent().unwrap())
                .kill_on_drop(true)
                .output()
                .await
                .unwrap();
            assert!(
                output.status.success(),
                "work browser host build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        target.join("campfire")
    };
    if scenario == "work" {
        assert!(binary.is_file(), "current campfire binary is missing");
    }
    let namespace = std::env::var("PARITY_NAMESPACE")
        .unwrap_or_else(|_| format!("ws11ui-ci-{}", std::process::id()));
    let offset = match scenario {
        "inbox" => 0,
        "inbox-filter" => 3,
        "work" => 6,
        _ => unreachable!(),
    };
    let mut command = tokio::process::Command::new("python3");
    command
        .arg(root.join("rust/reference-tools/views/agents_ui/check_cutover_browser.py"))
        .arg("--binary")
        .arg(binary)
        .arg("--test-host")
        .arg(test_host)
        .arg("--scenario")
        .arg(scenario)
        .env("PARITY_NAMESPACE", format!("{namespace}-{scenario}"))
        .current_dir(root);
    // The ignored-test CI job runs four nextest workers. Give the three paired
    // hosts disjoint ports and container namespaces rather than serializing CI.
    for (key, default) in [
        ("WS11UI_SYSTEM_REFERENCE_PORT", 52798_u16),
        ("WS11UI_SYSTEM_CANDIDATE_PORT", 52799_u16),
        ("WS11UI_SYSTEM_TARGET_PORT", 52797_u16),
    ] {
        let base = std::env::var(key)
            .map(|value| value.parse::<u16>().unwrap())
            .unwrap_or(default);
        command.env(key, (base + offset).to_string());
    }
    let output = command.kill_on_drop(true).output().await.unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{scenario}: paired Rails/Rust sequence or writer control failed\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

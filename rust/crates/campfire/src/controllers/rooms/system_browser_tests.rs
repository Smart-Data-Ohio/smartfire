//! Browser acceptance uses the real app/router/assets/Cable on a private parity seed.
//! The fixture routes below exist only in this test module; they never ship in the binary.
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, TestApp};
use axum::{
    Json,
    extract::Path,
    routing::{get, post},
};
use campfire_db::{Membership, Room, RoomType, User};
use serde_json::json;

#[tokio::test]
#[ignore = "requires Docker and the pinned Playwright image; run parity/system/ws13"]
async fn huddle_system_cases_in_real_browser() {
    let test = TestApp::boot_with_huddle(super::call_channel_tests::configured())
        .await
        .expect("build parity seeds");
    let app = test.booted.app.clone();
    let fixtures = app.clone();
    let state = app.clone();
    let router = test.booted.router.clone()
        .route("/__ws13__/fixture", post(move |Json(options): Json<serde_json::Value>| {
            let app = fixtures.clone();
            async move {
                let secrets = app.secrets.clone();
                Json(app.db.write(move |tx| {
                    use campfire_kit::Crypto;
                    let room = Room::create_for(tx, RoomType::Stage, Some("Town Hall"), DAVID, &[DAVID,JASON,KEVIN])?;
                    if options["speaker"]==true {
                        let mut member = Membership::find_by_room_and_user(tx.conn(),room.id,KEVIN)?.unwrap();
                        member.change_stage_role(tx,campfire_db::StageRole::Speaker)?;
                    }
                    let crypto = campfire_kit::RailsCrypto::new(secrets);
                    let mut people = serde_json::Map::new();
                    for (name,id) in [("david",DAVID),("jason",JASON),("kevin",KEVIN)] {
                        let session = campfire_db::Session::start_with(tx,id,campfire_db::NewSession {two_factor_verified:true,..Default::default()})?;
                        let cookie = campfire_kit::cookies::escape(&crypto.sign_cookie("session_token",&session.token,None));
                        let member = Membership::find_by_room_and_user(tx.conn(),room.id,id)?.unwrap();
                        people.insert(name.into(),json!({"id":id,"membership":member.id,"cookie":cookie}));
                    }
                    Ok(json!({"room":room.id,"people":people}))
                }).await.unwrap())
            }
        }))
        .route("/__ws13__/rooms/{id}", get(move |Path(id): Path<i64>| {
            let app = state.clone();
            async move {
                Json(app.db.read(move |conn| {
                    let mut members = serde_json::Map::new();
                    for member in Membership::for_room(conn,id)? {
                        let user = User::find(conn,member.user_id)?;
                        let (role,hand): (String,Option<campfire_db::Timestamp>) = conn.query_row("SELECT stage_role,hand_raised_at FROM memberships WHERE id=?",[member.id],|r|Ok((r.get(0)?,r.get(1)?)))?;
                        members.insert(user.name,json!({"role":role,"hand":hand.is_some()}));
                    }
                    Ok(json!({"members":members}))
                }).await.unwrap())
            }
        }));
    let listener = crate::test_support::bind_listener().await;
    let target = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let cache = std::path::PathBuf::from("/home/riels/.cache/rust-port/ws13");
    std::fs::create_dir_all(&cache).unwrap();
    let network = tempfile::Builder::new()
        .prefix("browser-")
        .tempdir_in(cache)
        .unwrap();
    let socket = network.path().join("upstream.sock");
    let mut forward = tokio::process::Command::new("node")
        .arg(root.join("parity/capture/forward.ts"))
        .arg(&socket)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    crate::test_support::eventually("browser loopback forwarder", || async { socket.exists() })
        .await;
    let image = std::env::var("WS13_PLAYWRIGHT_IMAGE")
        .expect("run parity/system/ws13 to build the pinned browser image");
    let output = tokio::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--init",
            "--name",
            "ws13-system-browser",
            "--network",
            "none",
            "--ipc",
            "host",
            "--cpus",
            "2",
        ])
        .arg("--user")
        .arg(format!("{}:{}", unsafe { libc::getuid() }, unsafe {
            libc::getgid()
        }))
        .arg("--volume")
        .arg(format!("{}:{}:ro", root.display(), root.display()))
        .arg("--volume")
        .arg(format!(
            "{}:{}",
            network.path().display(),
            network.path().display()
        ))
        .arg("--env")
        .arg(format!("HOME={}", network.path().display()))
        .arg("--env")
        .arg(format!("TMPDIR={}", network.path().display()))
        .arg("--env")
        .arg(format!("PARITY_UPSTREAM_SOCKET={}", socket.display()))
        .arg("--env")
        .arg(format!("WS13_SYSTEM_TARGET={target}"))
        .arg(image)
        .arg("node")
        .args(["--test", "--test-concurrency=8"])
        .arg(root.join("parity/system/ws13-stage.test.mjs"))
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    forward.kill().await.unwrap();
    forward.wait().await.unwrap();
    let _ = stop.send(());
    server.await.unwrap();
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(2))
        .await;
    assert!(
        output.status.success(),
        "browser acceptance failed: {}",
        output.status
    );
}

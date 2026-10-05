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
    run_browser(false).await;
}
#[tokio::test]
#[ignore = "requires the project-local LiveKit server; run parity/system/ws13-livekit"]
async fn livekit_stage_system_cases_in_real_browser() {
    run_browser(true).await;
}
async fn run_browser(real_livekit:bool) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let cache = std::env::var_os("WS13_BROWSER_SCRATCH").map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ws13-browser"));
    std::fs::create_dir_all(&cache).unwrap();
    let network = tempfile::Builder::new().prefix("browser-").tempdir_in(cache).unwrap();
    let listener = crate::test_support::bind_listener().await;
    let target = format!("http://{}", listener.local_addr().unwrap());
    let mut gateway = None;
    let mut config = super::call_channel_tests::configured();
    if real_livekit {
        config=crate::huddle::Config::from_env();
        assert_eq!(config.internal_url.as_deref(),Some("http://127.0.0.1:7880"),"only the project-local server is permitted");
        let port_file=network.path().join("gateway-port");
        let child=tokio::process::Command::new("node").arg(root.join("parity/system/ws13-livekit-gateway.mjs")).arg(&target).arg(&port_file).kill_on_drop(true).spawn().unwrap();
        crate::test_support::eventually("local LiveKit gateway",||async{port_file.exists()}).await;
        let port=std::fs::read_to_string(port_file).unwrap();
        config.public_url=Some(format!("ws://127.0.0.1:{port}"));
        gateway=Some(child);
    }
    let clock:campfire_kit::SharedClock=if real_livekit {std::sync::Arc::new(campfire_kit::clock::SystemClock)} else {crate::controllers::presenters::test_support::seed_clock()};
    let public_url=config.public_url.clone().unwrap();
    // The fixture's SDK URL and its real CSP must come from the same lookup,
    // just as Rails reads LIVEKIT_URL for both on each request.
    let test=TestApp::boot_with_settings(config,clock,&[("LIVEKIT_URL",&public_url)]).await.expect("build parity seeds");
    // Rails uses perform_enqueued_jobs only: for sightings. Keep jobs queued
    // until this adapter runs the same selected effect, rather than running
    // every join/presence callback implicitly in a background worker.
    test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
    let app = test.booted.app.clone();
    // System tests load the seven Rails room fixtures; parity's additional
    // populated Voice/Stage/Board rooms must not leak into those interactions.
    let seed_room_ids = ["pets","hq","watercooler","designers","david_and_jason","david_and_kevin","bender_and_kevin"].map(|label|campfire_db::fixtures::identify(label).to_string()).join(",");

    let quiet = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));

    let rings = queued_ring_adapter(app.clone(),quiet.clone());
    let fixtures = app.clone();

    let fixture_quiet=quiet.clone();

    let mutation_quiet=quiet.clone();
    let state = app.clone();
    let mutations = app.clone();
    let router = test.booted.router.clone()
        .route("/__ws13__/fixture", post(move |Json(options): Json<serde_json::Value>| {
            let app = fixtures.clone();
            let seed_room_ids = seed_room_ids.clone();

            fixture_quiet.store(0,std::sync::atomic::Ordering::SeqCst);
            async move {
                let secrets = app.secrets.clone();
                if real_livekit {
                    let rooms=app.db.read(Room::all).await.unwrap();
                    let service=crate::huddle::RoomService::new(app.config.huddle.clone());
                    for room in rooms {
                        let name=rails_compat::jwt::livekit::room_name(app.config.huddle.api_secret.as_deref().unwrap(),room.id);
                        service.delete_room(&name,app.clock.now().as_second()).await.unwrap();
                    }
                }
                Json(app.db.write(move |tx| {
                    use campfire_kit::Crypto;
                    // Each declaration starts with Rails' empty call/activity fixture.
                    // Soft-delete prior test-created channels so their IDs are never reused
                    // against a warm fragment cache. Original seed rooms remain intact.
                    tx.conn().execute_batch("DELETE FROM background_jobs; DELETE FROM activity_items; DELETE FROM huddle_cleanups; DELETE FROM huddle_grants; DELETE FROM streams;  UPDATE users SET inbox_preferences=NULL;")?;
                    tx.conn().execute(&format!("UPDATE rooms SET deleted_at=COALESCE(deleted_at,updated_at),direct_member_key=NULL WHERE id NOT IN ({seed_room_ids})"),[])?;

                    tx.conn().execute("UPDATE users SET dnd_enabled=0,dnd_until=NULL",[])?;
                    let room = if options["kind"] == "designers" {
                        Room::find(tx.conn(),654632876)?
                    } else if options["kind"] == "direct" {
                        Room::find(tx.conn(),186869642)?
                    } else if options["kind"] == "group" {
                        Room::create_for(tx,RoomType::Direct,None,DAVID,&[DAVID,JASON,KEVIN])?
                    } else if options["kind"] == "board" {
                        Room::create_for(tx,RoomType::Board,Some("Review Board"),DAVID,&[DAVID,JASON,KEVIN])?
                    } else {
                        Room::create_for(tx,RoomType::Stage,Some("Town Hall"),DAVID,&[DAVID,JASON,KEVIN])?
                    };
                    if options["jason_speaker"]==true {
                        let mut member=Membership::find_by_room_and_user(tx.conn(),room.id,JASON)?.unwrap();
                        member.change_stage_role(tx,campfire_db::StageRole::Speaker)?;
                    }
                    if options["speaker"]==true {
                        let mut member = Membership::find_by_room_and_user(tx.conn(),room.id,KEVIN)?.unwrap();
                        member.change_stage_role(tx,campfire_db::StageRole::Speaker)?;
                    }
                    let jz_seed:i64=tx.conn().query_row("SELECT id FROM users WHERE name='JZ'",[],|r|r.get(0))?;
                    // Rails reloads memberships between non-transactional system cases.
                    // Recover the original Designers member if an earlier revocation
                    // assertion failed before its explicit restoration.
                    let designers=Room::find(tx.conn(),654632876)?;
                    if Membership::find_by_room_and_user(tx.conn(),designers.id,jz_seed)?.is_none() {
                        designers.grant_to(tx,&[jz_seed])?;
                    }
                    let crypto = campfire_kit::RailsCrypto::new(secrets);
                    let mut people = serde_json::Map::new();
                    let jz:i64 = tx.conn().query_row("SELECT id FROM users WHERE name='JZ'",[],|r|r.get(0))?;
                    for (name,id) in [("david",DAVID),("jason",JASON),("kevin",KEVIN),("jz",jz)] {
                        let session = campfire_db::Session::start_with(tx,id,campfire_db::NewSession {two_factor_verified:true,..Default::default()})?;
                        let cookie = campfire_kit::cookies::escape(&crypto.sign_cookie("session_token",&session.token,None));
                        let member = Membership::find_by_room_and_user(tx.conn(),room.id,id)?;
                        people.insert(name.into(),json!({"id":id,"membership":member.map(|m|m.id),"session":session.id,"cookie":cookie}));
                    }
                    Ok(json!({"room":room.id,"people":people}))
                }).await.unwrap())
            }
        }))
        .route("/__ws13__/mutation", post(move |Json(options):Json<serde_json::Value>| {
            let app=mutations.clone();

            let mutation_quiet=mutation_quiet.clone();
            async move {
                if options["op"]=="server_remove" {
                    assert!(real_livekit,"server removal requires the project-local LiveKit fixture");
                    let room=options["room"].as_i64().unwrap();
                    let identity=options["identity"].as_str().unwrap();
                    let name=rails_compat::jwt::livekit::room_name(app.config.huddle.api_secret.as_deref().unwrap(),room);
                    crate::huddle::RoomService::new(app.config.huddle.clone()).remove_participant(&name,identity,app.clock.now().as_second()).await.unwrap();
                    return Json(json!({}));
                }
                if options["op"]=="reconnect" {
                    let user=options["user"].as_i64().unwrap();
                    let disconnected=app.cable.disconnect(&crate::channels::user_gid(user).to_string(),true);
                    return Json(json!({"disconnected":disconnected}));
                }

                if let Some(quiet)=options["quiet"].as_bool() { mutation_quiet.store(if quiet {2} else {1},std::sync::atomic::Ordering::SeqCst); }
                let config=campfire_db::models::room_delete::HuddleConfig {api_secret:app.config.huddle.api_secret.clone(),admin_configured:app.config.huddle.admin_configured()};
                let reply=app.db.write(move |tx| mutation(tx,&options,&config)).await.unwrap();
                if let Some(room)=reply["presence_room"].as_i64() {
                    tokio::task::spawn_blocking(move || crate::channels::huddle_effects::presence(&app,room)).await.unwrap().unwrap();
                }
                Json(reply)
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
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let socket = network.path().join("upstream.sock");
    let mut forward = tokio::process::Command::new("node")
        .arg(root.join("parity/capture/forward.ts"))
        .arg(&socket)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    crate::test_support::eventually("browser loopback forwarder", || async { socket.exists() })
        .await;
    let media_socket=network.path().join("media.sock");
    let mut media_forward=if real_livekit {
        let child=tokio::process::Command::new("node").arg(root.join("parity/system/ws13-media-network.mjs")).arg("host").arg(&media_socket).kill_on_drop(true).spawn().unwrap();
        crate::test_support::eventually("browser loopback media forwarder",||async{media_socket.exists()}).await;
        Some(child)
    } else {None};
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
            if real_livekit {"bridge"} else {"none"},
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
        .arg(format!("WS13_MEDIA_UPSTREAM_SOCKET={}",if real_livekit {media_socket.display().to_string()}else{String::new()}))
        .arg("--env")
        .arg(format!("WS13_SYSTEM_TARGET={target}"))
        .arg("--env")
        .arg(format!("WS13_LIVEKIT_GATEWAY_BYPASS={}",if real_livekit {app.config.huddle.public_url.as_deref().unwrap().trim_start_matches("ws://")} else {""}))
        .arg("--env")
        .arg(format!("WS13_ENABLE_INBOX_CASES={}",std::env::var("WS13_ENABLE_INBOX_CASES").unwrap_or_default()))
        .arg(image)
        .arg("node")
        .args(["--test", "--test-concurrency=4"])
        // Isolation for timing investigations; unset runs every declaration.
        .args(std::env::var("WS13_SYSTEM_TEST_NAME").ok().map(|name|format!("--test-name-pattern={name}")))
        .arg(root.join(if real_livekit {"parity/system/ws13-livekit-stage.test.mjs"} else if std::env::var_os("WS13_BOARD_REVIEW_ONLY").is_some() {"parity/system/ws13-board-review.test.mjs"} else if std::env::var_os("WS13_INVITATIONS_ONLY").is_some() {"parity/system/ws13-invitations.cases.mjs"} else {"parity/system/ws13-stage.test.mjs"}))
        .kill_on_drop(true)
        .status()
        .await
        .unwrap();

    {rings.abort();let _=rings.await;}
    forward.kill().await.unwrap();
    forward.wait().await.unwrap();
    if let Some(mut media)=media_forward.take() {media.kill().await.unwrap();media.wait().await.unwrap();}
    if let Some(mut gateway)=gateway {gateway.kill().await.unwrap();gateway.wait().await.unwrap();}
    let _ = stop.send(());
    server.await.unwrap();
    assert!(
        output.success(),
        "browser acceptance failed: {}",
        output
    );
}

/// Direct Ruby fixture mutations stay private; all callbacks/payloads/rendering
/// use their production domain functions (no synthetic Cable payloads).
fn mutation(tx:&mut campfire_db::Tx<'_>, options:&serde_json::Value, config:&campfire_db::models::room_delete::HuddleConfig)->campfire_db::Result<serde_json::Value> {
    use campfire_db::models::{huddle_grant::HuddleGrant,huddle_notices};
    use campfire_db::{Timestamp,Session};
    use jiff::SignedDuration;
    use rusqlite::params;
    let op=options["op"].as_str().unwrap();
    if op=="issue" {
        let room=options["room"].as_i64().unwrap();
        let user=options["user"].as_i64().unwrap();
        let session=match options["session"].as_i64() {
            Some(id)=>id,
            None=>Session::start(tx,user,Some("second device"),Some("127.0.0.2"))?.id,
        };
        let member=Membership::find_by_room_and_user(tx.conn(),room,user)?.unwrap();
        let grant=HuddleGrant::issue(tx,session,member.id,room,config)?;
        return Ok(json!({"id":grant.id,"session":grant.session_id}));
    }
    if op=="missed" {
        tx.conn().execute("UPDATE activity_items SET event_type='huddle_missed' WHERE user_id=? AND event_type='huddle_started'",[options["user"].as_i64().unwrap()])?;
        return Ok(json!({}));
    }
    if op=="grant_member" {
        let user=options["user"].as_i64().unwrap();
        let room=Room::find(tx.conn(),options["room"].as_i64().unwrap())?;
        room.grant_to(tx,&[user])?;
        return Ok(json!({}));
    }
    if op=="revoke_member" {
        let room=Room::find(tx.conn(),options["room"].as_i64().unwrap())?;
        room.revoke_from(tx,&[options["user"].as_i64().unwrap()])?;
        return Ok(json!({}));
    }
    if op=="preferences" {
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",params![options["preferences"].to_string(),options["user"].as_i64().unwrap()])?;

        if let Some(quiet)=options["quiet"].as_bool() {
            // Rails injects RingPolicy.quiet_check. The current WS13b issuance
            // emits inline, so provide that same decision through WS17's
            // existing persisted manual-DND input rather than replacing a frame.
            tx.conn().execute("UPDATE users SET dnd_enabled=?,dnd_until=NULL WHERE id=?",params![quiet,options["user"].as_i64().unwrap()])?;
        }
        return Ok(json!({}));
    }
    let id=options["grant"].as_i64().unwrap();
    let mut grant=HuddleGrant::find_by_id(tx.conn(),id)?.unwrap();
    match op {
        "columns"=>{
            if let Some(seen)=options.get("seen") {
                let at=seen.as_i64().map(|seconds|Timestamp::from_jiff(tx.now().jiff().checked_add(SignedDuration::from_secs(seconds)).unwrap()));
                tx.conn().execute("UPDATE huddle_grants SET last_seen_at=? WHERE id=?",params![at,id])?;
            }
            if options["revoked"]==true {
                tx.conn().execute("UPDATE huddle_grants SET revoked_at=? WHERE id=?",params![tx.now(),id])?;
            }
        }
        "seen"|"seen_presence"|"seen_join"=>{
            grant.record_seen(tx)?;
            if op=="seen_join" {huddle_notices::notify_join(tx,id)?;}
            return Ok(json!({"presence_room":(op=="seen_presence").then_some(grant.room_id)}));
        }
        "notify_leave"=>huddle_notices::notify_leave(tx,&grant)?,
        "revoke"=>grant.revoke(tx,true,config)?,
        "destroy_session"=>Session::find(tx.conn(),grant.session_id)?.destroy(tx)?,
        "out"=>return Ok(json!({"changed":grant.mark_out_of_call(tx,None)?})),
        "inspect"=>return Ok(json!({"last_seen":grant.last_seen_at.map(|at|at.as_microsecond()),"revoked":grant.revoked(),"activities":tx.conn().prepare("SELECT user_id,event_type,read_at IS NOT NULL,handled_at IS NOT NULL FROM activity_items WHERE source_type='HuddleGrant' AND source_id=?")?.query_map([id],|r|Ok(json!({"user":r.get::<_,i64>(0)?,"event":r.get::<_,String>(1)?,"read":r.get::<_,bool>(2)?,"handled":r.get::<_,bool>(3)?})))?.collect::<Result<Vec<_>,_>>()?})),
        _=>panic!("unknown fixture operation: {op}"),
    }
    Ok(json!({}))
}

/// Rails emits the ring inline. For E2E, drain only that durable effect through
/// WS13b's current API, with the same optional quiet_check injection as Rails.
/// Uses the merged main API directly; no source or compile-time overlay.
fn queued_ring_adapter(app:crate::app::App,quiet:std::sync::Arc<std::sync::atomic::AtomicU8>)->tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let decision=quiet.load(std::sync::atomic::Ordering::SeqCst);
            app.db.write(move |tx| {
                use campfire_db::models::huddle_invitations::{publish_queued_ring,publish_ring_with_policy,RingRequest};
                let mut statement=tx.conn().prepare("SELECT id,arguments FROM background_jobs WHERE job_class='Notifications::HuddleRingJob' AND status='ready' ORDER BY id")?;
                let rows=statement.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,serde_json::Value>(1)?)))?.collect::<Result<Vec<_>,_>>()?;
                drop(statement);
                for (id,arguments) in rows {
                    if decision==0 { publish_queued_ring(tx,id)?; }
                    else if arguments["delivered"]!=1 && arguments["cancelled"]!=1 && arguments["superseded"]!=1 {
                        let request:RingRequest=serde_json::from_value(arguments).unwrap();
                        publish_ring_with_policy(tx,&request,Some(&|_|decision==2))?;
                    }
                    tx.conn().execute("DELETE FROM background_jobs WHERE id=?",[id])?;
                }
                Ok(())
            }).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
}

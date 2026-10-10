use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, SEED_NOW, TestApp, david_cookie,
};
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::{Membership, Session};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn next(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let message = socket.next().await.expect("socket closed").unwrap();
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] != "ping" {
                    return value;
                }
            }
        }
    })
    .await
    .expect("broadcast did not reach the socket")
}

async fn subscribe(socket: &mut Socket, channel: &str, signed: String) {
    let identifier = json!({"channel":channel, "signed_stream_name":signed}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe", "identifier": identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(socket).await["type"], "confirm_subscription");
}

#[tokio::test]
async fn huddle_presence_reaches_real_sockets_and_rolled_back_revocation_stays_silent() {
    let config = crate::huddle::Config::from_lookup(|name| {
        Some(
            match name {
                "LIVEKIT_URL" => "wss://public.example.test",
                "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880",
                _ => "ws13-fixture-value",
            }
            .into(),
        )
    });
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let Some(test) = TestApp::boot_with_huddle_and_clock(config, clock).await else {
        return;
    };
    let app = test.booted.app.clone();
    let grant = app
        .db
        .write(|tx| {
            let session = Session::start(tx, DAVID, None, None)?;
            let membership =
                Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?.unwrap();
            HuddleGrant::issue(
                tx,
                session.id,
                membership.id,
                ALL_TALK,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-value".into()),
                    admin_configured: false,
                },
            )
        })
        .await
        .unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let room = app
        .db
        .read(|conn| campfire_db::Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    for (channel, stream) in [
        (
            "Turbo::StreamsChannel",
            super::broadcasts::Stream::user_rooms(DAVID),
        ),
        (
            "RoomMessagesChannel",
            super::broadcasts::Stream::room_messages(&room),
        ),
    ] {
        subscribe(
            &mut socket,
            channel,
            rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
        )
        .await;
    }
    let id = grant.id;
    app.db
        .write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), id)?
                .unwrap()
                .record_seen(tx)
        })
        .await
        .unwrap();
    for placement in ["sidebar", "header"] {
        let frame = next(&mut socket).await;
        let html = frame["message"].as_str().unwrap();
        assert!(
            html.contains(&format!(
                "{placement}_voice_participants_rooms_closed_{ALL_TALK}"
            )),
            "{html}"
        );
        assert!(html.contains("1 in huddle: David"), "{html}");
        assert!(campfire_cable::turbo::session_bound(html).is_none());
        assert!(!campfire_views::helpers::request_forgery::has_token_slots(
            html
        ));
    }
    let rolled_back = app
        .db
        .write(move |tx| -> campfire_db::Result<()> {
            HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().revoke(
                tx,
                false,
                &HuddleConfig::default(),
            )?;
            Err(campfire_db::Error::Other(
                "WS13 intentional rollback".into(),
            ))
        })
        .await;
    assert!(rolled_back.is_err());
    assert!(
        !app.db
            .read(move |conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap().revoked()))
            .await
            .unwrap()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), socket.next())
            .await
            .is_err(),
        "rolled-back presence escaped"
    );
    app.db
        .write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().revoke(
                tx,
                false,
                &HuddleConfig::default(),
            )
        })
        .await
        .unwrap();
    for _ in 0..2 {
        let frame = next(&mut socket).await;
        let html = frame["message"].as_str().unwrap();
        assert!(html.contains("Nobody in huddle"), "{html}");
        assert!(!html.contains("voice-stack--live"), "{html}");
    }
    socket.close(None).await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn stage_stream_callbacks_reach_real_sockets_and_rollback_stays_silent() {
    use campfire_db::models::stream::Stream;
    use campfire_db::{CachedStatements, Room};
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let app = test.booted.app.clone();
    let membership=app.db.write(|tx| {
        tx.conn().execute_cached("UPDATE rooms SET type='Rooms::Stage' WHERE id=?",[ALL_TALK])?;
        tx.conn().execute_cached("UPDATE memberships SET stage_role=CASE WHEN user_id=? THEN 'host' ELSE 'listener' END WHERE room_id=?",rusqlite::params![DAVID,ALL_TALK])?;
        Ok(Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().id)
    }).await.unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let room = app
        .db
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    for (channel, stream) in [
        (
            "Turbo::StreamsChannel",
            super::broadcasts::Stream::user_rooms(DAVID),
        ),
        (
            "RoomMessagesChannel",
            super::broadcasts::Stream::room_messages(&room),
        ),
    ] {
        subscribe(
            &mut socket,
            channel,
            rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
        )
        .await;
    }
    let stream = app
        .db
        .write(move |tx| Stream::create(tx, ALL_TALK, membership, DAVID, "1080p15", None))
        .await
        .unwrap();
    for phase in ["start", "end"] {
        let presenter = app.clone();
        let stage = app
            .db
            .read(move |conn| {
                super::huddle_effects::stage_model(&presenter, conn, ALL_TALK, membership)
            })
            .await
            .unwrap();
        for (partial, prefix) in [
            ("live_badge", "stage_live_badge"),
            ("live_dot", "sidebar_stage_live"),
            ("venue_live_dot", "event_stage_live"),
            ("panel_body", "stage_panel"),
        ] {
            let frame = next(&mut socket).await;
            let actual = frame["message"].as_str().unwrap();
            let expected = campfire_cable::turbo::action_tag(
                campfire_cable::turbo::Action::Replace,
                campfire_cable::turbo::Target::Target(&stage.dom_id(prefix)),
                Some(&stage.render(partial)),
                &[],
            );
            assert_eq!(actual, expected, "{phase} {partial}");
            assert!(campfire_cable::turbo::session_bound(actual).is_none());
            assert!(!campfire_views::helpers::request_forgery::has_token_slots(
                actual
            ));
        }
        if phase == "start" {
            let id = stream.id;
            let failed = app
                .db
                .write(move |tx| -> campfire_db::Result<()> {
                    Stream::find_by_id(tx.conn(), id)?
                        .unwrap()
                        .end(tx, Some(DAVID + 1))?;
                    Err(campfire_db::Error::Other("WS13 rollback".into()))
                })
                .await;
            assert!(failed.is_err());
            assert!(
                tokio::time::timeout(Duration::from_millis(150), socket.next())
                    .await
                    .is_err()
            );
            app.db
                .write(move |tx| {
                    Stream::find_by_id(tx.conn(), id)?
                        .unwrap()
                        .end(tx, Some(DAVID + 1))
                        .map(|_| ())
                })
                .await
                .unwrap();
        }
    }
    let frame = next(&mut socket).await;
    assert!(
        frame["message"]
            .as_str()
            .unwrap()
            .contains("data-huddle-stream-kind=\"stream-stopped\"")
    );
    let id = stream.id;
    app.db
        .write(move |tx| {
            Stream::find_by_id(tx.conn(), id)?
                .unwrap()
                .end(tx, Some(DAVID + 1))
                .map(|_| ())
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(150), socket.next())
            .await
            .is_err(),
        "repeat end broadcast"
    );
    socket.close(None).await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn stage_role_roster_panel_and_single_rejoin_reach_real_sockets_after_commit() {
    use campfire_db::models::stage_participation;
    use campfire_db::{CachedStatements, Room};
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let app = test.booted.app.clone();
    let membership=app.db.write(|tx| {
        tx.conn().execute_cached("UPDATE rooms SET type='Rooms::Stage' WHERE id=?",[ALL_TALK])?;
        tx.conn().execute_cached("UPDATE memberships SET stage_role=CASE WHEN user_id=? THEN 'host' ELSE 'listener' END WHERE room_id=?",rusqlite::params![campfire_db::fixtures::identify("jason"),ALL_TALK])?;
        Ok(Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().id)
    }).await.unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let room = app
        .db
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    for (channel, stream) in [
        (
            "Turbo::StreamsChannel",
            super::broadcasts::Stream::user_rooms(DAVID),
        ),
        (
            "RoomMessagesChannel",
            super::broadcasts::Stream::room_messages(&room),
        ),
    ] {
        subscribe(
            &mut socket,
            channel,
            rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
        )
        .await;
    }
    for role in ["speaker", "host"] {
        let change = app
            .db
            .write(move |tx| {
                stage_participation::change_role(
                    tx,
                    ALL_TALK,
                    DAVID,
                    Some(membership),
                    role,
                    &HuddleConfig::default(),
                )
            })
            .await
            .unwrap();
        assert!(change.is_ok());
        let presenter = app.clone();
        let stage = app
            .db
            .read(move |conn| {
                super::huddle_effects::stage_model(&presenter, conn, ALL_TALK, membership)
            })
            .await
            .unwrap();
        for (partial, prefix) in [("roster", "stage_roster"), ("panel_body", "stage_panel")] {
            let frame = next(&mut socket).await;
            let html = frame["message"].as_str().unwrap();
            assert_eq!(
                html,
                campfire_cable::turbo::action_tag(
                    campfire_cable::turbo::Action::Replace,
                    campfire_cable::turbo::Target::Target(&stage.dom_id(prefix)),
                    Some(&stage.render(partial)),
                    &[]
                )
            );
            assert!(!html.contains("data-huddle-rejoin-room-id"));
            assert!(campfire_cable::turbo::session_bound(html).is_none());
        }
        if role == "speaker" {
            let frame = next(&mut socket).await;
            assert_eq!(
                frame["message"].as_str().unwrap(),
                campfire_cable::turbo::action_tag(
                    campfire_cable::turbo::Action::Append,
                    campfire_cable::turbo::Target::Target("huddle_role_events"),
                    Some(&stage.render("role_event")),
                    &[]
                )
            );
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(150), socket.next())
                .await
                .is_err(),
            "duplicate rejoin or wrong target delivery"
        );
    }
    let failed = app
        .db
        .write(move |tx| -> campfire_db::Result<()> {
            assert!(
                stage_participation::change_role(
                    tx,
                    ALL_TALK,
                    DAVID,
                    Some(membership),
                    "listener",
                    &HuddleConfig::default()
                )?
                .is_ok()
            );
            Err(campfire_db::Error::Other("WS13 rollback".into()))
        })
        .await;
    assert!(failed.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(150), socket.next())
            .await
            .is_err()
    );
    assert_eq!(
        app.db
            .read(move |conn| Ok(Membership::find(conn, membership)?.stage_role))
            .await
            .unwrap(),
        Some(campfire_db::StageRole::Host)
    );
    socket.close(None).await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn last_host_departure_delivers_a_quiet_note_to_the_room_socket() {
    use campfire_db::{CachedStatements, Room};
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let app = test.booted.app.clone();
    let _membership=app.db.write(|tx| {
        tx.conn().execute_cached("UPDATE rooms SET type='Rooms::Stage' WHERE id=?",[ALL_TALK])?;
        tx.conn().execute_cached("UPDATE memberships SET stage_role=CASE WHEN user_id=? THEN 'host' ELSE 'listener' END WHERE room_id=?",rusqlite::params![campfire_db::fixtures::identify("jason"),ALL_TALK])?;
        Ok(Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().id)
    }).await.unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let room = app
        .db
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    subscribe(
        &mut socket,
        "RoomMessagesChannel",
        rails_compat::turbo::signed_stream_name(
            &app.secrets,
            &super::broadcasts::Stream::room_messages(&room).streamables(),
        ),
    )
    .await;
    app.db
        .write(|tx| {
            let host = Membership::find_by_room_and_user(
                tx.conn(),
                ALL_TALK,
                campfire_db::fixtures::identify("jason"),
            )?
            .unwrap();
            host.destroy(tx)
        })
        .await
        .unwrap();
    let frame = next(&mut socket).await;
    let html = frame["message"].as_str().unwrap();
    assert!(html.contains("message--system-note"), "{html}");
    assert!(html.contains("The stage ended because the last host left."));
    assert!(html.contains("<strong>Jason</strong>"));
    assert!(!html.contains("data-actions-url"));
    assert!(campfire_cable::turbo::session_bound(html).is_none());
    assert!(
        tokio::time::timeout(Duration::from_millis(150), socket.next())
            .await
            .is_err()
    );
    socket.close(None).await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn stage_quiet_note_html_matches_rails_bytes() {
    use campfire_db::{CachedStatements, NewMessage};
    let vector: Value =
        serde_json::from_str(include_str!("huddle_stage_note_vectors.json")).unwrap();
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        jiff::Timestamp::from_second(vector["now"].as_i64().unwrap()).unwrap(),
    ));
    let Some(test) =
        TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(), clock).await
    else {
        return;
    };
    test.db().write(|tx| {
        tx.conn().execute_cached("INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(9001,'WS13 Stage','Rooms::Stage',?,?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;
        tx.conn().execute_cached("UPDATE sqlite_sequence SET seq=1200000000 WHERE name='messages'",[])?;
        let note=campfire_db::Message::create(tx,NewMessage{room_id:9001,creator_id:DAVID,client_message_id:Some("ws13-stage-note-fixture".into()),system_note:true,body:Some("The stage ended because the last host left.".into()),..Default::default()})?;
        assert_eq!(note.id,1200000001);
        Ok(())
    }).await.unwrap();
    let app = test.booted.app.clone();
    let presenter = app.clone();
    let (_, html) = app
        .db
        .read(move |conn| super::huddle_effects::stage_note_html(&presenter, conn, 1200000001))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(html, vector["html"].as_str().unwrap());
}

#[tokio::test]
async fn presence_job_fanout_and_missing_room_match_rails_counts() {
    use campfire_db::models::huddle_grant::PresenceJob;
    use campfire_db::{Event, Room, RoomType};
    use campfire_kit::Crypto;
    let oracle: Value =
        serde_json::from_str(include_str!("../huddle/huddle_job_contract_vectors.json")).unwrap();
    let huddle = crate::huddle::Config::from_lookup(|key| {
        Some(
            match key {
                "LIVEKIT_URL" => "wss://huddle.example.test",
                "LIVEKIT_INTERNAL_URL" => "ws://livekit.example.test:7880",
                _ => "ws13b-fixture-value",
            }
            .into(),
        )
    });
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let test = TestApp::boot_with_huddle_and_clock(huddle, clock)
        .await
        .expect("WS13b requires the parity seed");
    let app = test.booted.app.clone();
    let jason = campfire_db::fixtures::identify("jason");
    let (grant, jason_token) = app
        .db
        .write(move |tx| {
            let room =
                Room::create_for(tx, RoomType::Voice, Some("Lounge"), DAVID, &[DAVID, jason])?;
            tx.conn()
                .execute("UPDATE rooms SET id=9001 WHERE id=?", [room.id])?;
            tx.conn().execute(
                "UPDATE memberships SET room_id=9001 WHERE room_id=?",
                [room.id],
            )?;
            let session = Session::start(tx, DAVID, None, None)?;
            let membership = Membership::find_by_room_and_user(tx.conn(), 9001, DAVID)?.unwrap();
            let grant = HuddleGrant::issue(
                tx,
                session.id,
                membership.id,
                9001,
                &HuddleConfig {
                    api_secret: Some("ws13b-fixture-value".into()),
                    admin_configured: false,
                },
            )?;
            tx.conn().execute(
                "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                rusqlite::params![tx.now(), grant.id],
            )?;
            let peer = Session::start_with(
                tx,
                jason,
                campfire_db::NewSession {
                    user_agent: None,
                    ip_address: None,
                    device_id: None,
                    two_factor_verified: true,
                },
            )?;
            Ok((grant.id, peer.token))
        })
        .await
        .unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie(
        "session_token",
        &jason_token,
        None,
    );
    let cookies = [
        david_cookie(),
        format!("session_token={}", campfire_kit::cookies::escape(&signed)),
    ];
    let mut sockets = Vec::new();
    for (user, cookie) in [DAVID, jason].into_iter().zip(cookies) {
        let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
        request
            .headers_mut()
            .insert("origin", format!("http://{addr}").parse().unwrap());
        request
            .headers_mut()
            .insert("cookie", cookie.parse().unwrap());
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        assert_eq!(next(&mut socket).await["type"], "welcome");
        let stream = super::broadcasts::Stream::user_rooms(user);
        subscribe(
            &mut socket,
            "Turbo::StreamsChannel",
            rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
        )
        .await;
        if user == DAVID {
            let room = app.db.read(|conn| Room::find(conn, 9001)).await.unwrap();
            let stream = super::broadcasts::Stream::room_messages(&room);
            subscribe(
                &mut socket,
                "RoomMessagesChannel",
                rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
            )
            .await;
        }
        sockets.push(socket);
    }
    app.db
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&PresenceJob { grant_id: grant }));
            Ok(())
        })
        .await
        .unwrap();
    let mut targets = std::collections::BTreeMap::new();
    let mut streams = std::collections::BTreeMap::<String, i64>::new();
    let room = app.db.read(|conn| Room::find(conn, 9001)).await.unwrap();
    for (index, socket) in sockets.iter_mut().enumerate() {
        for _ in 0..if index == 0 { 2 } else { 1 } {
            let frame = next(socket).await;
            let html = frame["message"].as_str().unwrap();
            assert!(html.contains("1 in voice: David"), "{html}");
            let target = if html.contains("sidebar_voice_participants") {
                "sidebar"
            } else {
                "header"
            };
            *targets.entry(format!("{index}:{target}")).or_insert(0) += 1;
            let stream = if target == "header" {
                super::broadcasts::Stream::room_messages(&room)
            } else {
                super::broadcasts::Stream::user_rooms(if index == 0 { DAVID } else { jason })
            };
            *streams.entry(stream.streamables().join(":")).or_insert(0) += 1;
        }
    }
    assert_eq!(
        targets,
        std::collections::BTreeMap::from([
            ("0:sidebar".into(), 1),
            ("0:header".into(), 1),
            ("1:sidebar".into(), 1)
        ])
    );
    assert_eq!(serde_json::json!(streams), oracle["presence"]["counts"]);
    app.db
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&PresenceJob { grant_id: -1 }));
            tx.conn()
                .execute("UPDATE huddle_grants SET room_id=-1 WHERE id=?", [grant])?;
            tx.emit_after_commit(Event::job(&PresenceJob { grant_id: grant }));
            Ok(())
        })
        .await
        .unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    assert_eq!(oracle["presence"]["missing"], serde_json::json!([]));
    for socket in &mut sockets {
        assert!(
            tokio::time::timeout(Duration::from_millis(150), socket.next())
                .await
                .is_err(),
            "missing room/grant broadcast"
        );
        socket.close(None).await.unwrap();
    }
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

use campfire_views::rendering::*;

use super::call_channel_broadcast_tests::{Socket, next, socket};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, TestApp};
use campfire_db::{Room, RoomType};
use std::time::Duration;
async fn until_marker(test: &TestApp, socket: &mut Socket, user: i64) -> Vec<String> {
    test.booted.app.broadcasts.replace(
        &crate::channels::broadcasts::Stream::user_rooms(user),
        "ws13_sidebar_barrier",
        "",
    );
    let mut htmls = Vec::new();
    loop {
        let frame = next(socket).await;
        let html = frame["message"].as_str().unwrap();
        if html.contains("ws13_sidebar_barrier") {
            break;
        }
        assert!(campfire_cable::turbo::session_bound(html).is_none());
        htmls.push(html.into());
    }
    htmls
}
#[tokio::test]
async fn configured_direct_callback_keeps_rails_membership_avatar_order() {
    use std::sync::Arc;
    use crate::controllers::presenters::test_support::SEED_NOW;
    let Some(test) = TestApp::boot_with_huddle_and_clock(
        super::call_channel_tests::configured(),
        Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
    ).await else { return; };
    let id = test.db().write(|tx| Ok(Room::find_or_create_direct_for(
        tx, &[DAVID, JASON, KEVIN, 773523953], KEVIN,
    )?.id)).await.unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router).with_graceful_shutdown(async {
            let _ = stopping.await;
        }).await.unwrap();
    });
    let mut client = socket(&test, addr, DAVID).await;
    test.db().write(move |tx| {
        Room::find(tx.conn(), id)?.rename_direct(tx, "Friday <&>", KEVIN)
    }).await.unwrap();
    let frames = until_marker(&test, &mut client, DAVID).await;
    let actual = frames.iter().find(|html| html.contains(
        &format!("target=\"list_rooms_direct_{id}\""),
    )).expect("missing configured direct row");
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/rooms/directory.json",
    )).unwrap();
    assert_eq!(id, oracle["setup"]["group_id"].as_i64().unwrap());
    let stream = format!("{}:rooms", crate::channels::user_gid(DAVID).to_param());
    let expected = oracle["operations"][0]["frames"].as_array().unwrap().iter()
        .filter(|frame| frame["stream"].as_str() == Some(stream.as_str()))
        .filter_map(|frame| frame["html"].as_str())
        .find(|html| html.contains(&format!("target=\"list_rooms_direct_{id}\"")))
        .unwrap();
    // Huddle configuration adds call controls; the Rails avatar association order
    // in the same recorded directory callback remains unchanged.
    fn profile_paths(html: &str) -> Vec<&str> {
        html.split("data-profile-card-url=\"").skip(1)
            .map(|part| part.split_once('"').unwrap().0).collect()
    }
    assert_eq!(profile_paths(actual), profile_paths(expected));
    assert!(actual.contains("voice-room__trailing"));
    client.close(None).await.unwrap();
    let _ = stop.send(());
    serving.await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(1)).await;
}
#[tokio::test]
async fn composed_sidebar_rename_callback_delivers_recipient_rows_and_headers() {
    let Some(test) = TestApp::boot_with_huddle(super::call_channel_tests::configured()).await
    else {
        return;
    };
    let room = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, KEVIN, JASON]))
        .await
        .unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let mut david = socket(&test, addr, DAVID).await;
    let mut kevin = socket(&test, addr, KEVIN).await;
    let id = room.id;
    test.db()
        .write(move |tx| {
            let mut room = Room::find(tx.conn(), id)?;
            room.rename_direct(tx, "Group <&>", DAVID)
        })
        .await
        .unwrap();
    for (user, client) in [(DAVID, &mut david), (KEVIN, &mut kevin)] {
        let frames = until_marker(&test, client, user).await;
        let row = frames
            .iter()
            .find(|html| html.contains(&format!("target=\"list_rooms_direct_{id}\"")))
            .expect("missing recipient row");
        assert!(row.contains("Group &lt;&amp;&gt;"));
        assert!(row.contains("profile-card-avatar"));
        assert!(row.contains("voice-room__trailing"));
        assert_eq!(row.contains("data-menu-can-delete=\"true\""), user == DAVID);
        assert!(!row.contains(&format!("data-profile-card-url=\"/users/{user}/card\"")));
        assert!(frames.iter().any(|html| {
            html.contains(&format!("target=\"header_rooms_direct_{id}\""))
                && html.contains("Group &lt;&amp;&gt;")
        }));
    }
    let mut browser = test.sign_in(KEVIN).await;
    let response = browser.get(&format!("/rooms/{id}")).await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    assert!(response.text().contains("<title>Group &lt;&amp;&gt;"));
    david.close(None).await.unwrap();
    kevin.close(None).await.unwrap();
    let _ = stop.send(());
    serving.await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(1)).await;
}

#[tokio::test]
async fn composed_sidebar_broadcast_rows_match_sixteen_complete_rails_renders() {
    use crate::controllers::{presenters::page, users::sidebars::composition};
    use campfire_db::{Membership, Timestamp};
    let Some(test) = TestApp::boot_with_huddle(super::call_channel_tests::configured()).await
    else {
        return;
    };
    let v: serde_json::Value =
        serde_json::from_str(include_str!("row_broadcast_vectors.json")).unwrap();
    assert_eq!(v["cases"].as_array().unwrap().len(), 16);
    test.db().write(|tx|{
        for (id,kind,name) in [(9401,"Rooms::Open",Some("Row <&>")),(9402,"Rooms::Closed",Some("Row <&>")),(9403,"Rooms::Direct",None)] {
            tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",rusqlite::params![id,kind,name,DAVID,Timestamp::from_second(1772467200),Timestamp::from_second(1772467200)])?;
        }Ok(())
    }).await.unwrap();
    for case in v["cases"].as_array().unwrap() {
        let input = case["input"].clone();
        let room_id = input["room"]["id"].as_i64().unwrap();
        if let Some(id) = input["membership_id"].as_i64() {
            let actor = input["actor_id"].as_i64().unwrap();
            let unread = input["unread"].as_bool().unwrap();
            let involvement = input["involvement"].as_str().unwrap().to_string();
            test.db().write(move |tx|{
                tx.conn().execute("INSERT INTO memberships(id,room_id,user_id,involvement,unread_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET involvement=excluded.involvement,unread_at=excluded.unread_at",rusqlite::params![id,room_id,actor,involvement,unread.then_some(tx.now()),tx.now(),tx.now()])?;Ok(())
            }).await.unwrap();
        }
        let app = test.booted.app.clone();
        let row = test
            .db()
            .read(move |conn| {
                let room = Room::find(conn, room_id)?;
                if let Some(id) = input["membership_id"].as_i64() {
                    let member_ids: Vec<i64> =
                        serde_json::from_value(input["member_ids"].clone()).unwrap();
                    composition::for_membership(
                        &app,
                        conn,
                        &Membership::find(conn, id)?,
                        Some(&member_ids),
                    )
                } else {
                    composition::neutral(&app, conn, &room)
                }
            })
            .await
            .unwrap();
        assert_eq!(
            row.epoch,
            Timestamp::from_second(1772467200)
                .as_second()
                .checked_mul(1000)
                .unwrap()
                .to_string()
        );
        let html =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                row.render_fragment(ctx, case["input"]["configured"].as_bool().unwrap())
            });
        assert_eq!(html, case["html"].as_str().unwrap(), "{}", case["name"]);
    }
    assert_eq!(v["headers"].as_array().unwrap().len(), 6);
    for case in v["headers"].as_array().unwrap() {
        let id = case["room_id"].as_i64().unwrap();
        let user = case["actor_id"].as_i64().unwrap();
        let name = case["room_name"].as_str().map(str::to_string);
        test.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE rooms SET name=? WHERE id=?",
                    rusqlite::params![name, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let app = test.booted.app.clone();
        let nav = test
            .db()
            .read(move |conn| {
                super::call_navigation::model(
                    &app,
                    conn,
                    &Room::find(conn, id)?,
                    &campfire_db::User::find(conn, user)?,
                )
            })
            .await
            .unwrap();
        let html =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                nav.identity(ctx)
            });
        assert_eq!(html, case["html"].as_str().unwrap(), "{}", case["name"]);
    }
    test.booted.jobs.shutdown(Duration::from_secs(1)).await;
}

#[tokio::test]
async fn composed_sidebar_mute_http_returns_json_and_recipient_menu_flags() {
    use crate::controllers::presenters::test_support::Req;
    use axum::http::{Method, StatusCode};
    let Some(test) = TestApp::boot_with_huddle(super::call_channel_tests::configured()).await
    else {
        return;
    };
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Closed,
                Some("Mute <&>"),
                DAVID,
                &[DAVID, KEVIN],
            )
        })
        .await
        .unwrap();
    let id = room.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE memberships SET unread_at=? WHERE room_id=? AND user_id=?",
                rusqlite::params![tx.now(), id, KEVIN],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let mut client = socket(&test, addr, KEVIN).await;
    let mut browser = test.sign_in(KEVIN).await;
    for (involvement, action) in [
        ("muted", "replace"),
        ("invisible", "remove"),
        ("mentions", "prepend"),
    ] {
        let reply = browser
            .write(
                Req::new(Method::PUT, &format!("/rooms/{id}/involvement"))
                    .header("accept", "application/json")
                    .form(&[("involvement", involvement)]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.is_empty());
        let frames = until_marker(&test, &mut client, KEVIN).await;
        let row = frames
            .iter()
            .find(|html| html.starts_with(&format!("<turbo-stream action=\"{action}\"")))
            .expect("missing row update");
        if action != "remove" {
            assert!(row.contains("data-menu-can-leave=\"true\""));
            assert!(row.contains("data-menu-can-delete=\"false\""));
            assert_eq!(
                row.contains("data-menu-muted=\"true\""),
                involvement == "muted"
            );
            assert!(!row.contains("sidebar-item__status"));
        }
    }
    client.close(None).await.unwrap();
    let _ = stop.send(());
    serving.await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(1)).await;
}

#[tokio::test]
async fn composed_sidebar_ooo_after_commit_updates_and_clears_real_subscriptions() {
    use futures_util::SinkExt;
    use serde_json::json;
    use tokio_tungstenite::tungstenite::Message;
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let room = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, KEVIN]))
        .await
        .unwrap();
    let id = room.id;
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let mut client = socket(&test, addr, DAVID).await;
    let stream = crate::channels::broadcasts::Stream::ooo_notice(KEVIN);
    let identifier=json!({"channel":"Turbo::StreamsChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&test.booted.app.secrets,&stream.streamables())}).to_string();
    client
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(&mut client).await["type"], "confirm_subscription");
    for (command, present) in [("/ooo tomorrow Trip <&>", true), ("/ooo off", false)] {
        test.db()
            .write(move |tx| {
                let result = campfire_db::slash_commands::dispatch(
                    tx,
                    &campfire_db::slash_commands::Context {
                        user_id: KEVIN,
                        room_id: id,
                        thread_id: None,
                        huddles_configured: false,
                    },
                    command,
                )?;
                assert_eq!(result.kind, "ephemeral");
                Ok(())
            })
            .await
            .unwrap();
        let frames = until_marker(&test, &mut client, DAVID).await;
        let html = frames
            .iter()
            .find(|html| html.contains(&format!("target=\"ooo_notice_user_{KEVIN}\"")))
            .expect("missing OOO callback");
        assert_eq!(html.contains("Trip &lt;&amp;&gt;"), present);
        if !present {
            assert_eq!(
                html,
                &format!(
                    "<turbo-stream action=\"update\" target=\"ooo_notice_user_{KEVIN}\"><template></template></turbo-stream>"
                )
            );
        }
    }
    client.close(None).await.unwrap();
    let _ = stop.send(());
    serving.await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(1)).await;
}

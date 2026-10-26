use super::*;

struct Observer<'a> {
    sync: Sync,
    browser: Browser<'a>,
}

impl<'a> Observer<'a> {
    async fn open(a: &'a TestApp, addr: SocketAddr) -> Self {
        let browser = a.sign_in(JASON).await;
        let mut sync = Sync::connect(addr, &browser.cookie_header(), &[]).await;
        sync.welcome().await;
        Self { sync, browser }
    }

    async fn sequence(&mut self, a: &TestApp) -> Vec<api::UserPresence> {
        let payload = serde_json::to_string(&api::SyncPayload::Typing(api::Typing {
            user_id: -1,
            on: false,
        }))
        .unwrap();
        let marker = a
            .booted
            .app
            .cable
            .sync_publish(campfire_cable::sync::SyncPublication::new(
                campfire_cable::sync::Audience::User(JASON),
                payload,
            ))
            .unwrap() as i64;
        let mut rows = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                while let Some(event) = self.sync.pending.pop_front() {
                    if event.seq == marker {
                        return;
                    }
                    if let api::SyncPayload::Presence(row) = event.payload
                        && row.user_id == DAVID
                    {
                        rows.push(row);
                    }
                }
                match self.sync.next().await {
                    Some(api::ServerFrame::Batch { events }) => self.sync.pending.extend(events),
                    Some(api::ServerFrame::Ping) => {}
                    other => panic!("expected a batch, got {other:?}"),
                }
            }
        })
        .await
        .expect("observer reached the publication marker");
        rows
    }

    async fn sees(&mut self, a: &TestApp, wanted: api::Presence, after: &str) {
        let rows = self.sequence(a).await;
        assert_eq!(
            rows.iter().map(|row| row.presence).collect::<Vec<_>>(),
            [wanted],
            "exact observer sequence after {after}"
        );
        let rest: api::PresenceList = parse(
            &self
                .browser
                .send(get(&format!("/api/v1/presence?ids={DAVID}")))
                .await,
        );
        assert_eq!(
            rest.presences
                .iter()
                .map(|row| row.presence)
                .collect::<Vec<_>>(),
            [wanted],
            "GET /api/v1/presence after {after}"
        );
    }
}

async fn device(a: &TestApp, addr: SocketAddr) -> Sync {
    let mut sync = Sync::connect(addr, &a.sign_in(DAVID).await.cookie_header(), &[]).await;
    sync.welcome().await;
    processed(&mut sync).await;
    sync
}

async fn processed(sync: &mut Sync) {
    sync.socket
        .send(WsMessage::Ping(vec![1].into()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match sync.socket.next().await {
                Some(Ok(WsMessage::Pong(_))) => return,
                Some(Ok(WsMessage::Text(text))) => {
                    if let api::ServerFrame::Batch { events } = serde_json::from_str(&text).unwrap()
                    {
                        sync.pending.extend(events);
                    }
                }
                Some(Ok(_)) => {}
                other => panic!("expected a pong, got {other:?}"),
            }
        }
    })
    .await
    .expect("server processed all preceding device frames");
}

async fn heartbeat(sync: &mut Sync, active: bool) {
    sync.send(json!({"t": "hb", "active": active})).await;
    processed(sync).await;
}

async fn close_device(a: &TestApp, sync: Sync, remaining: i64) {
    drop(sync);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let count: i64 = a
                .db()
                .read(|conn| {
                    Ok(conn.query_row(
                        "SELECT COUNT(*) FROM workspace_presence_leases WHERE user_id=?",
                        [DAVID],
                        |row| row.get(0),
                    )?)
                })
                .await
                .unwrap();
            if count == remaining {
                break;
            }
            tokio::task::yield_now().await;
        }
        a.db().write(|_| Ok(())).await.unwrap();
    })
    .await
    .expect("closed device lease removed and after-commit publications completed");
}

fn head(a: &TestApp) -> u64 {
    a.booted.app.cable.sync_position().unwrap().1
}

async fn age_davids_leases(a: &TestApp) {
    let aged = a
        .db()
        .write(|tx| {
            Ok(tx.conn().execute(
                "UPDATE workspace_presence_leases SET last_active_at = ? WHERE user_id = ?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(11)), DAVID],
            )?)
        })
        .await
        .unwrap();
    assert_eq!(aged, 2, "a lease for each of David's devices");
}

async fn one_device_idle_and_the_other_active(active_opened_first: bool) {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut jason = Observer::open(&a, addr).await;
    let first = device(&a, addr).await;
    let second = device(&a, addr).await;
    jason
        .sees(&a, api::Presence::Online, "both devices connected")
        .await;
    let (mut quiet, mut active) = if active_opened_first {
        (second, first)
    } else {
        (first, second)
    };

    for swapped in [false, true] {
        if swapped {
            std::mem::swap(&mut quiet, &mut active);
        }
        age_davids_leases(&a).await;
        heartbeat(&mut quiet, false).await;
        jason
            .sees(
                &a,
                api::Presence::Idle,
                "a quiet heartbeat with both devices past the idle cutoff",
            )
            .await;
        heartbeat(&mut active, true).await;
        jason
            .sees(
                &a,
                api::Presence::Online,
                "an active heartbeat from the other device",
            )
            .await;
    }

    close_device(&a, active, 1).await;
    jason
        .sees(
            &a,
            api::Presence::Idle,
            "the active device closed, leaving the idle one",
        )
        .await;
    close_device(&a, quiet, 0).await;
    jason
        .sees(&a, api::Presence::Offline, "the last device closed")
        .await;
    server.abort();
}

#[tokio::test]
async fn activity_on_a_second_device_brings_the_person_back_online() {
    one_device_idle_and_the_other_active(false).await;
}

#[tokio::test]
async fn activity_on_the_first_device_brings_the_person_back_online() {
    one_device_idle_and_the_other_active(true).await;
}

#[tokio::test]
async fn unchanged_heartbeats_do_not_publish_or_consume_replay_entries() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut observer = Observer::open(&a, addr).await;
    let mut first = device(&a, addr).await;
    observer
        .sees(&a, api::Presence::Online, "first device connected")
        .await;
    let before = head(&a);
    let mut second = device(&a, addr).await;
    let connected = head(&a);
    for _ in 0..4 {
        heartbeat(&mut first, false).await;
        heartbeat(&mut second, true).await;
    }
    assert_eq!(
        head(&a) - connected,
        0,
        "eight unchanged heartbeats must consume zero replay entries"
    );
    assert_eq!(
        connected - before,
        0,
        "second connection leaves the aggregate unchanged"
    );
    assert_eq!(observer.sequence(&a).await, []);
    close_device(&a, first, 1).await;
    assert_eq!(observer.sequence(&a).await, []);
    close_device(&a, second, 0).await;
    observer
        .sees(&a, api::Presence::Offline, "final device closed")
        .await;
    let stale_badge = || campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast {
        user_id: DAVID,
        presence: "online".into(),
        status_text: Some("outdated badge snapshot".into()),
    };
    let before = head(&a);
    a.db()
        .write(move |tx| {
            tx.emit_after_commit(campfire_db::Event::broadcast(&stale_badge()));
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        head(&a) - before,
        0,
        "a badge trigger rereads the unchanged offline aggregate"
    );
    assert_eq!(observer.sequence(&a).await, []);
    drop(observer);
    tokio::time::timeout(Duration::from_secs(5), async {
        while a.booted.app.cable.sync_wanted() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("all sync observers disconnected");
    let mut observer = Observer::open(&a, addr).await;
    let before = head(&a);
    a.db()
        .write(|tx| {
            tx.broadcast_after_commit_once(&campfire_db::broadcasts::Broadcast::UserStatus {
                user_id: DAVID,
            });
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        head(&a) - before,
        1,
        "the first value after an observer gap is published"
    );
    observer
        .sees(
            &a,
            api::Presence::Offline,
            "a new observer after the global connection gap",
        )
        .await;
    let before = head(&a);
    a.db()
        .write(|tx| {
            tx.broadcast_after_commit_once(&campfire_db::broadcasts::Broadcast::UserStatus {
                user_id: DAVID,
            });
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        head(&a) - before,
        0,
        "the new observer lifetime dedupes repeated offline values"
    );
    assert_eq!(observer.sequence(&a).await, []);
    server.abort();
}

#[tokio::test]
async fn concurrent_device_heartbeats_publish_one_change_without_a_stale_final_event() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut observer = Observer::open(&a, addr).await;
    let mut first = device(&a, addr).await;
    let second = device(&a, addr).await;
    observer
        .sees(&a, api::Presence::Online, "both connected")
        .await;
    age_davids_leases(&a).await;
    heartbeat(&mut first, false).await;
    observer
        .sees(&a, api::Presence::Idle, "all devices idle")
        .await;
    let before = head(&a);
    let start = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let send = |mut sync: Sync, active, start: std::sync::Arc<tokio::sync::Barrier>| {
        tokio::spawn(async move {
            start.wait().await;
            heartbeat(&mut sync, active).await;
            sync
        })
    };
    let quiet = send(first, false, start.clone());
    let active = send(second, true, start.clone());
    start.wait().await;
    let (quiet, active) = tokio::join!(quiet, active);
    let (quiet, active) = (quiet.unwrap(), active.unwrap());
    assert_eq!(
        head(&a) - before,
        1,
        "concurrent heartbeats publish exactly one aggregate change"
    );
    observer
        .sees(
            &a,
            api::Presence::Online,
            "concurrent active and quiet heartbeats completed",
        )
        .await;
    close_device(&a, quiet, 1).await;
    assert_eq!(observer.sequence(&a).await, []);
    close_device(&a, active, 0).await;
    observer
        .sees(&a, api::Presence::Offline, "last active device closed")
        .await;
    server.abort();
}

#[tokio::test]
async fn status_changes_and_heartbeats_share_the_effective_presence_value() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut observer = Observer::open(&a, addr).await;
    let mut first = device(&a, addr).await;
    let mut second = device(&a, addr).await;
    observer
        .sees(&a, api::Presence::Online, "both connected")
        .await;
    let mut browser = a.sign_in(DAVID).await;
    for (setting, text, expected) in [
        ("dnd", "Writing", api::Presence::Dnd),
        ("invisible", "Writing", api::Presence::Offline),
        ("invisible", "Reading", api::Presence::Offline),
        ("auto", "Reading", api::Presence::Online),
    ] {
        let reply = browser
            .write(json_body(
                Method::PATCH,
                "/api/v1/settings/status",
                &json!({
                    "presenceSetting": setting, "customStatusText": text, "customStatusEmoji": "",
                }),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let rows = observer.sequence(&a).await;
        assert_eq!(
            rows,
            [api::UserPresence {
                user_id: DAVID,
                presence: expected,
                status_text: Some(text.into())
            }]
        );
        let before = head(&a);
        heartbeat(&mut first, true).await;
        heartbeat(&mut second, false).await;
        assert_eq!(
            head(&a) - before,
            0,
            "heartbeats after {setting} must not republish the same effective value"
        );
        assert_eq!(observer.sequence(&a).await, []);
    }
    close_device(&a, first, 1).await;
    close_device(&a, second, 0).await;
    observer
        .sees(&a, api::Presence::Offline, "both devices closed")
        .await;
    server.abort();
}

#[tokio::test]
async fn expired_leases_publish_offline_once_and_a_live_device_can_reestablish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut observer = Observer::open(&a, addr).await;
    let mut first = device(&a, addr).await;
    let _second = device(&a, addr).await;
    observer
        .sees(&a, api::Presence::Online, "both connected")
        .await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE workspace_presence_leases SET expires_at=? WHERE user_id=?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(1)), DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let before = head(&a);
    assert_eq!(
        a.db()
            .write(|tx| campfire_db::WorkspacePresenceLease::prune(tx, 100))
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        head(&a) - before,
        1,
        "expiry sweep publishes one offline change for two expired leases"
    );
    observer
        .sees(&a, api::Presence::Offline, "expiry sweep")
        .await;
    let before = head(&a);
    assert_eq!(
        a.db()
            .write(|tx| campfire_db::WorkspacePresenceLease::prune(tx, 100))
            .await
            .unwrap(),
        0
    );
    assert_eq!(head(&a) - before, 0);
    heartbeat(&mut first, true).await;
    observer
        .sees(
            &a,
            api::Presence::Online,
            "a pruned device reestablished its lease",
        )
        .await;
    server.abort();
}

#[tokio::test]
async fn revoking_sessions_publishes_only_the_final_device_going_offline() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut observer = Observer::open(&a, addr).await;
    let _first = device(&a, addr).await;
    let _second = device(&a, addr).await;
    observer
        .sees(&a, api::Presence::Online, "both connected")
        .await;
    let sessions = a
        .db()
        .read(|conn| {
            Ok(conn
                .prepare(
                    "SELECT session_id FROM workspace_presence_leases WHERE user_id=? ORDER BY id",
                )?
                .query_map([DAVID], |row| row.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    assert_eq!(sessions.len(), 2);
    for (index, session_id) in sessions.into_iter().enumerate() {
        let before = head(&a);
        a.db()
            .write(move |tx| campfire_db::Session::find(tx.conn(), session_id)?.destroy(tx))
            .await
            .unwrap();
        if index == 0 {
            assert_eq!(
                head(&a) - before,
                0,
                "revoking one of two active sessions does not change presence"
            );
            assert_eq!(observer.sequence(&a).await, []);
        } else {
            assert_eq!(
                head(&a) - before,
                1,
                "session revocation publishes one final offline change"
            );
            observer
                .sees(&a, api::Presence::Offline, "the final session was revoked")
                .await;
        }
    }
    server.abort();
}

#[tokio::test]
async fn push_device_class_from_hello_survives_lease_reestablishment() {
    for class in [
        api::DeviceClass::Mobile,
        api::DeviceClass::Desktop,
        api::DeviceClass::Unknown,
    ] {
        let Some(a) = app(true).await else {
            return;
        };
        let (addr, server) = serve(&a).await;
        let browser = a.sign_in(DAVID).await;
        let mut sync = Sync::open_for_device(
            addr,
            &browser.cookie_header(),
            &[],
            Value::Null,
            Some(class),
        )
        .await;
        sync.welcome().await;
        processed(&mut sync).await;
        for recreate in [false, true] {
            if recreate {
                a.db()
                    .write(|tx| {
                        tx.conn().execute(
                            "DELETE FROM workspace_presence_leases WHERE user_id=?",
                            [DAVID],
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
                heartbeat(&mut sync, false).await;
            }
            let now = a.booted.app.db.env().now();
            let (saved, activity) = a
                .db()
                .read(move |conn| {
                    let saved = conn.query_row(
                        "SELECT device_class FROM workspace_presence_leases WHERE user_id=?",
                        [DAVID],
                        |row| row.get::<_, Option<String>>(0),
                    )?;
                    let activity = campfire_db::WorkspacePresenceLease::activity_by_user_id(
                        conn,
                        &[DAVID],
                        now,
                    )?;
                    Ok((saved, activity[&DAVID]))
                })
                .await
                .unwrap();
            assert_eq!(
                saved.as_deref(),
                Some(match class {
                    api::DeviceClass::Mobile => "mobile",
                    api::DeviceClass::Desktop => "desktop",
                    api::DeviceClass::Unknown => "unknown",
                })
            );
            assert_eq!(
                activity.overall,
                campfire_db::models::workspace_presence_lease::Presence::Online
            );
            assert_eq!(activity.active_desktop, class == api::DeviceClass::Desktop);
        }
        sync.socket.close(None).await.unwrap();
        server.abort();
    }
}

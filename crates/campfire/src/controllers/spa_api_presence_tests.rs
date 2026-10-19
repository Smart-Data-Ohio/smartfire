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

    async fn sees(&mut self, wanted: api::Presence, after: &str) {
        let event = self.sync.until(
            |event| {
                matches!(&event.payload, api::SyncPayload::Presence(row)
                    if row.user_id == DAVID && row.presence == wanted)
            },
            |_| false,
        );
        assert!(
            tokio::time::timeout(Duration::from_secs(5), event)
                .await
                .is_ok(),
            "Jason never saw David {wanted:?} after {after}"
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
    sync
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
        .sees(api::Presence::Online, "both devices connected")
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
        quiet.send(json!({"t": "hb", "active": false})).await;
        jason
            .sees(
                api::Presence::Idle,
                "a quiet heartbeat with both devices past the idle cutoff",
            )
            .await;
        active.send(json!({"t": "hb", "active": true})).await;
        jason
            .sees(
                api::Presence::Online,
                "an active heartbeat from the other device",
            )
            .await;
    }

    drop(active);
    jason
        .sees(
            api::Presence::Idle,
            "the active device closed, leaving the idle one",
        )
        .await;
    drop(quiet);
    jason
        .sees(api::Presence::Offline, "the last device closed")
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

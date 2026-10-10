//! The S5 `/api/v1` huddle and stage endpoints over the seeded app with `SPA_ENABLED` and huddles
//! configured: the classic gates as envelopes, the contract's shapes, the sync twins of the huddle
//! broadcasts, and the classic frames unchanged with the sync engine on.

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::{CachedStatements, Membership, Room, RoomType, StageRole};
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;

use crate::controllers::presenters::test_support::{
    BENDER_KEY, Browser, DAVID, HQ, JASON, KEVIN, Reply, Req, TestApp, seed_clock,
};

/// All Pets holds David but not Kevin.
const ALL_PETS: i64 = 104393281;

fn configured() -> crate::huddle::Config {
    crate::huddle::Config {
        public_url: Some("wss://public.example.test".into()),
        internal_url: Some("http://internal.example.test:7880".into()),
        api_key: Some("ws13-fixture-api-key".into()),
        api_secret: Some("ws13-fixture-api-secret".into()),
        gateway_secret: Some("ws13-fixture-gateway-secret".into()),
    }
}

fn domain() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("ws13-fixture-api-secret".into()),
        admin_configured: true,
    }
}

async fn app(spa: bool) -> Option<TestApp> {
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    TestApp::boot_with_settings(configured(), seed_clock(), env).await
}

fn get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

fn send_json(method: Method, path: &str, body: &Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

fn empty(method: Method, path: &str) -> Req {
    Req::new(method, path).header("accept", "application/json")
}

fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

/// The envelope's tag and message.
fn denial(reply: &Reply) -> (StatusCode, String, String) {
    let body = reply.json();
    let error = &body["error"];
    (
        reply.status,
        error["_tag"].as_str().unwrap_or_default().into(),
        error["message"].as_str().unwrap_or_default().into(),
    )
}

fn denied(status: StatusCode, tag: &str, message: &str) -> (StatusCode, String, String) {
    (status, tag.into(), message.into())
}

/// A stage room hosted by David, with Jason (made a non-administrator) listening and Kevin
/// speaking: the room and the three memberships.
async fn stage(a: &TestApp) -> (Room, i64, i64, i64) {
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=0 WHERE id=?", [JASON])?;
            let room = Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )?;
            let member = |tx: &campfire_db::Tx<'_>, user| {
                Membership::find_by_room_and_user(tx.conn(), room.id, user).map(|m| m.unwrap().id)
            };
            let (host, listener, speaker) =
                (member(tx, DAVID)?, member(tx, JASON)?, member(tx, KEVIN)?);
            Membership::find(tx.conn(), speaker)?.change_stage_role_with_config(
                tx,
                StageRole::Speaker,
                &domain(),
            )?;
            Ok((room, host, listener, speaker))
        })
        .await
        .unwrap()
}

async fn voice(a: &TestApp) -> Room {
    a.db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Voice,
                Some("Lounge"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )
        })
        .await
        .unwrap()
}

/// What the gateway does when the participant connects: the grant is seen, so they're in the
/// call (and the presence and join-notice jobs run).
async fn seen(a: &TestApp, grant_id: i64) {
    a.db()
        .write(move |tx| {
            let mut grant = HuddleGrant::find_by_id(tx.conn(), grant_id)?.unwrap();
            grant.record_seen(tx)?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn join(b: &mut Browser<'_>, room_id: i64) -> api::HuddleCredentials {
    let reply = b
        .write(empty(
            Method::POST,
            &format!("/api/v1/rooms/{room_id}/huddle"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

/// The LiveKit token's claims.
fn claims(credentials: &api::HuddleCredentials) -> Value {
    use base64::Engine as _;
    let payload = credentials.token.split('.').nth(1).expect("a JWT");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("base64url claims");
    serde_json::from_slice(&bytes).expect("JSON claims")
}

/// The token is the classic `Huddle#token`: this room, this identity, two minutes, and publishing
/// (with the classic sources) only when `publish`.
fn assert_grant(credentials: &api::HuddleCredentials, room_id: i64, publish: bool) {
    let claims = claims(credentials);
    let video = &claims["video"];
    assert_eq!(
        video["room"],
        rails_compat::jwt::livekit::room_name("ws13-fixture-api-secret", room_id)
    );
    assert_eq!(claims["sub"], credentials.identity.as_str());
    assert_eq!(
        claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
        120
    );
    assert_eq!(
        (video["canPublish"].as_bool(), credentials.can_publish),
        (Some(publish), publish)
    );
    let sources = if publish {
        json!(["microphone", "screen_share", "screen_share_audio", "camera"])
    } else {
        json!([])
    };
    assert_eq!(video["canPublishSources"], sources);
    assert_eq!(video["roomJoin"], true);
    assert_eq!(video["canSubscribe"], true);
}

async fn serve(a: &TestApp) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    (
        addr,
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() }),
    )
}

/// A sync socket (`spa_api_tests` has the full harness; this is the part these tests need).
struct Sync {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    pending: std::collections::VecDeque<api::SyncEvent>,
}

impl Sync {
    async fn connect(addr: SocketAddr, cookie: &str, topics: &[String]) -> Self {
        let mut request = format!("ws://{addr}/api/v1/sync")
            .into_client_request()
            .unwrap();
        let headers = request.headers_mut();
        headers.insert("cookie", cookie.parse().unwrap());
        headers.insert("host", "campfire.test".parse().unwrap());
        headers.insert("origin", "http://campfire.test".parse().unwrap());
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let hello = json!({"t": "hello", "v": 1, "resume": null, "topics": topics});
        socket
            .send(WsMessage::Text(hello.to_string().into()))
            .await
            .unwrap();
        let mut sync = Self {
            socket,
            pending: Default::default(),
        };
        assert!(matches!(
            sync.next().await,
            api::ServerFrame::Welcome { resumed: false, .. }
        ));
        sync
    }

    async fn next(&mut self) -> api::ServerFrame {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.socket.next())
                .await
                .unwrap_or_else(|_| panic!("no sync frame in time; unclaimed {:#?}", self.pending));
            match message {
                Some(Ok(WsMessage::Text(text))) => {
                    return serde_json::from_str(&text)
                        .unwrap_or_else(|error| panic!("{error}: {text}"));
                }
                Some(Ok(_)) => continue,
                other => panic!("the sync socket closed: {other:?}"),
            }
        }
    }

    /// Drops the events received but not claimed: what comes after is newer.
    fn forget(&mut self) {
        self.pending.clear();
    }

    /// The first event `wanted` picks out, in order; the others stay for later calls.
    async fn until<T>(&mut self, wanted: impl Fn(&api::SyncEvent) -> Option<T>) -> T {
        let mut looked = 0;
        loop {
            for index in looked..self.pending.len() {
                if let Some(found) = wanted(&self.pending[index]) {
                    self.pending.remove(index);
                    return found;
                }
            }
            looked = self.pending.len();
            match self.next().await {
                api::ServerFrame::Batch { events } => self.pending.extend(events),
                api::ServerFrame::Ping => {}
                other => panic!("expected a batch, got {other:?}"),
            }
        }
    }
}

fn presence_of(room_id: i64) -> impl Fn(&api::SyncEvent) -> Option<api::HuddlePresence> {
    move |event| match &event.payload {
        api::SyncPayload::HuddlePresence(presence) if presence.room_id == room_id => {
            Some(presence.clone())
        }
        _ => None,
    }
}

fn stage_of(room_id: i64) -> impl Fn(&api::SyncEvent) -> Option<api::StageState> {
    move |event| match &event.payload {
        api::SyncPayload::StageUpdated(stage) if stage.room_id == room_id => Some(stage.clone()),
        _ => None,
    }
}

#[tokio::test]
async fn the_huddle_endpoints_keep_the_classic_gates() {
    let Some(unconfigured) =
        TestApp::boot_seed_with_env("default", seed_clock(), &[("SPA_ENABLED", "1")]).await
    else {
        return;
    };
    let mut b = unconfigured.sign_in(DAVID).await;
    for reply in [
        b.send(get("/api/v1/huddles")).await,
        b.send(get(&format!("/api/v1/rooms/{HQ}/huddle"))).await,
        b.write(empty(Method::POST, &format!("/api/v1/rooms/{HQ}/huddle")))
            .await,
    ] {
        assert_eq!(
            denial(&reply),
            denied(
                StatusCode::SERVICE_UNAVAILABLE,
                "Unavailable",
                "Huddles are not configured"
            )
        );
        assert_eq!(reply.header("cache-control"), Some("no-store"));
    }
    drop(b);
    drop(unconfigured);

    let Some(a) = app(true).await else { return };
    let mut anonymous = a.anonymous();
    for path in [
        "/api/v1/huddles".to_string(),
        format!("/api/v1/rooms/{HQ}/huddle"),
    ] {
        let reply = anonymous.send(get(&path)).await;
        assert_eq!(
            (reply.status, denial(&reply).1),
            (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            "{path}"
        );
        let reply = anonymous
            .send(get(&format!("{path}?bot_key={BENDER_KEY}")))
            .await;
        assert_eq!(
            denial(&reply),
            denied(
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Bots cannot join huddles"
            ),
            "{path}"
        );
    }

    let mut kevin = a.sign_in(KEVIN).await;
    for reply in [
        kevin
            .send(get(&format!("/api/v1/rooms/{ALL_PETS}/huddle")))
            .await,
        kevin
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{ALL_PETS}/huddle"),
            ))
            .await,
        kevin
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{ALL_PETS}/huddle/leave"),
            ))
            .await,
    ] {
        assert_eq!(
            denial(&reply),
            denied(
                StatusCode::NOT_FOUND,
                "NotFound",
                "Room not found or inaccessible"
            )
        );
    }
    let forged = kevin
        .send(empty(Method::POST, &format!("/api/v1/rooms/{HQ}/huddle")))
        .await;
    assert_eq!(denial(&forged).1, "InvalidAuthenticityToken");
    let grants = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM huddle_grants", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(grants, 0, "no denied request issued a grant");
}

#[tokio::test]
async fn joining_answers_credentials_and_presence_and_notices_follow() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut kevin_sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;

    // Kevin is in HQ's call first: join notices go to the people in the call.
    let kevins = join(&mut kevin, HQ).await;
    seen(&a, kevins.grant_id).await;
    let presence = kevin_sync
        .until(|event| presence_of(HQ)(event).filter(|p| !p.participants.is_empty()))
        .await;
    assert_eq!(
        presence
            .participants
            .iter()
            .map(|p| p.user_id)
            .collect::<Vec<_>>(),
        vec![KEVIN]
    );

    let credentials = join(&mut david, HQ).await;
    assert_eq!(credentials.url, "wss://public.example.test");
    assert_eq!(
        (credentials.room_id, credentials.room_name.as_str()),
        (HQ, "HQ")
    );
    assert!(credentials.can_publish);
    assert_eq!(credentials.token.split('.').count(), 3);
    let grant = a
        .db()
        .read(move |conn| Ok(HuddleGrant::find_by_id(conn, credentials.grant_id)?.unwrap()))
        .await
        .unwrap();
    assert_eq!(grant.identity, credentials.identity);
    assert_eq!(grant.user_id, DAVID);

    // Issued but not connected yet: David isn't in the call.
    let list: api::HuddlePresenceList = parse(&david.send(get("/api/v1/huddles")).await);
    assert_eq!(list.rooms, vec![presence]);

    seen(&a, grant.id).await;
    let presence = kevin_sync
        .until(|event| presence_of(HQ)(event).filter(|p| p.participants.len() == 2))
        .await;
    // By name: David, then Kevin.
    assert_eq!(presence.participants[0].user_id, DAVID);
    assert_eq!(
        presence.participants[0].identities,
        vec![grant.identity.clone()]
    );
    assert_eq!(
        presence.participants[1].identities,
        vec![kevins.identity.clone()]
    );
    assert!(!presence.live);
    let notice = kevin_sync
        .until(|event| match &event.payload {
            api::SyncPayload::HuddleNotice(notice) => Some(notice.clone()),
            _ => None,
        })
        .await;
    match notice {
        api::HuddleNotice::Joined {
            room_id,
            user_id,
            in_call,
            rejoin,
            ..
        } => assert_eq!(
            (room_id, user_id, in_call, rejoin),
            (HQ, DAVID, true, false)
        ),
        other => panic!("expected a join notice, got {other:?}"),
    }

    kevin_sync.forget();
    let list: api::HuddlePresenceList = parse(&david.send(get("/api/v1/huddles")).await);
    assert_eq!(list.rooms, vec![presence.clone()]);
    assert_eq!(
        list.users.iter().map(|user| user.id).collect::<Vec<_>>(),
        vec![DAVID, KEVIN]
    );
    let detail: api::HuddleDetail =
        parse(&david.send(get(&format!("/api/v1/rooms/{HQ}/huddle"))).await);
    assert_eq!(
        (detail.room_name.as_str(), &detail.presence),
        ("HQ", &presence)
    );
    assert_eq!(detail.users.len(), 2);

    let left = david
        .write(empty(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/huddle/leave"),
        ))
        .await;
    assert_eq!(left.status, StatusCode::NO_CONTENT, "{}", left.text());
    let presence = kevin_sync
        .until(|event| presence_of(HQ)(event).filter(|p| p.participants.len() == 1))
        .await;
    assert_eq!(presence.participants[0].user_id, KEVIN);
    let notice = kevin_sync
        .until(|event| match &event.payload {
            api::SyncPayload::HuddleNotice(notice) => Some(notice.clone()),
            _ => None,
        })
        .await;
    assert!(
        matches!(
            notice,
            api::HuddleNotice::Left {
                room_id: HQ,
                user_id: DAVID,
                ..
            }
        ),
        "{notice:?}"
    );
    server.abort();
}

#[tokio::test]
async fn a_direct_call_rings_the_other_person() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let direct = a
        .db()
        .write(|tx| Room::find_or_create_direct_for(tx, &[DAVID, KEVIN], DAVID))
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut kevin_sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    let credentials = join(&mut david, direct.id).await;
    assert_eq!(credentials.room_name, "Kevin");
    let ring = kevin_sync
        .until(|event| match &event.payload {
            api::SyncPayload::HuddleRing(ring) => Some(ring.clone()),
            _ => None,
        })
        .await;
    assert_eq!(
        (
            ring.room_id,
            ring.event,
            ring.state,
            ring.caller_name.as_str()
        ),
        (
            direct.id,
            api::HuddleRingEvent::Started,
            api::HuddleRingState::Unread,
            "David"
        )
    );
    server.abort();
}

#[tokio::test]
async fn moderation_follows_call_moderation() {
    let Some(a) = app(true).await else { return };
    let room = voice(&a).await;
    let room_id = room.id;
    let (david_member, jason_member) = a
        .db()
        .read(move |c| {
            let id =
                |user| Membership::find_by_room_and_user(c, room_id, user).map(|m| m.unwrap().id);
            Ok((id(DAVID)?, id(JASON)?))
        })
        .await
        .unwrap();
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=0 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/api/v1/rooms/{}/huddle/moderation", room.id);
    let mut jason = a.sign_in(JASON).await;
    let reply = jason
        .write(send_json(
            Method::POST,
            &path,
            &json!({"membershipId": david_member, "action": "mute"}),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::FORBIDDEN,
            "Forbidden",
            "Only administrators and stage hosts can moderate calls"
        )
    );
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .write(send_json(
            Method::POST,
            &path,
            &json!({"membershipId": david_member, "action": "mute"}),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Validation",
            "You cannot moderate your own call session"
        )
    );
    let reply = david
        .write(send_json(
            Method::POST,
            &path,
            &json!({"membershipId": 1, "action": "mute"}),
        ))
        .await;
    assert_eq!(denial(&reply).1, "NotFound");
    let reply = david
        .write(send_json(
            Method::POST,
            &path,
            &json!({"membershipId": jason_member, "action": "ban"}),
        ))
        .await;
    assert_eq!(denial(&reply).1, "Validation");

    let reply = david
        .write(send_json(
            Method::POST,
            &path,
            &json!({"membershipId": jason_member, "action": "mute"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let muted = a
        .db()
        .read(move |c| Ok(Membership::find(c, jason_member)?.server_muted_at.is_some()))
        .await
        .unwrap();
    assert!(muted);
    // The muted member's next token can't publish.
    let credentials = join(&mut jason, room.id).await;
    assert!(!credentials.can_publish);
    // HQ isn't a call room: moderation there is a 404, as in the classic app.
    let reply = david
        .write(send_json(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/huddle/moderation"),
            &json!({"membershipId": jason_member, "action": "unmute"}),
        ))
        .await;
    assert_eq!(denial(&reply).1, "NotFound");
    // Nor is a direct message, though it has calls: moderation is for voice rooms and stages.
    let direct = a
        .db()
        .write(|tx| Room::find_or_create_direct_for(tx, &[DAVID, JASON], DAVID))
        .await
        .unwrap();
    let direct_jason = a
        .db()
        .read(move |c| {
            Ok(Membership::find_by_room_and_user(c, direct.id, JASON)?
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let reply = david
        .write(send_json(
            Method::POST,
            &format!("/api/v1/rooms/{}/huddle/moderation", direct.id),
            &json!({"membershipId": direct_jason, "action": "mute"}),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(StatusCode::NOT_FOUND, "NotFound", "Not found")
    );
}

#[tokio::test]
async fn the_livekit_token_is_the_classic_grant() {
    let Some(a) = app(true).await else { return };
    let (room, _, _, speaker) = stage(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    // A stage: the host and a speaker publish, a listener only listens.
    assert_grant(&join(&mut david, room.id).await, room.id, true);
    assert_grant(&join(&mut jason, room.id).await, room.id, false);
    let kevins = join(&mut kevin, room.id).await;
    assert_grant(&kevins, room.id, true);
    // A server mute takes publishing away from the next token.
    seen(&a, kevins.grant_id).await;
    let reply = david
        .write(send_json(
            Method::POST,
            &format!("/api/v1/rooms/{}/huddle/moderation", room.id),
            &json!({"membershipId": speaker, "action": "mute"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    assert_grant(&join(&mut kevin, room.id).await, room.id, false);
    // A voice room: everyone publishes.
    let lounge = voice(&a).await;
    assert_grant(&join(&mut jason, lounge.id).await, lounge.id, true);
}

#[tokio::test]
async fn a_stage_says_nothing_to_people_outside_it() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    // A stage without Kevin, and a voice room with him (the sentinel).
    let (room, jason_member) = a
        .db()
        .write(|tx| {
            let room =
                Room::create_for(tx, RoomType::Stage, Some("Board"), DAVID, &[DAVID, JASON])?;
            let jason = Membership::find_by_room_and_user(tx.conn(), room.id, JASON)?
                .unwrap()
                .id;
            Ok((room, jason))
        })
        .await
        .unwrap();
    let room_id = room.id;
    let lounge = voice(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut kevin_sync =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{room_id}")]).await;

    // Roster, hand and call changes on the stage.
    let reply = jason
        .write(empty(
            Method::POST,
            &format!("/api/v1/rooms/{room_id}/stage/hand"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let davids = join(&mut david, room_id).await;
    seen(&a, davids.grant_id).await;
    let reply = david
        .write(send_json(
            Method::PATCH,
            &format!("/api/v1/rooms/{room_id}/stage/members/{jason_member}"),
            &json!({"role": "speaker"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Then something Kevin does hear: everything before it has been delivered.
    let jasons = join(&mut jason, lounge.id).await;
    seen(&a, jasons.grant_id).await;
    kevin_sync.until(presence_of(lounge.id)).await;
    let leaked: Vec<_> = kevin_sync
        .pending
        .iter()
        .filter(|event| match &event.payload {
            api::SyncPayload::StageUpdated(stage) => stage.room_id == room_id,
            api::SyncPayload::HuddlePresence(presence) => presence.room_id == room_id,
            api::SyncPayload::HuddleNotice(_) | api::SyncPayload::HuddleRing(_) => true,
            _ => false,
        })
        .collect();
    assert!(leaked.is_empty(), "{leaked:#?}");

    // Every stage and moderation write is a 404 to him, as is reading it.
    let base = format!("/api/v1/rooms/{room_id}/stage");
    for reply in [
        kevin.send(get(&base)).await,
        kevin
            .write(empty(Method::POST, &format!("{base}/hand")))
            .await,
        kevin
            .write(empty(
                Method::DELETE,
                &format!("{base}/hand?membershipId={jason_member}"),
            ))
            .await,
        kevin
            .write(send_json(
                Method::PATCH,
                &format!("{base}/members/{jason_member}"),
                &json!({"role": "host"}),
            ))
            .await,
        kevin
            .write(send_json(
                Method::POST,
                &format!("{base}/stream"),
                &json!({"quality": "720p15"}),
            ))
            .await,
        kevin
            .write(empty(Method::DELETE, &format!("{base}/stream?streamId=1")))
            .await,
        kevin
            .write(send_json(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle/moderation"),
                &json!({"membershipId": jason_member, "action": "disconnect"}),
            ))
            .await,
        kevin
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle"),
            ))
            .await,
    ] {
        // The message varies by controller ("Not found" or "Room not found or inaccessible");
        // what matters is the 404, which says nothing about the stage.
        let (status, tag, _) = denial(&reply);
        assert_eq!(
            (status, tag.as_str()),
            (StatusCode::NOT_FOUND, "NotFound"),
            "{}",
            reply.text()
        );
    }
    server.abort();
}

#[tokio::test]
async fn the_stage_answers_its_state_and_follows_roles_hands_and_streams() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let (room, host, listener, speaker) = stage(&a).await;
    let room_id = room.id;
    let base = format!("/api/v1/rooms/{room_id}/stage");
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jason_sync =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{room_id}")]).await;

    let detail: api::StageDetail = parse(&jason.send(get(&base)).await);
    let roles: Vec<_> = detail
        .stage
        .members
        .iter()
        .map(|member| (member.membership_id, member.role))
        .collect();
    assert_eq!(
        roles,
        vec![
            (host, api::StageRole::Host),
            (listener, api::StageRole::Listener),
            (speaker, api::StageRole::Speaker)
        ]
    );
    assert!(detail.stage.live.is_none());
    assert_eq!(detail.users.len(), 3);
    // HQ isn't a stage.
    assert_eq!(
        denial(&david.send(get(&format!("/api/v1/rooms/{HQ}/stage"))).await).1,
        "NotFound"
    );

    // Hands: listeners only; a host lowers anyone's.
    let reply = kevin
        .write(empty(Method::POST, &format!("{base}/hand")))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Validation",
            "Only listeners can raise a hand"
        )
    );
    let raised: api::StageState = parse(
        &jason
            .write(empty(Method::POST, &format!("{base}/hand")))
            .await,
    );
    let hand = |stage: &api::StageState| {
        stage
            .members
            .iter()
            .find(|member| member.membership_id == listener)
            .and_then(|member| member.hand_raised_at.clone())
    };
    assert!(hand(&raised).is_some());
    let event = jason_sync.until(stage_of(room_id)).await;
    assert!(hand(&event).is_some());
    let reply = kevin
        .write(empty(
            Method::DELETE,
            &format!("{base}/hand?membershipId={listener}"),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::FORBIDDEN,
            "Forbidden",
            "Only hosts can lower another member's hand"
        )
    );
    let lowered: api::StageState = parse(
        &david
            .write(empty(
                Method::DELETE,
                &format!("{base}/hand?membershipId={listener}"),
            ))
            .await,
    );
    assert!(hand(&lowered).is_none());

    // Roles: hosts and administrators; the last host stays.
    let reply = kevin
        .write(send_json(
            Method::PATCH,
            &format!("{base}/members/{listener}"),
            &json!({"role": "speaker"}),
        ))
        .await;
    assert_eq!(denial(&reply).0, StatusCode::FORBIDDEN);
    let reply = david
        .write(send_json(
            Method::PATCH,
            &format!("{base}/members/{host}"),
            &json!({"role": "listener"}),
        ))
        .await;
    assert_eq!(
        (denial(&reply).0, denial(&reply).1),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let promoted: api::StageState = parse(
        &david
            .write(send_json(
                Method::PATCH,
                &format!("{base}/members/{listener}"),
                &json!({"role": "speaker"}),
            ))
            .await,
    );
    assert!(
        promoted
            .members
            .iter()
            .any(|m| m.membership_id == listener && m.role == api::StageRole::Speaker)
    );
    let role = jason_sync
        .until(|event| match &event.payload {
            api::SyncPayload::HuddleRole(role) if role.room_id == room_id => Some(*role),
            _ => None,
        })
        .await;
    assert_eq!(
        (role.stage_role, role.server_muted),
        (Some(api::StageRole::Speaker), false)
    );

    // Going live: in the call first; one stream at a time; the host stops someone else's.
    let reply = kevin
        .write(send_json(
            Method::POST,
            &format!("{base}/stream"),
            &json!({"quality": "1080p15"}),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::FORBIDDEN,
            "Forbidden",
            "Join the stage before going live"
        )
    );
    let credentials = join(&mut kevin, room_id).await;
    assert!(credentials.can_publish);
    seen(&a, credentials.grant_id).await;
    let reply = kevin
        .write(send_json(
            Method::POST,
            &format!("{base}/stream"),
            &json!({"quality": "1080p15"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let live: api::StageStream = parse(&reply);
    assert_eq!(
        (live.membership_id, live.quality, live.identity.as_deref()),
        (
            speaker,
            api::StreamQuality::P1080Fps15,
            Some(credentials.identity.as_str())
        )
    );
    let event = jason_sync
        .until(|event| stage_of(room_id)(event).filter(|stage| stage.live.is_some()))
        .await;
    assert_eq!(event.live.as_ref().map(|live| live.id), Some(live.id));
    jason_sync.forget();
    let jason_credentials = join(&mut jason, room_id).await;
    seen(&a, jason_credentials.grant_id).await;
    let reply = jason
        .write(send_json(
            Method::POST,
            &format!("{base}/stream"),
            &json!({"quality": "720p15"}),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(StatusCode::CONFLICT, "Conflict", "Kevin is already live")
    );
    let reply = jason
        .write(empty(
            Method::DELETE,
            &format!("{base}/stream?streamId={}", live.id),
        ))
        .await;
    assert_eq!(
        denial(&reply),
        denied(
            StatusCode::FORBIDDEN,
            "Forbidden",
            "Only the presenter or a host can stop the stream"
        )
    );
    // A stale stop leaves the live stream alone.
    let stale = david
        .write(empty(
            Method::DELETE,
            &format!("{base}/stream?streamId={}", live.id + 1000),
        ))
        .await;
    assert_eq!(stale.status, StatusCode::NO_CONTENT);
    let detail: api::StageDetail = parse(&david.send(get(&base)).await);
    assert_eq!(detail.stage.live.map(|live| live.id), Some(live.id));
    let kevin_sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    let stopped = david
        .write(empty(
            Method::DELETE,
            &format!("{base}/stream?streamId={}", live.id),
        ))
        .await;
    assert_eq!(stopped.status, StatusCode::NO_CONTENT, "{}", stopped.text());
    let mut kevin_sync = kevin_sync;
    let stopped = kevin_sync
        .until(|event| match &event.payload {
            api::SyncPayload::StageStreamStopped(stopped) => Some(stopped.room_id),
            _ => None,
        })
        .await;
    assert_eq!(stopped, room_id);
    jason_sync
        .until(|event| stage_of(room_id)(event).filter(|stage| stage.live.is_none()))
        .await;
    server.abort();
}

#[tokio::test]
async fn a_stage_goes_live_at_1080p60_and_every_viewer_reads_it_back() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let (room, _, _, speaker) = stage(&a).await;
    let room_id = room.id;
    let base = format!("/api/v1/rooms/{room_id}/stage");
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jason_sync =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{room_id}")]).await;
    let credentials = join(&mut kevin, room_id).await;
    seen(&a, credentials.grant_id).await;

    let reply = kevin
        .write(send_json(
            Method::POST,
            &format!("{base}/stream"),
            &json!({"quality": "1080p60"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let body: Value = parse(&reply);
    assert_eq!(body["quality"], "1080p60");
    let live: api::StageStream = parse(&reply);
    assert_eq!(
        (live.membership_id, live.quality),
        (speaker, api::StreamQuality::P1080Fps60)
    );
    // Stored as the same quality string as the classic three: no migration.
    let stored = a
        .db()
        .read(move |conn| campfire_db::models::stream::Stream::live_for_room(conn, room_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.quality, "1080p60");

    let event = jason_sync
        .until(|event| stage_of(room_id)(event).filter(|stage| stage.live.is_some()))
        .await;
    assert_eq!(
        event.live.map(|live| live.quality),
        Some(api::StreamQuality::P1080Fps60)
    );
    let detail: api::StageDetail = parse(&jason.send(get(&base)).await);
    assert_eq!(
        detail.stage.live.map(|live| (live.id, live.quality)),
        Some((live.id, api::StreamQuality::P1080Fps60))
    );
    server.abort();
}

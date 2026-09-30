use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const KEY: &str = "ws13-fixture-api-key";
const SECRET: &str = "ws13-fixture-api-secret";
const NOW: i64 = 1_767_268_800;

fn vectors() -> Value { serde_json::from_str(include_str!("protocol_vectors.json")).unwrap() }
fn config(internal: &str) -> Config {
    Config::from_lookup(|name| Some(match name {
        "LIVEKIT_URL" => "wss://huddle.example.test", "LIVEKIT_INTERNAL_URL" => internal,
        "LIVEKIT_API_KEY" => KEY, "LIVEKIT_API_SECRET" => SECRET, _ => "ws13-fixture-gateway-secret",
    }.into()))
}

#[test]
fn join_admin_and_opaque_names_match_pinned_rails() {
    let v = vectors();
    assert_eq!(livekit::room_name(SECRET, v["room_id"].as_i64().unwrap()), v["room_name"]);
    for token in v["tokens"].as_array().unwrap() {
        let can_publish = livekit::can_publish(token["muted"].as_bool().unwrap(), token["stage"].as_bool().unwrap(), token["role"].as_str());
        let p = livekit::Participant { name: "Fixture \"User\" ☃", identity: v["identity"].as_str().unwrap(), room_name: v["room_name"].as_str().unwrap(), can_publish };
        assert_eq!(livekit::participant_token_with_jti(KEY, SECRET, &p, NOW, v["jti"].as_str().unwrap()), token["token"], "{}", token["name"]);
        assert_eq!(livekit::verify(token["token"].as_str().unwrap(), KEY, SECRET, NOW).unwrap().identity, p.identity);
    }
    for (key, grant) in [("admin_remove", serde_json::json!({"roomAdmin": true, "room": v["room_name"]})), ("admin_delete", serde_json::json!({"roomCreate": true}))] {
        assert_eq!(livekit::admin_token_with_jti(KEY, SECRET, grant.as_object().unwrap(), NOW, v["jti"].as_str().unwrap()), v[key]);
    }
}

#[test]
fn strict_token_shapes_match_pinned_rails() {
    for case in vectors()["shapes"].as_array().unwrap() {
        let actual = livekit::verify(case["token"].as_str().unwrap(), KEY, SECRET, NOW).ok()
            .map(|c| serde_json::json!({ "identity": c.identity, "room_name": c.room_name }));
        assert_eq!(actual.unwrap_or(Value::Null), case["coordinates"], "{}", case["name"]);
    }
}

#[test]
fn listener_and_server_muted_members_never_receive_publish_grants() {
    let v = vectors();
    for name in ["listener", "missing_stage_role", "server_muted", "muted_host"] {
        let case = v["tokens"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap();
        let publish = livekit::can_publish(case["muted"].as_bool().unwrap(), case["stage"].as_bool().unwrap(), case["role"].as_str());
        assert!(!publish, "{name}");
        let grant = livekit::participant_video_grant("room", publish);
        assert_eq!(grant["canPublish"], false);
        assert_eq!(grant["canPublishSources"], serde_json::json!([]));
    }
}

#[test]
fn configured_endpoints_and_twirp_prefixes_match_rails() {
    for case in vectors()["urls"].as_array().unwrap() {
        let mut c = config(case["internal_url"].as_str().unwrap());
        c.public_url = case["public_url"].as_str().filter(|s| !s.is_empty()).map(str::to_string);
        assert_eq!(c.configured(), case["configured"].as_bool().unwrap(), "{case}");
        let endpoint = endpoint_uri(c.internal_url.as_deref().unwrap(), "RemoveParticipant").ok().map(|u| Value::String(u.to_s()));
        assert_eq!(endpoint.unwrap_or(Value::Null), case["endpoint"], "{case}");
    }
    for missing in ["LIVEKIT_URL", "LIVEKIT_INTERNAL_URL", "LIVEKIT_API_KEY", "LIVEKIT_API_SECRET", "LIVEKIT_GATEWAY_SECRET"] {
        let c = Config::from_lookup(|name| if name == missing { Some(" \t\n".into()) } else {
            Some(match name { "LIVEKIT_URL" => "wss://public.test", "LIVEKIT_INTERNAL_URL" => "ws://internal.test", _ => "fixture" }.into())
        });
        assert!(!c.configured(), "{missing}");
    }
}

#[derive(Clone, Debug)]
struct Received { head: String, body: Vec<u8> }
struct Server {
    url: String,
    received: Arc<Mutex<Vec<Received>>>,
    task: JoinHandle<()>,
}
impl Drop for Server { fn drop(&mut self) { self.task.abort(); } }

impl Server {
    async fn start(status: u16, delay: Duration) -> Self {
        let mut listener = None;
        for port in 52300..=52399 {
            if let Ok(bound) = TcpListener::bind(("127.0.0.1", port)).await { listener = Some(bound); break; }
        }
        let listener = listener.expect("no free WS13 test port");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 { break; }
                    head.push_str(&line);
                    if line == "\r\n" { break; }
                }
                let length = head.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                }).unwrap_or(0);
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).await.is_err() { continue; }
                log.lock().unwrap().push(Received { head, body });
                tokio::time::sleep(delay).await;
                // A Location is always present so a mistaken redirect implementation is observable.
                let reply = format!("HTTP/1.1 {status} Fixture\r\nLocation: /redirected\r\nContent-Length: 23\r\nConnection: close\r\n\r\nsensitive upstream body");
                let _ = reader.get_mut().write_all(reply.as_bytes()).await;
            }
        });
        Self { url, received, task }
    }
}

#[tokio::test]
async fn twirp_uses_only_the_requested_room_and_scoped_admin_grants() {
    let server = Server::start(200, Duration::ZERO).await;
    let service = RoomService::new(config(&format!("{}/prefix/?ignore#fragment", server.url)));
    service.remove_participant("opaque-room", "opaque-participant", NOW).await.unwrap();
    service.delete_room("opaque-room", NOW).await.unwrap();
    let requests = server.received.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    for (index, request) in requests.iter().enumerate() {
        let action = if index == 0 { "RemoveParticipant" } else { "DeleteRoom" };
        assert!(request.head.starts_with(&format!("POST /prefix/twirp/livekit.RoomService/{action} HTTP/1.1\r\n")), "{}", request.head);
        let authorization = request.head.lines().find_map(|line| line.strip_prefix("Authorization: ")).unwrap();
        let jwt = authorization.strip_prefix("Bearer ").unwrap();
        let claims = rails_compat::jwt::decode(jwt, rails_compat::jwt::Key::Hs256(SECRET.as_bytes()), &rails_compat::jwt::Validation::default(), NOW).unwrap().payload;
        assert_eq!(claims["iss"], KEY);
        assert_eq!(claims["exp"], NOW + 60);
        assert_eq!(claims["nbf"], NOW - 5);
        let grant = if index == 0 { serde_json::json!({"roomAdmin": true, "room": "opaque-room"}) } else { serde_json::json!({"roomCreate": true}) };
        assert_eq!(claims["video"], grant);
        assert!(claims.get("sub").is_none());
        let body = if index == 0 { serde_json::json!({"room": "opaque-room", "identity": "opaque-participant"}) } else { serde_json::json!({"room": "opaque-room"}) };
        assert_eq!(serde_json::from_slice::<Value>(&request.body).unwrap(), body);
    }
}

#[tokio::test]
async fn twirp_missing_room_and_participant_are_successes() {
    let server = Server::start(404, Duration::ZERO).await;
    let service = RoomService::new(config(&server.url));
    service.remove_participant("absent-room", "absent-participant", NOW).await.unwrap();
    service.delete_room("absent-room", NOW).await.unwrap();
    assert_eq!(server.received.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn twirp_failures_do_not_follow_redirects_or_expose_bodies() {
    for status in [302, 401, 403, 429, 500, 503] {
        let server = Server::start(status, Duration::ZERO).await;
        let error = RoomService::new(config(&server.url)).remove_participant("room", "participant", NOW).await.unwrap_err();
        assert_eq!(error.status, Some(status));
        assert_eq!(error.code, None);
        assert!(!error.to_string().contains("sensitive"));
        assert_eq!(server.received.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn twirp_invalid_urls_and_missing_configuration_fail_safely() {
    for url in ["ftp://internal.test", "relative", "http://bad host", "http://internal.test/ü"] {
        let error = RoomService::new(config(url)).delete_room("room", NOW).await.unwrap_err();
        assert_eq!(error.code, Some("URI::InvalidURIError"));
        assert!(!error.to_string().contains(url));
    }
    assert!(RoomService::new(Config::default()).delete_room("room", NOW).await.is_err());
}

#[tokio::test]
async fn twirp_read_timeout_is_reported_safely() {
    assert_eq!(OPEN_TIMEOUT, Duration::from_secs(3));
    assert_eq!(READ_TIMEOUT, Duration::from_secs(5));
    let server = Server::start(200, Duration::from_secs(10)).await;
    let service = RoomService::new(config(&server.url));
    let timeouts = Timeouts { open: OPEN_TIMEOUT, read: Duration::from_millis(40) };
    let error = service.post("DeleteRoom", serde_json::json!({"room": "room"}), serde_json::json!({"roomCreate": true}).as_object().unwrap(), NOW, &timeouts).await.unwrap_err();
    assert_eq!(error.code, Some("Net::ReadTimeout"));
    assert_eq!(server.received.lock().unwrap().len(), 1);
}

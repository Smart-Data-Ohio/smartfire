//! The booted app's own hub: sockets opened against the app's router, and broadcasts made the
//! ways the app makes them outside a channel (the models' events through the `Jobs` sink, and
//! work on the job runner), plus sign-out through the real controller. Over a copy of the
//! `default` parity seed; skipped (with a note) when it isn't built.
use axum::http::Method;
use campfire_db::{Room, User};
use serde_json::json;

use super::support::{Client, delivery, html_json, identifier};
use crate::channels::broadcasts::Stream;
use crate::channels::user_gid;
use crate::controllers::presenters::test_support::{
    DAVID, QUIET_CORNER, Req, TestApp, david_cookie,
};

const DISCONNECT_RECONNECT: &str = r#"{"type":"disconnect","reason":"remote","reconnect":true}"#;
const UNAUTHORIZED: &str = r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"#;

#[path = "directory_test.rs"]
mod directory;

struct Hub {
    app: TestApp,
    url: String,
    origin: String,
}

/// Boots the seeded app and serves its router on a local port. The seed's session for David was
/// created by Rails before two-step sign-in existed; it's marked verified here so the cable
/// accepts it.
async fn boot() -> Option<Hub> {
    let app = TestApp::boot().await?;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at = created_at WHERE user_id = ?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let listener = super::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Some(Hub {
        app,
        url: format!("ws://{addr}/cable"),
        origin: format!("http://{addr}"),
    })
}

impl Hub {
    async fn connect(&self, cookie: &str) -> Client {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = self.url.as_str().into_client_request().unwrap();
        let headers = request.headers_mut();
        headers.insert("origin", self.origin.parse().unwrap());
        headers.insert(
            "sec-websocket-protocol",
            "actioncable-v1-json, actioncable-unsupported"
                .parse()
                .unwrap(),
        );
        headers.insert("cookie", cookie.parse().unwrap());
        let (socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("upgrade");
        Client { socket }
    }

    async fn david(&self) -> Client {
        let mut client = self.connect(&david_cookie()).await;
        assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
        client
    }

    /// `turbo_stream_from *streamables` on the stock channel.
    fn turbo(&self, streamables: &[&str]) -> String {
        let signed =
            rails_compat::turbo::signed_stream_name(&self.app.booted.app.secrets, streamables);
        identifier(json!({ "channel": "Turbo::StreamsChannel", "signed_stream_name": signed }))
    }
}

#[tokio::test]
async fn signing_out_disconnects_every_socket_of_the_user() {
    let Some(hub) = boot().await else { return };
    let (mut first, mut second) = (hub.david().await, hub.david().await);
    let heartbeat = identifier(json!({ "channel": "HeartbeatChannel" }));
    first.confirm(&heartbeat).await;
    second.confirm(&heartbeat).await;

    let mut browser = hub.app.david();
    let reply = browser.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(reply.status, 302, "{}", reply.text());

    assert_eq!(
        first.until_closed().await,
        vec![DISCONNECT_RECONNECT.to_string()]
    );
    assert_eq!(
        second.until_closed().await,
        vec![DISCONNECT_RECONNECT.to_string()]
    );
    // The client reconnects with the cookie of the session that's gone.
    let mut again = hub.connect(&david_cookie()).await;
    assert_eq!(again.until_closed().await, vec![UNAUTHORIZED.to_string()]);
}

/// `Membership#broadcast_room_removal_to_user` goes out from the membership's commit through the
/// database's event sink (the app's `Jobs`), ahead of the disconnect.
#[tokio::test]
async fn model_broadcasts_reach_sockets_through_the_jobs_sink() {
    let Some(hub) = boot().await else { return };
    let mut david = hub.david().await;
    let user_rooms = hub.turbo(&[&user_gid(DAVID).to_param(), "rooms"]);
    david.confirm(&user_rooms).await;

    hub.app
        .db()
        .write(|tx| Room::find(tx.conn(), QUIET_CORNER)?.revoke_from(tx, &[DAVID]))
        .await
        .unwrap();

    let remove = format!(
        r#"<turbo-stream action="remove" target="list_rooms_closed_{QUIET_CORNER}"></turbo-stream>"#
    );
    assert_eq!(
        david.until_closed().await,
        vec![
            delivery(&user_rooms, &html_json(&remove)),
            DISCONNECT_RECONNECT.to_string()
        ]
    );
}

/// Work on the job runner (`perform_later`) broadcasts through the app's `Broadcasts`, which is
/// the hub the sockets subscribe to.
#[tokio::test]
async fn job_broadcasts_reach_sockets() {
    let Some(hub) = boot().await else { return };
    let mut david = hub.david().await;
    let rooms = hub.turbo(&["rooms"]);
    david.confirm(&rooms).await;

    let app = hub.app.booted.app.clone();
    let (done, finished) = tokio::sync::oneshot::channel();
    hub.app
        .booted
        .app
        .jobs
        .perform_later("ChannelsHubTest", async move {
            let delivered = app.broadcasts.remove(&Stream::rooms(), "list_rooms_open_1");
            let _ = done.send(delivered);
            Ok(())
        });
    assert_eq!(finished.await.unwrap(), 1);
    let remove = r#"<turbo-stream action="remove" target="list_rooms_open_1"></turbo-stream>"#;
    assert_eq!(
        david.next_text().await,
        delivery(&rooms, &html_json(remove))
    );
}

/// Deactivating closes the sockets for good through the same sink.
#[tokio::test]
async fn deactivation_through_the_booted_app_disconnects_for_good() {
    let Some(hub) = boot().await else { return };
    let mut david = hub.david().await;
    hub.app
        .db()
        .write(|tx| User::find(tx.conn(), DAVID)?.deactivate(tx))
        .await
        .unwrap();
    let frames = david.until_closed().await;
    assert_eq!(
        frames,
        vec![r#"{"type":"disconnect","reason":"remote","reconnect":false}"#.to_string()]
    );
}

#[tokio::test]
async fn direct_mute_keeps_the_rendered_sidebar_row() {
    use crate::controllers::presenters::test_support::DIRECT_DAVID_JASON;
    let hub = boot().await.expect("seed required");
    let mut client = hub.david().await;
    let user_rooms = hub.turbo(&[&user_gid(DAVID).to_param(), "rooms"]);
    client.confirm(&user_rooms).await;
    let mut browser = hub.app.david();
    let reply = browser
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{DIRECT_DAVID_JASON}/involvement"),
            )
            .form(&[("involvement", "muted")]),
        )
        .await;
    assert_eq!(reply.status, 302, "{}", reply.text());
    let frame = client.next_text().await;
    let frame: serde_json::Value = serde_json::from_str(&frame).unwrap();
    let html = frame["message"].as_str().unwrap();
    assert!(
        html.contains(&format!(r#"id="list_rooms_direct_{DIRECT_DAVID_JASON}""#)),
        "{html}"
    );
    assert!(
        !html.contains("<template></template>"),
        "Rails retains the direct-room row when muted: {html}"
    );
}

#[tokio::test]
async fn ordinary_quote_text_is_broadcast() {
    let hub = boot().await.expect("seed required");
    let mut client = hub.david().await;
    let rooms = hub.turbo(&["rooms"]);
    client.confirm(&rooms).await;
    let payload = "<p>The header is nonce=\"example\".</p>";
    let recipients =
        hub.app
            .booted
            .app
            .broadcasts
            .replace(&Stream::rooms(), "review_target", payload);
    assert_eq!(
        recipients, 1,
        "Plain text containing nonce= is not a session-bound attribute"
    );
    assert_eq!(
        client.next_text().await,
        delivery(
            &rooms,
            &html_json(&format!(
                r#"<turbo-stream action="replace" target="review_target"><template>{payload}</template></turbo-stream>"#
            ))
        )
    );
}

#[tokio::test]
async fn quote_text_post_delivers_to_socket() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    let hub = boot().await.expect("seed required");
    let mut client = hub.david().await;
    let room = hub
        .app
        .db()
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    let signed = rails_compat::turbo::signed_stream_name(
        &hub.app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    let messages =
        identifier(json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }));
    client.confirm(&messages).await;
    let mut browser = hub.app.david();
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("message[body]", "<div>nonce=\"example\"</div>")]),
        )
        .await;
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert!(
        reply.text().contains("nonce=\"example\""),
        "the quoted text survives rendering"
    );
    let frame = tokio::time::timeout(std::time::Duration::from_secs(1), client.next_text())
        .await
        .expect("POST succeeded but room subscribers received no message");
    let frame: serde_json::Value = serde_json::from_str(&frame).unwrap();
    assert_eq!(frame["identifier"], messages);
    let html = frame["message"].as_str().unwrap();
    assert!(html.contains(r#"nonce="example""#), "{html}");
    assert!(!html.contains("<template></template>"), "{html}");
}

#[tokio::test]
async fn request_warmed_message_broadcast_is_tokenless() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use crate::controllers::presenters::{Presenter, page};
    let hub = boot().await.expect("seed required");
    let message = hub
        .app
        .db()
        .read(|conn| {
            let id: i64 = conn.query_row(
                "SELECT id FROM messages WHERE room_id = ? ORDER BY created_at DESC LIMIT 1",
                [ALL_TALK],
                |r| r.get(0),
            )?;
            campfire_db::Message::find(conn, id)
        })
        .await
        .unwrap();
    hub.app.booted.app.fragment_cache.clear();
    let mut browser = hub.app.david();
    let response = browser
        .get(&format!("/rooms/{ALL_TALK}/messages/{}", message.id))
        .await;
    assert_eq!(response.status, 200);
    let mut client = hub.david().await;
    let rooms = hub.turbo(&["rooms"]);
    client.confirm(&rooms).await;
    let app = hub.app.booted.app.clone();
    let html = hub
        .app
        .db()
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let view = presenter.message(&message)?;
            let account = campfire_db::Account::first(conn)?;
            Ok(page::render_detached_at(
                &app,
                account.as_ref(),
                "http://example.org",
                |ctx| campfire_views::messages::message(ctx, &view),
            ))
        })
        .await
        .unwrap();
    assert_eq!(
        campfire_cable::turbo::session_bound(&html),
        None,
        "request-warmed cache pollutes detached broadcast rendering"
    );
    assert!(!html.contains("authenticity_token"), "{html}");
    assert!(
        !campfire_views::helpers::request_forgery::has_token_slots(&html),
        "{html}"
    );
    assert_eq!(
        hub.app
            .booted
            .app
            .broadcasts
            .replace(&Stream::rooms(), "warmed_message", &html),
        1
    );
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    assert_eq!(frame["identifier"], rooms);
    assert_eq!(
        frame["message"],
        format!(
            r#"<turbo-stream action="replace" target="warmed_message"><template>{html}</template></turbo-stream>"#
        )
    );
}

/// Test real cached token slots, rather than a string that only resembles one.
#[tokio::test]
async fn unresolved_token_slots_never_reach_a_socket() {
    use campfire_views::{fragment_cache, helpers::request_forgery};
    let hub = boot().await.expect("seed required");
    let mut client = hub.david().await;
    let rooms = hub.turbo(&["rooms"]);
    client.confirm(&rooms).await;
    let cache = &hub.app.booted.app.fragment_cache;
    cache.fetch("review-token-slot", || {
        request_forgery::token_tag("/session", "post").0
    });
    let slot: fragment_cache::Fragment = cache.get("review-token-slot").unwrap();
    assert!(request_forgery::has_token_slots(&slot));
    assert_eq!(
        hub.app
            .booted
            .app
            .broadcasts
            .replace(&Stream::rooms(), "token_slot", &slot),
        0
    );
    client.assert_silent().await;
}

/// FakePartials test the transport API; these checks also exercise each controller's wiring
/// to the real message, presentation, boost, shared-room and direct-room templates.
#[tokio::test]
async fn http_broadcasts_supply_real_nonempty_partials() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    let hub = boot().await.expect("seed required");
    // Rails broadcasts creation only for a new DM, so use a genuinely new member set.
    let peer = hub.app.db().write(|tx| Ok(User::create(tx, campfire_db::NewUser {
        name: "Broadcast Peer".into(), email_address: Some("broadcast-peer@example.test".into()), ..Default::default()
    })?.id)).await.unwrap();
    let mut client = hub.david().await;
    let rooms = hub.turbo(&["rooms"]);
    client.confirm(&rooms).await;
    let own_rooms = hub.turbo(&[&user_gid(DAVID).to_param(), "rooms"]);
    client.confirm(&own_rooms).await;
    let mut browser = hub.app.david();

    for (path, fields, target) in [
        (
            "/rooms/opens",
            vec![("room[name]", "Real open partial".to_string())],
            "shared_rooms",
        ),
        (
            "/rooms/closeds",
            vec![
                ("room[name]", "Real closed partial".to_string()),
                ("user_ids[]", DAVID.to_string()),
            ],
            "shared_rooms",
        ),
        (
            "/rooms/directs",
            vec![("user_ids[]", peer.to_string())],
            "direct_rooms",
        ),
    ] {
        let fields: Vec<_> = fields
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let response = browser
            .write(Req::new(Method::POST, path).form(&fields))
            .await;
        assert_eq!(response.status, 302, "{}", response.text());
        let html = broadcast_html(&mut client).await;
        assert!(html.contains(&format!(r#"target="{target}""#)), "{html}");
        if target == "direct_rooms" {
            let id = response.location().unwrap().rsplit('/').next().unwrap();
            assert!(html.contains(&format!(r#"id="list_rooms_direct_{id}""#)), "{html}");
        }
    }

    let reused = browser.write(Req::new(Method::POST, "/rooms/directs").form(&[("user_ids[]", &peer.to_string())])).await;
    assert_eq!(reused.status, 302);
    client.assert_silent().await;

    // The shared visibility row is supplied as well, despite its inherited HTML still
    // lacking the fork's membership/unread locals (explicitly partial in the contract).
    for level in ["invisible", "everything", "muted"] {
        let response = browser
            .write(
                Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement"))
                    .form(&[("involvement", level)]),
            )
            .await;
        assert_eq!(response.status, 302, "{}", response.text());
        if level == "invisible" {
            assert!(client.next_text().await.contains("remove"));
        } else {
            let html = broadcast_html(&mut client).await;
            assert!(
                html.contains(&format!(r#"id="list_rooms_closed_{ALL_TALK}""#)),
                "{html}"
            );
        }
    }

    let room = hub
        .app
        .db()
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    let signed = rails_compat::turbo::signed_stream_name(
        &hub.app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    let messages =
        identifier(json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }));
    client.confirm(&messages).await;
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("message[body]", "<div>Real message partial</div>")]),
        )
        .await;
    assert_eq!(response.status, 200, "{}", response.text());
    let html = broadcast_html(&mut client).await;
    assert!(html.contains("Real message partial"), "{html}");
    let id = hub
        .app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT id FROM messages WHERE creator_id = ? ORDER BY created_at DESC LIMIT 1",
                [DAVID],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let response = browser
        .write(
            Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{id}"))
                .form(&[("message[body]", "<div>Real presentation partial</div>")]),
        )
        .await;
    assert_eq!(response.status, 302, "{}", response.text());
    assert!(
        broadcast_html(&mut client)
            .await
            .contains("Real presentation partial")
    );
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/messages/{id}/boosts"))
                .form(&[("boost[content]", "👍")]),
        )
        .await;
    assert_eq!(response.status, 302, "{}", response.text());
    assert!(broadcast_html(&mut client).await.contains("👍"));
}

async fn broadcast_html(client: &mut Client) -> String {
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    let html = frame["message"]
        .as_str()
        .expect("Turbo Stream HTML")
        .to_string();
    let template = html
        .split_once("<template>")
        .and_then(|(_, rest)| rest.split_once("</template>"))
        .expect("rendered partial")
        .0;
    assert!(
        !template.trim().is_empty(),
        "broadcast has an empty partial: {html}"
    );
    assert_eq!(campfire_cable::turbo::session_bound(&html), None);
    assert!(!campfire_views::helpers::request_forgery::has_token_slots(
        &html
    ));
    html
}

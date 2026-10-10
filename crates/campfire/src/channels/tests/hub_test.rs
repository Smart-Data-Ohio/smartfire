//! The booted app's own hub: sockets opened against the app's router, and broadcasts made the
//! ways the app makes them outside a channel (the models' events through the `Jobs` sink, and
//! work on the job runner), plus sign-out through the real controller. Over a copy of the
//! `default` parity seed; skipped (with a note) when it isn't built.
use axum::http::Method;
use campfire_db::{Room, User};
use serde_json::json;

use super::support::{Client, delivery, identifier};
use crate::controllers::presenters::test_support::{
    DAVID, Req, TestApp, david_cookie,
};

const DISCONNECT_RECONNECT: &str = r#"{"type":"disconnect","reason":"remote","reconnect":true}"#;
const UNAUTHORIZED: &str = r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"#;

#[path = "directory_test.rs"]
mod directory;

#[path = "reads_test.rs"]
mod reads;



struct Hub {
    app: TestApp,
    url: String,
    origin: String,
}

/// Boots the seeded app and serves its router on a local port. The seed's session for David was
/// created by Rails before two-step sign-in existed; it's marked verified here so the cable
/// accepts it.
async fn boot() -> Option<Hub> {
    boot_with_test_clock(crate::controllers::presenters::test_support::seed_clock()).await
}

async fn boot_with_test_clock(clock: campfire_kit::SharedClock) -> Option<Hub> {
    let app = TestApp::boot_with_test_clock(clock).await?;
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

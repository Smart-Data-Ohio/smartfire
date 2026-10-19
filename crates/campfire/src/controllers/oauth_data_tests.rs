use super::agent_http_tests::{AGENT, initialize};
use super::presenters::test_support::{DAVID, Reply, Req, TestApp};
use campfire_db::models::oauth::{OAuthConnection, canonical_resource};
use campfire_kit::Method;
use rusqlite::params;
use serde_json::json;

const ACCESS: &str = "oauth-access-fixture";
const ROTATED: &str = "oauth-rotated-access-fixture";

async fn setup() -> TestApp {
    let app = TestApp::boot_with_clock_and_env(
        std::sync::Arc::new(campfire_kit::FrozenClock::new(
            "2026-03-02T16:00:00Z".parse().unwrap(),
        )),
        &[("APP_URL", "https://smartfire.example"), ("SMARTFIRE_MCP_OAUTH", "0")],
    )
    .await
    .expect("OAuth tests require restored default seed")
    .without_job_runner()
    .await;
    initialize(&app).await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET kind='personal',owner_id=? WHERE id=?",params![DAVID,AGENT])?;
        let client: i64 = tx.conn().query_row("INSERT INTO oauth_clients(client_id,registration,client_name,redirect_uris,created_at,updated_at) VALUES('https://client.example/metadata','cimd','Test client','[]',?,?) RETURNING id",params![tx.now(),tx.now()],|r|r.get(0))?;
        let connection = OAuthConnection::create(tx,AGENT,DAVID,client)?;
        tx.conn().execute("UPDATE oauth_connections SET auth_method='none' WHERE id=?",[connection.id])?;
        for (secret,kind) in [(ACCESS,"access"),(ROTATED,"access"),("oauth-refresh-fixture","refresh")] {
            tx.conn().execute("INSERT INTO oauth_tokens(oauth_connection_id,kind,token_digest,expires_at,created_at) VALUES(?,?,?,?,?)",params![connection.id,kind,campfire_db::user::digest_bot_token(secret),tx.now().since(jiff::SignedDuration::from_hours(1)),tx.now()])?;
        }
        assert_eq!(connection.resource,canonical_resource(&tx.env().default_url_origin));
        Ok(())
    }).await.unwrap();
    app
}

async fn request(app: &TestApp, secret: &str, method: Method, path: &str) -> Reply {
    app.anonymous()
        .send(
            Req::new(method, path)
                .header("authorization", &["Bearer", secret].join(" "))
                .header("accept", "application/json")
                .json_body(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
        )
        .await
}

#[tokio::test]
async fn oauth_data_mcp_accepts_access_and_rest_search_and_cable_refuse_it() {
    let app = setup().await;
    let accepted = request(&app, ACCESS, Method::POST, "/agents/mcp").await;
    assert_eq!(accepted.status.as_u16(), 200, "{}", accepted.text());
    let names = accepted.json()["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        names,
        campfire_controllers::controllers::agents::mcp::PERSONAL_TOOLS
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    let mut owner = app.sign_in(DAVID).await;
    assert_eq!(
        owner.get("/api/v1/me/agents").await.json()["oauthAvailable"],
        false
    );
    for path in ["/agents/events", "/agents/events/stream", "/agents/work", "/api/v1/search?q=test"] {
        let response = request(&app, ACCESS, Method::GET, path).await;
        assert_eq!(response.status.as_u16(), 401, "{path}: {}", response.text());
    }
    for path in [
        "/oauth/authorize",
        "/.well-known/oauth-authorization-server",
    ] {
        assert_eq!(
            app.anonymous()
                .send(Req::new(Method::GET, path))
                .await
                .status
                .as_u16(),
            404
        );
    }
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let service = campfire_kit::front::app_service(app.booted.router.clone());
    let shutdown = campfire_kit::front::Shutdown::when(std::future::pending());
    let server = tokio::spawn(campfire_kit::front::serve_plain(
        listener,
        service,
        campfire_kit::front::Protocol::Http1,
        Default::default(),
        shutdown,
    ));
    let mut upgrade = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    upgrade
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    upgrade.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    upgrade.headers_mut().insert(
        "authorization",
        ["Bearer", ACCESS].join(" ").parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(upgrade).await.unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        frame.into_text().unwrap(),
        r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"#
    );
    server.abort();
}

#[tokio::test]
async fn oauth_data_invalid_tokens_connections_and_personal_policy_return_401() {
    for sql in [
        "UPDATE oauth_tokens SET expires_at='2026-03-02 16:00:00'",
        "UPDATE oauth_tokens SET revoked_at='2026-03-02 16:00:00'",
        "UPDATE agent_credentials SET revoked_at='2026-03-02 16:00:00'",
        "UPDATE agent_credentials SET expires_at='2026-03-02 16:00:00'",
        "UPDATE oauth_clients SET blocked_at='2026-03-02 16:00:00'",
        "UPDATE oauth_connections SET resource='https://old-smartfire.example/agents/mcp'",
        "UPDATE oauth_connections SET expires_at='2026-03-02 16:00:00'",
        "UPDATE agents SET suspended_at='2026-03-02 16:00:00'",
        "UPDATE agents SET kind='workspace'",
        "UPDATE users SET status=1 WHERE id=127326141",
        "UPDATE accounts SET settings=json_set(COALESCE(settings,'{}'),'$.member_agents_enabled',json('false'))",
    ] {
        let app = setup().await;
        app.db()
            .write(move |tx| {
                tx.conn().execute(sql, [])?;
                Ok(())
            })
            .await
            .unwrap();
        let response = request(&app, ACCESS, Method::POST, "/agents/mcp").await;
        assert_eq!(response.status.as_u16(), 401, "{sql}: {}", response.text());
    }
    let app = setup().await;
    assert_eq!(
        request(&app, "oauth-refresh-fixture", Method::POST, "/agents/mcp")
            .await
            .status
            .as_u16(),
        401
    );
}

#[tokio::test]
async fn oauth_data_mcp_throttles_rotated_tokens_by_connection_credential() {
    let app = setup().await;
    for i in 0..600 {
        let secret = if i < 300 { ACCESS } else { ROTATED };
        assert_eq!(
            request(&app, secret, Method::POST, "/agents/mcp")
                .await
                .status
                .as_u16(),
            200
        );
    }
    assert_eq!(
        request(&app, ROTATED, Method::POST, "/agents/mcp")
            .await
            .status
            .as_u16(),
        429
    );
    // The same agent's separate static credential has its own bucket.
    assert_eq!(
        request(
            &app,
            super::agent_http_tests::SECRET,
            Method::POST,
            "/agents/mcp"
        )
        .await
        .status
        .as_u16(),
        200
    );
}

#[tokio::test]
async fn oauth_data_registered_periodic_sweep_preserves_live_then_cleans_dead_tokens() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            let old = tx.now().ago(jiff::SignedDuration::from_hours(24));
            tx.conn().execute("INSERT INTO oauth_clients(client_id,registration,client_name,redirect_uris,created_at,updated_at) VALUES('unused-dcr','dcr','Unused registration','[]',?,?)", params![old, old])?;
            tx.conn()
                .execute("UPDATE oauth_tokens SET expires_at=?", [tx.now()])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut periodic = crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
        reminders: std::time::Duration::from_secs(30),
        retention: std::time::Duration::from_secs(86400),
    });
    periodic.retain_tasks(&["OAuth sweep"]);
    let now = app.db().env().now();
    assert_eq!(
        periodic.tick(app.booted.app.clone(), now).await,
        ["OAuth sweep"]
    );
    assert_eq!(app.db().read(|conn| {
        Ok(conn.query_row("SELECT COUNT(*) FROM oauth_clients WHERE client_id='unused-dcr'", [], |r| r.get::<_, i64>(0))?)
    }).await.unwrap(), 0, "the scheduled sweep also cleans unused registrations");
    let count = |conn: &campfire_db::Connection| {
        Ok(
            conn.query_row("SELECT COUNT(*) FROM oauth_tokens", [], |r| {
                r.get::<_, i64>(0)
            })?,
        )
    };
    assert_eq!(app.db().read(count).await.unwrap(), 3);
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agent_credentials SET revoked_at=? WHERE token_digest LIKE 'oauth:%'",
                [tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        periodic
            .tick(
                app.booted.app.clone(),
                now.since(jiff::SignedDuration::from_hours(1))
            )
            .await,
        ["OAuth sweep"]
    );
    assert_eq!(app.db().read(count).await.unwrap(), 0);
}

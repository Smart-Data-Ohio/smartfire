//! Remaining test/controllers/users/bans_controller_test.rb criteria at d7c7de92.
//! WS9's audit producer and the WS3/WS8a durable queue/message writer run unchanged.
use crate::controllers::presenters::test_support::{Browser, HQ, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage, Session, TwoFactorSetupSecret, User};

async fn confirm(app: &TestApp) -> Browser<'_> {
    let mut browser = app.david();
    let response = browser
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    // Parity's authenticated target session uses loopback; Rails refuses to ban private IPs.
    // These original cases use public target addresses, so keep the browser seed out of that
    // unrelated validation branch without changing the administrator's session.
    app.db()
        .write(|tx| {
            Ok(tx.conn().execute(
                "UPDATE sessions SET ip_address='203.0.113.42' WHERE user_id=?",
                [KEVIN],
            )?)
        })
        .await
        .unwrap();
    browser
}

#[tokio::test]
async fn banning_a_user_removes_pending_two_factor_setup_and_sessions() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = confirm(&app).await;
    let secrets = app.booted.app.secrets.clone();
    let session_id = app
        .db()
        .write(move |tx| {
            let session = Session::start(tx, KEVIN, Some("Test"), Some("203.0.113.1"))?;
            TwoFactorSetupSecret::issue_for(
                tx,
                &rails_compat::ar_encryption::ArEncryption::new(&secrets),
                session.id,
            )?;
            Ok(session.id)
        })
        .await
        .unwrap();
    assert_eq!(
        app.db()
            .read(move |conn| Ok(conn.query_row(
                "SELECT count(*) FROM two_factor_setup_secrets WHERE session_id=?",
                [session_id],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
    let response = browser
        .write(Req::new(Method::POST, "/users/712064548/ban").form(&[]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND, "{}", response.text());
    assert_eq!(
        response.location(),
        Some("http://campfire.test/users/712064548")
    );
    let (sessions, setups, banned) = app
        .db()
        .read(move |conn| {
            Ok((
                Session::count_for_user(conn, KEVIN)?,
                conn.query_row(
                    "SELECT count(*) FROM two_factor_setup_secrets WHERE session_id=?",
                    [session_id],
                    |r| r.get::<_, i64>(0),
                )?,
                User::find(conn, KEVIN)?.is_banned(),
            ))
        })
        .await
        .unwrap();
    assert_eq!((sessions, setups, banned), (0, 0, true));
}

#[tokio::test]
async fn ban_http_enqueue_is_atomic_and_writes_one_durable_remove_job() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = confirm(&app).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TABLE ws8br2_job_inserts(class TEXT, queue TEXT, arguments TEXT); \
            CREATE TRIGGER ws8br2_observe_ban_job AFTER INSERT ON background_jobs WHEN NEW.job_class='RemoveBannedContentJob' \
            BEGIN INSERT INTO ws8br2_job_inserts VALUES (NEW.job_class, NEW.queue_name, NEW.arguments); END; \
            CREATE TRIGGER ws8br2_reject_ban_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='RemoveBannedContentJob' \
            BEGIN SELECT RAISE(ABORT, 'ban job rejected'); END;")?;
        Ok(())
    }).await.unwrap();
    let snapshot = |conn: &campfire_db::Connection| -> campfire_db::Result<_> {
        Ok((
            User::find(conn, KEVIN)?,
            Session::count_for_user(conn, KEVIN)?,
            conn.query_row("SELECT count(*) FROM bans", [], |r| r.get::<_, i64>(0))?,
            conn.query_row("SELECT count(*) FROM audit_logs", [], |r| {
                r.get::<_, i64>(0)
            })?,
        ))
    };
    let before = app.db().read(snapshot).await.unwrap();
    let rejected = browser
        .write(Req::new(Method::POST, "/users/712064548/ban").form(&[]))
        .await;
    assert_eq!(rejected.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        app.db().read(snapshot).await.unwrap(),
        before,
        "the user, sessions, bans and audit must roll back with the rejected job"
    );
    assert_eq!(
        app.db()
            .read(|conn| Ok(conn.query_row(
                "SELECT count(*) FROM ws8br2_job_inserts",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    app.db()
        .write(|tx| {
            Ok(tx
                .conn()
                .execute_batch("DROP TRIGGER ws8br2_reject_ban_job")?)
        })
        .await
        .unwrap();
    let accepted = browser
        .write(Req::new(Method::POST, "/users/712064548/ban").form(&[]))
        .await;
    assert_eq!(accepted.status, StatusCode::FOUND);
    let rows = app
        .db()
        .read(|conn| {
            let mut query =
                conn.prepare("SELECT class, queue, arguments FROM ws8br2_job_inserts")?;
            Ok(query
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "RemoveBannedContentJob");
    assert_eq!(rows[0].1, "default");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rows[0].2).unwrap(),
        serde_json::json!({"user_id": KEVIN})
    );
}

#[tokio::test]
async fn ban_http_removes_the_users_messages_through_the_real_runner() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = confirm(&app).await;
    let message_id = app
        .db()
        .write(|tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: HQ,
                    creator_id: KEVIN,
                    body: Some("Test message".into()),
                    client_message_id: Some("ws8br2-ban-message".into()),
                    ..Default::default()
                },
            )
            .map(|message| message.id)
        })
        .await
        .unwrap();
    let count = || {
        app.db().read(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM messages WHERE creator_id=?",
                [KEVIN],
                |r| r.get::<_, i64>(0),
            )?)
        })
    };
    assert!(count().await.unwrap() > 0);
    let response = browser
        .write(Req::new(Method::POST, "/users/712064548/ban").form(&[]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while count().await.unwrap() != 0 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "RemoveBannedContentJob did not delete the user's messages"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        app.db()
            .read(move |conn| Message::find_by_id(conn, message_id))
            .await
            .unwrap()
            .is_none()
    );
    let failed = app.db().read(|conn| Ok(conn.query_row("SELECT count(*) FROM background_jobs WHERE job_class='RemoveBannedContentJob' AND status='failed'", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
    assert_eq!(failed, 0);
}

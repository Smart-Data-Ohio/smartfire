use crate::controllers::presenters::test_support::{BENDER, DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::DndAllowedUser;
fn path(id: i64) -> String {
    format!("/users/{id}/dnd_allowance")
}

#[tokio::test]
async fn ws17_starring_and_unstarring_someone_for_dnd() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    let mut browser = app.david();
    let reply = browser.write(Req::new(Method::POST, &path(JASON))).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/users/{JASON}").as_str())
    );
    assert!(
        app.db()
            .read(|c| DndAllowedUser::find(c, DAVID, JASON))
            .await
            .unwrap()
            .is_some()
    );
    browser.write(Req::new(Method::DELETE, &path(JASON))).await;
    assert!(
        app.db()
            .read(|c| DndAllowedUser::find(c, DAVID, JASON))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn ws17_concurrent_stars_stay_a_single_exception() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    let mut one = app.david();
    let mut two = app.david();
    one.authenticity_token().await;
    two.authenticity_token().await;
    let (a, b) = tokio::join!(
        one.write(Req::new(Method::POST, &path(JASON))),
        two.write(Req::new(Method::POST, &path(JASON)))
    );
    assert!(a.location().is_some() && b.location().is_some());
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM dnd_allowed_users WHERE user_id=? AND allowed_user_id=?",
                rusqlite::params![DAVID, JASON],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn ws17_allowances_reject_self_bots_inactive_missing_and_anonymous() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    let mut browser = app.david();
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    for (id, status) in [
        (DAVID, StatusCode::UNPROCESSABLE_ENTITY),
        (BENDER, StatusCode::NOT_FOUND),
        (KEVIN, StatusCode::NOT_FOUND),
        (-1, StatusCode::NOT_FOUND),
    ] {
        let reply = browser.write(Req::new(Method::POST, &path(id))).await;
        assert_eq!(reply.status, status, "{id}: {}", reply.text());
    }
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM dnd_allowed_users WHERE user_id=?",
                [DAVID],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    let mut anonymous = app.anonymous();
    assert_eq!(
        anonymous
            .write(Req::new(Method::POST, &path(JASON)))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
}
#[tokio::test]
async fn ws17_starring_twice_stays_single_exception() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    for _ in 0..2 {
        assert_eq!(
            browser
                .write(Req::new(Method::POST, &path(JASON)))
                .await
                .location(),
            Some(format!("http://campfire.test/users/{JASON}").as_str())
        );
    }
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM dnd_allowed_users WHERE user_id=?",
                [DAVID],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn ws17_unique_index_losing_star_redirects_success() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    app.db().write(|tx| {
        DndAllowedUser::create(tx,KEVIN,JASON)?;
        // Deterministic real SQLite UNIQUE failure at the loser's insert. Rails' named test
        // stubs RecordNotUnique; this drives the same rescue through the HTTP/write stack.
        tx.conn().execute_batch(&format!("CREATE TRIGGER ws17_losing_star BEFORE INSERT ON dnd_allowed_users WHEN NEW.user_id={DAVID} BEGIN INSERT INTO dnd_allowed_users(user_id,allowed_user_id,created_at,updated_at) SELECT user_id,allowed_user_id,created_at,updated_at FROM dnd_allowed_users WHERE user_id={KEVIN}; END;"))?;
        Ok(())
    }).await.unwrap();
    assert_eq!(
        browser
            .write(Req::new(Method::POST, &path(JASON)))
            .await
            .location(),
        Some(format!("http://campfire.test/users/{JASON}").as_str())
    );
    assert!(
        app.db()
            .read(|c| DndAllowedUser::find(c, KEVIN, JASON))
            .await
            .unwrap()
            .is_some()
    );
}

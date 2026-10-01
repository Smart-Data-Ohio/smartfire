use crate::controllers::presenters::test_support::*;
use askama::Template;
use campfire_views::rooms::DirectsNew;

async fn picker(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let setup = name.to_owned();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM user_stars WHERE user_id=?",[DAVID])?;
        match setup.as_str() {
            "starred"=> {tx.conn().execute("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,149087659,tx.now(),tx.now()])?;},
            "escaped"=> {tx.conn().execute("INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES (?,?,?,?,?)",rusqlite::params![9100000001_i64,"Renée <&> Dupont","picker@example.test",tx.now(),tx.now()])?;},
            "empty"=> {tx.conn().execute("UPDATE users SET status=1 WHERE id!=?",[DAVID])?;},
            _=> {}
        }
        Ok(())
    }).await.unwrap();
    let people = app
        .db()
        .read(|c| {
            campfire_db::models::user::presentation::directory(
                c,
                DAVID,
                campfire_db::Timestamp::from_jiff(seed_clock().now()),
            )
        })
        .await
        .unwrap();
    let state = app.booted.app.clone();
    let users = app
        .db()
        .read(move |conn| {
            let presenter = crate::controllers::presenters::Presenter::new(conn, &state, None);
            people
                .into_iter()
                .map(|p| {
                    Ok(campfire_views::rooms::DirectPickerUser {
                        user: presenter.user_view(p.user.id)?,
                        bot: p.user.is_bot(),
                        agent: p.agent.is_some(),
                        starred: p.starred,
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    let actual = crate::controllers::users::people_tests::render_with(
        &app,
        |_| {},
        |ctx| {
            DirectsNew { ctx, users: &users }
                .as_content()
                .render()
                .unwrap()
        },
    );
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/users_dm_picker.json"
    ))
    .unwrap();
    let case = vectors["picker"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/picker-{name}.actual"), &actual).unwrap();
        std::fs::write(
            format!("{dir}/picker-{name}.expected"),
            case["html"].as_str().unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        actual,
        case["html"].as_str().unwrap(),
        "{name}: complete picker bytes"
    );
}
#[tokio::test]
async fn seed_picker() {
    picker("seed").await;
}
#[tokio::test]
async fn starred_picker() {
    picker("starred").await;
}
#[tokio::test]
async fn escaped_picker() {
    picker("escaped").await;
}
#[tokio::test]
async fn empty_picker() {
    picker("empty").await;
}
#[tokio::test]
async fn picker_requires_session_and_renders_current_viewer_rows_in_its_frame() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let reply = app.anonymous().get("/rooms/directs/new").await;
    assert_eq!(reply.status, axum::http::StatusCode::FOUND);
    assert!(reply.location().unwrap().ends_with("/session/new"));
    let reply = app
        .david()
        .send(
            Req::new(axum::http::Method::GET, "/rooms/directs/new")
                .header("turbo-frame", "direct_rooms_control"),
        )
        .await;
    assert_eq!(reply.status, axum::http::StatusCode::OK);
    let body = reply.text();
    assert!(body.contains("<turbo-frame id=\"direct_rooms_control\" target=\"_top\">"));
    assert!(body.contains("id=\"dm_picker_filter\""));
    assert!(body.contains("name=\"authenticity_token\""));
    assert!(!body.contains("Start Ping"));
    assert!(!body.contains(&format!("id=\"pick_user_{DAVID}\"")));
    assert!(body.contains("aria-label=\"Select Jason\""));
}

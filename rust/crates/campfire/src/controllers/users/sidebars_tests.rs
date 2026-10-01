//! Recipient-specific row permissions through the real sidebar endpoint/cache.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::Room;

#[test]
fn direct_labels_match_rails_ascii_word_splitting() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/rooms/directory.json"
    ))
    .unwrap();
    for case in fixture["labels"].as_array().unwrap() {
        let members: Vec<_> = case["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| campfire_views::users::UserSummary {
                name: name.as_str().unwrap().into(),
                ..Default::default()
            })
            .collect();
        let actual = crate::controllers::presenters::accounts::sidebar_direct_label(None, &members);
        assert_eq!(
            actual,
            case["label"].as_str().unwrap(),
            "{:?}",
            case["names"]
        );
    }
}

#[tokio::test]
async fn seeded_group_row_matches_rails_membership_order() {
    let app = TestApp::boot().await.expect("seed required");
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/sidebar/rows.json"
    ))
    .unwrap();
    let row = fixtures
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "direct_seed_group")
        .unwrap();
    let reply = app.david().get(&campfire_routes::user_sidebar()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.text().contains(row["html"].as_str().unwrap()),
        "seeded sidebar row differs from Rails bytes"
    );
}

fn direct_link(html: &str, id: i64) -> String {
    let marker = format!("<a class=\"direct__link\" data-room-id=\"{id}\"");
    let start = html.find(&marker).expect("room's direct link");
    html[start..].split('>').next().unwrap().to_string()
}
#[tokio::test]
async fn sidebar_menu_uses_membership_viewer_and_invalidates_role_cache() {
    let app = TestApp::boot().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], KEVIN)?.id))
        .await
        .unwrap();
    let david = app.david().get(&campfire_routes::user_sidebar()).await;
    assert_eq!(david.status, StatusCode::OK);
    assert!(direct_link(&david.text(), id).contains("data-menu-can-delete=\"true\""));
    let mut kevin = app.sign_in(KEVIN).await;
    let ordinary = kevin.get(&campfire_routes::user_sidebar()).await;
    assert_eq!(ordinary.status, StatusCode::OK);
    assert!(direct_link(&ordinary.text(), id).contains("data-menu-can-delete=\"false\""));
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let promoted = kevin.get(&campfire_routes::user_sidebar()).await;
    assert_eq!(promoted.status, StatusCode::OK);
    assert!(direct_link(&promoted.text(), id).contains("data-menu-can-delete=\"true\""));
}

#[tokio::test]
async fn seeded_sidebar_collections_render_the_complete_rails_frame() {
    use askama::Template;
    use campfire_views::helpers::request_forgery::{
        AuthenticityTokens, RequestSecrets, rendering_with,
    };
    struct Tokens;
    impl AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    let app = TestApp::boot_frozen().await.expect("seed required");
    let goldens: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/sidebar/page.json"
    ))
    .unwrap();
    for name in ["seed_admin", "seed_member", "seed_outsider"] {
        let fixture = goldens
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        let id = fixture["user"]["id"].as_i64().unwrap();
        let secrets = app.booted.app.secrets.clone();
        let (sidebar, user, account) = app
            .db()
            .read(move |conn| {
                let user = campfire_db::User::find(conn, id)?;
                let sidebar =
                    crate::controllers::presenters::accounts::sidebar(conn, &secrets, &user)?;
                Ok((
                    sidebar,
                    crate::controllers::presenters::user_summary(&secrets, &user),
                    campfire_db::Account::first(conn)?.unwrap(),
                ))
            })
            .await
            .unwrap();
        let asset = |name: &str| campfire_assets::asset_path(name);
        let signer = |_: &[&str]| String::new();
        let ctx = campfire_views::ViewContext {
            current_user: None,
            account: campfire_views::AccountSummary {
                name: account.name.clone(),
                logo_url: crate::controllers::presenters::accounts::fresh_account_logo_path(
                    Some(&account),
                    None,
                ),
                has_logo: false,
            },
            flash_notice: None,
            flash_alert: None,
            platform: Default::default(),
            vapid_public_key: None,
            asset_path: &asset,
            importmap_tags: "",
            stylesheet_tags: "",
            custom_styles: None,
            cable_url: "/cable".into(),
            base_url: "http://campfire.test".into(),
            request_url: "http://campfire.test/".into(),
            referrer: None,
            last_room_visited_id: None,
            app_version: "parity".into(),
            signed_stream_name: &signer,
            time_zone: campfire_views::time::Zone::utc(),
            chrome: Default::default(),
        };
        let page = campfire_views::users::SidebarShow {
            ctx: &ctx,
            current_user: user,
            rooms_stream: String::new(),
            user_rooms_stream: String::new(),
            favorite_memberships: sidebar.favorite_memberships,
            categories: sidebar.categories,
            direct_memberships: sidebar.direct_memberships,
            direct_placeholder_users: sidebar.direct_placeholder_users,
            other_memberships: sidebar.other_memberships,
            voice_memberships: sidebar.voice_memberships,
            can_create_rooms: true,
        };
        let actual = rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || page.as_content().render().unwrap(),
        );
        if actual != fixture["html"].as_str().unwrap() {
            if let Ok(dir) = std::env::var("WS8BR_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(format!("{dir}/{name}.sql-actual"), &actual).unwrap();
            }
            let expected = fixture["html"].as_str().unwrap();
            let byte = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            panic!(
                "{name}: seeded frame mismatch at byte {byte}, actual {}, Rails {}",
                actual.len(),
                expected.len()
            );
        }
    }
}

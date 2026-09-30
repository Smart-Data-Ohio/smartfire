use crate::controllers::presenters::{
    page,
    test_support::{DAVID, TestApp},
};
#[tokio::test]
async fn full_sidebar_request_composes_workspace_destinations_and_profile_card_trigger() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let html = browser.get("/users/me/sidebar").await.text();
    for marker in [
        "workspace-identity",
        "workspace-destinations",
        "Activity inbox",
        "Work threads",
        "room-menu",
        "room-category__new",
        "data-profile-card-url",
        "Direct messages",
    ] {
        assert!(html.contains(marker), "missing {marker}");
    }
}

#[tokio::test]
async fn full_sidebar_matches_seventeen_complete_post_fix_rails_renders() {
    use campfire_views::helpers::request_forgery::{RequestSecrets, rendering_with};
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../full_sidebar_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 17);
    let Some(test) = TestApp::boot().await else {
        return;
    };
    for case in vectors["cases"].as_array().unwrap() {
        let sidebar: campfire_views::users::sidebar::Sidebar =
            serde_json::from_value(case["input"].clone()).unwrap();
        let actual =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || sidebar.render(ctx),
                )
            });
        let expected = case["html"].as_str().unwrap();
        if actual != expected {
            let scratch = std::path::PathBuf::from(std::env::var_os("TMPDIR").unwrap());
            std::fs::write(scratch.join("sidebar-actual.html"), &actual).unwrap();
            std::fs::write(scratch.join("sidebar-expected.html"), expected).unwrap();
            let at = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            panic!(
                "{} mismatch {at}: {:?} / {:?}",
                case["name"],
                actual.get(at.saturating_sub(70)..(at + 160).min(actual.len())),
                expected.get(at.saturating_sub(70)..(at + 160).min(expected.len()))
            );
        }
    }
}

struct Tokens;
impl campfire_views::helpers::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}

#[tokio::test]
async fn full_sidebar_request_places_persisted_favorites_categories_and_group_dm_permissions() {
    use crate::controllers::presenters::test_support::{JASON, KEVIN, QUIET_CORNER};
    use campfire_db::{Room, RoomCategory, RoomType};
    let Some(test) = TestApp::boot_with_huddle(crate::huddle::Config {
        public_url: Some("wss://public.example.test".into()),
        internal_url: Some("http://internal.example.test:7880".into()),
        api_key: Some("ws13-fixture-api-key".into()),
        api_secret: Some("ws13-fixture-api-secret".into()),
        gateway_secret: Some("ws13-fixture-gateway-secret".into()),
    })
    .await
    else {
        return;
    };
    let (group,category,stage)=test.db().write(|tx| {
        let group=Room::create_for(tx,RoomType::Direct,Some("Creator <&> group"),KEVIN,&[KEVIN,DAVID,JASON])?;
        let category=RoomCategory::create(tx,KEVIN,"Project <&> team",1,true)?;
        tx.conn().execute("UPDATE memberships SET room_category_id=?, involvement='muted' WHERE room_id=? AND user_id=?",rusqlite::params![category.id,QUIET_CORNER,KEVIN])?;
        let stage=Room::for_user_of_type(tx.conn(),KEVIN,RoomType::Stage)?.remove(0);
        tx.conn().execute("UPDATE memberships SET favorite_position=1 WHERE room_id=? AND user_id=?",rusqlite::params![stage.id,KEVIN])?;
        Ok((group.id,category.id,stage.id))
    }).await.unwrap();
    let mut browser = test.sign_in(KEVIN).await;
    let html = browser.get("/users/me/sidebar").await.text();
    let favorites = html.find("id=\"favorite_rooms\"").unwrap();
    let stage_row = html
        .find(&format!("id=\"list_rooms_stage_{stage}\""))
        .unwrap();
    let channels = html.find("id=\"shared_rooms\"").unwrap();
    assert!(favorites < stage_row && stage_row < channels);
    assert_eq!(
        html.matches(&format!("id=\"list_rooms_stage_{stage}\""))
            .count(),
        1
    );
    let category_list = html
        .find(&format!(
            "id=\"category_rooms_{category}\" class=\"sidebar-list\" hidden"
        ))
        .unwrap();
    let quiet = html
        .find(&format!("id=\"list_rooms_closed_{QUIET_CORNER}\""))
        .unwrap();
    assert!(category_list < quiet && quiet < html.find("id=\"board_rooms\"").unwrap());
    assert_eq!(
        html.matches(&format!("id=\"list_rooms_closed_{QUIET_CORNER}\""))
            .count(),
        1
    );
    assert!(html.contains("sidebar-item room btn muted"));
    let dm = html
        .find(&format!("id=\"list_rooms_direct_{group}\""))
        .unwrap();
    let link = html[dm..].split("</a>").next().unwrap();
    assert!(
        link.contains("data-menu-can-delete=\"false\""),
        "group creators still need admin deletion rights"
    );
    assert!(link.contains("Creator &lt;&amp;&gt; group"));
    assert!(link.contains("data-action=\"click-&gt;profile-card#open\""));
    assert!(html.contains("data-controller=\"badge-dot dm-presence huddle-presence\""));
}

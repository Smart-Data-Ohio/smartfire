use super::super::presenters::test_support::*;
use askama::Template;
use axum::http::StatusCode;
use campfire_db::{Session, Timestamp, WorkspacePresenceLease};
use campfire_views::{ViewContext, helpers as h};

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_people.json")).unwrap()
}

async fn setup_case(app: &TestApp, id: i64, setup: serde_json::Value) {
    app.db().write(move |tx| {
        if let Some(attributes) = setup["attributes"].as_object() {
            for (key,value) in attributes {
                let value = match value {
                    serde_json::Value::String(s) if key.ends_with("_at") => rusqlite::types::Value::Text(Timestamp::from_jiff(s.parse().unwrap()).to_db()),
                    serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    serde_json::Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                    _ => panic!("unexpected fixture value"),
                };
                tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,id])?;
            }
        }
        if setup["lease"] == true {
            let session = Session::start(tx,id,Some("ws8br2"),Some("127.0.0.1"))?;
            let lease = WorkspacePresenceLease::establish(tx,id,session.id)?.unwrap();
            if setup["idle"] == true {
                tx.conn().execute("UPDATE workspace_presence_leases SET last_active_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(11)),lease.id])?;
            }
        }
        if setup["star"] == true {
            tx.conn().execute("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,id,tx.now(),tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
}

pub(super) fn render(app: &TestApp, f: impl FnOnce(&ViewContext) -> String) -> String {
    render_with(app, |_| {}, f)
}

pub(crate) fn render_with(
    app: &TestApp,
    configure: impl FnOnce(&mut ViewContext),
    f: impl FnOnce(&ViewContext) -> String,
) -> String {
    struct Tokens;
    impl h::request_forgery::AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer =
        |parts: &[&str]| rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, parts);
    let mut ctx = ViewContext {
        current_user: Some(campfire_views::CurrentUser {
            id: DAVID,
            name: "David".into(),
            administrator: true,
            bot: false,
            avatar_url: String::new(),
            preferences: Default::default(),
        }),
        account: campfire_views::AccountSummary {
            name: "Signal".into(),
            logo_url: String::new(),
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
        request_url: "http://campfire.test/users".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: &signer,
        time_zone: campfire_views::time::Zone::utc(),
        chrome: Default::default(),
    };
    configure(&mut ctx);
    h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        },
        || f(&ctx),
    )
}

fn assert_bytes(name: &str, actual: &str, expected: &str) {
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.actual"), actual).unwrap();
        std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
    }
    assert_eq!(actual, expected, "{name}: complete Rails bytes");
}

/// The complete fragment byte oracle remains above. At the HTTP boundary compare the
/// same DOM, retaining every attribute/text node except the random CSRF value.
/// None of the original Rails assertions inspect that random value.
pub(crate) fn assert_http_fragment(
    actual: &str,
    expected: &str,
    tag: &str,
    attr: &str,
    value: &str,
) {
    use campfire_richtext::dom::{Dom, NodeData, NodeId};
    fn project(dom: &Dom, id: NodeId) -> serde_json::Value {
        match &dom.node(id).data {
            NodeData::Element(element) => {
                let mut attrs = dom.attrs(id);
                if dom.name(id) == "input" && dom.attr(id, "name") == Some("authenticity_token") {
                    attrs.retain(|(name, _)| name != "value");
                }
                attrs.sort();
                serde_json::json!([
                    element.name.local.to_string(),
                    attrs,
                    dom.children(id)
                        .iter()
                        .map(|id| project(dom, *id))
                        .collect::<Vec<_>>()
                ])
            }
            NodeData::Text(text) => serde_json::json!(["text", text]),
            NodeData::Comment(text) => serde_json::json!(["comment", text]),
            _ => panic!("unexpected node inside HTTP fragment"),
        }
    }
    let fragment = |body| {
        let mut dom = Dom::new();
        let root = dom.parse_fragment(body).unwrap();
        let matches = dom
            .descendants(root)
            .into_iter()
            .filter(|id| dom.name(*id) == tag && dom.attr(*id, attr) == Some(value))
            .collect::<Vec<_>>();
        assert_eq!(
            matches.len(),
            1,
            "exact HTTP {tag}[{attr}={value}] cardinality"
        );
        project(&dom, matches[0])
    };
    assert_eq!(
        fragment(actual),
        fragment(expected),
        "complete routed {tag}[{attr}={value}] fragment"
    );
}

async fn card_case(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors = vectors();
    let case = vectors["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    let id = case["user_id"].as_i64().unwrap();
    setup_case(&app, id, case["setup"].clone()).await;
    let secrets = app.booted.app.secrets.clone();
    let facts = app
        .db()
        .read(move |conn| {
            campfire_db::models::user::presentation::card(
                conn,
                DAVID,
                id,
                Timestamp::from_jiff(SEED_NOW.parse().unwrap()),
            )
        })
        .await
        .unwrap()
        .unwrap();
    let person = super::super::presenters::people::person(&secrets, facts);
    let reply = app.david().get(&campfire_routes::user_card(id)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.text().contains("<!DOCTYPE"));
    assert!(reply.text().contains("id=\"user_card\""));
    let actual = render(&app, |ctx| {
        campfire_views::users::Card { ctx, person }
            .render()
            .unwrap()
    });
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    let status_links = dom.descendants(root).into_iter().filter(|node| {
        dom.name(*node) == "a" && dom.text_content(*node).trim() == "Set a status"
    }).collect::<Vec<_>>();
    assert_eq!(status_links.len(), usize::from(id == DAVID), "only your own card offers Set a status");
    if id == DAVID {
        assert_eq!(dom.attr(status_links[0], "href"), Some(campfire_routes::edit_user_status().as_str()));
        assert_eq!(dom.attr(status_links[0], "data-turbo-frame"), None, "status link uses normal navigation");
    }
    assert_http_fragment(
        &reply.text(),
        case["html"].as_str().unwrap(),
        "turbo-frame",
        "id",
        "user_card",
    );
    assert_bytes(name, &actual, case["html"].as_str().unwrap());
}

#[tokio::test]
async fn card_shows_identity_presence_role_and_actions_for_a_peer() {
    card_case("peer_online").await;
}
#[tokio::test]
async fn offline_peers_read_offline() {
    card_case("peer_offline").await;
}
#[tokio::test]
async fn card_shows_presence_dot_and_custom_status_badge() {
    card_case("custom_status").await;
}
#[tokio::test]
async fn own_card_offers_editing_your_profile() {
    card_case("own").await;
}
#[tokio::test]
async fn agents_can_be_messaged_but_not_called() {
    card_case("agent").await;
}
#[tokio::test]
async fn inactive_users_show_status_without_message_actions() {
    card_case("deactivated").await;
    card_case("banned").await;
}
#[tokio::test]
async fn additional_presence_star_and_mention_states_match_rails() {
    for name in [
        "starred",
        "idle",
        "invisible",
        "dnd",
        "expired_status",
        "unmentionable",
    ] {
        card_case(name).await;
    }
}

#[tokio::test]
async fn directories_match_complete_rails_body_and_starred_order() {
    let vectors = vectors();
    for case in vectors["directories"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("seed required");
        setup_case(&app, KEVIN, serde_json::json!({"star":case["starred"]})).await;
        let facts = app
            .db()
            .read(|conn| {
                campfire_db::models::user::presentation::directory(
                    conn,
                    DAVID,
                    Timestamp::from_jiff(SEED_NOW.parse().unwrap()),
                )
            })
            .await
            .unwrap();
        let ids: Vec<_> = facts.iter().map(|p| p.user.id).collect();
        assert_eq!(serde_json::to_value(ids).unwrap(), case["ids"]);
        let people = facts
            .into_iter()
            .map(|p| super::super::presenters::people::person(&app.booted.app.secrets, p))
            .collect();
        let actual = render(&app, |ctx| {
            campfire_views::users::Directory { ctx, people }
                .as_content()
                .render()
                .unwrap()
        });
        assert_bytes("directory", &actual, case["html"].as_str().unwrap());
        let reply = app.david().get("/users").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_http_fragment(
            &reply.text(),
            case["html"].as_str().unwrap(),
            "ul",
            "class",
            "people-directory",
        );
    }
}

#[tokio::test]
async fn cards_require_sign_in_and_unknown_people_are_not_found() {
    let app = TestApp::boot().await.expect("seed required");
    assert_eq!(
        app.anonymous()
            .get(&campfire_routes::user_card(JASON))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        app.david()
            .get(&campfire_routes::user_card(-1))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn directory_requires_sign_in_and_excludes_the_viewer() {
    let app = TestApp::boot().await.expect("seed required");
    let anonymous = app.anonymous().get("/users").await;
    assert_eq!(anonymous.status, StatusCode::FOUND);
    assert_eq!(
        anonymous.location(),
        Some("http://campfire.test/session/new")
    );
    let reply = app.david().get("/users").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        !reply
            .text()
            .contains(&format!("id=\"select_user_{DAVID}\""))
    );
    assert!(reply.text().contains("people-directory__row"));
}

#[tokio::test]
async fn index_lists_active_members_with_presence_and_selection() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    setup_case(&app, JASON, serde_json::json!({"lease":true})).await;
    let inactive = app
        .db()
        .write(|tx| {
            let id: i64 = tx
                .conn()
                .query_row("SELECT id FROM users WHERE name='JZ'", [], |r| r.get(0))?;
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [id])?;
            Ok(id)
        })
        .await
        .unwrap();
    let reply = app.david().get("/users").await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(&html).unwrap();
    for (id, expected) in [(JASON, 1), (DAVID, 0), (inactive, 0)] {
        let id = id.to_string();
        let checkboxes = dom
            .descendants(root)
            .into_iter()
            .filter(|node| {
                dom.name(*node) == "input"
                    && dom.attr(*node, "data-multi-select-target") == Some("checkbox")
                    && dom.attr(*node, "data-user-id") == Some(id.as_str())
            })
            .count();
        assert_eq!(checkboxes, expected, "original selection selector for {id}");
    }
    let bars = dom
        .descendants(root)
        .into_iter()
        .filter(|node| dom.attr(*node, "data-multi-select-target") == Some("bar"))
        .count();
    assert_eq!(bars, 1);
    assert!(html.contains(&format!("id=\"select_user_{JASON}\"")));
    assert!(!html.contains(&format!("id=\"select_user_{inactive}\"")));
    let has_class = |node, class: &str| dom.attr(node, "class").is_some_and(|value| value.split_ascii_whitespace().any(|value| value == class));
    let nodes = dom.descendants(root);
    assert!(nodes.iter().any(|node| has_class(*node, "people-directory__presence") && dom.text_content(*node).trim() == "Online"), "original Online presence selector");
    assert!(nodes.iter().any(|node| has_class(*node, "profile-card__badge") && dom.text_content(*node).trim() == "Agent"), "original Agent badge selector");
    assert!(nodes.iter().filter(|node| has_class(**node, "people-directory__row")).count() >= 2, "original minimum two directory row elements");
    assert!(!html.contains(&format!("id=\"select_user_{DAVID}\"")));
    assert!(!nodes.iter().any(|node| has_class(*node, "people-directory__row") && dom.text_content(*node).contains("JZ")), "inactive JZ has no directory row");
    // users_controller_test.rb:200 excludes every JZ input, independently of
    // whether it participates in the multi-select controller.
    let inactive_id = inactive.to_string();
    let inactive_inputs = nodes
        .iter()
        .filter(|node| {
            dom.name(**node) == "input"
                && dom.attr(**node, "data-user-id") == Some(inactive_id.as_str())
        })
        .count();
    assert_eq!(inactive_inputs, 0, "original inactive-user input selector");
}

#[tokio::test]
async fn profile_message_buttons_carry_the_accessible_name() {
    let app = TestApp::boot().await.expect("seed required");
    for (id, name) in [(KEVIN, "Kevin"), (BENDER, "Bender Bot")] {
        let reply = app.david().get(&campfire_routes::user(id)).await;
        assert_eq!(reply.status, StatusCode::OK);
        let html = reply.text();
        let mut dom = campfire_richtext::dom::Dom::new();
        let root = dom.parse_fragment(&html).unwrap();
        let label = format!("Message {name}");
        let buttons = dom
            .descendants(root)
            .into_iter()
            .filter(|node| {
                dom.name(*node) == "button" && dom.attr(*node, "aria-label") == Some(label.as_str())
            })
            .count();
        assert_eq!(buttons, 1, "exact Message button selector");
        if id == KEVIN {
            assert!(
                dom.descendants(root)
                    .into_iter()
                    .any(|node| dom.name(node) == "button"
                        && dom.text_content(node).trim() == "Ban Kevin"),
                "original Ban Kevin button selector"
            );
        }
        assert_eq!(
            html.matches(&format!("aria-label=\"Message {name}\""))
                .count(),
            1
        );
        if id == KEVIN {
            assert!(html.contains("Ban Kevin"));
        }
        assert_eq!(dom.descendants(root).into_iter().filter(|node| {
            dom.name(*node) == "img" && dom.attr(*node, "aria-label").is_some()
        }).count(), 0, "original img[aria-label] selector");
    }
}

#[tokio::test]
async fn review_card_markup_matches_rails() {
    card_case("review_markup").await;
}

#[tokio::test]
async fn own_public_profile_matches_the_original_show_request() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let reply = app.david().get(&campfire_routes::user(DAVID)).await;
    assert_eq!(reply.status, StatusCode::OK, "original own-user GET");
}

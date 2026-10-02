use super::*;
use campfire_views::rooms::{MemberPanel, MemberPanelToggle};
use campfire_views::shared::MultiSelectBar;

#[test]
fn member_panel_matches_pinned_rails_bytes_for_every_room_kind_and_viewer() {
    let data: Value =
        serde_json::from_str(include_str!("../golden/member_panel/partials.json")).unwrap();
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    for case in data["panels"].as_array().unwrap() {
        let viewer = if case["viewer"] == "david" {
            "David"
        } else {
            "Kevin"
        };
        let ctx = context(Some(viewer), &asset, &signer, "");
        let panel = MemberPanel {
            ctx: &ctx,
            room_id: case["room_id"].as_i64().unwrap(),
        };
        assert!(compare(
            case["name"].as_str().unwrap(),
            &render(&panel),
            case["html"].as_str().unwrap()
        ));
    }
}

#[test]
fn selection_bar_matches_pinned_rails_bytes_with_both_exit_controls() {
    let data: Value =
        serde_json::from_str(include_str!("../golden/member_panel/partials.json")).unwrap();
    for exit_button in [false, true] {
        assert!(compare(
            &format!("multi_select_{exit_button}"),
            &render(&MultiSelectBar { exit_button }),
            data["selection_bars"][exit_button.to_string()]
                .as_str()
                .unwrap()
        ));
    }
}

#[test]
fn member_panel_keeps_each_requests_selection_token_and_no_member_state() {
    struct SessionToken(&'static str);
    impl h::request_forgery::AuthenticityTokens for SessionToken {
        fn global(&self) -> String {
            self.0.into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            assert_eq!((action, method), ("/rooms/directs", "post"));
            self.0.into()
        }
    }
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let mut outputs = Vec::new();
    for (viewer, token) in [("David", "viewer-one-token"), ("Kevin", "viewer-two-token")] {
        let ctx = context(Some(viewer), &asset, &signer, "");
        outputs.push(h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(SessionToken(token)),
                csp_nonce: None,
            },
            || {
                MemberPanel {
                    ctx: &ctx,
                    room_id: 42,
                }
                .render()
                .unwrap()
            },
        ));
    }
    assert!(outputs[0].contains("viewer-one-token"));
    assert!(!outputs[0].contains("viewer-two-token"));
    assert!(outputs[1].contains("viewer-two-token"));
    assert!(!outputs[1].contains("viewer-one-token"));
    assert_eq!(
        outputs[0].replace("viewer-one-token", "TOKEN"),
        outputs[1].replace("viewer-two-token", "TOKEN")
    );
    assert!(!outputs[0].contains("David"));
    assert!(!outputs[1].contains("Kevin"));
}

#[test]
fn member_panel_toggle_matches_pinned_rails_bytes() {
    let data: Value =
        serde_json::from_str(include_str!("../golden/member_panel/partials.json")).unwrap();
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(Some("David"), &asset, &signer, "");
    assert!(compare(
        "member_panel_toggle",
        &render(&MemberPanelToggle { ctx: &ctx }),
        data["toggle"].as_str().unwrap()
    ));
}

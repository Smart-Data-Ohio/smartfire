use askama::Template;
use campfire_views::{
    helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with},
    messages::RoomKind,
    rooms::{FormLayout, FormRoom},
};
#[path = "support/context.rs"]
mod common;
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
#[test]
fn room_form_layout_including_icon_errors_matches_complete_rails_bytes() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/room_icons.json")).unwrap();
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    let mut failures = 0;
    for row in oracle["forms"].as_array().unwrap() {
        let mut room: FormRoom = serde_json::from_value(row["room"].clone()).unwrap();
        room.icon = room
            .icon_name
            .as_deref()
            .and_then(campfire_views::messages::reactions::static_icon);
        let kind = if row["kind"] == "open" {
            RoomKind::Open
        } else {
            RoomKind::Closed
        };
        let actual = rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: None,
            },
            || {
                FormLayout {
                    ctx: &ctx,
                    room: &room,
                    can_administer: true,
                    kind,
                    content: "<p>Access fixture</p>".into(),
                }
                .render()
                .unwrap()
            },
        );
        let expected = row["html"].as_str().unwrap();
        if actual != expected {
            failures += 1;
            if let Ok(dir) = std::env::var("WS8BR_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                let name = format!(
                    "{}-{}",
                    row["kind"].as_str().unwrap(),
                    row["room"]["icon_name"].as_str().unwrap_or("none")
                );
                std::fs::write(format!("{dir}/{name}.actual"), actual).unwrap();
                std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
            }
        }
    }
    assert_eq!(failures, 0, "complete room form-layout byte mismatches");
}

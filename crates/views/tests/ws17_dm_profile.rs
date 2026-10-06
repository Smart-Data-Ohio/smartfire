use askama::Template;
use campfire_views::{
    AccountSummary, Platform, ViewContext, helpers as h, layouts, users::statuses::*,
};
use serde_json::Value;
#[path = "../../../test-support/asset_goldens.rs"]
mod asset_goldens;
struct Tokens;
impl h::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
#[test]
fn ws17_dm_wrapper_profile_badges_and_allowance_controls_match_pinned_rails_bytes() {
    let golden: Value = serde_json::from_str(include_str!("golden/ws17-dm-profile.json")).unwrap();
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = ViewContext {
        current_user: None,
        account: AccountSummary {
            name: "37signals".into(),
            logo_url: String::new(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Platform::default(),
        vapid_public_key: None,
        asset_path: &asset,
        importmap_tags: "",
        stylesheet_tags: "",
        custom_styles: None,
        cable_url: "/cable".into(),
        base_url: "http://campfire.test".into(),
        request_url: "http://campfire.test/users/127326141".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: &signer,
        time_zone: campfire_views::time::Zone::utc(),
        chrome: layouts::Chrome::default(),
    };
    let compare = |name: &str, actual: String, expected: &str| {
        if let Ok(dir) = std::env::var("WS17_SETTINGS_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/{name}.actual"), &actual).unwrap();
            std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
        }
        assert!(asset_goldens::compare(name, &actual, expected), "{name}");
    };
    for row in golden["rows"].as_array().unwrap() {
        let members: Vec<OooNoticeMember> = serde_json::from_value(row["members"].clone()).unwrap();
        compare(
            row["name"].as_str().unwrap(),
            OooNotices {
                direct: row["direct"].as_bool().unwrap(),
                members: &members,
            }
            .render()
            .unwrap(),
            row["html"].as_str().unwrap(),
        );
    }
    for row in golden["profiles"].as_array().unwrap() {
        let status = ProfileStatus {
            user_id: row["user_id"].as_i64().unwrap(),
            stream_name: row["stream_name"].as_str().unwrap().into(),
            presence: row["presence"].as_str().unwrap().into(),
            status_text: row["status_text"].as_str().map(str::to_string),
            dnd_allowed: false,
        };
        compare(
            row["name"].as_str().unwrap(),
            ProfileStatusSection { status: &status }.render().unwrap(),
            row["html"].as_str().unwrap(),
        );
    }
    h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        },
        || {
            for row in golden["allowances"].as_array().unwrap() {
                let status = ProfileStatus {
                    user_id: 127326141,
                    stream_name: String::new(),
                    presence: "offline".into(),
                    status_text: None,
                    dnd_allowed: row["allowed"].as_bool().unwrap(),
                };
                compare(
                    "allowance",
                    DndAllowance {
                        ctx: &ctx,
                        status: &status,
                    }
                    .render()
                    .unwrap(),
                    row["html"].as_str().unwrap(),
                );
            }
        },
    );
}

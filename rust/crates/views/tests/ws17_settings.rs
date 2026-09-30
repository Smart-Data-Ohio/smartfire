use askama::Template;
use campfire_views::{AccountSummary, Platform, ViewContext, helpers as h, layouts, users};

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
fn ws17_owned_settings_html_matches_complete_rails_partials() {
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("golden/ws17-settings.json")).unwrap();
    let asset = |_: &str| golden["check_asset"].as_str().unwrap().into();
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
        request_url: "http://campfire.test/users/me/profile".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: &signer,
        time_zone: campfire_views::time::Zone::utc(),
        chrome: layouts::Chrome::default(),
    };
    let mut failures = Vec::new();
    for row in golden["rows"].as_array().unwrap() {
        let data: users::SettingsFormData = serde_json::from_value(row["data"].clone()).unwrap();
        for (name, actual) in h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || {
                [
                    (
                        "status",
                        users::StatusForm {
                            ctx: &ctx,
                            data: &data,
                        }
                        .render()
                        .unwrap(),
                    ),
                    (
                        "notifications",
                        users::NotificationForm {
                            ctx: &ctx,
                            data: &data,
                        }
                        .render()
                        .unwrap(),
                    ),
                ]
            },
        ) {
            let expected = row["html"][name].as_str().unwrap();
            if actual != expected {
                let state = row["name"].as_str().unwrap();
                let byte = actual
                    .bytes()
                    .zip(expected.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                if let Ok(dir) = std::env::var("WS17_SETTINGS_DIFF_DIR") {
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(format!("{dir}/{state}-{name}.actual"), &actual).unwrap();
                    std::fs::write(format!("{dir}/{state}-{name}.expected"), expected).unwrap();
                }
                failures.push(format!(
                    "{state}/{name}: byte {byte}; Rust {}, Rails {}",
                    actual.len(),
                    expected.len()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

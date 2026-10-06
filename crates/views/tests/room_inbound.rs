use askama::Template;
use campfire_views::{
    helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with},
    rooms::{InboundEmailSection, InboundEmailView},
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
fn inbound_email_section_matches_complete_rails_bytes_for_every_display_state() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/inbound_section.json")).unwrap();
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    for row in oracle["states"].as_array().unwrap() {
        let email: InboundEmailView = serde_json::from_value(row.clone()).unwrap();
        let actual = rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: None,
            },
            || {
                InboundEmailSection {
                    ctx: &ctx,
                    email: &email,
                    can_administer: row["can_administer"].as_bool().unwrap(),
                }
                .render()
                .unwrap()
            },
        );
        assert_eq!(actual, row["html"].as_str().unwrap(), "{}", row["name"]);
    }
}

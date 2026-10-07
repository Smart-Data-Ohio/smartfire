//! Google sign-in forms and domains keep Rails contracts; profile panels keep byte goldens.
use askama::Template;
use campfire_views::{
    helpers::request_forgery::{self, AuthenticityTokens, RequestSecrets},
    sessions::GoogleSignIn,
};
#[path = "../../../test-support/form_contracts.rs"]
mod form_contracts;
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
fn sign_in_partial_preserves_rails_form_and_domains() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/google_sign_in_html.json")).unwrap();
    let html = request_forgery::rendering_with(
        RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: None,
        },
        || {
            GoogleSignIn {
                domains: vec!["smartdata.net".into(), "cnbssoftware.com".into(), "other.test".into()],
            }
            .render()
            .unwrap()
        },
    );
    form_contracts::assert_forms("Google sign-in", &html, v["html"].as_str().unwrap());
    form_contracts::assert_text(&html, "Sign in with Google");
    form_contracts::assert_text(&html, "Other email addresses can sign in with email and password.");
    form_contracts::assert_text(&html, "Google sign-in for @smartdata.net, @cnbssoftware.com, and @other.test accounts.");

}

#[test]
fn google_profile_panels_match_pinned_rails() {
    use campfire_views::users::google::{Calendar, SignIn};
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/google_profile_html.json")).unwrap();
    request_forgery::rendering_with(
        RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: None,
        },
        || {
            for c in v["calendar"].as_array().unwrap() {
                let data = serde_json::from_value(c.clone()).unwrap();
                assert_eq!(
                    Calendar { data }.render().unwrap(),
                    c["html"],
                    "{}",
                    c["name"]
                );
            }
            for c in v["signin"].as_array().unwrap() {
                let data = serde_json::from_value(c.clone()).unwrap();
                assert_eq!(
                    SignIn { data }.render().unwrap(),
                    c["html"],
                    "{}",
                    c["name"]
                );
            }
        },
    );
}
#[test]
fn google_drive_chips_match_pinned_rails() {
    use campfire_views::messages::{DriveAttachments, MessageView};
    use serde_json::json;
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/google_profile_html.json")).unwrap();
    for c in v["drive"].as_array().unwrap() {
        let message:MessageView=serde_json::from_value(json!({"id":c["id"],"client_message_id":c["client_message_id"],"room_id":1,"room_name":"room","creator":{"id":1,"name":"Member","title":"Member","avatar_url":"/avatar"},"created_at":"2026-09-23T10:00:00Z","updated_at":"2026-09-23T10:00:00Z","all_emoji":false,"content":{"type":"text","html":""},"details":{"drive_urls":c["urls"]}})).unwrap();
        assert_eq!(
            DriveAttachments { message: &message }.render().unwrap(),
            c["html"]
        );
    }
}

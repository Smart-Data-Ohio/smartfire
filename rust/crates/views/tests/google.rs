//! Exact pinned Rails bytes, including the SVG, form token and whitespace.
use askama::Template;
use campfire_views::{sessions::GoogleSignIn, helpers::request_forgery::{self, AuthenticityTokens, RequestSecrets}};
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self)->String {"GLOBAL".into()}
    fn for_form(&self,action:&str,method:&str)->String {format!("{method}:{action}")}
}
#[test]
fn sign_in_partial_matches_pinned_rails() {
    let v:serde_json::Value=serde_json::from_str(include_str!("../../../vectors/google_sign_in_html.json")).unwrap();
    let html=request_forgery::rendering_with(RequestSecrets {tokens:Box::new(Tokens),csp_nonce:None},||GoogleSignIn {domains:v["domains"].as_str().unwrap()}.render().unwrap());
    assert_eq!(html,v["html"].as_str().unwrap());
    assert_ne!(html.replace("Sign in with Google","Sign in"),v["html"]);
    assert_ne!(html.replace("<svg ","<svg  "),v["html"]);
}

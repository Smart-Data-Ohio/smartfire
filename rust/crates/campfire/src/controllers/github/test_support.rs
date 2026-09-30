use super::card_tests::Fresh;
use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use campfire_kit::Crypto;
use serde_json::{Value, json};
use tower::ServiceExt;
pub(super) fn session(fresh: &Fresh, value: &Value) -> String {
    let raw = campfire_kit::RailsCrypto::new(fresh.app.secrets.clone()).encrypt_cookie(
        "_campfire_session",
        value,
        None,
    );
    format!(
        "{}; _campfire_session={}",
        fresh.cookie,
        url::form_urlencoded::byte_serialize(raw.as_bytes()).collect::<String>()
    )
}
pub(super) fn response_session(fresh: &Fresh, headers: &HeaderMap) -> Value {
    headers
        .get_all("set-cookie")
        .iter()
        .find_map(|h| {
            h.to_str()
                .ok()?
                .split(';')
                .next()?
                .strip_prefix("_campfire_session=")
        })
        .and_then(|raw| {
            campfire_kit::RailsCrypto::new(fresh.app.secrets.clone()).decrypt_cookie(
                "_campfire_session",
                &percent_encoding::percent_decode_str(raw).decode_utf8_lossy(),
                fresh.app.clock.now(),
            )
        })
        .unwrap_or(json!({}))
}
pub(super) async fn request(
    fresh: &Fresh,
    method: &str,
    path: &str,
    body: Value,
    mut values: Value,
) -> (u16, HeaderMap, String) {
    // Use the actual CSRF/session protocol, with deterministic global token bytes.
    let raw = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, [7u8; 32]);
    values["_csrf_token"] = json!(raw);
    let token = campfire_kit::csrf::mask(&[7u8; 32], [9u8; 32]);
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("Host", "example.org")
                .header("Cookie", session(fresh, &values))
                .header("X-CSRF-Token", token)
                .header("Content-Type", "application/json")
                .body(if method == "GET" {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    (status, headers, body)
}
pub(super) fn sudo() -> Value {
    json!({"sudo_verified_at":1767268800})
}

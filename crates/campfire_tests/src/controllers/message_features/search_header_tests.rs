//! Compare the same Rack application stack; bare Rails omits config.ru's Deflater.
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use serde_json::Value;

#[tokio::test]
async fn complete_search_headers_match_rails_when_both_include_the_production_deflater() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/search_headers.json"
    ))
    .unwrap();
    // Match the committed Rails fixture's push configuration as well as its request headers.
    let env = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../parity/.env.reference"
    ))
    .unwrap();
    let vapid = env
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
        .collect::<Vec<_>>();
    assert_eq!(
        vapid
            .iter()
            .find(|(key, _)| *key == "VAPID_PUBLIC_KEY")
            .unwrap()
            .1,
        vector["vapid_public_key"].as_str().unwrap()
    );
    let app = TestApp::boot_frozen_with_env(&vapid)
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = app.david();
    let outputs = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.scratch/ws8bm2-c/http-bodies");
    std::fs::create_dir_all(&outputs).unwrap();
    let mut bare_differences = 0;
    for (index, case) in vector["cases"].as_array().unwrap().iter().enumerate() {
        // Browser supplies a default Accept; an explicit empty value reproduces
        // the missing-header Rails probe without changing production negotiation.
        let mut req = Req::new(Method::GET, case["path"].as_str().unwrap())
            .header("accept", case["accept"].as_str().unwrap_or(""));
        if case["frame"].as_bool().unwrap() {
            req = req.header("turbo-frame", "test-frame");
        }
        let response = browser.send(req).await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        let expected = case["headers"]["Vary"].as_str();
        if case["stack"] == "rack" {
            assert_eq!(
                response.header("vary"),
                expected,
                "{}; frame={} accept={}",
                case["path"],
                case["frame"],
                case["accept"]
            );
        } else {
            bare_differences += usize::from(response.header("vary") != expected);
        }
        for name in ["Content-Type", "Cache-Control", "X-Frame-Options"] {
            assert_eq!(
                response.header(&name.to_ascii_lowercase()),
                case["headers"][name].as_str(),
                "{name}: {}",
                case["path"]
            );
        }
        std::fs::write(outputs.join(format!("{index}.html")), response.text()).unwrap();
        if let Some(count) = case["search_token_inputs"].as_u64() {
            let text = response.text();
            let start = text.find("<section id=\"message-area\"").unwrap();
            let end = start + text[start..].find("</section>").unwrap() + "</section>".len();
            assert_eq!(
                text[start..end]
                    .matches("name=\"authenticity_token\"")
                    .count(),
                count as usize
            );
        }
        if let Some(section) = case["section"].as_str() {
            let text = response.text();
            let start = text.find("<section id=\"message-area\"").unwrap();
            let end = start + text[start..].find("</section>").unwrap() + "</section>".len();
            assert_eq!(
                &text[start..end],
                section,
                "search content bytes: {}",
                case["path"]
            );
        }
    }
    assert_eq!(
        bare_differences, 18,
        "bare Rails omits Rack::Deflater on every control and search response"
    );
    println!(
        "WS8bm2 search header probe: 18/18 production-stack Vary/header comparisons; 12/12 exact no-match search sections; search form token presence matches; bare Rails has 18 Accept-Encoding differences, including shared profile controls"
    );
}

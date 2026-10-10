//! Frozen media bytes/digests and ActionDispatch::Static responses from the Rails precompile.
use campfire_static_assets::{StaticRequest, asset_path, serve};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(format!(
            "{}/tests/reference/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn get(path: &str) -> campfire_static_assets::StaticResponse {
    serve(&StaticRequest {
        method: "GET",
        path,
        ..Default::default()
    })
    .unwrap_or_else(|| panic!("{path} isn't served"))
}

#[test]
fn every_recorded_media_digest_serves_the_original_bytes() {
    let aliases: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../legacy-digests.json")).unwrap();
    let hashes = fixture("compiled_sha256.json");
    assert_eq!(aliases.len(), 215);
    for (digest, logical) in aliases {
        let response = get(&format!("/assets/{digest}"));
        assert_eq!(
            sha256(&response.body),
            hashes[&digest].as_str().unwrap(),
            "{digest}"
        );
        assert_eq!(response.body, get(&asset_path(&logical)).body, "{logical}");
    }
    assert!(campfire_static_assets::try_asset_path("application.js").is_err());
    assert!(campfire_static_assets::try_asset_path("base.css").is_err());
}

#[test]
fn auth_and_font_paths_match_the_pre_extraction_manifest() {
    for (logical, digest) in fixture("auth-digests.json").as_object().unwrap() {
        assert_eq!(
            campfire_static_assets::digested_path(logical),
            digest.as_str(),
            "{logical}"
        );
    }
}

#[test]
fn public_and_media_files_are_served_like_action_dispatch_static() {
    for case in fixture("static_responses.json").as_array().unwrap() {
        let env = &case["env"];
        let request = StaticRequest {
            method: case["method"].as_str().unwrap(),
            path: case["path"].as_str().unwrap(),
            range: env["HTTP_RANGE"].as_str(),
            accept_encoding: env["HTTP_ACCEPT_ENCODING"].as_str(),
            if_modified_since: None,
        };
        let label = format!("{} {} {env}", request.method, request.path);
        if case["status"] == 404 && case["headers"].get("x-cascade").is_some() {
            assert!(serve(&request).is_none(), "{label}");
            continue;
        }
        let response = serve(&request).unwrap_or_else(|| panic!("{label} not served"));
        assert_eq!(
            response.status as u64,
            case["status"].as_u64().unwrap(),
            "{label}"
        );
        let ours: BTreeMap<_, _> = response
            .headers
            .iter()
            .filter(|(name, _)| *name != "last-modified")
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        let theirs: BTreeMap<_, _> = case["headers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str().unwrap()))
            .collect();
        assert_eq!(ours, theirs, "{label}");
        assert_eq!(
            sha256(&response.body),
            case["body_sha256"].as_str().unwrap(),
            "{label}"
        );
    }
}

#[test]
fn last_modified_round_trips_to_a_304() {
    let response = get("/robots.txt");
    let last_modified = response.header("last-modified").unwrap().to_string();
    let not_modified = campfire_static_assets::serve(&campfire_static_assets::StaticRequest {
        method: "GET",
        path: "/robots.txt",
        if_modified_since: Some(&last_modified),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(not_modified.status, 304);
    assert!(not_modified.headers.is_empty() && not_modified.body.is_empty());
}

#[test]
fn head_requests_have_no_body() {
    let response = campfire_static_assets::serve(&campfire_static_assets::StaticRequest {
        method: "HEAD",
        path: "/robots.txt",
        ..Default::default()
    })
    .unwrap();
    assert_eq!(response.status, 200);
    assert!(response.body.is_empty());
    assert_eq!(response.header("content-length"), Some("99"));
}

#[test]
fn multiple_ranges_are_multipart() {
    let sound = campfire_static_assets::audio_path("56k.mp3");
    let response = campfire_static_assets::serve(&campfire_static_assets::StaticRequest {
        method: "GET",
        path: &sound,
        range: Some("bytes=0-1, 4-5"),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(response.status, 206);
    // Rack sets multipart/byteranges, then Static overwrites it with the file's type.
    assert_eq!(response.header("content-type"), Some("audio/mpeg"));
    let body = String::from_utf8_lossy(&response.body);
    assert!(
        body.starts_with("\r\n--AaB03x\r\ncontent-type: audio/mpeg\r\ncontent-range: bytes 0-1/")
    );
    assert!(body.ends_with("\r\n--AaB03x--\r\n"));
}

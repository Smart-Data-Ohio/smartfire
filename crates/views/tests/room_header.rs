//! Exact pinned Rails header bytes, including per-viewer direct-room names.
use campfire_views::rooms::{HeaderIdentity, header_identity};
use serde_json::Value;
#[path = "support/context.rs"]
mod common;

#[test]
fn header_identity_matches_rails_for_every_kind_and_recipient() {
    let fixtures: Value =
        serde_json::from_str(include_str!("golden/rooms/directory.json")).unwrap();
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    let mut failures = 0;
    for row in fixtures["headers"].as_array().unwrap() {
        let header = HeaderIdentity {
            id: row["id"].as_i64().unwrap(),
            param_key: row["param_key"].as_str().unwrap().into(),
            direct: row["direct"].as_bool().unwrap(),
            kind_label: row["kind_label"].as_str().unwrap().into(),
            display_name: row["display_name"].as_str().unwrap().into(),
            icon: row["icon_name"]
                .as_str()
                .and_then(campfire_views::messages::reactions::static_icon),
        };
        let actual = header_identity(&ctx, &header).to_string();
        let expected = row["html"].as_str().unwrap();
        if actual != expected {
            failures += 1;
            let name = row["name"].as_str().unwrap();
            eprintln!(
                "{name}: actual {} bytes, Rails {} bytes",
                actual.len(),
                expected.len()
            );
            if let Ok(dir) = std::env::var("WS8BR_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(format!("{dir}/{name}.actual"), actual).unwrap();
                std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
            }
        }
    }
    assert_eq!(failures, 0, "header byte mismatches");
}

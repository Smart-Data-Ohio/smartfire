use super::*;
use campfire_views::agents::history::*;
#[test]
fn agent_history_pages_and_cards_match_pinned_rails_bytes() {
    let data: Value =
        serde_json::from_str(include_str!("../golden/agent_history_ui/pages.json")).unwrap();
    let env = include_str!("../../../../parity/.env.reference");
    let secrets = rails_compat::Secrets::new(
        env.lines()
            .find_map(|s| s.strip_prefix("SECRET_KEY_BASE="))
            .unwrap(),
    );
    let signer = |parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets, parts);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let styles = campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")]).html;
    let mut mismatches = Vec::new();
    for section in ["pages", "cards"] {
        for (name, expected) in data[section].as_object().unwrap() {
            let f = &data["facts"][name];
            let ctx = context(
                Some(f["viewer"].as_str().unwrap()),
                &asset,
                &signer,
                &styles,
            );
            let now = data["now"].as_str().unwrap().parse().unwrap();
            let agent_id = data["agent_id"].as_i64().unwrap();
            let bot_id = data["bot_id"].as_i64().unwrap();
            let bot_name = data["bot_name"].as_str().unwrap().to_owned();
            let filter = f["filter"].as_str().map(str::to_owned);
            let page = f["page"].as_i64().unwrap_or(1);
            let has_next = f["has_next"].as_bool().unwrap_or(false);
            let actual = if section == "cards" {
                render(&ApprovalCard {
                    ctx: &ctx,
                    approval: &serde_json::from_value::<Approval>(f["approval"].clone()).unwrap(),
                    now,
                })
            } else if name.starts_with("approvals") {
                render(&Approvals {
                    ctx: &ctx,
                    agent_id,
                    bot_id,
                    bot_name,
                    approvals: serde_json::from_value(f["approvals"].clone()).unwrap(),
                    filter,
                    page,
                    has_next,
                    now,
                })
            } else {
                render(&Ledger {
                    ctx: &ctx,
                    agent_id,
                    bot_id,
                    bot_name,
                    events: serde_json::from_value(f["events"].clone()).unwrap(),
                    filter,
                    page,
                    has_next,
                })
            };
            if !compare(name, &actual, expected.as_str().unwrap()) {
                mismatches.push(name);
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "Rails agent history UI differences: {mismatches:?}"
    );
}

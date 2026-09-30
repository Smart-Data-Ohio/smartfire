use super::*;
use campfire_views::accounts::{self, Bot, BotAgentForm, BotForm, BotGithubAccount, BotRoom};
use campfire_views::users::{Role, UserSummary};

fn goldens() -> Value {
    serde_json::from_str(include_str!("../golden/bots_ui/pages.json")).unwrap()
}
fn string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}
fn form(facts: &Value) -> BotForm {
    let agent = &facts["agent"];
    BotForm {
        error_fields: facts["error_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().into())
            .collect(),
        errors: string(&facts["errors"]),
        name: string(&facts["name"]),
        webhook_url: string(&facts["webhook_url"]),
        avatar_attachment_url: string(&facts["avatar_attachment_url"]),
        persisted: true,
        icon_name: string(&facts["icon_name"]),
        icon: facts["icon_name"]
            .as_str()
            .and_then(campfire_views::messages::reactions::static_icon),
        agent: agent.is_object().then(|| BotAgentForm {
            id: agent["id"].as_i64().unwrap(),
            owner_id: agent["owner_id"].as_i64(),
            provider: string(&agent["provider"]),
            runtime: string(&agent["runtime"]),
            description: string(&agent["description"]),
            daily_message_cap: agent["daily_message_cap"].as_i64(),
            daily_board_post_cap: agent["daily_board_post_cap"].as_i64(),
            daily_external_action_cap: agent["daily_external_action_cap"].as_i64(),
            suspended: !agent["suspended_at"].is_null(),
            errors: string(&facts["agent_errors"]),
            error_fields: facts["agent_error_fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().into())
                .collect(),
        }),
        budget_usage_line: string(&facts["budget_usage_line"]).unwrap_or_default(),
        signing_secret: string(&facts["signing_secret"]),
        github: facts["github"].is_object().then(|| BotGithubAccount {
            usable: facts["github"]["usable"].as_bool().unwrap(),
            login: string(&facts["github"]["login"]).unwrap(),
            disconnected_reason: string(&facts["github"]["disconnected_reason"]),
        }),
    }
}

#[test]
fn bot_management_pages_match_pinned_rails_bytes() {
    let env = include_str!("../../../../parity/.env.reference");
    let secrets = rails_compat::Secrets::new(
        env.lines()
            .find_map(|line| line.strip_prefix("SECRET_KEY_BASE="))
            .unwrap(),
    );
    let signer = |parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets, parts);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let styles = campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")]).html;
    let data = goldens();
    let ctx = context(Some("David"), &asset, &signer, &styles);
    let bots = data["facts"]["bots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bot| Bot {
            user: UserSummary {
                id: bot["id"].as_i64().unwrap(),
                name: string(&bot["name"]).unwrap(),
                avatar_path: string(&bot["avatar_path"]).unwrap(),
                role: Role::Bot,
                ..Default::default()
            },
            kind: string(&bot["kind"]),
            owner_name: string(&bot["owner_name"]),
            icon: None,
            rooms: bot["rooms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|room| BotRoom {
                    id: room["id"].as_i64().unwrap(),
                    name: string(&room["name"]).unwrap(),
                })
                .collect(),
        })
        .collect();
    let mut mismatches = Vec::new();
    for (name, actual) in [
        ("index", render(&accounts::BotsIndex { ctx: &ctx, bots })),
        (
            "new",
            render(&accounts::BotsNew {
                ctx: &ctx,
                bot: BotForm::default(),
            }),
        ),
        (
            "key",
            render(&accounts::BotKey {
                ctx: &ctx,
                bot_name: "Bender Bot",
                bot_key: &format!(
                    "{}-fixtureKey12",
                    data["facts"]["edit_admin"]["id"].as_i64().unwrap()
                ),
            }),
        ),
    ] {
        if !compare(
            &format!("bots_{name}"),
            &actual,
            data["pages"][name].as_str().unwrap(),
        ) {
            mismatches.push(name);
        }
    }
    for name in [
        "edit_admin",
        "edit_owner",
        "edit_suspended",
        "edit_legacy",
        "edit_github_connected_admin",
        "edit_github_connected_owner",
        "edit_github_rejected",
        "edit_github_unreadable",
        "edit_invalid_agent",
    ] {
        let ctx = context(
            Some(
                if name == "edit_owner" || name == "edit_github_connected_owner" {
                    "Kevin"
                } else {
                    "David"
                },
            ),
            &asset,
            &signer,
            &styles,
        );
        let bot = &data["facts"][name];
        let actual = render(&accounts::BotsEdit {
            ctx: &ctx,
            bot_id: bot["id"].as_i64().unwrap(),
            bot: form(bot),
        });
        if !compare(
            &format!("bots_{name}"),
            &actual,
            data["pages"][name].as_str().unwrap(),
        ) {
            mismatches.push(name);
        }
    }
    let mut invalid = form(&data["facts"]["new_invalid"]);
    invalid.persisted = false;
    let actual = render(&accounts::BotsNew {
        ctx: &ctx,
        bot: invalid,
    });
    if !compare(
        "bots_new_invalid",
        &actual,
        data["pages"]["new_invalid"].as_str().unwrap(),
    ) {
        mismatches.push("new_invalid");
    }
    assert!(
        mismatches.is_empty(),
        "Rails bot UI differences: {mismatches:?}"
    );
}

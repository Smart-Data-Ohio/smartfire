use super::*;
use campfire_views::{
    agents::{Directory, DirectoryAgent, StatusBadge, ThreadSteps},
    users::{Role, UserSummary},
};

fn string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}
fn agent(value: &Value) -> DirectoryAgent {
    DirectoryAgent {
        id: value["id"].as_i64().unwrap(),
        user: UserSummary {
            id: value["user_id"].as_i64().unwrap_or_default(),
            name: string(&value["name"]).unwrap_or_default(),
            avatar_path: string(&value["avatar_path"]).unwrap_or_default(),
            role: Role::Bot,
            ..Default::default()
        },
        icon: None,
        kind_description: string(&value["kind_description"]).unwrap_or_default(),
        status: string(&value["status"]).unwrap(),
        status_note: string(&value["status_note"]),
        suspended: value["suspended"].as_bool().unwrap(),
        created_at: value["created_at"].as_str().unwrap().parse().unwrap(),
        status_changed_at: string(&value["status_changed_at"]).map(|time| time.parse().unwrap()),
        last_seen_at: string(&value["last_seen_at"]).map(|time| time.parse().unwrap()),
    }
}

#[test]
fn directory_status_and_thread_step_bytes_match_pinned_rails() {
    let data: Value = serde_json::from_str(include_str!("../golden/agents_ui/pages.json")).unwrap();
    let env = include_str!("../../../../parity/.env.reference");
    let secrets = rails_compat::Secrets::new(
        env.lines()
            .find_map(|line| line.strip_prefix("SECRET_KEY_BASE="))
            .unwrap(),
    );
    let signer = |parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets, parts);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let styles = campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")]).html;
    let now = "2026-02-10T12:00:00Z".parse().unwrap();
    let mut mismatches = Vec::new();
    for name in ["admin", "member", "empty"] {
        let ctx = context(
            Some(if name == "admin" { "David" } else { "Kevin" }),
            &asset,
            &signer,
            &styles,
        );
        let agents = if name == "empty" {
            vec![]
        } else {
            data["agents"]
                .as_array()
                .unwrap()
                .iter()
                .map(agent)
                .collect()
        };
        let actual = render(&Directory {
            ctx: &ctx,
            agents,
            now,
        });
        if !compare(
            &format!("agents_{name}"),
            &actual,
            data["pages"][name].as_str().unwrap(),
        ) {
            mismatches.push(name.to_string());
        }
    }
    let ctx = context(None, &asset, &signer, &styles);
    for (name, value) in data["badges"].as_object().unwrap() {
        let agent = agent(&value["facts"]);
        let actual = StatusBadge {
            ctx: &ctx,
            agent: &agent,
            now,
        }
        .render()
        .unwrap();
        if !compare(
            &format!("status_{name}"),
            &actual,
            value["html"].as_str().unwrap(),
        ) {
            mismatches.push(name.clone());
        }
    }
    for (name, steps) in [
        (
            "thread_steps",
            serde_json::from_value(data["steps"].clone()).unwrap(),
        ),
        ("thread_steps_empty", vec![]),
    ] {
        let actual = ThreadSteps {
            thread_id: 42,
            steps,
        }
        .render()
        .unwrap();
        if !compare(name, &actual, data[name].as_str().unwrap()) {
            mismatches.push(name.to_string());
        }
    }
    for pair in data["durations"].as_array().unwrap() {
        let duration_ms = pair[0].as_i64().unwrap();
        let step = campfire_views::messages::parts::AgentStep {
            name: String::new(),
            status: "done".into(),
            duration_ms: Some(duration_ms),
            input_summary: None,
            output_summary: None,
        };
        assert_eq!(
            step.duration(),
            pair[1].as_str().unwrap(),
            "duration {duration_ms}ms"
        );
    }
    assert!(
        mismatches.is_empty(),
        "Rails agent UI differences: {mismatches:?}"
    );
}

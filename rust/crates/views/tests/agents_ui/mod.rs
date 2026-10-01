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

#[test]
fn bot_profiles_and_workspace_navigation_match_pinned_rails() {
    let data: Value = serde_json::from_str(include_str!("../golden/agents_ui/pages.json")).unwrap();
    let env=include_str!("../../../../parity/.env.reference");
    let secrets=rails_compat::Secrets::new(env.lines().find_map(|l|l.strip_prefix("SECRET_KEY_BASE=")).unwrap());
    let signer=|parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets,parts);
    let asset=|path: &str|campfire_assets::asset_path(path);
    let styles=campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track","reload")]).html;
    let now="2026-02-10T12:00:00Z".parse().unwrap();
    let mut differences=Vec::new();
    for viewer in ["admin","member"] {
        let ctx=context(Some(if viewer=="admin" {"David"} else {"Kevin"}),&asset,&signer,&styles);
        let record=&data["profiles"][viewer];let f=&record["facts"];
        let profile=campfire_views::agents::Profile {
            agent:agent(f),provider_runtime:f["provider_runtime"].as_str().unwrap().into(),
            description:string(&f["description"]),rooms:serde_json::from_value(f["rooms"].clone()).unwrap(),
            has_rooms:f["has_rooms"].as_bool().unwrap(),hidden_room_count:f["hidden_room_count"].as_u64().unwrap() as usize,
            grants:f["grants"].as_str().unwrap().into(),management:serde_json::from_value(f["management"].clone()).unwrap(),
        };
        let actual=render(&campfire_views::users::Show{ctx:&ctx,user:profile.agent.user.clone(),transfer_id:String::new(),profile_status:None,agent_profile:Some(profile),now});
        if !compare(&format!("bot_profile_{viewer}"),&actual,record["html"].as_str().unwrap()) { differences.push(viewer); }
    }
    let ctx=context(Some("Kevin"),&asset,&signer,&styles);
    let actual=campfire_views::users::WorkspaceDestinations{ctx:&ctx}.render().unwrap();
    if !compare("workspace_navigation",&actual,data["workspace_navigation"].as_str().unwrap()) {differences.push("navigation");}
    assert!(differences.is_empty(),"Pinned profile/navigation differences: {differences:?}");
}

#[test]
fn inbox_page_bytes_match_pinned_rails() {
    let data:Value=serde_json::from_str(include_str!("../golden/agents_ui/pages.json")).unwrap();
    let env=include_str!("../../../../parity/.env.reference");let secrets=rails_compat::Secrets::new(env.lines().find_map(|l|l.strip_prefix("SECRET_KEY_BASE=")).unwrap());
    let signer=|parts:&[&str]|rails_compat::turbo::signed_stream_name(&secrets,parts);let asset=|path:&str|campfire_assets::asset_path(path);
    let styles=campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track","reload")]).html;let ctx=context(Some("David"),&asset,&signer,&styles);
    let now="2026-02-10T12:00:00Z".parse().unwrap();let mut differences=Vec::new();
    for filter in ["unread","read","handled"] {
        let record=&data["inboxes"][filter];let items:Vec<campfire_views::activity::Item>=serde_json::from_value(record["items"].clone()).unwrap();
        let actual=render(&campfire_views::activity::Inbox{ctx:&ctx,items:&items,filter,type_filter:"all",before:None,next_cursor:None,unread_count:record["unread_count"].as_u64().unwrap() as usize,now});
        if !compare(&format!("inbox_{filter}"),&actual,record["html"].as_str().unwrap()) {differences.push(filter);}
    }
    assert!(differences.is_empty(),"Pinned inbox differences: {differences:?}");
}

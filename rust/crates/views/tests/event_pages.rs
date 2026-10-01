use askama::Template;
use campfire_views::{AccountSummary, CurrentUser, Platform, ViewContext, helpers as h, layouts};
use serde_json::Value;
fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/golden/core/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn facts() -> Value {
    serde_json::from_str(&fixture("facts.json")).unwrap()
}

fn context<'a>(
    user: Option<&str>,
    asset: &'a dyn Fn(&str) -> String,
    signer: &'a dyn Fn(&[&str]) -> String,
    styles: &'a str,
) -> ViewContext<'a> {
    let facts = facts();
    let current_user = user.map(|name| {
        let user = &facts["users"][name];
        CurrentUser {
            id: user["id"].as_i64().unwrap(),
            name: user["name"].as_str().unwrap().into(),
            administrator: user["administrator"].as_bool().unwrap(),
            bot: user["bot"].as_bool().unwrap(),
            avatar_url: user["avatar_path"].as_str().unwrap().into(),
            preferences: layouts::UserPreferences {
                tour_completed: user["tour_completed"].as_bool().unwrap(),
                ..Default::default()
            },
        }
    });
    let recent_searches = user
        .map(|name| {
            facts["users"][name]["searches"]
                .as_array()
                .unwrap()
                .iter()
                .map(|search| layouts::RecentSearch {
                    id: search["id"].as_i64().unwrap(),
                    query: search["query"].as_str().unwrap().into(),
                })
                .collect()
        })
        .unwrap_or_default();
    let env = include_str!("../../../parity/.env.reference");
    ViewContext {
        current_user,
        account: AccountSummary {
            name: "37signals".into(),
            logo_url: facts["account"]["logo_path"].as_str().unwrap().into(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Platform {
            mac: true,
            chrome: true,
            desktop: true,
            browser: "Chrome".into(),
            operating_system: "macOS".into(),
            ..Default::default()
        },
        vapid_public_key: Some(
            env.lines()
                .find_map(|line| line.strip_prefix("VAPID_PUBLIC_KEY="))
                .unwrap()
                .into(),
        ),
        asset_path: asset,
        importmap_tags: campfire_assets::javascript_importmap_tags(),
        stylesheet_tags: styles,
        custom_styles: None,
        cable_url: "/cable".into(),
        base_url: "http://campfire.test".into(),
        request_url: "http://campfire.test/".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: signer,
        time_zone: campfire_views::time::Zone::utc(),
        chrome: layouts::Chrome {
            service_worker_auto_register: true,
            brand_icon_names: facts["brand_icon_names"]
                .as_str()
                .unwrap()
                .split(',')
                .map(str::to_string)
                .collect(),
            recent_searches,
            ..Default::default()
        },
    }
}

struct Tokens;
impl h::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}

fn render(template: &impl Template) -> String {
    h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        },
        || template.render().unwrap(),
    )
}

#[test]
fn full_event_pages_match_pinned_rails_bytes() {
    compare_pages(include_str!("golden/event-pages.json"));
}
#[test]
fn full_event_forms_match_pinned_rails_bytes() {
    compare_pages(include_str!("golden/event-forms.json"));
}
fn compare_pages(source: &str) {
    let vectors: Value = serde_json::from_str(source).unwrap();
    let env = include_str!("../../../parity/.env.reference");
    let secrets = rails_compat::Secrets::new(
        env.lines()
            .find_map(|l| l.strip_prefix("SECRET_KEY_BASE="))
            .unwrap(),
    );
    let signer = |parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets, parts);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let styles = campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")]).html;
    let mut errors = Vec::new();
    for (index, vector) in vectors.as_array().unwrap().iter().enumerate() {
        let mut ctx = context(
            Some(vector["user"].as_str().unwrap()),
            &asset,
            &signer,
            &styles,
        );
        ctx.account.logo_url = vector["logo"].as_str().unwrap().into();
        ctx.current_user.as_mut().unwrap().avatar_url = vector["avatar"].as_str().unwrap().into();
        ctx.time_zone =
            campfire_views::time::Zone::lookup(vector["zone"].as_str().unwrap()).unwrap();
        let actual = if vector["kind"] == "index" {
            let view = serde_json::from_value(vector["view"].clone()).unwrap();
            render(&campfire_views::events::pages::Index {
                ctx: &ctx,
                view: &view,
            })
        } else if vector["kind"] == "show" {
            let view = serde_json::from_value(vector["view"].clone()).unwrap();
            render(&campfire_views::events::pages::Show {
                ctx: &ctx,
                view: &view,
            })
        } else if vector["kind"] == "new" {
            let view = serde_json::from_value(vector["view"].clone()).unwrap();
            render(&campfire_views::events::forms::New {
                ctx: &ctx,
                view: &view,
            })
        } else {
            let view = serde_json::from_value(vector["view"].clone()).unwrap();
            render(&campfire_views::events::forms::Edit {
                ctx: &ctx,
                view: &view,
            })
        };
        let expected = vector["html"].as_str().unwrap();
        if actual != expected {
            let byte = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            errors.push(format!(
                "state {index} {} {}: byte {byte}: actual {:?}, expected {:?}",
                vector["kind"],
                vector["user"],
                &actual[byte.saturating_sub(40)..(byte + 100).min(actual.len())],
                &expected[byte.saturating_sub(40)..(byte + 100).min(expected.len())]
            ));
            if let Ok(dir) = std::env::var("WS14E_PAGE_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(format!("{dir}/{index}.actual"), &actual).unwrap();
                std::fs::write(format!("{dir}/{index}.expected"), expected).unwrap();
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

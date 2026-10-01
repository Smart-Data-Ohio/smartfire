//! The owned shell regions compare complete Rails bytes; owner fragments are separate inputs.
use askama::Template;
use campfire_views::{CurrentUser, messages, rooms};
#[path = "support/context.rs"]
mod common;
use campfire_views::helpers::request_forgery::{
    AuthenticityTokens, RequestSecrets, rendering_with,
};
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("golden/rooms/shell.json")).unwrap()
}
fn view(row: &serde_json::Value) -> rooms::ShowView {
    let fragments = &row["owner_fragments"];
    let text = |key: &str| fragments[key].as_str().unwrap().to_string();
    rooms::ShowView {
        ooo_notice_members: Vec::new(),
        room: rooms::RoomView {
            id: row["room_id"].as_i64().unwrap(),
            kind: if row["kind"] == "direct" {
                messages::RoomKind::Direct
            } else {
                messages::RoomKind::Closed
            },
            name: row["room_name"].as_str().map(str::to_string),
            display_name: row["display_name"].as_str().unwrap().into(),
            header: None,
            involvement: row["involvement"].as_str().unwrap().into(),
        },
        user: messages::UserView {
            id: row["user_id"].as_i64().unwrap(),
            name: row["user_name"].as_str().unwrap().into(),
            title: row["user_name"].as_str().unwrap().into(),
            avatar_url: row["avatar_path"].as_str().unwrap().into(),
            icon: None,
        },
        updated_at: row["updated_at"].as_str().unwrap().parse().unwrap(),
        messages: vec![],
        invitation: false,
        join_code: String::new(),
        messages_stream_name: row["signed_stream_name"].as_str().unwrap().into(),
        shell: rooms::ShellComponents {
            pins_panel: text("pins_panel"),
            thread_panel: text("thread_panel"),
            message_template: Some(text("message_template")),
            composer: Some(text("composer")),
            poll_builder: text("poll_builder"),
            ..Default::default()
        },
        scroll_to_unread_divider: row["scroll_to_unread_divider"].as_bool(),
        jump_to_unread_url: None,
        unread_divider_message_id: None,
        unread_count: 0,
    }
}
#[test]
fn empty_room_shell_regions_match_rails() {
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let mut failures = 0;
    for row in fixtures() {
        let show = view(&row);
        let mut ctx = common::context(&asset, &signer);
        ctx.platform.browser = "Mozilla".into();
        ctx.current_user = Some(CurrentUser {
            id: show.user.id,
            name: show.user.name.clone(),
            administrator: true,
            bot: false,
            avatar_url: show.user.avatar_url.clone(),
            preferences: Default::default(),
        });
        let page = rooms::Show {
            ctx: &ctx,
            show: &show,
        };
        let parts = rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || {
                [
                    ("head", page.as_head().render().unwrap()),
                    ("nav", page.as_nav().render().unwrap()),
                    ("member_panel", page.as_member_panel().render().unwrap()),
                    ("thread_panel", page.as_thread_panel().render().unwrap()),
                    ("footer", page.as_footer().render().unwrap()),
                    ("body", page.as_content().render().unwrap()),
                ]
            },
        );
        for (name, actual) in parts {
            let expected = row["parts"][name].as_str().unwrap();
            if actual != expected {
                if let Ok(dir) = std::env::var("WS8BR_DIFF_DIR") {
                    std::fs::create_dir_all(&dir).unwrap();
                    let prefix = format!("{dir}/{}-{name}", row["name"].as_str().unwrap());
                    std::fs::write(format!("{prefix}.actual"), &actual).unwrap();
                    std::fs::write(format!("{prefix}.expected"), expected).unwrap();
                }
                let byte = actual
                    .bytes()
                    .zip(expected.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                eprintln!(
                    "{} {name}: first differing byte {byte}, actual {}, Rails {}",
                    row["name"],
                    actual.len(),
                    expected.len()
                );
                failures += 1;
            }
        }
    }
    assert_eq!(failures, 0, "owned shell region byte mismatches");
}
#[test]
fn message_list_seam_renders_the_empty_collection_or_supplied_owner_output() {
    let row = fixtures().remove(0);
    let mut show = view(&row);
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    assert_eq!(rooms::room_message_list(&ctx, &show).0, "");
    show.shell.message_list = Some("<div id=\"owned-message\">WS8b-m</div>\n".into());
    assert_eq!(
        rooms::room_message_list(&ctx, &show).0,
        "<div id=\"owned-message\">WS8b-m</div>\n"
    );
    assert!(
        rooms::Show {
            ctx: &ctx,
            show: &show
        }
        .as_content()
        .render()
        .unwrap()
        .contains("<div id=\"owned-message\">WS8b-m</div>\n")
    );
}

#[test]
fn unread_jump_controls_match_rails() {
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    for row in fixtures() {
        let mut show = view(&row);
        show.unread_divider_message_id = Some(101);
        let local = rooms::Show {
            ctx: &ctx,
            show: &show,
        }
        .as_content()
        .render()
        .unwrap();
        assert!(
            local.contains(row["jump_buttons"]["divider"].as_str().unwrap()),
            "divider jump bytes"
        );
        show.jump_to_unread_url = Some(format!("/rooms/{}?message_id=101", show.room.id));
        let linked = rooms::Show {
            ctx: &ctx,
            show: &show,
        }
        .as_content()
        .render()
        .unwrap();
        assert!(
            linked.contains(row["jump_buttons"]["link"].as_str().unwrap()),
            "off-page link bytes"
        );
    }
}

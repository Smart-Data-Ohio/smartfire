//! Isolated shell-seam checks. Complete Rails byte acceptance runs through room HTTP tests.
use askama::Template;
use campfire_views::{messages, rooms};
#[path = "support/context.rs"]
mod common;
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("golden/rooms/shell.json")).unwrap()
}
fn view(row: &serde_json::Value) -> rooms::ShowView {
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
        shell: rooms::ShellComponents::default(),
        scroll_to_unread_divider: row["scroll_to_unread_divider"].as_bool(),
        jump_to_unread_url: None,
        unread_divider_message_id: None,
        unread_divider_index:None,
        unread_count: 0,
        ooo_notice_members: vec![],
    }
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

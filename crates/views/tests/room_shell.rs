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
        navigation: None,
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
        unread_divider_index: None,
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

#[test]
fn review_pr175_real_list_renderer_overrides_fallback_once_for_each_viewer() {
    let asset = |name: &str| campfire_assets::asset_path(name);
    let signer = |_: &[&str]| String::new();
    let ctx = common::context(&asset, &signer);
    let mut show = view(&fixtures()[0]);
    show.messages = (0..8)
        .map(|i| messages::MessageItem::Fragment {
            client_message_id: format!("review_{i}"),
            room_id: show.room.id,
            html: std::sync::Arc::new(format!("<div id=\"message_review_{i}\">Message {i}</div>")),
        })
        .collect();
    for (index, count) in [(Some(2), 6), (Some(6), 2), (Some(2), 6), (None, 0)] {
        let owner = messages::RoomIndex {
            ctx: &ctx,
            messages: &show.messages,
            unread_index: index,
            unread_count: count,
        }
        .render()
        .unwrap();
        show.shell.message_list = Some(owner.clone());
        // Fallback facts are populated as in PR177; neither can append another list or divider.
        show.unread_divider_index = Some(1);
        show.unread_count = 99;
        let actual = rooms::room_message_list(&ctx, &show).to_string();
        assert_eq!(actual, owner);
        assert_eq!(
            actual.matches("id=\"unread-divider\"").count(),
            usize::from(index.is_some())
        );
        assert_eq!(actual.matches("id=\"message_review_").count(), 8);
        if let Some(index) = index {
            let start = actual.find("<div id=\"unread-divider\"").unwrap();
            let end = start + actual[start..].find("</div>").unwrap() + 6;
            assert!(actual[start..end].contains(&format!("data-unread-count=\"{count}\"")));
            assert!(
                actual[end..]
                    .trim_start()
                    .starts_with(&format!("<div id=\"message_review_{index}\""))
            );
        }
    }
}

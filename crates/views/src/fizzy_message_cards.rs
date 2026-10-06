//! `rooms/fizzy/message_cards/new.html.erb`.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
use serde_json::Value;
pub struct FormView {
    pub back_path: String,
    pub action: String,
    pub room_name: String,
    pub plain: String,
    pub creator: String,
    pub connected: bool,
    pub boards: Value,
    pub board_id: String,
    pub title: String,
    pub description: String,
    pub user_name: String,
    pub account_name: String,
}
impl FormView {
    fn options(&self) -> h::Html {
        let mut html = String::from("<option value=\"\">Choose a board</option>\n");
        if let Some(boards) = self.boards.as_array() {
            html.push_str(
                &boards
                    .iter()
                    .map(|b| {
                        let id = b["id"].as_str().unwrap_or("");
                        h::content_tag_text(
                            "option",
                            h::attrs()
                                .attr_opt("selected", (id == self.board_id).then_some("selected"))
                                .attr("value", id),
                            b["name"].as_str().unwrap_or(""),
                        )
                        .0
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        h::raw(html)
    }
    fn excerpt(&self) -> String {
        campfire_richtext::ruby::truncate(&self.plain, 280, "...")
    }
}
#[derive(Template)]
#[template(path="rooms/fizzy/message_cards/new.html",blocks=["head","content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a FormView,
}
impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Create Fizzy card".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}

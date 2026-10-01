//! Byte-for-byte Rails board rows and listings. All inputs are gathered by the app presenter.
use super::RoomView;
use super::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;

#[derive(Clone, Debug)]
pub struct Row {
    pub id: i64,
    pub room_id: i64,
    pub name: String,
    pub work_status: String,
    pub work_label: String,
    pub lifecycle: String,
    pub owner_id: Option<i64>,
    pub owner_label: String,
    pub agent: bool,
    pub tags: Vec<String>,
    pub replies: i64,
    pub links: i64,
    pub updated_at: jiff::Timestamp,
}
#[derive(Template)]
#[template(path = "rooms/boards/_row.html")]
pub struct RowPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub row: &'a Row,
    pub column: bool,
}
impl RowPartial<'_> {
    fn dom_id(&self) -> String {
        format!(
            "{}_channel_thread_{}",
            if self.column {
                "board_column_row"
            } else {
                "board_row"
            },
            self.row.id
        )
    }
    fn path(&self) -> String {
        format!("/rooms/{}/threads/{}", self.row.room_id, self.row.id)
    }
    fn replies(&self) -> String {
        format!(
            "{} {}",
            self.row.replies,
            if self.row.replies == 1 {
                "reply"
            } else {
                "replies"
            }
        )
    }
    fn links(&self) -> String {
        format!(
            "{} linked {}",
            self.row.links,
            if self.row.links == 1 {
                "object"
            } else {
                "objects"
            }
        )
    }
    fn updated_time(&self) -> h::Html {
        h::content_tag_text(
            "time",
            h::attrs()
                .attr("datetime", self.ctx.time_zone.iso8601(self.row.updated_at))
                .data("local_time_target", "datetime"),
            &self.ctx.time_zone.to_fs(self.row.updated_at, "long"),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Listing {
    pub room: RoomView,
    pub board_view: bool,
    pub status: String,
    pub owner: String,
    pub tag: String,
    pub current_user_id: i64,
    pub can_administer: bool,
    pub posts: Vec<Row>,
    pub owner_options: Vec<(String, String)>,
    pub tag_counts: Vec<(String, i64)>,
    pub any_posts: bool,
    pub has_more: bool,
    pub page: i64,
    pub digest: Option<(String, String)>,
    pub stream_name: String,
}
#[derive(Template)]
#[template(path="rooms/boards/index.html", blocks=["head", "content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub board: &'a Listing,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.board.room.display_name.clone())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar room-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl Index<'_> {
    fn multi_select_bar(&self) -> h::Html {
        h::raw(
            crate::shared::MultiSelectBar { exit_button: true }
                .render()
                .expect("member selection bar"),
        )
    }
    fn view(&self) -> &'static str {
        if self.board.board_view {
            "board"
        } else {
            "list"
        }
    }
    fn filter_status(&self) -> &str {
        if self.board.board_view {
            "all"
        } else {
            &self.board.status
        }
    }
    fn path(&self, view: &str, include_status: bool, more: bool) -> String {
        use crate::helpers::{Param, with_query};
        let mut params = vec![
            ("owner", Param::One(self.board.owner.clone())),
            ("view", Param::One(view.into())),
        ];
        if include_status {
            params.push(("status", Param::One(self.board.status.clone())));
        }
        if !campfire_richtext::ruby::is_blank(&self.board.tag) {
            params.push(("tag", Param::One(self.board.tag.clone())));
        }
        if more {
            params.push(("page", Param::One((self.board.page + 1).to_string())));
        }
        with_query(&campfire_routes::room(self.board.room.id), params)
    }
    fn list_link(&self) -> h::Html {
        h::link_to_text(
            "List",
            &self.path("list", true, false),
            h::attrs().class(if self.board.board_view {
                "btn board__view"
            } else {
                "btn board__view active"
            }),
        )
    }
    fn board_link(&self) -> h::Html {
        h::link_to_text(
            "Board",
            &self.path("board", false, false),
            h::attrs().class(if self.board.board_view {
                "btn board__view active"
            } else {
                "btn board__view"
            }),
        )
    }
    fn clear_link(&self) -> h::Html {
        h::link_to_text(
            "Clear",
            &format!("/rooms/{}?view={}", self.board.room.id, self.view()),
            h::attrs().class("btn board__clear"),
        )
    }
    fn more_link(&self) -> h::Html {
        h::link_to_text(
            "Load more",
            &self.path(self.view(), !self.board.board_view, true),
            h::attrs().class("btn"),
        )
    }
    fn selected_options(&self, options: &[(String, String)], selected: &str) -> h::Html {
        h::raw(
            options
                .iter()
                .map(|(label, value)| {
                    let attrs = if value == selected {
                        h::attrs().attr("selected", "selected")
                    } else {
                        h::attrs()
                    };
                    h::content_tag_text("option", attrs.attr("value", value.as_str()), label).0
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
    fn status_options(&self) -> h::Html {
        self.selected_options(
            &[
                ("Open".into(), "open".into()),
                ("Done".into(), "done".into()),
                ("All".into(), "all".into()),
            ],
            &self.board.status,
        )
    }
    fn owner_options(&self) -> h::Html {
        self.selected_options(&self.board.owner_options, &self.board.owner)
    }
    fn tag_options(&self) -> h::Html {
        let mut options = vec![("Any tag".into(), String::new())];
        options.extend(
            self.board
                .tag_counts
                .iter()
                .map(|(name, count)| (format!("{name} ({count})"), name.clone())),
        );
        self.selected_options(&options, &self.board.tag)
    }
    fn rows(&self, status: Option<&str>) -> h::Html {
        h::raw(
            self.board
                .posts
                .iter()
                .filter(|row| status.is_none_or(|status| row.work_status == status))
                .map(|row| {
                    RowPartial {
                        ctx: self.ctx,
                        row,
                        column: self.board.board_view,
                    }
                    .render()
                    .expect("board row renders")
                })
                .collect::<String>(),
        )
    }
    fn column_count(&self, status: &str) -> usize {
        self.board
            .posts
            .iter()
            .filter(|row| row.work_status == status)
            .count()
    }
    fn digest_text(&self, text: &str) -> h::Html {
        use std::sync::LazyLock;
        static PARAGRAPHS: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new("\n{2,}").unwrap());
        let text = campfire_richtext::sanitizer::sanitize(
            text,
            &campfire_richtext::sanitizer::SafeList::defaults(),
        )
        .expect("valid digest HTML")
        .replace("\r\n", "\n")
        .replace('\r', "\n");
        let mut paragraphs = PARAGRAPHS.split(&text).collect::<Vec<_>>();
        while paragraphs.last() == Some(&"") {
            paragraphs.pop();
        }
        if paragraphs.is_empty() {
            return h::raw("<p></p>");
        }
        h::raw(
            paragraphs
                .into_iter()
                .map(|paragraph| {
                    let chars = paragraph.chars().collect::<Vec<_>>();
                    let mut body = String::new();
                    for (index, ch) in chars.iter().enumerate() {
                        body.push(*ch);
                        if *ch == '\n' && index > 0 && index + 1 < chars.len() {
                            body.push_str("<br />");
                        }
                    }
                    format!("<p>{body}</p>")
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    }
}

#[derive(Template)]
#[template(path="rooms/boards/new.html", blocks=["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a super::ClosedFormView,
}
impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("New chat room".into())
    }
}
#[derive(Template)]
#[template(path="rooms/boards/edit.html", blocks=["head", "content"])]
pub struct Edit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub form: &'a super::ClosedFormView,
    pub github: crate::github::subscriptions::Section,
}
impl Page for Edit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!(
            "Edit settings for {}",
            self.form.room.name.as_deref().unwrap_or_default()
        ))
    }
}
impl Edit<'_> {
    fn room_id(&self) -> i64 {
        self.form.room.id.expect("persisted board")
    }
}

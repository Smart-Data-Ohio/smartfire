//! Plain view models for `app/views/searches`.
pub use campfire_presentation::searches::*;

use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::{Page, RecentSearch},
    messages::MessageItem,
};
use askama::Template;
#[derive(Template)]
#[template(path="searches/index.html",blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub index: &'a IndexView,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some(
            self.index
                .query
                .as_ref()
                .map_or_else(|| "Search".into(), |q| format!("Search: {q}")),
        )
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar searches")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
#[derive(Template)]
#[template(path = "searches/_filters.html")]
pub struct Filters<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub chips: &'a [Chip],
}
#[derive(Template)]
#[template(path = "searches/_sections.html")]
pub struct Sections<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub sections: &'a [Section],
}
#[derive(Template)]
#[template(path = "searches/_page_recents.html")]
pub struct PageRecents<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub searches: &'a [RecentSearch],
}
#[derive(Template)]
#[template(path = "searches/_dropdown_recents.html")]
pub struct DropdownRecents<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub searches: &'a [RecentSearch],
}
#[derive(Template)]
#[template(path = "searches/_load_older.html")]
pub struct LoadOlder<'a> {
    pub query: &'a str,
    pub oldest_id: i64,
}
impl LoadOlder<'_> {
    pub fn path(&self) -> String {
        format!(
            "{}?before={}&q={}",
            campfire_routes::searches(),
            self.oldest_id,
            h::url::cgi_escape(self.query)
        )
    }
}








pub struct IndexView {
    pub query: Option<String>,
    pub q: Option<String>,
    pub messages: Vec<MessageItem>,
    pub recent_searches: Vec<RecentSearch>,
    pub return_to_room: Option<(i64, String)>,
    pub has_more: bool,
    pub oldest_id: Option<i64>,
    pub chips: Vec<Chip>,
    pub sections: Vec<Section>,
}
impl IndexView {
    pub fn filters(&self, ctx: &ViewContext) -> h::Html {
        h::raw(
            Filters {
                ctx,
                chips: &self.chips,
            }
            .render()
            .expect("filters"),
        )
    }
    pub fn sections(&self, ctx: &ViewContext) -> h::Html {
        h::raw(
            Sections {
                ctx,
                sections: &self.sections,
            }
            .render()
            .expect("sections"),
        )
    }
    pub fn recents(&self, ctx: &ViewContext) -> h::Html {
        h::raw(
            PageRecents {
                ctx,
                searches: &self.recent_searches,
            }
            .render()
            .expect("recents"),
        )
    }
    pub fn older(&self) -> h::Html {
        h::raw(
            LoadOlder {
                query: self.q.as_deref().unwrap_or_default(),
                oldest_id: self.oldest_id.expect("older cursor"),
            }
            .render()
            .expect("load older"),
        )
    }
    pub fn no_results(&self) -> bool {
        self.messages.is_empty() && self.sections.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_paths_escape_the_query_like_cgi_escape() {
        assert_eq!(
            search_path(r#"pizza & "pie" *~"#),
            "/searches?q=pizza+%26+%22pie%22+%2A~"
        );
    }
}

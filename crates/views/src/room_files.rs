//! Plain view models for Rooms::FilesController; no database access during rendering.
use crate::helpers::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;

#[derive(Template)]
#[template(path="rooms/files/index.html",blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub list: &'a Listing,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Files in {}", self.list.room_name))
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}

impl Index<'_>{pub fn human_size(&self,size:&i64)->String {human_size(*size)}}
pub use campfire_presentation::room_files::*;

pub trait ListingRendering {
    fn form(&self) -> h::FormWith;
    fn type_link(&self, kind: &str) -> h::Html;
    fn datetime(&self, ctx: &ViewContext, time: &jiff::Timestamp) -> h::Html;
}
impl ListingRendering for Listing {
fn form(&self) -> h::FormWith {
        h::form_with(self.path())
            .method("get")
            .class("room-files__search")
    }
    fn type_link(&self, kind: &str) -> h::Html {
        let mut attrs = h::attrs().class(if kind == self.file_type {
            "btn room-files__type active"
        } else {
            "btn room-files__type"
        });
        if kind == self.file_type {
            attrs = attrs.aria("current", "page");
        }
        let title = kind[..1].to_uppercase() + &kind[1..];
        h::link_to_text(&title, &self.query_path(kind, None, None), attrs)
    }

    fn datetime(&self, ctx: &ViewContext, time: &jiff::Timestamp) -> h::Html {
        crate::time::local_datetime_tag(&ctx.time_zone, *time, "time", h::attrs(), "")
    }
}

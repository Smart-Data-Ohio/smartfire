//! Plain view models for Rooms::FilesController; no database access during rendering.
use crate::helpers::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;
pub const TYPES: [&str; 5] = ["all", "images", "videos", "documents", "other"];
pub struct Upload {
    pub filename: String,
    pub download_path: String,
    pub byte_size: i64,
    pub content_type: String,
    pub creator_name: String,
    pub created_at: jiff::Timestamp,
    pub message_path: String,
}
pub struct Drive {
    pub url: String,
    pub creator_name: String,
    pub created_at: jiff::Timestamp,
    pub message_path: String,
}
pub struct Listing {
    pub room_id: i64,
    pub room_name: String,
    pub file_type: String,
    pub filename: String,
    pub upload_page: i64,
    pub drive_page: i64,
    pub raw_page: Option<String>,
    pub raw_drive_page: Option<String>,
    pub uploads: Vec<Upload>,
    pub drives: Vec<Drive>,
    pub more_uploads: bool,
    pub more_drive: bool,
}
impl Listing {
    pub fn path(&self) -> String {
        format!("/rooms/{}/files", self.room_id)
    }
    pub fn form(&self) -> h::FormWith {
        h::form_with(self.path())
            .method("get")
            .class("room-files__search")
    }
    pub fn query_path(&self, kind: &str, page: Option<String>, drive: Option<String>) -> String {
        let mut params = vec![("type", h::Param::One(kind.into()))];
        if !self.filename.is_empty() {
            params.push(("filename", h::Param::One(self.filename.clone())));
        }
        if let Some(v) = page {
            params.push(("page", h::Param::One(v)));
        }
        if let Some(v) = drive {
            params.push(("drive_page", h::Param::One(v)));
        }
        h::with_query(&self.path(), params)
    }
    pub fn type_link(&self, kind: &str) -> h::Html {
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
    pub fn more_uploads_path(&self) -> String {
        self.query_path(
            &self.file_type,
            Some((self.upload_page + 1).to_string()),
            self.raw_drive_page.clone(),
        )
    }
    pub fn more_drive_path(&self) -> String {
        self.query_path(
            &self.file_type,
            self.raw_page.clone(),
            Some((self.drive_page + 1).to_string()),
        )
    }
    pub fn datetime(&self, ctx: &ViewContext, time: &jiff::Timestamp) -> h::Html {
        crate::time::local_datetime_tag(&ctx.time_zone, *time, "time", h::attrs(), "")
    }
}
/// Rails' default number_to_human_size: base 1024, three significant digits, stripped decimals.
pub fn human_size(size: i64) -> String {
    if size == 1 {
        return "1 Byte".into();
    }
    if size.unsigned_abs() < 1024 {
        return format!("{size} Bytes");
    }
    let exponent = ((size.unsigned_abs() as f64).log(1024.0).floor() as usize).min(6);
    let value = size as f64 / 1024_f64.powi(exponent as i32);
    let precision = 2 - value.abs().log10().floor() as i32;
    let factor = 10_f64.powi(precision);
    let rounded = (value * factor).round() / factor;
    let text = format!("{:.*}", precision.max(0) as usize, rounded);
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    format!(
        "{text} {}",
        ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB"][exponent]
    )
}
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

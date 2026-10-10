use crate::helpers as h;

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
impl Listing {
    pub fn path(&self) -> String {
        format!("/rooms/{}/files", self.room_id)
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
}

impl Listing {     pub fn query_path(&self, kind: &str, page: Option<String>, drive: Option<String>) -> String {
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

}

//! `app/views/two_factor`: no credential material is retained outside the request.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
#[derive(Template)]
#[template(path = "two_factor/setups/show.html", blocks = ["head", "content"])]
pub struct Setup<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub key: String,
    pub qr: String,
}
impl Page for Setup<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Set up two-step sign-in".into())
    }
}
#[derive(Template)]
#[template(path = "two_factor/backup_codes/show.html", blocks = ["head", "content"])]
pub struct BackupCodes<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub codes: Vec<String>,
    pub signed_out: usize,
    pub continue_url: String,
}
impl Page for BackupCodes<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Backup codes".into())
    }
}
impl BackupCodes<'_> {
    fn text(&self) -> String {
        self.codes.join("\n")
    }
    fn download(&self) -> String {
        let encoded = h::url::cgi_escape(&self.text());
        format!("data:text/plain,{encoded}")
    }
}
#[derive(Template)]
#[template(path = "two_factor/challenges/show.html", blocks = ["head", "content"])]
pub struct Challenge<'a> {
    pub ctx: &'a ViewContext<'a>,
}
impl Page for Challenge<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Two-step sign-in".into())
    }
}

#[derive(Clone, Default)]
pub struct ProfileData {
    pub confirmed_at: Option<jiff::Timestamp>,
    pub devices: Vec<Device>,
    pub google: bool,
}
#[derive(Clone)]
pub struct Device {
    pub id: i64,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub last_used_at: Option<jiff::Timestamp>,
}
#[derive(Template)]
#[template(path = "users/profiles/_two_factor.html")]
pub struct Profile<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: ProfileData,
    pub now: jiff::Timestamp,
}
impl Profile<'_> {
    fn submit(&self, label: &str, class: &str) -> h::Html {
        submit(label, class)
    }
    fn confirmed_date(&self) -> String {
        self.ctx
            .time_zone
            .format(self.data.confirmed_at.unwrap(), "%B %d, %Y")
    }
    fn agent(&self, device: &Device) -> String {
        device
            .user_agent
            .as_deref()
            .filter(|s| !s.chars().all(char::is_whitespace))
            .unwrap_or("Unknown browser")
            .into()
    }
    fn last_used(&self, device: &Device) -> String {
        let mut parts = Vec::new();
        if let Some(ip) = &device.ip_address {
            parts.push(ip.clone());
        }
        if let Some(at) = device.last_used_at {
            parts.push(format!(
                "last used {} ago",
                crate::time::distance_of_time_in_words(&self.ctx.time_zone, at, self.now, false)
            ));
        }
        parts.join(" · ")
    }
}
pub fn submit(label: &str, class: &str) -> h::Html {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .attr("value", label)
            .class(class)
            .data("disable_with", label),
    )
}

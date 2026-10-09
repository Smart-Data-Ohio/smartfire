//! Two-factor enrollment, the sign-in challenge, and the backup-code page.

use askama::Template;

use crate::helpers::{self as h, filters};
use crate::layouts::Page;

#[derive(Template)]
#[template(path = "two_factor/setups/show.html", blocks = ["head", "content"])]
pub struct Setup<'a> {
    pub ctx: &'a crate::Context<'a>,
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
    pub ctx: &'a crate::Context<'a>,
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
    pub ctx: &'a crate::Context<'a>,
}

impl Page for Challenge<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Two-step sign-in".into())
    }
}

//! Plain inputs for credential and grant management. Digest/secret storage stays in WS11.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;

#[derive(Clone, Debug)]
pub enum CredentialExpiry {
    Time(jiff::Timestamp),
    Extended { datetime: String, expired: bool },
}
#[derive(Clone, Debug)]
pub struct Credential {
    pub id: i64,
    pub name: String,
    pub last_four: String,
    pub created_by: String,
    pub created_at: jiff::Timestamp,
    pub expires_at: Option<CredentialExpiry>,
    pub last_used_at: Option<jiff::Timestamp>,
    pub revoked: bool,
}
#[derive(Clone, Debug, Default)]
pub struct CredentialForm {
    pub name: Option<String>,
    pub expires_at: Option<String>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}
#[derive(Template)]
#[template(path = "accounts/bots/credentials/_credential.html")]
pub struct CredentialRow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub credential: &'a Credential,
    pub now: jiff::Timestamp,
}
impl CredentialRow<'_> {
    fn confirm(&self) -> String {
        format!(
            "Revoke {}? This takes effect immediately.",
            self.credential.name
        )
    }
    fn datetime(&self, at: &jiff::Timestamp) -> h::Html {
        h::local_datetime_tag(&self.ctx.time_zone, *at, "datetime", h::attrs(), "")
    }

    fn expiry_datetime(&self, at: &CredentialExpiry) -> h::Html {
        match at {
            CredentialExpiry::Time(at) => self.datetime(at),
            CredentialExpiry::Extended { datetime, .. } => h::content_tag(
                "time",
                h::attrs()
                    .attr("datetime", datetime)
                    .data("local_time_target", "datetime"),
                "",
            ),
        }
    }
    fn expired(&self) -> bool {
        self.credential
            .expires_at
            .as_ref()
            .is_some_and(|at| match at {
                CredentialExpiry::Time(at) => *at <= self.now,
                CredentialExpiry::Extended { expired, .. } => *expired,
            })
    }
    fn ago(&self, at: &jiff::Timestamp) -> String {
        h::time_ago_in_words(&self.ctx.time_zone, *at, self.now)
    }
}
#[derive(Template)]
#[template(path="accounts/bots/credentials/index.html", blocks=["head","content"])]
pub struct Credentials<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub bot_name: String,
    pub credentials: Vec<Credential>,
    pub credential: CredentialForm,
    pub now: jiff::Timestamp,
}
impl Credentials<'_> {
    fn rows(&self) -> askama::Result<h::Html> {
        let mut html = String::new();
        for credential in &self.credentials {
            html.push_str(
                &CredentialRow {
                    ctx: self.ctx,
                    bot_id: self.bot_id,
                    credential,
                    now: self.now,
                }
                .render()?,
            );
        }
        Ok(h::raw(html))
    }
}
impl Page for Credentials<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Agent credentials".into())
    }
}
#[derive(Template)]
#[template(path="accounts/bots/credentials/show.html",blocks=["head","content"])]
pub struct CredentialCreated<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub secret: String,
}
impl Page for CredentialCreated<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Credential created".into())
    }
}

#[derive(Clone, Debug)]
pub struct Grant {
    pub id: i64,
    pub capability: String,
    pub room_name: String,
    pub granted_by: String,
    pub created_at: jiff::Timestamp,
    pub revoked: bool,
}
#[derive(Clone, Debug, Default)]
pub struct GrantForm {
    pub capability: Option<String>,
    pub room_id: Option<String>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}
#[derive(Template)]
#[template(path = "accounts/bots/grants/_grant.html")]
pub struct GrantRow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub grant: &'a Grant,
}
impl GrantRow<'_> {
    fn confirm(&self) -> String {
        format!(
            "Revoke {}? This takes effect immediately.",
            self.grant.capability
        )
    }
    fn datetime(&self) -> h::Html {
        h::local_datetime_tag(
            &self.ctx.time_zone,
            self.grant.created_at,
            "datetime",
            h::attrs(),
            "",
        )
    }
}
#[derive(Template)]
#[template(path="accounts/bots/grants/index.html",blocks=["head","content"])]
pub struct Grants<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub bot_name: String,
    pub legacy: bool,
    pub grants: Vec<Grant>,
    pub grant: GrantForm,
    pub rooms: Vec<(String, String)>,
}
impl Grants<'_> {
    fn back(&self) -> String {
        if self.ctx.can_administer() {
            h::routes::edit_account_bot(self.bot_id)
        } else {
            h::routes::user(self.bot_id)
        }
    }
    fn capabilities(&self) -> Vec<(String, String)> {
        [
            "read_messages",
            "post_messages",
            "react",
            "manage_threads",
            "external_action",
            "fizzy",
            "dm_anyone",
        ]
        .into_iter()
        .map(|name| (name.into(), name.into()))
        .collect()
    }
    fn rows(&self) -> askama::Result<h::Html> {
        let mut html = String::new();
        for grant in &self.grants {
            html.push_str(
                &GrantRow {
                    ctx: self.ctx,
                    bot_id: self.bot_id,
                    grant,
                }
                .render()?,
            );
        }
        Ok(h::raw(html))
    }
}
impl Page for Grants<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Capability grants".into())
    }
}

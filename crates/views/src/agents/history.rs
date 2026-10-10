//! Plain history/ledger facts. Authorization, expiry and decisions stay in WS11.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
#[derive(Template)]
#[template(path = "agent_approvals/_card.html")]
pub struct ApprovalCard<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub approval: &'a Approval,
    pub now: jiff::Timestamp,
}
impl ApprovalCard<'_> {
    fn expires_in(&self) -> String {
        h::distance_of_time_in_words(
            &self.ctx.time_zone,
            self.now,
            self.approval.expires_at,
            false,
        )
    }
    fn status_label(&self) -> &str {
        if self.approval.status == "approved" {
            "Approved"
        } else {
            "Denied"
        }
    }
    fn service(&self) -> &str {
        if self.approval.action.starts_with("fizzy.") {
            "Fizzy"
        } else {
            "GitHub"
        }
    }
    fn decide_path(&self) -> String {
        format!("/agent_approvals/{}", self.approval.id)
    }
    fn approve_path(&self) -> String {
        format!("{}?decision=approved", self.decide_path())
    }
}
#[derive(Template)]
#[template(path="agents/approvals/for_agent.html",blocks=["head","content"])]
pub struct Approvals<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agent_id: i64,
    pub bot_id: i64,
    pub bot_name: String,
    pub approvals: Vec<Approval>,
    pub filter: Option<String>,
    pub page: i64,
    pub has_next: bool,
    pub now: jiff::Timestamp,
}
impl Approvals<'_> {
    fn back(&self) -> String {
        back(self.ctx, self.bot_id)
    }
    fn path(&self) -> String {
        format!("/agents/{}/approvals", self.agent_id)
    }
    fn page_path(&self, page: i64) -> String {
        page_path(&self.path(), "status", self.filter.as_deref(), page)
    }
    fn choices(&self) -> Vec<(String, String)> {
        choices(
            "All statuses",
            &["pending", "approved", "denied", "cancelled", "expired"],
        )
    }
    fn card(&self, approval: &Approval) -> askama::Result<h::Html> {
        Ok(h::raw(
            ApprovalCard {
                ctx: self.ctx,
                approval,
                now: self.now,
            }
            .render()?,
        ))
    }
}
impl Page for Approvals<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Agent approvals".into())
    }
}
#[derive(Template)]
#[template(path = "agents/events/_event.html")]
pub struct EventRow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub event: &'a LedgerEvent,
}
impl EventRow<'_> {
    fn datetime(&self) -> h::Html {
        h::local_datetime_tag(
            &self.ctx.time_zone,
            self.event.created_at,
            "datetime",
            h::attrs(),
            "",
        )
    }
    fn attempts(&self) -> String {
        format!(
            "{} attempt{}",
            self.event.webhook_attempts,
            if self.event.webhook_attempts == 1 {
                ""
            } else {
                "s"
            }
        )
    }
    fn truncate(&self, text: &str) -> String {
        h::truncate(text, 140, "...")
    }
}
#[derive(Template)]
#[template(path="agents/events/ledger.html",blocks=["head","content"])]
pub struct Ledger<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agent_id: i64,
    pub bot_id: i64,
    pub bot_name: String,
    pub events: Vec<LedgerEvent>,
    pub filter: Option<String>,
    pub page: i64,
    pub has_next: bool,
}
impl Ledger<'_> {
    fn back(&self) -> String {
        back(self.ctx, self.bot_id)
    }
    fn path(&self) -> String {
        format!("/agents/{}/events", self.agent_id)
    }
    fn page_path(&self, page: i64) -> String {
        page_path(&self.path(), "outcome", self.filter.as_deref(), page)
    }
    fn choices(&self) -> Vec<(String, String)> {
        choices(
            "All outcomes",
            &["pending", "delivered", "acknowledged", "suppressed"],
        )
    }
    fn rows(&self) -> askama::Result<h::Html> {
        let mut html = String::new();
        for event in &self.events {
            html.push_str(
                &EventRow {
                    ctx: self.ctx,
                    event,
                }
                .render()?,
            );
        }
        Ok(h::raw(html))
    }
}
impl Page for Ledger<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Agent activity".into())
    }
}
fn back(ctx: &ViewContext, bot_id: i64) -> String {
    if ctx.can_administer() {
        h::routes::edit_account_bot(bot_id)
    } else {
        h::routes::user(bot_id)
    }
}
pub use campfire_presentation::agents::history::*;

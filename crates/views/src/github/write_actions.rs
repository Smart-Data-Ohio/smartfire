//! Per-viewer write frame; forms receive request-local CSRF tokens through FormWith.
use crate::helpers as h;
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct WriteActions {
    pub thread_id: i64,
    pub room_id: i64,
    pub pull_request_id: i64,
    pub linked: bool,
    pub usable: bool,
    pub login: String,
    pub notice: Option<String>,
    pub alert: Option<String>,
    pub comment_body: Option<String>,
    pub review_body: Option<String>,
    pub reviewers_body: Option<String>,
}
fn submit(label: &str) -> String {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .value(label)
            .class("btn")
            .data("disable-with", label),
    )
    .0
}
impl WriteActions {
    pub fn render(&self) -> String {
        let mut html = format!(
            "<turbo-frame class=\"github-pr-write\" id=\"github_write_actions_channel_thread_{}\">\n",
            self.thread_id
        );
        if let Some(notice) = self
            .notice
            .as_deref()
            .filter(|s| !s.chars().all(char::is_whitespace))
        {
            html.push_str(&format!(
                "    <p class=\"github-pr-write__notice\" role=\"status\">{}</p>\n",
                h::escape(notice)
            ));
        }
        if let Some(alert) = self
            .alert
            .as_deref()
            .filter(|s| !s.chars().all(char::is_whitespace))
        {
            html.push_str(&format!(
                "    <p class=\"github-pr-write__error\" role=\"alert\">{}</p>\n",
                h::escape(alert)
            ));
        }
        html.push_str("\n\n");
        if self.usable {
            let hidden = h::hidden_field_tag(
                "pull_request_id",
                Some(&self.pull_request_id.to_string()),
                h::attrs(),
            )
            .0;
            let form = h::form_with(format!(
                "/rooms/{}/github/pull_request_comments",
                self.room_id
            ))
            .class("github-pr-write__comment");
            let textarea = form.text_area(
                "body",
                self.comment_body.as_deref(),
                h::attrs()
                    .attr("rows", 2)
                    .attr("required", true)
                    .class("input")
                    .attr(
                        "placeholder",
                        format!("Comment on GitHub as @{}…", self.login),
                    )
                    .attr("aria-label", "Comment on GitHub"),
            );
            html.push_str(&format!(
                "    {}\n",
                form.wrap(&format!(
                    "\n      {hidden}\n      {textarea}\n      {}\n",
                    submit("Comment on GitHub")
                ))
                .0
            ));
            let form = h::form_with(format!(
                "/rooms/{}/github/pull_request_review_requests",
                self.room_id
            ))
            .class("github-pr-write__request-review");
            let field = form.text_field(
                "reviewers",
                None,
                h::attrs()
                    .class("input").attr_opt("value",self.reviewers_body.as_deref())
                    .attr("placeholder", "GitHub usernames")
                    .attr("aria-label", "GitHub usernames"),
            );
            html.push_str(&format!(
                "    {}\n",
                form.wrap(&format!(
                    "\n      {hidden}\n      {field}\n      {}\n",
                    submit("Request review")
                ))
                .0
            ));
            let form = h::form_with(format!(
                "/rooms/{}/github/pull_request_reviews",
                self.room_id
            ))
            .class("github-pr-write__review");
            let textarea = form.text_area(
                "body",
                self.review_body.as_deref(),
                h::attrs()
                    .attr("rows", 2)
                    .class("input")
                    .attr("placeholder", "Review note (required to request changes)")
                    .attr("aria-label", "Review note"),
            );
            let approve = h::button_tag(
                h::attrs().name("event").value("APPROVE").class("btn"),
                "Approve",
            );
            let changes = h::button_tag(
                h::attrs()
                    .name("event")
                    .value("REQUEST_CHANGES")
                    .class("btn btn--negative"),
                "Request changes",
            );
            html.push_str(&format!("    {}",form.wrap(&format!("\n      {hidden}\n      {textarea}\n      <div class=\"github-pr-write__review-buttons\">\n        {approve}\n        {changes}\n      </div>\n")).0));
        } else if self.linked {
            html.push_str("    <p class=\"github-pr-write__connect\">\n      GitHub rejected your token. <a data-turbo-frame=\"_top\" href=\"/users/me/profile\">Reconnect GitHub</a> to comment and review from here.\n    </p>\n");
        } else {
            html.push_str("    <p class=\"github-pr-write__connect\">\n      <a data-turbo-frame=\"_top\" href=\"/users/me/profile\">Connect GitHub</a> to comment and review from here as yourself.\n    </p>\n");
        }
        html.push_str("</turbo-frame>");
        html
    }
}

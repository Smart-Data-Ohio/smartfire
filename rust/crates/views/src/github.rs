//! Session-independent GitHub partials. Data and authorization are supplied by the app.
use crate::{ViewContext, helpers as h};
#[derive(Clone, Debug, Default)]
pub struct Card {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub display_full_name: String,
    pub state: Option<String>,
    pub private: Option<bool>,
    pub title: Option<String>,
    pub fetch_error: Option<String>,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub base_branch: Option<String>,
    pub head_branch: Option<String>,
    pub review_decision: Option<String>,
    pub check_status: Option<String>,
    pub html_url: Option<String>,
    pub github_updated_at: Option<jiff::Timestamp>,
    pub files_loaded: bool,
    pub files: Vec<File>,
    pub files_total: i64,
    pub discussion_thread: Option<i64>,
}
#[derive(Clone, Debug, Default)]
pub struct File {
    pub filename: String,
    pub status: Option<String>,
    pub additions: i64,
    pub deletions: i64,
}
#[derive(Clone, Debug)]
pub struct CardMessage {
    pub id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
}
fn present(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .filter(|s| !s.chars().all(char::is_whitespace))
}
pub fn state_label(state: Option<&str>) -> &'static str {
    match state {
        Some("merged") => "Merged",
        Some("closed") => "Closed",
        Some("draft") => "Draft",
        _ => "Open",
    }
}
pub fn review_label(state: Option<&str>) -> Option<&'static str> {
    match state {
        Some("approved") => Some("Approved"),
        Some("changes_requested") => Some("Changes requested"),
        Some("review_required") => Some("Review required"),
        _ => None,
    }
}
pub fn checks_label(state: Option<&str>) -> &'static str {
    match state {
        Some("passing") => "Checks passing",
        Some("pending") => "Checks pending",
        Some("failing") => "Checks failing",
        _ => "No checks",
    }
}
pub fn card(ctx: &ViewContext<'_>, pr: &Card, message: Option<&CardMessage>) -> String {
    let full = h::escape(&format!("{}/{}", pr.owner, pr.repo));
    let mut html = format!(
        "<article class=\"github-pr-card github-pr-card--{}\" data-github-pr=\"{full}#{}\">\n  <header class=\"github-pr-card__header\">\n    <span class=\"github-pr-card__repo\">{}</span>\n    <span class=\"github-pr-card__number\">#{}</span>\n    <span class=\"github-pr-card__state\">{}</span>\n  </header>\n\n",
        h::escape(pr.state.as_deref().unwrap_or("unknown")),
        pr.number,
        h::escape(&pr.display_full_name),
        pr.number,
        state_label(pr.state.as_deref())
    );
    if let Some(error) = present(&pr.fetch_error) {
        html.push_str(&format!("    <p class=\"github-pr-card__error\">\n      Couldn’t load this pull request.\n      <span class=\"github-pr-card__error-detail\">{}</span>\n    </p>\n",h::escape(error)));
    } else if let Some(title) = present(&pr.title) {
        html.push_str(&format!("    <p class=\"github-pr-card__title\">{}</p>\n\n    <div class=\"github-pr-card__meta\">\n",h::escape(title)));
        if let Some(author) = present(&pr.author_login) {
            html.push_str("        <span class=\"github-pr-card__author\">\n");
            if let Some(avatar) = present(&pr.author_avatar_url) {
                html.push_str(&format!(
                    "            {}\n",
                    h::image_tag(
                        ctx,
                        avatar,
                        h::attrs()
                            .attr("alt", "")
                            .attr("size", "16x16")
                            .class("github-pr-card__avatar")
                            .attr("loading", "lazy")
                    )
                    .0
                ));
            }
            html.push_str(&format!(
                "          {}\n        </span>\n",
                h::escape(author)
            ));
        }
        html.push('\n');
        if let (Some(base), Some(head)) = (present(&pr.base_branch), present(&pr.head_branch)) {
            html.push_str(&format!("        <span class=\"github-pr-card__branches\" title=\"Base ← head\">\n          {} ← {}\n        </span>\n",h::escape(base),h::escape(head)));
        }
        html.push('\n');
        if let Some(label) = review_label(pr.review_decision.as_deref()) {
            html.push_str(&format!("        <span class=\"github-pr-card__review github-pr-card__review--{}\">{label}</span>\n",h::escape(pr.review_decision.as_deref().unwrap_or(""))));
        }
        html.push_str(&format!("\n        <span class=\"github-pr-card__checks github-pr-card__checks--{}\">{}</span>\n    </div>\n",h::escape(pr.check_status.as_deref().unwrap_or("none")),checks_label(pr.check_status.as_deref())));
    } else {
        html.push_str("    <p class=\"github-pr-card__loading\">Loading pull request…</p>\n");
    }
    html.push_str("\n  <footer class=\"github-pr-card__footer\">\n");
    if let Some(at) = pr.github_updated_at {
        html.push_str(&format!(
            "      <span class=\"github-pr-card__updated\">Updated {}</span>\n",
            crate::time::local_datetime_tag(&ctx.time_zone, at, "time", h::attrs(), "").0
        ));
    }
    let url = pr.html_url.clone().unwrap_or_else(|| {
        format!(
            "https://github.com/{}/{}/pull/{}",
            pr.owner, pr.repo, pr.number
        )
    });
    html.push_str(&format!("    <a class=\"github-pr-card__link\" target=\"_blank\" rel=\"noopener noreferrer\" href=\"{}\">View on GitHub</a>\n",h::escape(&url)));
    if let Some(message) = message.filter(|m| m.thread_id.is_none()) {
        if let Some(thread) = pr.discussion_thread {
            html.push_str(&format!("        <a class=\"github-pr-card__discuss\" data-turbo-frame=\"_top\" href=\"/rooms/{}/threads/{thread}\">Discuss</a>\n",message.room_id));
        } else {
            html.push_str(&format!("        <form data-turbo-frame=\"_top\" class=\"github-pr-card__discuss-form\" method=\"post\" action=\"/rooms/{}/github/pull_request_threads\"><button class=\"github-pr-card__discuss\" type=\"submit\">Discuss</button><input type=\"hidden\" name=\"message_id\" value=\"{}\" /><input type=\"hidden\" name=\"pull_request_id\" value=\"{}\" /></form>\n",message.room_id,message.id,pr.id));
        }
    }
    html.push_str("  </footer>\n</article>\n");
    html
}
pub fn frame_id(pr_id: i64, message_id: Option<i64>, thread_id: Option<i64>) -> String {
    match message_id {
        Some(id) => format!("card_for_message_{id}_github_pull_request_{pr_id}"),
        None => format!(
            "card_for_thread_{}_github_pull_request_{pr_id}",
            thread_id.map(|id| id.to_string()).unwrap_or_default()
        ),
    }
}
fn frame(pr_id: i64, room_id: i64, message_id: Option<i64>, thread_id: Option<i64>) -> String {
    let param = match message_id {
        Some(id) => format!("message_id={id}"),
        None => format!("thread_id={}", thread_id.unwrap_or_default()),
    };
    h::turbo_frame_tag(
        &frame_id(pr_id, message_id, thread_id),
        Some(&format!(
            "/rooms/{room_id}/github/pull_requests/{pr_id}/card?{param}"
        )),
        None,
        h::attrs()
            .attr("loading", "lazy")
            .class("github-pr-card-frame"),
        "",
    )
    .0
}
pub fn cards(
    ctx: &ViewContext<'_>,
    message_key: &str,
    message: &CardMessage,
    prs: &[Card],
) -> String {
    let mut html = format!(
        "<div id=\"github_pr_cards_message_{}\" class=\"github-pr-cards\">",
        h::escape(message_key)
    );
    for pr in prs {
        html.push_str("\n    ");
        if pr.private == Some(false) {
            html.push_str(&card(ctx, pr, Some(message)));
            html.push('\n');
        } else {
            html.push_str(&frame(pr.id, message.room_id, Some(message.id), None));
            html.push('\n');
        }
    }
    html.push_str("</div>\n");
    html
}
pub fn files_summary(pr: &Card) -> String {
    let mut html = String::from(
        "<section class=\"github-pr-files\" aria-label=\"Files changed\">\n  <h2 class=\"github-pr-files__heading\">Files changed</h2>\n",
    );
    if !pr.files_loaded {
        html.push_str("    <p class=\"github-pr-files__loading\">Loading files…</p>\n");
    } else if pr.files.is_empty() {
        html.push_str("    <p class=\"github-pr-files__empty\">No files changed.</p>\n");
    } else {
        html.push_str("    <ul class=\"github-pr-files__list\">\n");
        for file in &pr.files {
            html.push_str(&format!("        <li class=\"github-pr-files__file\">\n          <span class=\"github-pr-files__path\">{}</span>\n",h::escape(&file.filename)));
            if let Some(status) = present(&file.status) {
                let lower = status.replace('_', " ").to_lowercase();
                let mut chars = lower.chars();
                let human = chars
                    .next()
                    .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                    .unwrap_or_default();
                html.push_str(&format!(
                    "            <span class=\"github-pr-files__status\">{}</span>\n",
                    h::escape(&human)
                ));
            }
            html.push_str(&format!(
                "          <span class=\"github-pr-files__counts\">+{} −{}</span>\n        </li>\n",
                file.additions, file.deletions
            ));
        }
        html.push_str("    </ul>\n");
        let more = pr.files_total - pr.files.len() as i64;
        if more > 0 {
            html.push_str(&format!(
                "      <p class=\"github-pr-files__more\">and {more} more on GitHub</p>\n"
            ));
        }
    }
    html.push_str("</section>\n");
    html
}
pub fn thread_header(ctx: &ViewContext<'_>, room_id: i64, thread_id: i64, pr: &Card) -> String {
    let gid = h::gid_param("ChannelThread", thread_id);
    let signed = (ctx.signed_stream_name)(&[&gid, "messages"]);
    let stream = h::builder_tag(
        "turbo-cable-stream-source",
        h::attrs()
            .attr("channel", "RoomMessagesChannel")
            .attr("signed-stream-name", signed),
    );
    let mut html = format!(
        "<div id=\"github_pr_header_channel_thread_{thread_id}\" class=\"github-pr-thread-header\">\n  {}\n    ",
        stream.0
    );
    if pr.private == Some(false) {
        html.push_str(&card(ctx, pr, None));
        html.push_str("\n    ");
        html.push_str(&files_summary(pr));
        html.push('\n');
    } else {
        html.push_str(&frame(pr.id, room_id, None, Some(thread_id)));
        html.push('\n');
    }
    let write = h::turbo_frame_tag(
        &format!("github_write_actions_channel_thread_{thread_id}"),
        Some(&format!(
            "/rooms/{room_id}/github/pull_request_write_actions/{}",
            pr.id
        )),
        None,
        h::attrs().class("github-pr-write"),
        "\n    <p class=\"github-pr-write__loading\">Loading GitHub actions…</p>\n",
    );
    html.push_str(&format!("  {}</div>\n", write.0));
    html
}

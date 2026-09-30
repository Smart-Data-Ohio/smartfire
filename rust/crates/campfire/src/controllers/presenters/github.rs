//! GitHub rows to session-independent card partial data.
use crate::app::AppState;
use crate::integrations::github::{
    blank,
    client::{ruby_string, ruby_to_i},
    pull_requests::PullRequest,
    threads::PullRequestThread,
};
use campfire_db::{Account, Connection, Message, Result};
use campfire_views::github::{Card, CardMessage, File};
use serde_json::Value;

pub fn card(conn: &Connection, pr: &PullRequest, room_id: i64) -> Result<Card> {
    Ok(Card {
        id: pr.id,
        owner: pr.owner.clone(),
        repo: pr.repo.clone(),
        number: pr.number,
        display_full_name: pr.display_full_name()?,
        state: pr.state.clone(),
        private: pr.private,
        title: pr.title.clone(),
        fetch_error: pr.fetch_error.clone(),
        author_login: pr.author_login.clone(),
        author_avatar_url: pr.author_avatar_url.clone(),
        base_branch: pr.base_branch.clone(),
        head_branch: pr.head_branch.clone(),
        review_decision: pr.review_decision.clone(),
        check_status: pr.check_status.clone(),
        html_url: pr.html_url.clone(),
        github_updated_at: pr.github_updated_at.map(|t| t.jiff()),
        files_loaded: pr.changed_files.as_ref().is_some_and(|s| !blank(s)),
        files: Vec::new(),
        files_total: 0,
        discussion_thread: PullRequestThread::for_room_pr(conn, room_id, pr.id)?
            .map(|m| m.channel_thread_id),
    })
}
pub fn card_with_files(conn: &Connection, pr: &PullRequest, room_id: i64) -> Result<Card> {
    let mut data = card(conn, pr, room_id)?;
    let summary = pr.changed_files_summary()?;
    let mut files = Vec::new();
    for file in summary["files"]
        .as_array()
        .expect("summary files are Array")
    {
        let Some(file) = file.as_object() else {
            return Err(campfire_db::Error::Other(
                "Invalid changed file shape".into(),
            ));
        };
        let field = |name: &str| file.get(name).unwrap_or(&Value::Null);
        let integer = |name: &str| -> Result<i64> {
            match field(name) {
                Value::Null => Ok(0),
                Value::Number(n) => Ok(n
                    .as_i64()
                    .unwrap_or_else(|| n.as_f64().unwrap_or_default() as i64)),
                Value::String(s) => Ok(ruby_to_i(s)),
                _ => Err(campfire_db::Error::Other(
                    "Invalid changed file count".into(),
                )),
            }
        };
        files.push(File {
            filename: ruby_string(field("filename")),
            status: field("status").as_str().map(str::to_owned),
            additions: integer("additions")?,
            deletions: integer("deletions")?,
        });
    }
    data.files = files;
    data.files_total = summary["total_count"].as_i64().unwrap_or_default();
    Ok(data)
}
pub fn shared_card(conn: &Connection, pr: &PullRequest, room_id: i64, files: bool) -> Result<Card> {
    if pr.private != Some(false) {
        return Ok(Card { id:pr.id, owner:pr.owner.clone(), repo:pr.repo.clone(), number:pr.number, private:pr.private, ..Default::default() });
    }
    if files {card_with_files(conn, pr, room_id)} else {card(conn, pr, room_id)}
}
pub fn message_cards(conn: &Connection, app: &AppState, message: &Message) -> Result<String> {
    let cards = PullRequest::for_message(conn, message.id)?
        .iter()
        .map(|pr| shared_card(conn, pr, message.room_id, false))
        .collect::<Result<Vec<_>>>()?;
    let account = Account::first(conn)?;
    Ok(super::page::render_detached(app, account.as_ref(), |ctx| {
        campfire_views::github::cards(
            ctx,
            &message.client_message_id,
            &CardMessage {
                id: message.id,
                room_id: message.room_id,
                thread_id: message.thread_id,
            },
            &cards,
        )
    }))
}
pub fn cache_stamp(conn: &Connection, message: &Message) -> Result<String> {
    let prs = PullRequest::for_message(conn, message.id)?;
    if prs.is_empty() {
        return Ok(String::new());
    }
    let newest = prs.iter().map(|p| p.updated_at).max().expect("nonempty");
    let threads: Option<campfire_db::Timestamp> = conn.query_row(
        "SELECT MAX(updated_at) FROM github_pull_request_threads WHERE room_id=?",
        [message.room_id],
        |r| r.get(0),
    )?;
    Ok(format!(
        "github:{}:{}",
        newest.to_db(),
        threads.map(|t| t.to_db()).unwrap_or_default()
    ))
}

//! GitHub reference reconciliation registered on Message's create/edit hooks.
use super::{blank, jobs::FetchPullRequestJob};
use campfire_db::{Event, Message, Tx};
use rusqlite::{OptionalExtension, params};
use std::sync::LazyLock;
static URL: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/(?:pull|pulls)/([0-9]+)\b",
    )
    .unwrap()
});
/// Numbers retain decimal precision until the model checks SQLite's integer range.
pub fn extract(text: &str) -> Vec<(String, String, String)> {
    let mut triples = Vec::new();
    for capture in URL.captures_iter(text) {
        let owner = &capture[1];
        let repo = &capture[2];
        if [".", ".."].contains(&owner) || [".", ".."].contains(&repo) {
            continue;
        }
        let number = capture[3].trim_start_matches('0');
        let number = if number.is_empty() { "0" } else { number };
        let triple = (owner.to_owned(), repo.to_owned(), number.to_owned());
        if !triples.contains(&triple) {
            triples.push(triple);
            if triples.len() == 4 {
                break;
            }
        }
    }
    triples
}

pub fn pull_request_url(url: &str) -> bool {
    !extract(url).is_empty()
}

pub fn sync(tx: &mut Tx<'_>, message: &Message, enqueue_fetches: bool) -> campfire_db::Result<()> {
    let mut text = campfire_db::models::message_reference::non_code_text(
        &message.body_html(tx.conn())?.unwrap_or_default(),
    )?;
    if let Some(note) = message.forward_note.as_ref().filter(|note| !blank(note)) {
        text.push('\n');
        text.push_str(note);
    }
    let triples = extract(&text);
    let now = tx.now();
    let cutoff = now.ago(jiff::SignedDuration::from_mins(10));
    let mut prs = Vec::new();
    for (owner, repo, number) in triples {
        let owner = owner.to_lowercase();
        let repo = repo.to_lowercase();
        let number = number
            .parse::<i64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| {
                campfire_db::Error::Other("Number must be greater than 0 and fit SQLite".into())
            })?;
        tx.conn().execute("INSERT INTO github_pull_requests (owner,repo,number,created_at,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(owner,repo,number) DO NOTHING",params![owner,repo,number,now,now])?;
        let id = tx.conn().query_row(
            "SELECT id FROM github_pull_requests WHERE owner=? AND repo=? AND number=?",
            params![owner, repo, number],
            |r| r.get::<_, i64>(0),
        )?;
        prs.push(id);
    }
    let mut stmt = tx.conn().prepare(
        "SELECT id,github_pull_request_id FROM github_pull_request_references WHERE message_id=?",
    )?;
    let existing = stmt
        .query_map([message.id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    for (id, pr) in existing {
        if !prs.contains(&pr) {
            tx.conn().execute(
                "DELETE FROM github_pull_request_references WHERE id=?",
                [id],
            )?;
        }
    }
    for id in prs {
        let created=tx.conn().query_row("INSERT INTO github_pull_request_references (message_id,github_pull_request_id,created_at,updated_at) VALUES (?,?,?,?) ON CONFLICT(message_id,github_pull_request_id) DO NOTHING RETURNING id",params![message.id,id,now,now],|r|r.get::<_,i64>(0)).optional()?.is_some();
        let stale = tx.conn().query_row(
            "SELECT fetched_at IS NULL OR fetched_at < ? FROM github_pull_requests WHERE id=?",
            params![cutoff, id],
            |r| r.get::<_, bool>(0),
        )?;
        if enqueue_fetches&&(created||stale)&&tx.conn().execute("UPDATE github_pull_requests SET fetch_requested_at=? WHERE id=? AND (fetch_requested_at IS NULL OR fetch_requested_at < ?)",params![now,id,cutoff])?==1 {tx.emit_after_commit(Event::job(&FetchPullRequestJob {pull_request_id:id}));}
    }
    Ok(())
}

#[cfg(test)]
mod tests;

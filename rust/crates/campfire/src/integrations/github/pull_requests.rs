//! `Github::PullRequest`: identity, persisted card data and callback descriptions.
use super::{
    accounts::{Account, Accounts},
    blank,
    client::ruby_to_i,
};
use campfire_db::{Broadcast, Connection, Database, Errors, Event, Message, Result, Timestamp, Tx};
use rusqlite::{OptionalExtension, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::LazyLock;

pub const STALE_AFTER: jiff::SignedDuration = jiff::SignedDuration::from_mins(10);

#[derive(Clone, Debug)]
pub struct PullRequest {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub title: Option<String>,
    pub html_url: Option<String>,
    pub state: Option<String>,
    pub private: Option<bool>,
    pub payload: Option<Value>,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub head_branch: Option<String>,
    pub base_branch: Option<String>,
    pub review_decision: Option<String>,
    pub check_status: Option<String>,
    pub fetch_error: Option<String>,
    pub changed_files: Option<String>,
    pub github_updated_at: Option<Timestamp>,
    pub fetched_at: Option<Timestamp>,
    pub fetch_requested_at: Option<Timestamp>,
    pub updated_at: Timestamp,
}
impl PullRequest {
    pub fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let payload: Option<String> = r.get("payload")?;
        Ok(Self {
            id: r.get("id")?,
            owner: r.get("owner")?,
            repo: r.get("repo")?,
            number: r.get("number")?,
            title: r.get("title")?,
            html_url: r.get("html_url")?,
            state: r.get("state")?,
            private: r.get("private")?,
            payload: payload.map(|s| serde_json::from_str(&s).unwrap_or(Value::Null)),
            author_login: r.get("author_login")?,
            author_avatar_url: r.get("author_avatar_url")?,
            head_branch: r.get("head_branch")?,
            base_branch: r.get("base_branch")?,
            review_decision: r.get("review_decision")?,
            check_status: r.get("check_status")?,
            fetch_error: r.get("fetch_error")?,
            changed_files: r.get("changed_files")?,
            github_updated_at: r.get("github_updated_at")?,
            fetched_at: r.get("fetched_at")?,
            fetch_requested_at: r.get("fetch_requested_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row(
            "SELECT * FROM github_pull_requests WHERE id=?",
            [id],
            Self::from_row,
        )
        .optional()?
        .ok_or(campfire_db::Error::RecordNotFound("Github::PullRequest"))
    }
    pub fn for_message(conn: &Connection, message_id: i64) -> Result<Vec<Self>> {
        Ok(conn.prepare("SELECT p.* FROM github_pull_requests p JOIN github_pull_request_references r ON r.github_pull_request_id=p.id WHERE r.message_id=? ORDER BY p.owner,p.repo,p.number")?.query_map([message_id],Self::from_row)?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn for_reference(tx: &Tx<'_>, owner: &str, repo: &str, number: i64) -> Result<Self> {
        let (owner, repo) = (owner.to_lowercase(), repo.to_lowercase());
        if let Some(pr) = tx
            .conn()
            .query_row(
                "SELECT * FROM github_pull_requests WHERE owner=? AND repo=? AND number=?",
                params![owner, repo, number],
                Self::from_row,
            )
            .optional()?
        {
            return Ok(pr);
        }
        validate(tx.conn(), None, &owner, &repo, number, false)?.into_result()?;
        tx.conn().execute("INSERT INTO github_pull_requests (owner,repo,number,created_at,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(owner,repo,number) DO NOTHING",params![owner,repo,number,tx.now(),tx.now()])?;
        Ok(tx.conn().query_row(
            "SELECT * FROM github_pull_requests WHERE owner=? AND repo=? AND number=?",
            params![owner, repo, number],
            Self::from_row,
        )?)
    }
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
    pub fn display_full_name(&self) -> Result<String> {
        static NAME: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r"^[^/ \t\r\n\x0B\x0C]+/[^/ \t\r\n\x0B\x0C]+$").unwrap()
        });
        static URL: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r"github\.com/([^/ \t\r\n\x0B\x0C]+/[^/ \t\r\n\x0B\x0C]+)").unwrap()
        });
        let mut v = self.payload.as_ref();
        for key in ["base", "repo", "full_name"] {
            v = match v {
                None | Some(Value::Null) => None,
                Some(Value::Object(obj)) => obj.get(key),
                _ => return Err(campfire_db::Error::Other("Invalid PR payload dig".into())),
            };
        }
        if let Some(name) = v.and_then(Value::as_str).filter(|s| NAME.is_match(s)) {
            return Ok(name.into());
        }
        Ok(URL
            .captures(self.html_url.as_deref().unwrap_or(""))
            .map(|c| c[1].to_string())
            .filter(|s| !blank(s))
            .unwrap_or_else(|| self.full_name()))
    }
    pub fn stale(&self, now: Timestamp) -> bool {
        self.fetched_at.is_none_or(|at| at < now.ago(STALE_AFTER))
    }
    pub fn fetch_requested_recently(&self, now: Timestamp) -> bool {
        self.fetch_requested_at
            .is_some_and(|at| at >= now.ago(STALE_AFTER))
    }
    pub fn claim_fetch_request(&mut self, tx: &Tx<'_>) -> Result<bool> {
        if self.fetch_requested_recently(tx.now()) {
            return Ok(false);
        }
        let won=tx.conn().execute("UPDATE github_pull_requests SET fetch_requested_at=? WHERE id=? AND (fetch_requested_at IS NULL OR fetch_requested_at < ?)",params![tx.now(),self.id,tx.now().ago(STALE_AFTER)])?==1;
        if won {
            self.fetch_requested_at = Some(tx.now());
        }
        Ok(won)
    }
    pub fn release_fetch_request(&mut self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute(
            "UPDATE github_pull_requests SET fetch_requested_at=NULL WHERE id=?",
            [self.id],
        )?;
        self.fetch_requested_at = None;
        Ok(())
    }
    pub async fn visible_to(
        &self,
        db: &Database,
        accounts: &Accounts,
        user_id: Option<i64>,
    ) -> Result<bool> {
        if self.private == Some(false) {
            return Ok(true);
        }
        let Some(user_id) = user_id else {
            return Ok(false);
        };
        let Some(account) = db
            .read(move |conn| Account::for_user(conn, user_id))
            .await?
        else {
            return Ok(false);
        };
        accounts
            .can_read_repository(account.id, &self.owner, &self.repo)
            .await
    }
    pub async fn agent_payload(
        &self,
        db: &Database,
        accounts: &Accounts,
        agent_id: Option<i64>,
    ) -> Result<Value> {
        let owner = match agent_id {
            Some(id) => {
                db.read(move |conn| {
                    Ok(conn
                        .query_row("SELECT owner_id FROM agents WHERE id=?", [id], |r| {
                            r.get::<_, Option<i64>>(0)
                        })
                        .optional()?
                        .flatten())
                })
                .await?
            }
            None => None,
        };
        let details = self.visible_to(db, accounts, owner).await?;
        Ok(
            json!({"url":self.html_url.as_ref().filter(|s|!blank(s)).cloned().unwrap_or_else(||format!("https://github.com/{}/pull/{}",self.full_name(),self.number)),
            "owner":self.owner,"repo":self.repo,"number":self.number,"title":if details {self.title.as_ref()} else {None},"state":self.state,
            "head_branch":if details {self.head_branch.as_ref()} else {None},"base_branch":if details {self.base_branch.as_ref()} else {None},
            "review_decision":self.review_decision,"checks_state":self.check_status}),
        )
    }
    pub fn changed_files_summary(&self) -> Result<Value> {
        let parsed = self
            .changed_files
            .as_ref()
            .filter(|s| !blank(s))
            .and_then(|s| serde_json::from_str::<Value>(s).ok());
        let Some(Value::Object(parsed)) = parsed else {
            return Ok(json!({"files":[],"total_count":0}));
        };
        let files = match parsed.get("files") {
            None | Some(Value::Null) => vec![],
            Some(Value::Array(a)) => a.clone(),
            Some(Value::Object(o)) => o.iter().map(|(k, v)| json!([k, v])).collect(),
            Some(v) => vec![v.clone()],
        };
        let total = match parsed.get("total_count") {
            None | Some(Value::Null) => 0,
            Some(Value::Number(n)) => n
                .as_i64()
                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
            Some(Value::String(s)) => ruby_to_i(s),
            _ => {
                return Err(campfire_db::Error::Other(
                    "Invalid files total_count.to_i".into(),
                ));
            }
        };
        Ok(json!({"total_count":total.max(files.len() as i64),"files":files}))
    }
}

fn validate(
    conn: &Connection,
    id: Option<i64>,
    owner: &str,
    repo: &str,
    number: i64,
    unique: bool,
) -> Result<Errors> {
    let mut errors = Errors::default();
    if blank(owner) {
        errors.add("owner", "can't be blank");
    }
    if blank(repo) {
        errors.add("repo", "can't be blank");
    }
    if number <= 0 {
        errors.add("number", "must be greater than 0");
    }
    if unique&&conn.query_row("SELECT EXISTS(SELECT 1 FROM github_pull_requests WHERE owner=? AND repo=? AND number=? AND id!=COALESCE(?,0))",params![owner,repo,number,id],|r|r.get::<_,bool>(0))? {errors.add("number","has already been taken");}
    Ok(errors)
}

/// Whitelisted persisted attributes, shared by the fetcher and webhook privacy save.
/// Validation precedes every save; claim/release/collapse deliberately skip callbacks.
pub fn update(
    tx: &mut Tx<'_>,
    id: i64,
    attributes: &[(&'static str, SqlValue)],
) -> Result<PullRequest> {
    let original = PullRequest::find(tx.conn(), id)?;
    let string = |column: &str, fallback: &str| -> Result<String> {
        match attributes
            .iter()
            .find(|(key, _)| *key == column)
            .map(|(_, v)| v)
        {
            None => Ok(fallback.into()),
            Some(SqlValue::Text(v)) => Ok(v.to_lowercase()),
            Some(SqlValue::Null) => Ok(String::new()),
            _ => Err(campfire_db::Error::Other(
                "Invalid repository name type".into(),
            )),
        }
    };
    let owner = string("owner", &original.owner)?.to_lowercase();
    let repo = string("repo", &original.repo)?.to_lowercase();
    let number = match attributes
        .iter()
        .find(|(key, _)| *key == "number")
        .map(|(_, v)| v)
    {
        None => original.number,
        Some(SqlValue::Integer(n)) => *n,
        _ => return Err(campfire_db::Error::Other("Invalid PR integer type".into())),
    };
    validate(tx.conn(), Some(id), &owner, &repo, number, true)?.into_result()?;
    let mut attrs = Vec::new();
    for (column, value) in attributes {
        if ![
            "owner",
            "repo",
            "number",
            "title",
            "html_url",
            "state",
            "private",
            "payload",
            "author_login",
            "author_avatar_url",
            "base_branch",
            "head_branch",
            "head_sha",
            "review_decision",
            "check_status",
            "fetch_error",
            "fetched_at",
            "fetch_requested_at",
            "github_updated_at",
            "changed_files",
            "changed_files_fetched_at",
        ]
        .contains(column)
        {
            return Err(campfire_db::Error::Other("Unknown PR attribute".into()));
        }
        if !["owner", "repo", "number"].contains(column) {
            attrs.push((*column, value.clone()));
        }
    }
    attrs.extend([
        ("owner", SqlValue::Text(owner)),
        ("repo", SqlValue::Text(repo)),
        ("number", SqlValue::Integer(number)),
    ]);
    let changed =
        tx.conn()
            .query_row("SELECT * FROM github_pull_requests WHERE id=?", [id], |r| {
                for (column, value) in &attrs {
                    if r.get::<_, SqlValue>(*column)? != *value {
                        return Ok(true);
                    }
                }
                Ok(false)
            })?;
    if changed {
        attrs.push(("updated_at", SqlValue::Text(tx.now().to_db())));
    }
    let assignments = attrs
        .iter()
        .map(|(c, _)| format!("{c}=?"))
        .collect::<Vec<_>>()
        .join(",");
    let id_value = SqlValue::Integer(id);
    if changed {
        tx.conn().execute(
            &format!("UPDATE github_pull_requests SET {assignments} WHERE id=?"),
            rusqlite::params_from_iter(
                attrs
                    .iter()
                    .map(|(_, v)| v)
                    .chain(std::iter::once(&id_value)),
            ),
        )?;
    }
    // `_cards` requests stale rows after the PR save has committed. The bounded
    // enqueue runs on this writer in its own transaction, so queue failure releases
    // its claim and leaves the saved card available, exactly like the helper rescue.
    tx.after_commit(move |tx| {
        if let Err(error) = campfire_db::run_write(tx.conn(), tx.env(), |inner| {
            let ids=inner.conn().prepare("SELECT DISTINCT github_pull_request_id FROM github_pull_request_references WHERE message_id IN (SELECT message_id FROM github_pull_request_references WHERE github_pull_request_id=?)")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            request_refresh(inner, &ids)
        }) { tracing::warn!(pull_request_id=id, %error, "Skipping PR refresh enqueue"); }
        Ok(())
    });
    tx.broadcast_after_commit_once(&CardUpdated {
        pull_request_id: id,
    });
    PullRequest::find(tx.conn(), id)
}

pub fn request_refresh(tx: &mut Tx<'_>, ids: &[i64]) -> Result<()> {
    for id in ids {
        let mut pr = PullRequest::find(tx.conn(), *id)?;
        if pr.stale(tx.now()) && pr.claim_fetch_request(tx)? {
            tx.emit_after_commit(Event::job(&super::jobs::FetchPullRequestJob {
                pull_request_id: *id,
            }));
        }
    }
    Ok(())
}

pub fn collapse_case_duplicates(tx: &Tx<'_>) -> Result<()> {
    let rows = tx
        .conn()
        .prepare("SELECT id,owner,repo,number FROM github_pull_requests ORDER BY id")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut winners = std::collections::HashMap::new();
    for (id, owner, repo, number) in rows {
        let key = (owner.to_lowercase(), repo.to_lowercase(), number);
        if let Some(winner) = winners.get(&key).copied() {
            for (table, scope) in [
                ("github_pull_request_references", "message_id"),
                ("github_pull_request_threads", "room_id"),
            ] {
                tx.conn().execute(&format!("DELETE FROM {table} AS loser WHERE github_pull_request_id=? AND EXISTS(SELECT 1 FROM {table} winner WHERE winner.github_pull_request_id=? AND winner.{scope}=loser.{scope})"),params![id,winner])?;
                tx.conn().execute(
                    &format!(
                        "UPDATE {table} SET github_pull_request_id=? WHERE github_pull_request_id=?"
                    ),
                    params![winner, id],
                )?;
            }
            tx.conn()
                .execute("DELETE FROM github_pull_requests WHERE id=?", [id])?;
        } else {
            winners.insert(key, id);
        }
    }
    tx.conn().execute("UPDATE github_pull_requests SET owner=LOWER(owner),repo=LOWER(repo) WHERE owner!=LOWER(owner) OR repo!=LOWER(repo)",[])?;
    Ok(())
}

pub async fn payload_for_message(
    db: &Database,
    accounts: &Accounts,
    message: &Message,
    agent_id: Option<i64>,
) -> Result<Option<Value>> {
    let Some(thread) = message.thread_id else {
        return Ok(None);
    };
    let pr=db.read(move|conn|Ok(conn.query_row("SELECT p.* FROM github_pull_requests p JOIN github_pull_request_threads t ON t.github_pull_request_id=p.id WHERE t.channel_thread_id=?",[thread],PullRequest::from_row).optional()?)).await?;
    match pr {
        None => Ok(None),
        Some(pr) => Ok(Some(pr.agent_payload(db, accounts, agent_id).await?)),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardUpdated {
    pub pull_request_id: i64,
}
impl Broadcast for CardUpdated {
    const KIND: &'static str = "Github::PullRequest#broadcast_card_updates";
}

#[cfg(test)]
mod tests;

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
pub fn subscription_section(
    conn: &Connection,
    room: &campfire_db::Room,
    user: &campfire_db::User,
) -> Result<campfire_views::github::subscriptions::Section> {
    use campfire_views::github::subscriptions::{Section, Subscription};
    let can_administer = user.can_administer(Some(room.creator_id), false) && !room.direct();
    let subscriptions = if can_administer {
        crate::integrations::github::subscriptions::RepositorySubscription::for_room(conn, room.id)?
            .into_iter()
            .map(|s| Subscription {
                id: s.id,
                full_name: s.full_name(),
                events: s
                    .events
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(Section {
        room_id: room.id,
        can_administer,
        administrator: user.is_administrator(),
        subscriptions,
    })
}

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
        return Ok(Card {
            id: pr.id,
            owner: pr.owner.clone(),
            repo: pr.repo.clone(),
            number: pr.number,
            private: pr.private,
            ..Default::default()
        });
    }
    if files {
        card_with_files(conn, pr, room_id)
    } else {
        card(conn, pr, room_id)
    }
}
pub fn message_cards(conn: &Connection, app: &AppState, message: &Message) -> Result<String> {
    message_cards_in_zone(conn, app, message, &super::page::renderer_time_zone())
}
pub fn message_cards_in_zone(conn: &Connection, app: &AppState, message: &Message, zone: &campfire_views::time::Zone) -> Result<String> {
    let cards = PullRequest::for_message(conn, message.id)?
        .iter()
        .map(|pr| shared_card(conn, pr, message.room_id, false))
        .collect::<Result<Vec<_>>>()?;
    let account = Account::first(conn)?;
    Ok(super::page::render_detached_in_zone(app, account.as_ref(), "http://example.org", zone, |ctx| {
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

/// Public account display data for the owning profile/bot page. `usable?` may mark an
/// unreadable credential disconnected; do it outside a read closure and reload that reason.
pub async fn connection(
    app: &AppState,
    user_id: i64,
) -> Result<campfire_views::github::connections::Connection> {
    use crate::integrations::github::accounts::Account as GithubAccount;
    let account = app
        .db
        .read(move |conn| GithubAccount::for_user(conn, user_id))
        .await?;
    let usable = if let Some(account) = &account {
        app.github_accounts.usable(account.id).await?
    } else {
        false
    };
    let account = app
        .db
        .read(move |conn| GithubAccount::for_user(conn, user_id))
        .await?;
    Ok(campfire_views::github::connections::Connection {
        linked: account.is_some(),
        usable,
        login: account
            .as_ref()
            .map(|a| a.github_login.clone())
            .unwrap_or_default(),
        reason: account.as_ref().and_then(|a| a.disconnected_reason.clone()),
        app_token: account.as_ref().is_some_and(|a| a.token_source == "app"),
        app_configured: app.github_app.configured(),
    })
}

/// WS8b-m calls this after authorizing the thread's room. Empty for ordinary threads.
/// Uses the same file-bearing, private-safe card adapter as the registered cable callback.
pub fn thread_header(
    conn: &Connection,
    ctx: &campfire_views::ViewContext<'_>,
    thread: &campfire_db::ChannelThread,
) -> Result<String> {
    use rusqlite::OptionalExtension;
    let pr_id=conn.query_row("SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? AND room_id=?",rusqlite::params![thread.id,thread.room_id],|r|r.get::<_,i64>(0)).optional()?;
    let Some(id) = pr_id else {
        return Ok(String::new());
    };
    let pr = PullRequest::find(conn, id)?;
    let data = shared_card(conn, &pr, thread.room_id, true)?;
    Ok(campfire_views::github::thread_header(
        ctx,
        thread.room_id,
        thread.id,
        &data,
    ))
}

impl super::Presenter<'_> {
    /// The owned thread page calls WS15g's private-safe adapter after room authorization.
    /// Rendering records refresh intent; the caller enqueues after releasing this reader.
    pub fn github_thread_header(&self, thread: &campfire_db::ChannelThread) -> Result<campfire_views::helpers::Html> {
        use rusqlite::OptionalExtension;
        let id = self.conn.query_row(
            "SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? AND room_id=?",
            rusqlite::params![thread.id, thread.room_id], |r| r.get::<_, i64>(0),
        ).optional()?;
        if let Some(id) = id {
            let pr = PullRequest::find(self.conn, id)?;
            if pr.stale(campfire_db::Timestamp::from_jiff(self.now)) { self.remember_github_refresh(id); }
        }
        let base = self.cache_base_url.as_deref().unwrap_or("http://example.org");
        super::page::render_detached_in_zone(self.app, None, base, &self.render_zone, |ctx| thread_header(self.conn, ctx, thread))
            .map(campfire_views::helpers::raw)
    }
}

//! GitHub rows to session-independent card partial data.
use crate::app::AppState;
use crate::integrations::github::{
    pull_requests::PullRequest,
    threads::PullRequestThread,
};
use campfire_db::{Account, Connection, Message, Result};
use campfire_views::github::{Card, CardMessage};
pub fn message_cards_in_zone(
    conn: &Connection,
    app: &AppState,
    message: &Message,
    zone: &campfire_views::time::Zone,
) -> Result<String> {
    let cards = PullRequest::for_message(conn, message.id)?
        .iter()
        .map(|pr| shared_card(conn, pr, message.room_id, false))
        .collect::<Result<Vec<_>>>()?;
    let account = Account::first(conn)?;
    Ok(render_message_cards(
        app,
        message,
        zone,
        account.as_ref(),
        &cards,
    ))
}
/// One persisted-fact snapshot for a callback's entire reference set. Callback rendering
/// uses the detached renderer zone, just like the single-message owner adapter.
pub fn message_cards_for_messages(
    conn: &Connection,
    app: &AppState,
    messages: &[Message],
    account: Option<&Account>,
) -> Result<std::collections::HashMap<i64, String>> {
    let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
    let prs = PullRequest::for_messages(conn, &ids)?;
    let public: Vec<_> = messages
        .iter()
        .filter(|m| {
            prs.get(&m.id)
                .is_some_and(|cards| cards.iter().any(|pr| pr.private == Some(false)))
        })
        .map(|m| m.id)
        .collect();
    let discussions = PullRequestThread::for_messages(conn, &public)?;
    let zone = super::page::renderer_time_zone();
    messages
        .iter()
        .map(|message| {
            let cards = prs
                .get(&message.id)
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|pr| {
                    shared_card_in_discussion(
                        pr,
                        discussions.get(&(message.room_id, pr.id)).copied(),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((
                message.id,
                render_message_cards(app, message, &zone, account, &cards),
            ))
        })
        .collect()
}

pub(crate) fn render_message_cards(
    app: &AppState,
    message: &Message,
    zone: &campfire_views::time::Zone,
    account: Option<&Account>,
    cards: &[Card],
) -> String {
    super::page::render_detached_in_zone(app, account, "http://example.org", zone, |ctx| {
        campfire_views::github::cards(
            ctx,
            &message.client_message_id,
            &CardMessage {
                id: message.id,
                room_id: message.room_id,
                thread_id: message.thread_id,
            },
            cards,
        )
    })
}

/// Test adapter for the file-bearing, private-safe owner header.
/// Production uses the presenter and the bounded callback adapter.
#[cfg(any(test, feature = "test-support"))]
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
    thread_header_for_pull_request(conn, ctx, thread, &pr)
}

fn thread_header_for_pull_request(
    conn: &Connection,
    ctx: &campfire_views::ViewContext<'_>,
    thread: &campfire_db::ChannelThread,
    pr: &PullRequest,
) -> Result<String> {
    let data = shared_card(conn, pr, thread.room_id, true)?;
    Ok(campfire_views::github::thread_header(
        ctx,
        thread.room_id,
        thread.id,
        &data,
    ))
}

pub trait GithubRendering {
    fn github_thread_header(
        &self,
        thread: &campfire_db::ChannelThread,
    ) -> Result<campfire_views::helpers::Html>;
}
impl GithubRendering for super::Presenter<'_> {
    /// The owned thread page calls WS15g's private-safe adapter after room authorization.
    /// Rendering records refresh intent; the caller enqueues after releasing this reader.
    fn github_thread_header(
        &self,
        thread: &campfire_db::ChannelThread,
    ) -> Result<campfire_views::helpers::Html> {
        use rusqlite::OptionalExtension;
        let id = self.conn.query_row(
            "SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? AND room_id=?",
            rusqlite::params![thread.id, thread.room_id], |r| r.get::<_, i64>(0),
        ).optional()?;
        let pr = id.map(|id| PullRequest::find(self.conn, id)).transpose()?;
        if let Some(pr) = &pr
            && pr.stale(campfire_db::Timestamp::from_jiff(self.now))
        {
            self.remember_github_refresh(pr.id);
        }
        let base = self
            .cache_base_url
            .as_deref()
            .unwrap_or("http://example.org");
        super::page::render_detached_in_zone(self.app, None, base, &self.render_zone, |ctx| {
            match &pr {
                Some(pr) => thread_header_for_pull_request(self.conn, ctx, thread, pr),
                None => Ok(String::new()),
            }
        })
        .map(campfire_views::helpers::raw)
    }
}
pub use campfire_runtime::presenters::github::*;

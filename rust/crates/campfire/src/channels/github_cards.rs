//! Registered `PullRequest#broadcast_card_updates`, rendered with no current user/session.
use super::broadcasts::{Stream, message_dom_id, thread_dom_id};
use crate::{
    app::App,
    controllers::presenters::{github, page},
    integrations::github::pull_requests::{CardUpdated, PullRequest},
};
use campfire_cable::turbo::Action;
use campfire_db::{Account, ChannelThread, Room};
pub fn publish(app: &App, event: &CardUpdated) -> anyhow::Result<()> {
    let id = event.pull_request_id;
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        let pr = PullRequest::find(conn, id)?;
        use crate::integrations::message_batches::{self, Reference};
        let account = Account::first(conn)?;
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::GithubPullRequest(id), after)?;
            let room_ids: Vec<_> = messages
                .iter()
                .map(|m| m.room_id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let rooms: std::collections::HashMap<_, _> = Room::for_ids(conn, &room_ids)?
                .into_iter()
                .map(|r| (r.id, r))
                .collect();
            let cards =
                github::message_cards_for_messages(conn, &app, &messages, account.as_ref())?;
            for message in &messages {
                let room = rooms
                    .get(&message.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                let html = cards
                    .get(&message.id)
                    .expect("all referencing messages rendered");
                app.broadcasts.turbo(
                    &Stream::conversation(room, message),
                    Action::Replace,
                    &message_dom_id(message, Some("github_pr_cards")),
                    Some(html),
                    true,
                );
            }
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
        }
        publish_headers(conn, &app, &pr, account.as_ref())?;
        Ok(())
    })?;
    Ok(())
}

fn publish_headers(
    conn: &campfire_db::Connection,
    app: &App,
    pr: &PullRequest,
    account: Option<&Account>,
) -> campfire_db::Result<()> {
    use crate::integrations::message_batches;
    let id = pr.id;
    // Rails finds mappings in id order and preloads their threads. Bound
    // that association snapshot too, preserving ordering and orphan skips.
    let mut after = None;
    let mut header_card = None;
    loop {
        let mappings = conn.prepare("SELECT id,channel_thread_id,room_id FROM github_pull_request_threads WHERE github_pull_request_id=?1 AND (?2 IS NULL OR id>?2) ORDER BY id LIMIT ?3")?
            .query_map(rusqlite::params![id,after,message_batches::SIZE],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let ids: Vec<_> = mappings.iter().map(|m| m.1).collect();
        let threads: std::collections::HashMap<_, _> = ChannelThread::for_ids(conn, &ids)?
            .into_iter()
            .map(|t| (t.id, t))
            .collect();
        for &(_, id, room_id) in &mappings {
            let Some(thread) = threads.get(&id) else {
                continue;
            };
            let html = if thread.room_id == room_id {
                if header_card.is_none() {
                    header_card = Some(github::thread_header_card(&pr)?);
                }
                page::render_detached(&app, account, |ctx| {
                    campfire_views::github::thread_header(
                        ctx,
                        room_id,
                        id,
                        header_card.as_ref().expect("header card loaded"),
                    )
                })
            } else {
                String::new()
            };
            app.broadcasts.turbo(
                &Stream::thread_messages(id),
                Action::Replace,
                &thread_dom_id(id, "github_pr_header"),
                Some(&html),
                true,
            );
        }
        if mappings.len() < message_batches::SIZE {
            break;
        }
        after = mappings.last().map(|m| m.0);
    }
    Ok(())
}

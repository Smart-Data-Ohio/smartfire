//! Registered `PullRequest#broadcast_card_updates`, rendered with no current user/session.
use super::broadcasts::{Stream, message_dom_id, thread_dom_id};
use crate::{
    app::App,
    controllers::presenters::{github, page},
    integrations::github::pull_requests::{CardUpdated, PullRequest},
};
use campfire_cable::turbo::Action;
use campfire_db::{Account, ChannelThread, Message, Room};
pub fn publish(app: &App, event: &CardUpdated) -> anyhow::Result<()> {
    let id = event.pull_request_id;
    let app = app.clone();
    app.db.clone().read_blocking(move|conn| {
        PullRequest::find(conn,id)?;
        let ids=conn.prepare("SELECT id FROM messages WHERE id IN (SELECT message_id FROM github_pull_request_references WHERE github_pull_request_id=?) ORDER BY id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut messages = Message::for_ids(conn, &ids)?;
        messages.sort_by_key(|m| m.id);
        let room_ids: Vec<_> = messages.iter().map(|m| m.room_id).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let rooms: std::collections::HashMap<_, _> = Room::for_ids(conn, &room_ids)?.into_iter().map(|r| (r.id, r)).collect();
        let account=Account::first(conn)?;
        let cards = github::message_cards_for_messages(conn, &app, &messages, account.as_ref())?;
        for message in &messages {
            let room = rooms.get(&message.room_id).ok_or(campfire_db::Error::RecordNotFound("Room"))?;
            let html = cards.get(&message.id).expect("all referencing messages rendered");
            app.broadcasts.turbo(&Stream::conversation(room,message),Action::Replace,&message_dom_id(message,Some("github_pr_cards")),Some(html),true);
        }
        let ids=conn.prepare("SELECT channel_thread_id FROM github_pull_request_threads WHERE github_pull_request_id=? ORDER BY id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for id in ids {
            if let Some(thread)=ChannelThread::find_by_id(conn,id)? {
                let html=page::render_detached(&app,account.as_ref(),|ctx|github::thread_header(conn,ctx,&thread))?;
                app.broadcasts.turbo(&Stream::thread_messages(id),Action::Replace,&thread_dom_id(id,"github_pr_header"),Some(&html),true);
            }
        }
        Ok(())
    })?;
    Ok(())
}

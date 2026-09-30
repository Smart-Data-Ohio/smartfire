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
        let pr=PullRequest::find(conn,id)?;
        let ids=conn.prepare("SELECT id FROM messages WHERE id IN (SELECT message_id FROM github_pull_request_references WHERE github_pull_request_id=?) ORDER BY id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for id in ids {
            let message=Message::find(conn,id)?;let room=Room::find(conn,message.room_id)?;
            let html=github::message_cards(conn,&app,&message)?;
            app.broadcasts.turbo(&Stream::conversation(&room,&message),Action::Replace,&message_dom_id(&message,Some("github_pr_cards")),Some(&html),true);
        }
        let ids=conn.prepare("SELECT channel_thread_id FROM github_pull_request_threads WHERE github_pull_request_id=? ORDER BY id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let account=Account::first(conn)?;
        for id in ids {
            if let Some(thread)=ChannelThread::find_by_id(conn,id)? {
                let card=github::card(conn,&pr,thread.room_id)?;
                let html=page::render_detached(&app,account.as_ref(),|ctx|campfire_views::github::thread_header(ctx,thread.room_id,thread.id,&card));
                app.broadcasts.turbo(&Stream::thread_messages(id),Action::Replace,&thread_dom_id(id,"github_pr_header"),Some(&html),true);
            }
        }
        Ok(())
    })?;
    Ok(())
}

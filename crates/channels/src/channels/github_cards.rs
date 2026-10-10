//! PR card updates and discussion headers publish independently of classic rendering.
use crate::{app::App, integrations::github::pull_requests::CardUpdated};
use campfire_db::ChannelThread;

pub fn publish(app: &App, event: &CardUpdated) -> anyhow::Result<()> {
    let id = event.pull_request_id;
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::GithubPullRequest(id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE { break; }
            after = messages.last().map(|message| message.id);
        }
        let mut after = None;
        loop {
            let mappings = conn.prepare("SELECT id,channel_thread_id,room_id FROM github_pull_request_threads WHERE github_pull_request_id=?1 AND (?2 IS NULL OR id>?2) ORDER BY id LIMIT ?3")?
                .query_map(rusqlite::params![id, after, message_batches::SIZE], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let ids = mappings.iter().map(|mapping| mapping.1).collect::<Vec<_>>();
            let threads = ChannelThread::for_ids(conn, &ids)?.into_iter().map(|thread| (thread.id, thread.room_id)).collect::<std::collections::HashMap<_, _>>();
            for &(_, thread_id, room_id) in &mappings {
                if threads.get(&thread_id) == Some(&room_id) {
                    crate::cable::sync::publish(&app.cable,
                        campfire_cable::sync::Audience::Topic(crate::cable::sync::thread_topic(thread_id)),
                        &campfire_api_types::SyncPayload::ThreadGithubUpdated(campfire_api_types::ThreadGithubUpdated { room_id, thread_id, pull_request_id: id }));
                    app.broadcasts.thread_updated(thread_id);
                }
            }
            if mappings.len() < message_batches::SIZE { break; }
            after = mappings.last().map(|mapping| mapping.0);
        }
        Ok(())
    })?;
    Ok(())
}

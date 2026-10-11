use crate::app::App;
use crate::queue::Registry;
use campfire_db::models::announcement;
use campfire_jobs::{Execution, JobKind, JobResult, Outcome};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Sync(announcement::SyncJob);
impl campfire_db::Job for Sync {
    const CLASS: &'static str = "Announcement::SyncJob";
}
impl JobKind for Sync {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Delete(announcement::DeleteCopiesJob);
impl campfire_db::Job for Delete {
    const CLASS: &'static str = "Announcement::DeleteCopiesJob";
}
impl JobKind for Delete {}

pub(super) fn register(registry: &mut Registry) {
    registry.register(sync);
    registry.register(delete);
}

async fn sync(app: App, job: Sync, _: Execution) -> JobResult {
    deliver(app, job.0.publication_id).await?;
    Ok(Outcome::Done)
}

pub(crate) async fn deliver(app: App, publication: i64) -> campfire_db::Result<()> {
    app.db
        .write(move |tx| {
            for (message, created) in announcement::deliver(tx, publication)? {
                if created {
                    campfire_db::models::agent_posting::broadcast_create(tx, &message)?;
                } else {
                    tx.emit_after_commit(campfire_db::Event::broadcast(
                        &campfire_db::broadcasts::Broadcast::MessageUpdated {
                            message_id: message.id,
                        },
                    ));
                }
            }
            Ok(())
        })
        .await
}

async fn delete(app: App, job: Delete, _: Execution) -> JobResult {
    delete_delivered(app, job.0.publication_id, job.0.message_ids).await?;
    Ok(Outcome::Done)
}

pub(crate) async fn delete_delivered(
    app: App,
    publication: i64,
    ids: Vec<i64>,
) -> campfire_db::Result<()> {
    let removed = app
        .db
        .write(move |tx| announcement::delete_copies(tx, publication, &ids))
        .await?;
    for (message, parent_thread) in removed {
        let runtime = app.clone();
        app.db
            .read(move |conn| {
                if let Some(room) = campfire_db::Room::find_by_id(conn, message.room_id)? {
                    runtime.broadcasts.message_remove(&room, &message);
                    if let Some(thread_id) = parent_thread {
                        for membership in campfire_db::Membership::for_room(conn, room.id)? {
                            runtime.broadcasts.thread_refresh(membership.user_id, room.id, thread_id);
                        }
                    }
                }
                Ok(())
            })
            .await?;
    }
    Ok(())
}

//! Messaging maintenance workers. Keep registration additive for other domain owners.
use super::Registry;
use crate::app::App;
use campfire_db::models::{retention, room_delete};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Destroy(room_delete::DestroyJob);
impl campfire_db::Job for Destroy {
    const CLASS: &'static str = "Room::DestroyJob";
}
impl JobKind for Destroy {}
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Prune(retention::PruneJob);
impl campfire_db::Job for Prune {
    const CLASS: &'static str = "Retention::PruneJob";
}
impl JobKind for Prune {}
pub(super) fn register(registry: &mut Registry) {
    registry.register(destroy);
    registry.register(prune);
}
async fn destroy(app: App, job: Destroy, _: Execution) -> JobResult {
    room_delete::perform(&app.db, job.0.room_id).await?;
    Ok(Outcome::Done)
}
async fn prune(app: App, _: Prune, _: Execution) -> JobResult {
    retention::perform(&app.db).await?;
    Ok(Outcome::Done)
}

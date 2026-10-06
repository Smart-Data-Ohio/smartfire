//! Calendar::EntrySync: reserve deterministic copies, then converge through Google's API.
use super::{
    api::{self, ApiRequest},
    calendar,
};
use crate::{app::App, queue::Registry};
use campfire_db::{
    Timestamp, User,
    models::{
        google_account::GoogleAccount, google_calendar::SyncEntryJob, google_entry as entries,
    },
};
use campfire_jobs::{Execution, JobKind, JobResult, RetryPolicy};
use hyper::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Sync(SyncEntryJob);
impl campfire_db::Job for Sync {
    const CLASS: &'static str = "Calendar::SyncEntryJob";
}
impl JobKind for Sync {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().attempts(8).retry_on(|e| {
            e.downcast_ref::<api::Error>()
                .is_some_and(api::Error::unavailable)
        })
    }
}
pub fn register(registry: &mut Registry) {
    registry.register(perform);
}
async fn perform(app: App, job: Sync, execution: Execution) -> JobResult {
    calendar::job_result(sync(&app, job.0.event_id, job.0.user_id).await, &execution)
}
pub async fn sync(app: &App, event_id: i64, user_id: i64) -> api::Result<()> {
    let event = app
        .db
        .read(move |conn| {
            if User::find_by_id(conn, user_id)?.is_none() {
                return Ok(None);
            }
            entries::event(conn, event_id)
        })
        .await?;
    let Some(event) = event else { return Ok(()) };
    let usable = calendar::usable(app, user_id).await?;
    let room_id = event.room_id;
    let desired = usable
        && event.cancelled_at.is_none()
        && app
            .db
            .read(move |c| entries::response_is_notifying(c, event_id, user_id, room_id))
            .await?;
    if desired {
        let entry = app
            .db
            .write(move |tx| entries::reserve(tx, event_id, user_id))
            .await?;
        let payload = entries::payload(&event, app.config.mail.app_url.as_deref());
        let path = format!("{}/{}", api::EVENTS, entry.google_event_id);
        let result = if entry.synced_at.is_none() {
            let insert = with_id(&payload, &entry.google_event_id);
            match call(app, user_id, Method::POST, api::EVENTS, Some(&insert)).await {
                Err(api::Error::Conflict(_)) => {
                    let mut confirmed = payload.clone();
                    confirmed["status"] = json!("confirmed");
                    call(app, user_id, Method::PUT, &path, Some(&confirmed)).await
                }
                result => result,
            }
        } else {
            match call(app, user_id, Method::PUT, &path, Some(&payload)).await {
                Err(api::Error::NotFound(_)) => {
                    call(
                        app,
                        user_id,
                        Method::POST,
                        api::EVENTS,
                        Some(&with_id(&payload, &entry.google_event_id)),
                    )
                    .await
                }
                result => result,
            }
        };
        match result {
            Ok(_) => {
                app.db.write(move |tx| entries::success(tx, &entry)).await?;
            }
            Err(error) => {
                let retry = error.unavailable();
                let connected = app
                    .db
                    .read(move |c| {
                        Ok(GoogleAccount::for_user(c, user_id)?.is_some_and(|a| a.connected()))
                    })
                    .await?;
                let summary = summary(&error);
                app.db
                    .write(move |tx| {
                        if retry || connected {
                            entries::failure(tx, &entry, &summary)
                        } else {
                            entries::delete(tx, &entry)
                        }
                    })
                    .await?;
                tracing::warn!(
                    event_id,
                    user_id,
                    "Calendar::EntrySync failed: {}",
                    error.class()
                );
                if retry {
                    return Err(error);
                }
                if let api::Error::Storage(e) = error {
                    return Err(e.into());
                }
            }
        }
    } else if let Some(entry) = app
        .db
        .read(move |c| entries::find(c, event_id, user_id))
        .await?
    {
        if usable {
            let path = format!("{}/{}", api::EVENTS, entry.google_event_id);
            match call(app, user_id, Method::DELETE, &path, None).await {
                Ok(_) | Err(api::Error::NotFound(_)) => {}
                Err(error) => {
                    let summary = summary(&error);
                    app.db
                        .write(move |tx| entries::failure(tx, &entry, &summary))
                        .await?;
                    tracing::warn!(
                        event_id,
                        user_id,
                        "Calendar::EntrySync delete failed: {}",
                        error.class()
                    );
                    if error.unavailable() {
                        return Err(error);
                    }
                    if let api::Error::Storage(e) = error {
                        return Err(e.into());
                    }
                    return Ok(());
                }
            }
        }
        app.db.write(move |tx| entries::delete(tx, &entry)).await?;
    }
    Ok(())
}
fn with_id(payload: &Value, id: &str) -> Value {
    let mut value = payload.clone();
    value["id"] = json!(id);
    value
}
async fn call(
    app: &App,
    user_id: i64,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> api::Result<Value> {
    app.google
        .api()
        .request(
            &app.db,
            &app.secrets,
            user_id,
            ApiRequest::calendar(method, path, payload),
            Timestamp::from_jiff(app.clock.now()),
        )
        .await
}
fn summary(error: &api::Error) -> String {
    campfire_richtext::ruby::truncate(
        &format!("{}: {}", error.class().rsplit("::").next().unwrap(), error),
        250,
        "...",
    )
}

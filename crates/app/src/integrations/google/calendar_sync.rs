//! Rails Calendar::InboundSync and Calendar::MeetLink. Event writes belong to WS14e.
use super::{
    api::{self, ApiRequest},
    calendar, entry_sync,
};
use crate::{app::App, queue::Registry};
use campfire_db::{
    CalendarEvent, Timestamp, User,
    models::{
        google_calendar::{InboundSyncJob, MeetLinkJob},
        google_entry,
    },
};
use campfire_jobs::{Execution, JobKind, JobResult, RetryPolicy};
use hyper::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

macro_rules! consumer {
    ($name:ident, $args:ty, $class:literal) => {
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct $name($args);
        impl campfire_db::Job for $name {
            const CLASS: &'static str = $class;
        }
        impl JobKind for $name {
            fn retry_policy() -> RetryPolicy {
                RetryPolicy::application_job()
                    .attempts(8)
                    .retry_on(|error| {
                        error
                            .downcast_ref::<api::Error>()
                            .is_some_and(api::Error::unavailable)
                    })
            }
        }
    };
}
consumer!(Inbound, InboundSyncJob, "Calendar::InboundSyncJob");
consumer!(Meet, MeetLinkJob, "Calendar::MeetLinkJob");

pub fn register(registry: &mut Registry) {
    registry.register(inbound_job);
    registry.register(meet_job);
}
async fn inbound_job(app: App, job: Inbound, execution: Execution) -> JobResult {
    calendar::job_result(inbound(&app, job.0.0.0).await, &execution)
}
async fn meet_job(app: App, job: Meet, execution: Execution) -> JobResult {
    calendar::job_result(meet(&app, job.0.event_id).await, &execution)
}
fn now(app: &App) -> Timestamp {
    Timestamp::from_jiff(app.clock.now())
}

pub async fn inbound(app: &App, user_id: i64) -> api::Result<()> {
    let Some(user) = app.db.read(move |c| User::find_by_id(c, user_id)).await? else {
        return Ok(());
    };
    if !calendar::usable(app, user_id).await? {
        return Ok(());
    }
    let api = app.google.api();
    let mut credentials = api.credentials(&app.db, &app.secrets, user_id).await?;
    let at = now(app);
    // find_each ignores the relation's event_id order and batches by entry primary key.
    let entries = app
        .db
        .read(move |c| google_entry::inbound_entries(c, user_id, at))
        .await?;
    for (entry, room_id) in entries {
        let eligible = user.is_active()
            && !user.is_bot()
            && app
                .db
                .read(move |c| {
                    Ok(c.query_row(
                        "SELECT EXISTS(SELECT 1 FROM memberships WHERE room_id=? AND user_id=?)",
                        [room_id, user_id],
                        |r| r.get::<_, bool>(0),
                    )?)
                })
                .await?;
        if !eligible {
            continue;
        }
        let event_id = entry.event_id;
        let local = app
            .db
            .read(move |c| {
                Ok(c.query_row(
                    "SELECT response FROM event_attendances WHERE event_id=? AND user_id=?",
                    [event_id, user_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()?)
            })
            .await?;
        let path = format!("{}/{}", api::EVENTS, entry.google_event_id);
        let remote = api
            .request_with(
                &mut credentials,
                &app.db,
                &app.secrets,
                ApiRequest::calendar(Method::GET, &path, None),
                now(app),
            )
            .await;
        let declined = match remote {
            Ok(value) => value
                .as_object()
                .is_some_and(|h| h.get("status") == Some(&json!("cancelled"))),
            Err(api::Error::NotFound(_)) => true,
            Err(error) if error.unavailable() => return Err(error),
            Err(error @ api::Error::Unauthorized(_)) => {
                let summary = campfire_richtext::ruby::truncate(
                    &format!("Unauthorized: {error}"),
                    250,
                    "...",
                );
                app.db
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE calendar_push_channels SET last_error=? WHERE user_id=?",
                            rusqlite::params![summary, user_id],
                        )?;
                        Ok(())
                    })
                    .await?;
                tracing::warn!(user_id, "Calendar::InboundSync failed: {}", error.class());
                return Ok(());
            }
            Err(api::Error::Storage(error)) => return Err(error.into()),
            Err(error) => {
                tracing::warn!(
                    event_id,
                    "Calendar::InboundSync entry failed: {}",
                    error.class()
                );
                continue;
            }
        };
        if declined && matches!(local.as_deref(), Some("going" | "maybe")) {
            // The owner validates and copies head responses to followers. Callback queue
            // rows share this writer; publication only happens after its COMMIT.
            app.db
                .write(move |tx| CalendarEvent::respond(tx, event_id, user_id, "declined", false))
                .await?;
        }
    }
    Ok(())
}

pub async fn meet(app: &App, event_id: i64) -> api::Result<()> {
    let event = match app.db.read(move |c| CalendarEvent::find(c, event_id)).await {
        Ok(event) => event,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !event.needs_meet_link() || !calendar::usable(app, event.organizer_id).await? {
        return Ok(());
    }
    let api = app.google.api();
    // Rails retains the organizer account loaded before EntrySync, including its token snapshot.
    let mut credentials = api
        .credentials(&app.db, &app.secrets, event.organizer_id)
        .await?;
    entry_sync::sync(app, event_id, event.organizer_id).await?;
    let user_id = event.organizer_id;
    let entry = app
        .db
        .read(move |c| {
            Ok(google_entry::find(c, event_id, user_id)?.filter(|entry| entry.synced_at.is_some()))
        })
        .await?;
    let Some(entry) = entry else { return Ok(()) };
    let error = app
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT last_error FROM event_calendar_entries WHERE id=?",
                [entry.id],
                |r| r.get::<_, Option<String>>(0),
            )?)
        })
        .await?;
    if error.as_deref().is_some_and(|e| !api::blank(e)) {
        return Ok(());
    }
    let path = format!(
        "{}/{}?conferenceDataVersion=1",
        api::EVENTS,
        entry.google_event_id
    );
    let payload = json!({"conferenceData":{"createRequest":{"requestId":format!("meet-{event_id}-{}",entry.id)}}});
    let response = match api
        .request_with(
            &mut credentials,
            &app.db,
            &app.secrets,
            ApiRequest::calendar(Method::PATCH, &path, Some(&payload)),
            now(app),
        )
        .await
    {
        Ok(value) => value,
        Err(error) if error.unavailable() => return Err(error),
        Err(api::Error::Storage(error)) => return Err(error.into()),
        Err(error) => {
            tracing::warn!(event_id, "Calendar::MeetLink failed: {}", error.class());
            return Ok(());
        }
    };
    if let Some(link) = response
        .as_object()
        .and_then(|h| h.get("hangoutLink"))
        .filter(|v| !ruby_blank(v))
    {
        let link = campfire_richtext::ruby::json_value_to_s(link);
        app.db
            .write(move |tx| CalendarEvent::save_meet_link(tx, event_id, Some(link)))
            .await?;
    } else if response.as_object().is_some() {
        let conference = response.get("conferenceData").unwrap_or(&Value::Null);
        let request = dig(conference, "createRequest")?;
        if dig(request, "status")? == &json!("pending") {
            return Err(api::Error::Unavailable(
                "Google Calendar conference still pending".into(),
            ));
        }
    }
    Ok(())
}
fn ruby_blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => api::blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(h) => h.is_empty(),
        _ => false,
    }
}
fn dig<'a>(value: &'a Value, key: &str) -> api::Result<&'a Value> {
    match value {
        Value::Null => Ok(&Value::Null),
        Value::Object(h) => Ok(h.get(key).unwrap_or(&Value::Null)),
        _ => Err(campfire_db::Error::Other(
            "Rails conference data does not support dig with a named key".into(),
        )
        .into()),
    }
}
use rusqlite::OptionalExtension;

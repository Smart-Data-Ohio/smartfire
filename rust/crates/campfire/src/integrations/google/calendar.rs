//! Calendar consumers. Arguments come from Rails; SyncEntry/MeetLink are produced by WS14e.
use super::api::{self, ApiRequest, Credentials};
use crate::{app::App, jobs::Registry};
use campfire_db::{
    Timestamp,
    models::{
        google_account::GoogleAccount,
        google_calendar::{DisconnectCleanupJob, WatchChannelJob},
        room_delete::RemoteDeleteJob,
    },
};
use campfire_jobs::{Execution, JobError, JobKind, JobResult, Outcome, RetryPolicy};
use hyper::Method;
use rails_compat::{
    ar_encryption::ArEncryption,
    calendar_credentials::{self, Snapshot},
};
use rand::RngCore;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
macro_rules! calendar_job {
    ($name:ident,$args:ty,$class:literal) => {
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
calendar_job!(
    Cleanup,
    DisconnectCleanupJob,
    "Calendar::DisconnectCleanupJob"
);
calendar_job!(Remote, RemoteDeleteJob, "Calendar::RemoteDeleteJob");
calendar_job!(Watch, WatchChannelJob, "Calendar::WatchChannelJob");
pub fn register(registry: &mut Registry) {
    registry.register(cleanup_job);
    registry.on_exhausted::<Cleanup, _>(|app, job, error| {
        let class = error
            .downcast_ref::<api::Error>()
            .map_or("Google::Client::Unavailable", api::Error::class);
        tracing::error!(
            error_class = class,
            "Calendar::DisconnectCleanupJob failed after retries"
        );
        app.errors.report(
            error,
            class,
            serde_json::json!({"job":"Calendar::DisconnectCleanupJob","account_id":job.0.0.2}),
        );
    });
    registry.register(remote_job);
    registry.register(watch_job);
}
fn now(app: &App) -> Timestamp {
    Timestamp::from_jiff(app.clock.now())
}
pub async fn usable(app: &App, user_id: i64) -> api::Result<bool> {
    let enc = ArEncryption::new(&app.secrets);
    Ok(app
        .db
        .write(move |tx| {
            let Some(mut a) = GoogleAccount::for_user(tx.conn(), user_id)? else {
                return Ok(false);
            };
            Ok(a.usable(tx, &enc)? && a.calendar())
        })
        .await?)
}
async fn cleanup_job(app: App, job: Cleanup, execution: Execution) -> JobResult {
    let (ids, blob, id) = job.0.0;
    finish_cleanup(cleanup(&app, ids, blob, id).await, &execution, id)
}
/// Rails' retry_on block reports exhausted cleanup failures and completes the job.
pub(crate) fn finish_cleanup(
    result: api::Result<()>,
    execution: &Execution,
    _account_id: Option<i64>,
) -> JobResult {
    let mut result = job_result(result, execution);
    if let Err(JobError::RetryGroup {
        key: GOOGLE_RETRY,
        discard_exhausted,
        ..
    }) = &mut result
    {
        *discard_exhausted = true;
    }
    result
}

pub(crate) const GOOGLE_RETRY: &str = "[Google::Client::Unavailable]";
pub(crate) const INHERITED_RETRY: &str = "[Timeout::Error, Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout, ActiveRecord::Deadlocked, ActiveRecord::StatementTimeout, SQLite3::BusyException]";

/// Rails counts each retry_on handler separately, including inherited handlers.
pub(crate) fn job_result(result: api::Result<()>, execution: &Execution) -> JobResult {
    match result {
        Err(error) if error.unavailable() => Err(JobError::retry_group(error, GOOGLE_RETRY, 8)),
        result => application_result(result, execution),
    }
}

pub(crate) fn application_result(result: api::Result<()>, _execution: &Execution) -> JobResult {
    match result {
        Ok(()) => Ok(Outcome::Done),
        Err(error) => {
            let error = match error {
                api::Error::Storage(e) => anyhow::Error::new(e),
                e => anyhow::Error::new(e),
            };
            if (RetryPolicy::application_job().retry_on)(&error) {
                Err(JobError::retry_group(error, INHERITED_RETRY, 5))
            } else {
                Err(error.into())
            }
        }
    }
}

pub async fn cleanup(
    app: &App,
    ids: Vec<String>,
    blob: Value,
    account_id: Option<i64>,
) -> api::Result<()> {
    let snapshot = if let Some(s) = blob.as_str() {
        calendar_credentials::decrypt_snapshot(&app.secrets, s, app.clock.now())
    } else {
        blob.as_object()
            .filter(|h| !h.is_empty())
            .map(|hash| Snapshot {
                access_token: hash
                    .get("access_token")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                refresh_token: hash
                    .get("refresh_token")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                access_token_expires_at: hash
                    .get("access_token_expires_at")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse().ok()),
            })
    };
    let Some(snapshot) = snapshot else {
        tracing::warn!(
            ?account_id,
            "Calendar::DisconnectCleanupJob skipped cleanup: credentials expired or unreadable"
        );
        return Ok(());
    };
    let mut credentials = Credentials::snapshot(snapshot);
    let api = app.google.api();
    for id in ids {
        let path = format!("{}/{id}", api::EVENTS);
        match api
            .request_with(
                &mut credentials,
                &app.db,
                &app.secrets,
                ApiRequest::calendar(Method::DELETE, &path, None),
                now(app),
            )
            .await
        {
            Ok(_) | Err(api::Error::NotFound(_) | api::Error::Unauthorized(_)) => {}
            Err(e) if e.unavailable() => return Err(e),
            Err(api::Error::Storage(e)) => return Err(e.into()),
            Err(e) => tracing::warn!(
                "Calendar::DisconnectCleanupJob could not remove {}: {}",
                id,
                e.class()
            ),
        }
    }
    match api
        .revoke_token(credentials.tokens.refresh_token.as_deref())
        .await
    {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!("Calendar::DisconnectCleanupJob could not revoke the grant: rejected")
        }
        Err(e) if e.unavailable() => return Err(e),
        Err(e) => tracing::warn!(
            "Calendar::DisconnectCleanupJob could not revoke the grant: {}",
            e.class()
        ),
    }
    Ok(())
}
async fn remote_job(app: App, job: Remote, execution: Execution) -> JobResult {
    let (user_id, id) = job.0.0;
    job_result(remote_delete(&app, user_id, &id).await, &execution)
}
pub async fn remote_delete(app: &App, user_id: i64, id: &str) -> api::Result<()> {
    if !usable(app, user_id).await? {
        return Ok(());
    }
    let path = format!("{}/{id}", api::EVENTS);
    match app
        .google
        .api()
        .request(
            &app.db,
            &app.secrets,
            user_id,
            ApiRequest::calendar(Method::DELETE, &path, None),
            now(app),
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.unavailable() => Err(e),
        Err(api::Error::Storage(e)) => Err(e.into()),
        Err(e) => {
            tracing::warn!(
                "Calendar::RemoteDeleteJob could not remove {} for user {}: {}",
                id,
                user_id,
                e.class()
            );
            Ok(())
        }
    }
}
async fn watch_job(app: App, job: Watch, execution: Execution) -> JobResult {
    job_result(watch(&app, job.0.0.0).await, &execution)
}
pub async fn watch(app: &App, user_id: i64) -> api::Result<()> {
    let api = app.google.api();
    let Some(_) = api
        .config
        .webhook_url
        .as_ref()
        .filter(|_| api.config.configured())
    else {
        return Ok(());
    };
    let account = app
        .db
        .read(move |conn| GoogleAccount::for_user(conn, user_id))
        .await?;
    watch_with_account(app, user_id, account).await
}
async fn usable_account(
    app: &App,
    mut account: GoogleAccount,
) -> api::Result<(GoogleAccount, bool)> {
    let enc = ArEncryption::new(&app.secrets);
    Ok(app
        .db
        .write(move |tx| {
            let usable = account.usable(tx, &enc)? && account.calendar();
            Ok((account, usable))
        })
        .await?)
}
async fn watch_with_account(
    app: &App,
    user_id: i64,
    account: Option<GoogleAccount>,
) -> api::Result<()> {
    let api = app.google.api();
    let Some(address) = api
        .config
        .webhook_url
        .as_ref()
        .filter(|_| api.config.configured())
    else {
        return Ok(());
    };
    let Some(account) = account else {
        return Ok(());
    };
    let (account, usable) = usable_account(app, account).await?;
    if !usable {
        return Ok(());
    }
    let mut credentials = api
        .credentials_from_account(&app.db, &app.secrets, account)
        .await?;
    let old = app
        .db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT id,channel_id,resource_id FROM calendar_push_channels WHERE user_id=?",
                    [user_id],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()?)
        })
        .await?;
    let channel_id = uuid::Uuid::new_v4().to_string();
    let mut bytes = [0; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let payload = json!({"id":channel_id,"type":"web_hook","address":address,"token":token});
    let response = match api
        .request_with(
            &mut credentials,
            &app.db,
            &app.secrets,
            ApiRequest::calendar(
                Method::POST,
                "/calendar/v3/calendars/primary/events/watch",
                Some(&payload),
            ),
            now(app),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if e.unavailable() => return Err(e),
        Err(api::Error::Storage(e)) => return Err(e.into()),
        Err(e) => {
            if let Some((id, _, _)) = old {
                let summary = summary(&e);
                app.db
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE calendar_push_channels SET last_error=? WHERE id=?",
                            rusqlite::params![summary, id],
                        )?;
                        Ok(())
                    })
                    .await?;
            }
            return Ok(());
        }
    };
    if let Some((_, old_channel, Some(resource_id))) = old
        && !api::blank(&resource_id)
    {
        let payload = json!({"id":old_channel,"resourceId":resource_id});
        let _ = api
            .request(
                &app.db,
                &app.secrets,
                user_id,
                ApiRequest::calendar(Method::POST, "/calendar/v3/channels/stop", Some(&payload)),
                now(app),
            )
            .await;
    }
    let digest = campfire_db::models::google_calendar::PushChannel::digest(&token);
    let resource = response["resourceId"].as_str().map(str::to_owned);
    let milliseconds = api::integer(&response["expiration"]);
    let expires =
        (milliseconds > 0).then(|| Timestamp::from_microsecond(milliseconds.saturating_mul(1000)));
    app.db
        .write(move |tx| {
            campfire_db::models::google_calendar::replace_watch(
                tx,
                user_id,
                &channel_id,
                &digest,
                resource.as_deref(),
                expires,
            )
        })
        .await?;
    Ok(())
}
/// Calendar::PushChannel#stop_remote!: HTTP outside the writer, while the grant still exists.
/// Return the loaded row id so a concurrent replacement is not destroyed by the disconnect.
pub async fn stop_remote(app: &App, user_id: i64) -> api::Result<Option<i64>> {
    let channel = app
        .db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT id,channel_id,resource_id FROM calendar_push_channels WHERE user_id=?",
                    [user_id],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()?)
        })
        .await?;
    let Some((id, channel_id, resource_id)) = channel else {
        return Ok(None);
    };
    if let Some(resource_id) = resource_id.filter(|s| !api::blank(s))
        && usable(app, user_id).await?
    {
        let payload = json!({"id":channel_id,"resourceId":resource_id});
        if let Err(api::Error::Storage(error)) = app
            .google
            .api()
            .request(
                &app.db,
                &app.secrets,
                user_id,
                ApiRequest::calendar(Method::POST, "/calendar/v3/channels/stop", Some(&payload)),
                now(app),
            )
            .await
        {
            return Err(error.into());
        }
    }
    Ok(Some(id))
}

fn summary(e: &api::Error) -> String {
    let class = e.class().rsplit("::").next().unwrap();
    campfire_richtext::ruby::truncate(&format!("{class}: {e}"), 250, "...")
}
pub async fn renew(app: &App) -> api::Result<()> {
    let api = app.google.api();
    if !api.config.configured() || api.config.webhook_url.is_none() {
        return Ok(());
    }
    let now = now(app);
    let channels = app
        .db
        .read(|conn| {
            Ok(conn
                .prepare("SELECT id,user_id,expires_at FROM calendar_push_channels ORDER BY id")?
                .query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<Timestamp>>(2)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await?;
    let user_ids = channels
        .iter()
        .map(|(_, user, _)| *user)
        .collect::<Vec<_>>();
    let mut accounts = app
        .db
        .read(move |conn| {
            Ok(GoogleAccount::for_users(conn, &user_ids)?
                .into_iter()
                .map(|account| (account.user_id, account))
                .collect::<std::collections::HashMap<_, _>>())
        })
        .await?;
    for (id, user, expiry) in channels {
        let account = accounts.remove(&user);
        let result = async {
            let account = if let Some(account) = account {
                let (account, usable) = usable_account(app, account).await?;
                usable.then_some(account)
            } else {
                None
            };
            if let Some(account) = account {
                if expiry.is_none_or(|t| t <= now.since(jiff::SignedDuration::from_hours(24))) {
                    watch_with_account(app, user, Some(account)).await?;
                }
            } else {
                app.db
                    .write(move |tx| {
                        tx.conn()
                            .execute("DELETE FROM calendar_push_channels WHERE id=?", [id])?;
                        Ok(())
                    })
                    .await?;
            }
            Ok::<_, api::Error>(())
        }
        .await;
        if let Err(e) = result {
            tracing::error!(
                "Calendar::PushChannel renewal failed for user {}: {}",
                user,
                e.class()
            );
        }
    }
    // Rails' healing pass loads accounts, then watch_for resolves the user's grant afresh.
    let missing = app.db.read(|conn| {
        Ok(conn.prepare("SELECT g.user_id FROM google_accounts g WHERE g.disconnected_reason IS NULL AND NOT EXISTS(SELECT 1 FROM calendar_push_channels p WHERE p.user_id=g.user_id) ORDER BY g.id")?
            .query_map([], |r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await?;
    for user in missing {
        if let Err(e) = watch(app, user).await {
            tracing::error!(
                "Calendar::PushChannel watch failed for user {}: {}",
                user,
                e.class()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn execution(count: u32) -> Execution {
        Execution {
            id: 1,
            executions: count,
            enqueued_at: Timestamp::from_second(1),
            scheduled_at: Timestamp::from_second(1),
        }
    }
    #[test]
    fn google_calendar_cleanup_retry_exhaustion_completes_after_report() {
        assert!(matches!(
            finish_cleanup(
                Err(api::Error::Unavailable("timeout".into())),
                &execution(7),
                Some(1)
            ),
            Err(JobError::RetryGroup {
                key: GOOGLE_RETRY,
                attempts: 8,
                discard_exhausted: true,
                ..
            })
        ));
        assert!(matches!(
            finish_cleanup(
                Err(api::Error::Unavailable("timeout".into())),
                &execution(8),
                Some(1)
            ),
            Err(JobError::RetryGroup {
                key: GOOGLE_RETRY,
                attempts: 8,
                discard_exhausted: true,
                ..
            })
        ));
        assert!(matches!(
            finish_cleanup(
                Err(api::Error::Forbidden("permanent".into())),
                &execution(8),
                None
            ),
            Err(JobError::Error(_))
        ));
    }
    #[test]
    fn google_calendar_inherits_five_attempt_sqlite_retry_policy() {
        let busy = || {
            api::Error::Storage(campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                None,
            )))
        };
        assert!(matches!(
            job_result(Err(busy()), &execution(4)),
            Err(JobError::RetryGroup {
                key: INHERITED_RETRY,
                attempts: 5,
                ..
            })
        ));
        assert!(matches!(
            job_result(Err(busy()), &execution(5)),
            Err(JobError::RetryGroup {
                key: INHERITED_RETRY,
                attempts: 5,
                ..
            })
        ));
    }
    #[test]
    fn google_meeting_refresh_does_not_inherit_calendar_google_retries() {
        assert!(matches!(
            application_result(
                Err(api::Error::Unavailable("timeout".into())),
                &execution(1)
            ),
            Err(JobError::Error(_))
        ));
    }
    #[test]
    fn google_calendar_retry_policy_matches_eight_polynomial_attempts() {
        for policy in [
            Cleanup::retry_policy(),
            Remote::retry_policy(),
            Watch::retry_policy(),
        ] {
            assert_eq!(policy.attempts, 8);
            assert_eq!(
                policy.retry_delay(1, None, 0.0),
                Some(std::time::Duration::from_secs(3))
            );
            assert!(policy.retry_delay(8, None, 0.0).is_none());
            assert!((policy.retry_on)(&anyhow::Error::new(
                api::Error::RateLimited("quota".into())
            )));
            assert!(!(policy.retry_on)(&anyhow::Error::new(
                api::Error::Forbidden("permission".into())
            )));
        }
    }
}

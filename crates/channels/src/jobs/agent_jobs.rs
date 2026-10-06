//! Agent delivery jobs: one ledger claim per POST, durable retries and shared sync replies.
use super::integrations as jobs;
use crate::integrations::webhook;
use crate::net::Network;
use crate::{app::App, queue::Registry};
use campfire_db::models::agent_delivery::{self as domain, AgentEvent, AttemptOutcome};
use campfire_db::{Message, Room, User, Webhook};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use serde::{Deserialize, Serialize};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Delivery(pub domain::DeliveryJob);
impl campfire_db::Job for Delivery {
    const CLASS: &'static str = "Agent::DeliveryJob";
}
impl JobKind for Delivery {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventWebhook {
    Published(domain::EventWebhookJob),
    Deleted(Box<campfire_db::models::agent_work_events::DeletedWorkWebhookJob>),
}
impl campfire_db::Job for EventWebhook {
    const CLASS: &'static str = "Agent::EventWebhookJob";
}
impl JobKind for EventWebhook {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().retry_on(|_| true)
    }
}
pub(super) fn register(r: &mut Registry) {
    r.register(deliver);
    r.register(post);
}
pub async fn deliver(app: App, job: Delivery, _: Execution) -> JobResult {
    app.db
        .write(move |tx| domain::perform_delivery(tx, job.0.event_id))
        .await?;
    Ok(Outcome::Done)
}
async fn post(app: App, job: EventWebhook, execution: Execution) -> JobResult {
    post_deferred_with_network(&app, job, execution.id, &app.subscription_network).await?;
    Ok(Outcome::Done)
}

pub async fn post_deferred_with_network(
    app: &App,
    job: EventWebhook,
    job_id: i64,
    net: &Network,
) -> anyhow::Result<()> {
    use rusqlite::OptionalExtension;
    let job = match job {
        EventWebhook::Published(job) => Some(job),
        EventWebhook::Deleted(_) => app.db.write(move |tx| {
            // A claim may have loaded the intent just before after_commit bound
            // it. Re-read the row, so deletion of its published event stays a no-op.
            let current: Option<serde_json::Value> = tx.conn().query_row(
                "SELECT arguments FROM background_jobs WHERE id=? AND job_class='Agent::EventWebhookJob'",[job_id],|r|r.get(0)).optional()?;
            let Some(current) = current else {return Ok(None)};
            let current: EventWebhook=serde_json::from_value(current).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
            Ok(Some(match current {
                EventWebhook::Published(job) => job,
                // Rails never reconstructs an after_destroy_commit side effect
                // after a process stop or callback failure. The atomic intent
                // survives, but missing publication makes this job a no-op.
                EventWebhook::Deleted(_) => return Ok(None),
            }))
        }).await?,
    };
    if let Some(job) = job {
        post_with_network(app, job, net).await?;
    }
    Ok(())
}

pub async fn post_with_network(
    app: &App,
    job: domain::EventWebhookJob,
    net: &Network,
) -> anyhow::Result<()> {
    let event = app
        .db
        .write(move |tx| domain::claim_webhook(tx, &job))
        .await?;
    let Some(event) = event else {
        return Ok(());
    };
    let result = post_event(app, &event, net).await;
    app.db
        .write(move |tx| domain::finish_webhook(tx, &event, result))
        .await?;
    Ok(())
}
pub async fn post_event(app: &App, e: &AgentEvent, net: &Network) -> AttemptOutcome {
    let id = e.agent_id;
    let e = e.clone();
    let encryption = app.ar_encryption.clone();
    let db = app.db.clone();
    let event = e.clone();
    let threads = match app
        .db
        .read(move |conn| {
            let mut ids = Vec::new();
            if let Some(message_id) = event.message_id
                && let Some(message) = Message::find_by_id(conn, message_id)?
                && let Some(thread_id) = message.thread_id
            {
                ids.push(thread_id);
            }
            if let Some(thread_id) = event
                .metadata
                .get("thread_id")
                .and_then(serde_json::Value::as_i64)
            {
                ids.push(thread_id);
            }
            Ok(ids)
        })
        .await
    {
        Ok(ids) => ids,
        Err(error) => return AttemptOutcome::Retry(error.to_string(), None),
    };
    let access_result =
        if campfire_db::models::agent_delivery::MESSAGE_TYPES.contains(&e.event_type.as_str()) {
            app.agent_repositories
                .resolve_messages(&app.db, id, threads)
                .await
        } else {
            app.agent_repositories
                .resolve_work(&app.db, id, threads)
                .await
        };
    let access = match access_result {
        Ok(access) => access,
        Err(error) => return AttemptOutcome::Retry(error.to_string(), None),
    };
    let prepared = app
        .db
        .write(move |tx| {
            let user_id: i64 =
                tx.conn()
                    .query_row("SELECT user_id FROM agents WHERE id=?", [id], |r| r.get(0))?;
            let bot = User::find(tx.conn(), user_id)?;
            let Some(webhook) = Webhook::find_by_user(tx.conn(), user_id)? else {
                return Ok(None);
            };
            let payload = campfire_db::models::agent_payloads::build(
                tx.conn(),
                &*db.env().rich_text,
                &e,
                &access,
            )?;
            let campfire_db::models::agent_payloads::Payload::Ready { body, sync_message } =
                payload
            else {
                let campfire_db::models::agent_payloads::Payload::Unavailable(error) = payload
                else {
                    unreachable!()
                };
                return Err(campfire_db::Error::Other(format!(
                    "Agent::Delivery::UndeliverableWebhook: {error}"
                )));
            };
            let sync = sync_message
                .map(|message| {
                    Ok::<_, campfire_db::Error>((Room::find(tx.conn(), message.room_id)?, *message))
                })
                .transpose()?;
            let secret = campfire_db::models::agent_access::ensure_webhook_signing_secret(
                tx,
                &encryption,
                id,
            )?;
            Ok(Some((webhook, secret, body, bot, sync)))
        })
        .await;
    let prepared = match prepared {
        Ok(Some(v)) => v,
        Ok(None) => {
            return AttemptOutcome::Permanent(
                "Agent::Delivery::UndeliverableWebhook: Webhook no longer available".into(),
            );
        }
        Err(campfire_db::Error::Other(error))
            if error.starts_with("Agent::Delivery::UndeliverableWebhook: ") =>
        {
            return AttemptOutcome::Permanent(error);
        }
        Err(error) => return AttemptOutcome::Retry(error.to_string(), None),
    };
    let (webhook, secret, payload, bot, sync) = prepared;
    let response = match webhook::post_payload(
        net,
        webhook.url.as_deref().unwrap_or(""),
        payload,
        Some(&secret),
        || app.clock.now(),
    )
    .await
    {
        Ok(v) => v,
        Err(error) => return classify_error(error),
    };
    let result = classify_response(
        response.status,
        response
            .headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        app.clock.now(),
    );
    if matches!(result, AttemptOutcome::Delivered)
        && let Some((room, message)) = sync
    {
        // Rails suppresses every sync-reply extraction/storage/broadcast error after a successful Agent POST.
        if let Err(error) = receive_sync_reply(app, &bot, &room, message, response).await {
            tracing::warn!(%error,"Agent webhook sync reply failed");
        }
    }
    result
}
async fn receive_sync_reply(
    app: &App,
    bot: &User,
    room: &Room,
    message: Message,
    response: webhook::Posted,
) -> anyhow::Result<()> {
    let reply = match webhook::reply(response.status, response.content_type, response.body)? {
        webhook::WebhookReply::None => return Ok(()),
        webhook::WebhookReply::Text(text) => {
            jobs::create_text_reply(app, room, bot, Some(message), text).await?
        }
        webhook::WebhookReply::Attachment(attachment) => {
            jobs::create_attachment_reply(app, room, bot, message, attachment).await?
        }
    };
    jobs::broadcast_create(app, room, &reply).await
}
fn classify_error(error: webhook::WebhookError) -> AttemptOutcome {
    use crate::net::http::HttpError;
    use webhook::WebhookError;
    let (name, permanent) = match &error {
        WebhookError::Guard(crate::net::guard::GuardError::Violation(_)) => {
            ("RestrictedHTTP::Violation", true)
        }
        WebhookError::Guard(crate::net::guard::GuardError::Unresolvable) => {
            ("Surfguard::Unresolvable", true)
        }
        WebhookError::InvalidUrl(_) => ("URI::InvalidURIError", true),
        WebhookError::Http(HttpError::OpenTimeout) => ("Net::OpenTimeout", false),
        WebhookError::Http(HttpError::ReadTimeout) => ("Net::ReadTimeout", false),
        WebhookError::Http(HttpError::Io(e))
            if e.kind() == std::io::ErrorKind::ConnectionRefused =>
        {
            ("Errno::ECONNREFUSED", false)
        }
        _ => ("StandardError", false),
    };
    let detail = format!("{name}: {error}");
    if permanent {
        AttemptOutcome::Permanent(detail)
    } else {
        AttemptOutcome::Retry(detail, None)
    }
}
fn classify_response(status: u16, hint: Option<&str>, now: jiff::Timestamp) -> AttemptOutcome {
    if (200..300).contains(&status) {
        return AttemptOutcome::Delivered;
    }
    if status == 408 || status == 429 || status >= 500 {
        AttemptOutcome::Retry(
            format!(
                "Agent::Delivery::RetryableWebhookResponse: Webhook endpoint returned {status}"
            ),
            parse_retry_after(hint, now),
        )
    } else {
        AttemptOutcome::Permanent(format!(
            "Agent::Delivery::PermanentWebhookResponse: Webhook endpoint returned {status}"
        ))
    }
}
fn parse_retry_after(value: Option<&str>, now: jiff::Timestamp) -> Option<Duration> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let delay = if value.bytes().all(|b| b.is_ascii_digit()) {
        value.parse::<f64>().unwrap_or(f64::INFINITY)
    } else {
        let date = httpdate::parse_http_date(value).ok()?;
        let now = UNIX_EPOCH
            + Duration::from_secs_f64(
                now.as_second() as f64 + now.subsec_nanosecond() as f64 / 1e9,
            );
        date.duration_since(now)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };
    Some(Duration::from_secs_f64(delay.clamp(0.0, 3600.0)))
}

#[cfg(test)]
#[test]
fn ws11_retry_after_and_response_policy_match_rails_vectors() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_delivery_contract.json"
    ))
    .unwrap();
    let now = "2026-03-02T16:00:00Z".parse().unwrap();
    for case in vectors["responses"].as_array().unwrap() {
        let actual = classify_response(case["status"].as_u64().unwrap() as u16, Some("600"), now);
        match actual {
            AttemptOutcome::Delivered => assert_eq!(case["webhook_status"], "delivered"),
            AttemptOutcome::Permanent(error) => {
                assert_eq!(case["webhook_status"], "failed");
                assert_eq!(case["last_error"], error);
            }
            AttemptOutcome::Retry(error, wait) => {
                assert_eq!(case["webhook_status"], "pending");
                assert_eq!(case["last_error"], error);
                assert_eq!(case["next_delay"].as_f64(), wait.map(|d| d.as_secs_f64()));
            }
        }
    }
    for case in vectors["retry_after"].as_array().unwrap() {
        let wait = parse_retry_after(case["hint"].as_str(), now).unwrap_or(Duration::from_secs(3));
        assert_eq!(case["delay"].as_f64(), Some(wait.as_secs_f64()));
    }
}


//! Agent delivery jobs: one ledger claim per POST, durable retries and shared sync replies.
use super::net::Network;
use super::{jobs, webhook};
use crate::{app::App, jobs::Registry};
use campfire_db::models::agent_delivery::{self as domain, AgentEvent, AttemptOutcome};
use campfire_db::{Message, Room, User, Webhook};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use serde::{Deserialize, Serialize};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Delivery(domain::DeliveryJob);
impl campfire_db::Job for Delivery {
    const CLASS: &'static str = "Agent::DeliveryJob";
}
impl JobKind for Delivery {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct EventWebhook(domain::EventWebhookJob);
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
async fn deliver(app: App, job: Delivery, _: Execution) -> JobResult {
    app.db
        .write(move |tx| domain::perform_delivery(tx, job.0.event_id))
        .await?;
    Ok(Outcome::Done)
}
async fn post(app: App, job: EventWebhook, _: Execution) -> JobResult {
    post_with_network(&app, job.0, &Network::system()).await?;
    Ok(Outcome::Done)
}

pub(super) async fn post_with_network(
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
async fn post_event(app: &App, e: &AgentEvent, net: &Network) -> AttemptOutcome {
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
    let access = match app
        .agent_repositories
        .resolve_threads(&app.db, id, threads)
        .await
    {
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
    use super::net::http::HttpError;
    use webhook::WebhookError;
    let (name, permanent) = match &error {
        WebhookError::Guard(super::net::guard::GuardError::Violation(_)) => {
            ("RestrictedHTTP::Violation", true)
        }
        WebhookError::Guard(super::net::guard::GuardError::Unresolvable) => {
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
mod tests {
    use super::super::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
    use super::*;
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };
    async fn pending(app: &App) -> AgentEvent {
        app.db
            .write(|tx| {
                let message = Message::create(
                    tx,
                    campfire_db::NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        body: Some("Trigger".into()),
                        ..Default::default()
                    },
                )?;
                let agent_id = tx.conn().query_row(
                    "SELECT id FROM agents WHERE user_id=?",
                    [BENDER],
                    |r| r.get(0),
                )?;
                let e = AgentEvent::create(
                    tx,
                    domain::NewEvent {
                        agent_id,
                        message_id: Some(message.id),
                        room_id: Some(ALL_TALK),
                        event_type: "mention".into(),
                        outcome: Some("delivered".into()),
                        ..Default::default()
                    },
                )?;
                tx.conn().execute(
                    "UPDATE agent_events SET webhook_status='pending' WHERE id=?",
                    [e.id],
                )?;
                tx.conn().execute(
                    "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                    [BENDER],
                )?;
                Ok(AgentEvent::find(tx.conn(), e.id)?.unwrap())
            })
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn ws11_repository_webhook_uses_the_live_owner_decision() {
        struct Readable;
        impl super::super::agent_repositories::RepositoryReader for Readable {
            fn readable(
                &self,
                request: super::super::agent_repositories::RepositoryRequest,
            ) -> super::super::net::BoxFuture<'_, campfire_db::Result<bool>> {
                assert_eq!(request.user_id, DAVID);
                assert_eq!(
                    (request.owner.as_str(), request.repo.as_str()),
                    ("private", "repo")
                );
                Box::pin(async { Ok(true) })
            }
        }
        let test = TestApp::boot().await.expect("default seed");
        let app = &test.booted.app;
        let event = pending(app).await;
        let (id, message_id) = (event.agent_id, event.message_id.unwrap());
        let crypto = app.ar_encryption.clone();
        app.db.write(move|tx| {
            tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?",rusqlite::params![DAVID,id])?;
            tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,created_at,updated_at) VALUES (?,'ws11-owner',?,?,?)",rusqlite::params![DAVID,crypto.encrypt("ws11-public-test-token"),tx.now(),tx.now()])?;
            let thread=campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("PR".into()),..Default::default()})?;
            tx.conn().execute("UPDATE messages SET thread_id=? WHERE id=?",rusqlite::params![thread.id,message_id])?;
            tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,private,created_at,updated_at) VALUES (900140000,'Private','Repo',1,'Owner-visible title',1,?,?)",rusqlite::params![tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (900140000,?,?,?,?)",rusqlite::params![ALL_TALK,thread.id,tx.now(),tx.now()])?;
            Ok(())
        }).await.unwrap();
        app.agent_repositories.install(Arc::new(Readable));
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
        let net = network(
            Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
            Arc::new(MappingDialer {
                public: HashSet::from(["93.184.216.34".parse().unwrap()]),
                to: server.addr,
                dialed: Mutex::new(vec![]),
            }),
        );
        post_with_network(
            app,
            domain::EventWebhookJob {
                event_id: event.id,
                attempt: Some(0),
            },
            &net,
        )
        .await
        .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&server.received()[0].body).unwrap();
        assert_eq!(body["pull_request"]["title"], "Owner-visible title");
    }

    #[tokio::test]
    async fn ws11_agent_webhook_http_retries_claims_and_permanent_failures() {
        let test = TestApp::boot().await.expect("default seed");
        let app = &test.booted.app;
        for status in [201, 200, 429, 408, 500, 404, 302] {
            let e = pending(app).await;
            let mut route = Route::new("POST", "*", "/hook", status);
            route.headers.push(("Retry-After".into(), "600".into()));
            let server = FakeServer::start(vec![route]).await;
            let net = network(
                Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
                Arc::new(MappingDialer {
                    public: HashSet::from(["93.184.216.34".parse().unwrap()]),
                    to: server.addr,
                    dialed: Mutex::new(vec![]),
                }),
            );
            let job = domain::EventWebhookJob {
                event_id: e.id,
                attempt: Some(0),
            };
            let (a, b) = tokio::join!(
                post_with_network(app, job.clone(), &net),
                post_with_network(app, job, &net)
            );
            a.unwrap();
            b.unwrap();
            assert_eq!(
                server.received().len(),
                1,
                "one POST for duplicate attempt {status}"
            );
            assert!(
                server.received()[0]
                    .header("X-Smartfire-Signature")
                    .is_some()
            );
            let id = e.id;
            let actual = app
                .db
                .read(move |c| AgentEvent::find(c, id))
                .await
                .unwrap()
                .unwrap();
            if (200..300).contains(&status) {
                assert_eq!(actual.webhook_status, "delivered");
                assert_eq!(actual.webhook_attempts, 1);
                assert!(actual.webhook_last_error.is_none());
            } else if [429, 408, 500].contains(&status) {
                assert_eq!(actual.webhook_status, "pending");
                assert_eq!(actual.webhook_attempts, 1);
                let delta = actual
                    .webhook_next_attempt_at
                    .unwrap()
                    .jiff()
                    .as_microsecond()
                    - actual.created_at.jiff().as_microsecond();
                assert!((600_000_000..605_000_000).contains(&delta));
                let attempts=app.db.read(move |c| c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=? AND json_extract(arguments,'$.attempt')=1",[id],|r|r.get::<_,i64>(0)).map_err(Into::into)).await.unwrap();
                assert_eq!(attempts, 1);
            } else {
                assert_eq!(actual.webhook_status, "failed");
                assert_eq!(actual.webhook_attempts, 0);
            }
        }
    }
    #[tokio::test]
    async fn ws11_agent_private_guard_and_locked_sync_reply_settle_once() {
        let test = TestApp::boot().await.expect("default seed");
        let app = &test.booted.app;
        let e = pending(app).await;
        let id = e.id;
        app.db
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE webhooks SET url='http://127.0.0.1:8080/hook' WHERE user_id=?",
                    [BENDER],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
        let dialer = Arc::new(MappingDialer {
            public: HashSet::new(),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        });
        let net = network(Arc::new(FakeResolver::default()), dialer.clone());
        post_with_network(
            app,
            domain::EventWebhookJob {
                event_id: id,
                attempt: Some(0),
            },
            &net,
        )
        .await
        .unwrap();
        assert!(dialer.dialed.lock().unwrap().is_empty());
        let e = app
            .db
            .read(move |c| AgentEvent::find(c, id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(e.webhook_status, "failed");
        assert_eq!(e.webhook_attempts, 0);
        let e = pending(app).await;
        let id = e.id;
        let message_id = e.message_id.unwrap();
        app.db
            .write(move |tx| {
                let mut thread = campfire_db::ChannelThread::create(
                    tx,
                    campfire_db::NewChannelThread {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        name: Some("Locked sync".into()),
                        ..Default::default()
                    },
                )?;
                tx.conn().execute(
                    "UPDATE messages SET thread_id=? WHERE id=?",
                    rusqlite::params![thread.id, message_id],
                )?;
                thread.lock_conversation(tx)?;
                Ok(())
            })
            .await
            .unwrap();
        let mut route = Route::new("POST", "*", "/hook", 200);
        route
            .headers
            .push(("Content-Type".into(), "text/plain".into()));
        route.body = b"Answer".to_vec();
        let server = FakeServer::start(vec![route]).await;
        let net = network(
            Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
            Arc::new(MappingDialer {
                public: HashSet::from(["93.184.216.34".parse().unwrap()]),
                to: server.addr,
                dialed: Mutex::new(vec![]),
            }),
        );
        post_with_network(
            app,
            domain::EventWebhookJob {
                event_id: id,
                attempt: Some(0),
            },
            &net,
        )
        .await
        .unwrap();
        let e = app
            .db
            .read(move |c| AgentEvent::find(c, id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(e.webhook_status, "delivered");
        assert_eq!(e.webhook_attempts, 1);
    }

    #[tokio::test]
    async fn ws11_approval_queue_failure_rolls_back_decision_ledger_and_inbox() {
        use campfire_db::{AgentApproval, NewApproval};
        let test = TestApp::boot().await.expect("default seed");
        let db = test.db();
        let approval = db
            .write(|tx| {
                let agent_id = tx.conn().query_row(
                    "SELECT id FROM agents WHERE user_id=?",
                    [BENDER],
                    |r| r.get(0),
                )?;
                AgentApproval::create(
                    tx,
                    NewApproval {
                        agent_id,
                        action: "deploy".into(),
                        summary: "Ship".into(),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let id = approval.id;
        db.write(|tx| {
            tx.conn().execute_batch("CREATE TRIGGER ws11_reject_decision_webhook BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected decision webhook'); END;")?;
            Ok(())
        }).await.unwrap();
        let failed = db
            .write(move |tx| {
                let mut approval = AgentApproval::find(tx.conn(), id)?.unwrap();
                let user = User::find(tx.conn(), DAVID)?;
                assert!(approval.decide(tx, "approved", &user, None)?.is_empty());
                Ok(())
            })
            .await;
        assert!(
            failed.is_err(),
            "durable queue insertion must be part of settlement"
        );
        db.read(move |c| {
            assert_eq!(AgentApproval::find(c,id)?.unwrap().status,"pending");
            let events:i64=c.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",[id],|r|r.get(0))?;
            let handled:i64=c.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NOT NULL",[id],|r|r.get(0))?;
            assert_eq!((events,handled),(0,0));Ok(())
        }).await.unwrap();
        db.write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER ws11_reject_decision_webhook")?;
            Ok(())
        })
        .await
        .unwrap();
        db.write(move |tx| {
            let mut approval = AgentApproval::find(tx.conn(), id)?.unwrap();
            let user = User::find(tx.conn(), DAVID)?;
            assert!(approval.decide(tx, "approved", &user, None)?.is_empty());
            Ok(())
        })
        .await
        .unwrap();
        db.read(move |c| {
            assert_eq!(AgentApproval::find(c,id)?.unwrap().status,"approved");
            let event= c.query_row("SELECT id FROM agent_events WHERE agent_approval_id=?",[id],|r|r.get::<_,i64>(0))?;
            let jobs:i64=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[event],|r|r.get(0))?;
            assert_eq!(jobs,1);Ok(())
        }).await.unwrap();
    }

    #[tokio::test]
    async fn ws11_slash_queue_failure_rolls_back_the_invocation() {
        use campfire_db::{AgentSlashCommand, NewAgentSlashCommand};
        let test = TestApp::boot().await.expect("default seed");
        let db = test.db();
        db.write(|tx| {
            let agent_id=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
            tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[agent_id])?;
            Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
            AgentSlashCommand::create(tx,NewAgentSlashCommand {agent_id,room_id:ALL_TALK,name:"inspect".into(),..Default::default()})?;
            tx.conn().execute_batch("CREATE TRIGGER ws11_reject_slash_webhook BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected slash webhook'); END;")?;
            Ok(())
        }).await.unwrap();
        let failed = db
            .write(|tx| {
                let context = campfire_db::slash_commands::Context {
                    user_id: DAVID,
                    room_id: ALL_TALK,
                    thread_id: None,
                    huddles_configured: false,
                };
                let result = campfire_db::slash_commands::dispatch(tx, &context, "/inspect queue")?;
                assert_eq!(result.kind, "ephemeral");
                Ok(())
            })
            .await;
        assert!(
            failed.is_err(),
            "invocation and webhook queue insertion must commit together"
        );
        db.read(|c| {
            let count: i64 = c.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE event_type='slash_command'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(count, 0);
            Ok(())
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn ws11_work_delete_queue_failure_rolls_back_thread_and_ledger() {
        let test = TestApp::boot().await.expect("default seed");
        let db = test.db();
        let thread_id=db.write(|tx| {
            let agent_id:i64=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
            tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[agent_id])?;
            Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
            let thread=campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Delete work".into()),work_status:Some("planned".into()),..Default::default()})?;
            tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?",rusqlite::params![BENDER,thread.id])?;
            tx.conn().execute_batch("CREATE TRIGGER ws11_reject_work_webhook BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected work webhook'); END;")?;
            Ok(thread.id)
        }).await.unwrap();
        assert!(
            db.write(
                move |tx| campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                    .destroy_by(tx, Some(DAVID))
            )
            .await
            .is_err()
        );
        db.read(move |conn| {
            assert!(campfire_db::ChannelThread::find_by_id(conn,thread_id)?.is_some());
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='work_unassigned' AND json_extract(metadata,'$.thread_id')=?",[thread_id],|r|r.get::<_,i64>(0))?,0);
            Ok(())
        }).await.unwrap();
    }
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

#[cfg(test)]
#[tokio::test]
async fn ws11_recovery_continues_after_one_durable_enqueue_failure() {
    use crate::controllers::presenters::test_support::{BENDER, TestApp};
    let test = TestApp::boot().await.expect("default seed");
    let db = test.db();
    let (first,second)=db.write(|tx| {
        let agent_id=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        let new=||domain::NewEvent {agent_id,event_type:"github_action_completed".into(),..Default::default()};
        let a=AgentEvent::create(tx,new())?;let b=AgentEvent::create(tx,new())?;
        tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id IN (?,?)",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(3)),a.id,b.id])?;
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_recovery BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' AND json_extract(NEW.arguments,'$.event_id')={} BEGIN SELECT RAISE(ABORT,'WS11 one rejected candidate'); END;",a.id))?;
        Ok((a.id,b.id))
    }).await.unwrap();
    crate::jobs::periodic::stranded_agent_webhooks(db)
        .await
        .unwrap();
    db.read(move |c| {
        let a=AgentEvent::find(c,first)?.unwrap();let b=AgentEvent::find(c,second)?.unwrap();
        assert!(a.webhook_next_attempt_at<b.webhook_next_attempt_at);
        let jobs:i64=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[second],|r|r.get(0))?;
        assert_eq!(jobs,1);
        Ok(())
    }).await.unwrap();
}

#[cfg(test)]
#[path = "agent_payload_tests.rs"]
mod payload_tests;

#[cfg(test)]
mod case_tests;

#[cfg(test)]
mod webhook_cases;

#[cfg(test)]
mod recovery_cases;

#[cfg(test)]
mod delivery_path_cases;

#[cfg(test)]
mod webhook_key_cases;

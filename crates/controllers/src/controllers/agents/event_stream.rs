use super::{EventPolling, agent_api, concerns, ensure_anywhere, field, ruby_i64};
use crate::app::{App, AppCtx};
use axum::body::{Body, Bytes};
use campfire_app::state::agent_streams::{Permit, Timing};
use campfire_db::models::agent_access;
use campfire_kit::{Ctx, Response, Result, StatusCode, format};
use serde_json::{Value, json};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{Instant, sleep_until},
};

pub async fn event_stream(c: &mut Ctx) -> Result {
    let started = Instant::now();
    c.params
        .insert("format", campfire_kit::Param::Str("json".into()));
    let identity = agent_api::require_token(c, false).await?;
    ensure_anywhere(c, identity.agent_id, "read_messages").await?;
    agent_api::throttle(c, 120, "agents/events", "stream")?;
    agent_api::no_store(c);
    let Some(permit) = c.app().agent_event_streams.acquire(identity.credential_id) else {
        c.set_header("retry-after", "1");
        return Ok(c.render(
            StatusCode::TOO_MANY_REQUESTS,
            &format::JSON,
            json!({"error":"rate_limited"}).to_string(),
        ));
    };
    let cursor = c
        .request
        .header("last-event-id")
        .map(|value| ruby_i64(&json!(value)))
        .unwrap_or_else(|| field(c, "since").as_ref().map_or(0, ruby_i64));
    let timing = c.app().config.agent_event_stream;
    let deadline = c
        .current::<campfire_kit::front::WriteDeadline>()
        .map(|front| front.started + (front.end - front.started).mul_f64(0.8))
        .map_or(started + timing.lifetime, |front| {
            front.min(started + timing.lifetime)
        });
    let (stopped, receiver) = watch::channel(false);
    let secret = concerns::agent_bearer_secret(c.request.header("authorization"))
        .expect("authenticated Bearer credential")
        .to_owned();
    let polling = EventPolling::for_request(c)?;
    let monitor = tokio::spawn(monitor(
        c.app().clone(),
        identity,
        secret,
        timing,
        deadline,
        stopped,
    ));
    let state = EventStream {
        polling,
        agent_id: identity.agent_id,
        cursor,
        limit: field(c, "limit"),
        timing,
        deadline,
        next_poll: started,
        next_ping: started + timing.keepalive,
        initial: true,
        stopped: receiver,
        monitor,
        _permit: permit,
        pending: None,
    };
    let stream = futures_util::stream::unfold(state, |mut state| async move {
        match state.next().await {
            Ok(Some(bytes)) => Some((Ok::<_, std::convert::Infallible>(bytes), state)),
            Ok(None) => None,
            Err(error) => {
                tracing::warn!(%error, "Agent event stream closed");
                None
            }
        }
    });
    let mut response = Response::new(StatusCode::OK)
        .content_type("text/event-stream")
        .header("cache-control", "no-cache, no-transform")
        .header("x-accel-buffering", "no")
        .header("no-gzip-compression", "1");
    response.body = campfire_kit::response::Body::Stream(Body::from_stream(stream));
    Ok(response)
}

struct EventStream {
    polling: EventPolling,
    agent_id: i64,
    cursor: i64,
    limit: Option<Value>,
    timing: Timing,
    deadline: Instant,
    next_poll: Instant,
    next_ping: Instant,
    initial: bool,
    stopped: watch::Receiver<bool>,
    monitor: JoinHandle<()>,
    _permit: Permit,
    pending: Option<futures_util::future::BoxFuture<'static, Result<Value>>>,
}

impl EventStream {
    async fn next(&mut self) -> Result<Option<Bytes>> {
        if self.initial {
            self.initial = false;
            return Ok(Some(Bytes::from_static(b"retry: 1000\n\n")));
        }
        loop {
            if *self.stopped.borrow() || Instant::now() >= self.deadline {
                return Ok(None);
            }
            if self.pending.is_none() {
                let polling = self.polling.clone();
                let (agent_id, cursor, limit, next_poll) = (
                    self.agent_id,
                    self.cursor,
                    self.limit.clone(),
                    self.next_poll,
                );
                self.pending = Some(Box::pin(async move {
                    sleep_until(next_poll).await;
                    polling.page(agent_id, Some(json!(cursor)), limit).await
                }));
            }
            tokio::select! {
                biased;
                _ = self.stopped.changed() => return Ok(None),
                _ = sleep_until(self.deadline) => return Ok(None),
                _ = sleep_until(self.next_ping) => {
                    self.next_ping = Instant::now() + self.timing.keepalive;
                    return Ok(Some(Bytes::from_static(b": ping\n\n")));
                }
                page = self.pending.as_mut().unwrap() => {
                    self.pending = None;
                    let page = page?;
                    let invalid_page = || campfire_kit::Error::internal(anyhow::anyhow!("Invalid agent polling page"));
                    let next = page["next_since"].as_i64().ok_or_else(invalid_page)?;
                    self.next_poll = if next == self.cursor {
                        Instant::now() + self.timing.recheck
                    } else { Instant::now() };
                    self.cursor = next;
                    if *self.stopped.borrow() { return Ok(None); }
                    let mut bytes = String::new();
                    for event in page["events"].as_array().ok_or_else(invalid_page)? {
                        use std::fmt::Write;
                        let id = event["id"].as_i64().ok_or_else(invalid_page)?;
                        let kind = event["event_type"].as_str().ok_or_else(invalid_page)?;
                        write!(bytes, "id: {id}\nevent: {kind}\ndata: {event}\n\n").unwrap();
                    }
                    if *self.stopped.borrow() || Instant::now() >= self.deadline { return Ok(None); }
                    if !bytes.is_empty() { return Ok(Some(Bytes::from(bytes))); }
                }
            }
        }
    }
}

impl Drop for EventStream {
    fn drop(&mut self) {
        self.monitor.abort();
    }
}

async fn monitor(
    app: App,
    identity: concerns::CurrentAgent,
    secret: String,
    timing: Timing,
    deadline: Instant,
    stopped: watch::Sender<bool>,
) {
    let mut fresh_until = Instant::now() + timing.recheck;
    let mut next_check = Instant::now();
    loop {
        tokio::select! {
            biased;
            _ = stopped.closed() => return,
            _ = sleep_until(deadline.min(fresh_until)) => {
                tracing::debug!(lifetime = Instant::now() >= deadline, "Agent event stream health deadline expired");
                break;
            },
            _ = sleep_until(next_check) => {}
        }
        let secret = secret.clone();
        let clock = app.clock.clone();
        let app_url = app.db.env().default_url_origin.clone();
        let allowed = app.db.read(move |conn| {
            let checked_at = Instant::now();
            let now = campfire_db::Timestamp::from_jiff(clock.now());
            let _snapshot = campfire_db::Snapshot::begin(conn)?;
            let authenticated = agent_access::verify_identity(conn, &secret, now, &app_url)?
                .is_some_and(|current| {
                    !current.oauth
                        && current.agent_id == identity.agent_id
                        && current.credential_id == identity.credential_id
                });
            let allowed = authenticated
                && agent_access::has_capability_anywhere(conn, identity.agent_id, "read_messages")?;
            Ok((allowed, checked_at))
        });
        let allowed = tokio::select! {
            biased;
            _ = stopped.closed() => return,
            _ = sleep_until(deadline.min(fresh_until)) => {
                tracing::debug!(lifetime = Instant::now() >= deadline, "Agent event stream health deadline expired during database check");
                break;
            },
            allowed = allowed => allowed,
        };
        match allowed {
            Ok((true, checked_at)) => {
                fresh_until = checked_at + timing.recheck;
                next_check = checked_at + timing.recheck / 2;
            }
            Ok((false, _)) => {
                tracing::debug!("Agent event stream is no longer authorized");
                break;
            }
            Err(error) => {
                tracing::warn!(%error, "Agent event stream credential check failed");
                break;
            }
        }
    }
    stopped.send_replace(true);
}

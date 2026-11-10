//! Every delivery attempt claims the ledger and checks current access before signing.
use crate::{app::App, net::Network, queue::Registry};
use campfire_db::{
    Timestamp,
    models::{
        agent_delivery::AgentEvent,
        mcp_events::{self as domain, Category, DeliveryJob, DeliveryOutcome, Subscription},
    },
};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use campfire_runtime::presenters::Presenter;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Job(pub DeliveryJob);
impl campfire_db::Job for Job {
    const CLASS: &'static str = "Agent::McpEventDeliveryJob";
}
impl JobKind for Job {
    const QUEUE: &'static str = crate::queue::WEBHOOKS_QUEUE;
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
pub(super) fn register(r: &mut Registry) {
    r.register(deliver);
}
async fn deliver(app: App, job: Job, _: Execution) -> JobResult {
    post_with_network(&app, job.0, &app.subscription_network).await?;
    Ok(Outcome::Done)
}
pub async fn post_with_network(app: &App, job: DeliveryJob, net: &Network) -> anyhow::Result<()> {
    let Some(delivery) = app.db.write(move |tx| domain::claim(tx, &job)).await? else {
        return Ok(());
    };
    let event_id = delivery.event_id;
    let event = app
        .db
        .read(move |conn| AgentEvent::find(conn, event_id))
        .await?;
    let access = match event {
        Some(event) => {
            app.agent_repositories
                .resolve_events(&app.db, event.agent_id, vec![event.id])
                .await?
        }
        None => campfire_db::models::agent_payloads::RepositoryAccess::default(),
    };
    let d = delivery.clone();
    let state = app.clone();
    let prepared=app.db.write(move |tx| {
        let Some(s)=Subscription::find(tx.conn(),&d.subscription_id)? else {return Ok(None)};
        let pending: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM mcp_event_deliveries WHERE id=? AND attempts=? AND status='pending')", rusqlite::params![d.id,d.attempts], |r| r.get(0))?;
        if !pending { return Ok(None) }
        if campfire_db::Agent::find(tx.conn(),s.agent_id)?.is_some_and(|a|a.paused()) {domain::suppress(tx,&d)?;return Ok(None)}
        if s.refresh_before<=tx.now() {domain::expire(tx,&d)?;return Ok(None)}
        if !domain::principal_live(tx.conn(),s.credential_id,tx.now(),&tx.env().default_url_origin)? {
            domain::finish(tx,&d,DeliveryOutcome::Terminate(domain::Termination::AccessRevoked))?;
            return Ok(None);
        }
        if !campfire_db::models::agent_event_access::delivery_authorized(tx.conn(),d.event_id)? {domain::suppress(tx,&d)?;return Ok(None)}
        let Some(event)=AgentEvent::find(tx.conn(),d.event_id)? else {domain::suppress(tx,&d)?;return Ok(None)};
        let cached = d.body.as_ref().map(|body| serde_json::from_str::<serde_json::Value>(body)
            .map_err(|e| campfire_db::Error::Other(e.to_string()))).transpose()?;
        let mut event = event;
        // Polling acknowledgement does not change the occurrence owed to this webhook.
        if event.outcome.as_deref() == Some("acknowledged")
            && let Some(outcome) = cached.as_ref().and_then(|body| body["data"]["outcome"].as_str())
        {
            event.outcome = Some(outcome.to_owned());
        }
        let mut presenter=Presenter::new(tx.conn(),&state,None);
        presenter.cache_base_url=Some(tx.env().default_url_origin.clone());
        presenter.current_user_id=Some(campfire_db::Agent::find(tx.conn(),s.agent_id)?.expect("live agent").user_id);
        presenter.use_viewer_zone(presenter.current_user_id.expect("agent viewer"))?;
        let Some(data)=campfire_db::models::agent_event_polling::event_payload(tx.conn(),&event,tx.now(),&access,|m|presenter.agent_message_payload(m))? else {domain::suppress(tx,&d)?;return Ok(None)};
        if cached.as_ref().is_some_and(|body| body["data"] != data) {
            domain::suppress_access_changed(tx, &d)?;
            return Ok(None);
        }
        let body = match d.body.clone() {
            Some(body) => {
                let cursor = domain::delivery_cursor(tx.conn(), &s.id, Some(d.id))?;
                let mut cached = cached.expect("parsed cached body");
                let previous = cached["cursor"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
                // Replay can add earlier obligations. Only lower the cursor; event data and ID stay immutable.
                if cursor < previous {
                    cached["cursor"] = json!(cursor.to_string());
                    let body = cached.to_string();
                    if tx.conn().execute("UPDATE mcp_event_deliveries SET body=? WHERE id=? AND attempts=? AND status='pending'", rusqlite::params![body,d.id,d.attempts])? == 0 { return Ok(None) }
                    body
                } else {
                    body
                }
            },
            None => {
                let cursor=domain::delivery_cursor(tx.conn(),&s.id,Some(d.id))?;
                let body=json!({"eventId":d.webhook_id,"name":event.event_type,"timestamp":event.created_at.jiff().to_string(),"data":data,"cursor":cursor.to_string()}).to_string();
                if !campfire_app::integrations::mcp_events::body_fits(&body) {domain::finish(tx,&d,DeliveryOutcome::Permanent(Category::Http4xx))?;return Ok(None)}
                if tx.conn().execute("UPDATE mcp_event_deliveries SET body=? WHERE id=? AND attempts=? AND status='pending'", rusqlite::params![body,d.id,d.attempts])? == 0 { return Ok(None) }
                body
            }
        };
        Ok(Some((s,body)))
    }).await?;
    let Some((subscription, body)) = prepared else {
        return Ok(());
    };
    let secret = app.ar_encryption.decrypt(&subscription.secret)?;
    let now = Timestamp::from_jiff(app.clock.now());
    let previous = subscription
        .previous_secret
        .as_ref()
        .filter(|_| {
            subscription
                .previous_secret_expires_at
                .is_some_and(|t| t > now)
        })
        .map(|s| app.ar_encryption.decrypt(s))
        .transpose()?;
    let result = campfire_app::integrations::mcp_events::post(
        net,
        &subscription.url,
        campfire_app::integrations::mcp_events::Signing {
            secret: &secret,
            previous: previous.as_deref(),
            id: &delivery.webhook_id,
            subscription: Some(&subscription.id),
        },
        body,
        || app.clock.now(),
    )
    .await;
    let outcome = campfire_app::integrations::mcp_events::outcome(&result, app.clock.now());
    app.db
        .write(move |tx| domain::finish(tx, &delivery, outcome))
        .await?;
    Ok(())
}

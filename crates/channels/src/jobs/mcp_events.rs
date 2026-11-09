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
enum Prepared {
    Skip,
    Terminated(Subscription),
    Event(Subscription, String),
}
pub async fn post_with_network(app: &App, job: DeliveryJob, net: &Network) -> anyhow::Result<()> {
    let Some(delivery) = app.db.write(move |tx| domain::claim(tx, &job)).await? else {
        return Ok(());
    };
    let d = delivery.clone();
    let state = app.clone();
    let prepared=app.db.write(move |tx| {
        let Some(s)=Subscription::find(tx.conn(),&d.subscription_id)? else {return Ok(Prepared::Skip)};
        if s.refresh_before<=tx.now() {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)}
        if !domain::principal_live(tx.conn(),s.credential_id,tx.now(),&tx.env().default_url_origin)? {
            // Delete before sending so concurrent delivery jobs emit only one termination.
            domain::finish(tx,&d,DeliveryOutcome::Terminate)?;
            return Ok(Prepared::Terminated(s));
        }
        if !campfire_db::models::agent_event_access::delivery_authorized(tx.conn(),d.event_id)? {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)}
        let Some(event)=AgentEvent::find(tx.conn(),d.event_id)? else {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)};
        let mut presenter=Presenter::new(tx.conn(),&state,None);
        presenter.cache_base_url=Some(tx.env().default_url_origin.clone());
        presenter.current_user_id=Some(campfire_db::Agent::find(tx.conn(),s.agent_id)?.expect("live agent").user_id);
        presenter.use_viewer_zone(presenter.current_user_id.expect("agent viewer"))?;
        let Some(data)=campfire_db::models::agent_event_polling::event_payload(tx.conn(),&event,tx.now(),|m|presenter.agent_message_payload(m))? else {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)};
        let earlier:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM mcp_event_deliveries WHERE subscription_id=? AND status='pending' AND agent_event_id<?)",rusqlite::params![s.id,event.id],|r|r.get(0))?;
        let cursor=if earlier {s.watermark} else {s.watermark.max(event.id)};
        let body=json!({"eventId":event.id.to_string(),"name":event.event_type,"timestamp":event.created_at.jiff().to_string(),"data":data,"cursor":cursor.to_string()}).to_string();
        if body.len()>256*1024 {domain::finish(tx,&d,DeliveryOutcome::Permanent(Category::Http4xx))?;return Ok(Prepared::Skip)}
        Ok(Prepared::Event(s,body))
    }).await?;
    let (subscription, body, terminated) = match prepared {
        Prepared::Skip => return Ok(()),
        Prepared::Terminated(s) => (
            s,
            json!({"type":"terminated","error":{"code":-32024,"message":"Access revoked"}})
                .to_string(),
            true,
        ),
        Prepared::Event(s, body) => (s, body, false),
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
        &secret,
        previous.as_deref(),
        &delivery.webhook_id,
        Some(&subscription.id),
        body,
        app.clock.now(),
    )
    .await;
    if !terminated {
        let outcome = campfire_app::integrations::mcp_events::outcome(&result, app.clock.now());
        app.db
            .write(move |tx| domain::finish(tx, &delivery, outcome))
            .await?;
    }
    Ok(())
}

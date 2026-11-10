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
    Event(Subscription, String),
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
        let Some(s)=Subscription::find(tx.conn(),&d.subscription_id)? else {return Ok(Prepared::Skip)};
        if s.refresh_before<=tx.now() {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)}
        if !domain::principal_live(tx.conn(),s.credential_id,tx.now(),&tx.env().default_url_origin)? {
            domain::finish(tx,&d,DeliveryOutcome::Terminate(domain::Termination::AccessRevoked))?;
            return Ok(Prepared::Skip);
        }
        if !campfire_db::models::agent_event_access::delivery_authorized(tx.conn(),d.event_id)? {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)}
        let Some(event)=AgentEvent::find(tx.conn(),d.event_id)? else {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)};
        let mut presenter=Presenter::new(tx.conn(),&state,None);
        presenter.cache_base_url=Some(tx.env().default_url_origin.clone());
        presenter.current_user_id=Some(campfire_db::Agent::find(tx.conn(),s.agent_id)?.expect("live agent").user_id);
        presenter.use_viewer_zone(presenter.current_user_id.expect("agent viewer"))?;
        let Some(data)=campfire_db::models::agent_event_polling::event_payload(tx.conn(),&event,tx.now(),&access,|m|presenter.agent_message_payload(m))? else {domain::suppress(tx,&d)?;return Ok(Prepared::Skip)};
        let cursor=domain::delivery_cursor(tx.conn(),&s.id,Some(d.id))?;
        let body=json!({"eventId":d.webhook_id,"name":event.event_type,"timestamp":event.created_at.jiff().to_string(),"data":data,"cursor":cursor.to_string()}).to_string();
        if !campfire_app::integrations::mcp_events::body_fits(&body) {domain::finish(tx,&d,DeliveryOutcome::Permanent(Category::Http4xx))?;return Ok(Prepared::Skip)}
        Ok(Prepared::Event(s,body))
    }).await?;
    let (subscription, body) = match prepared {
        Prepared::Skip => return Ok(()),
        Prepared::Event(s, body) => (s, body),
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

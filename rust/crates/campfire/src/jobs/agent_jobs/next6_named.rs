//! Execute the committed work webhook through the durable runner. Only DNS/TCP is routed locally.
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::test_support::TestApp;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
pub(crate) struct Next6Delivery {
    server: FakeServer,
}
impl Next6Delivery {
    pub(crate) async fn start(t: &TestApp) -> Self {
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
        let address = server.addr.to_string();
        // Per-database fixture routing for the producer mutation control. No
        // process-wide environment address races between parallel declarations.
        t.db()
            .write(move |tx| {
                tx.conn().execute_batch(
                    "CREATE TABLE ws11_next6_http_observer(address TEXT NOT NULL)",
                )?;
                tx.conn().execute(
                    "INSERT INTO ws11_next6_http_observer(address) VALUES(?)",
                    [address],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        Self { server }
    }
    pub(crate) async fn deliver(&self, t: &TestApp, agent: i64) -> Value {
        let before_draining = self.server.received();
        assert!(
            before_draining.is_empty(),
            "assignment must not block on HTTP before draining jobs: {before_draining:?}"
        );
        let queued=t.db().read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id') IN (SELECT id FROM agent_events WHERE agent_id=?)",[agent],|r|r.get::<_,i64>(0))?)).await.unwrap();
        let net = network(
            Arc::new(FakeResolver::new([])),
            Arc::new(MappingDialer {
                public: HashSet::from(["93.184.216.34".parse().unwrap()]),
                to: self.server.addr,
                dialed: Mutex::new(vec![]),
            }),
        );
        let mut registry = Registry::new();
        registry.register(move |app: App, job: EventWebhook, execution: Execution| {
            let net = net.clone();
            async move {
                post_deferred_with_network(&app, job, execution.id, &net).await?;
                Ok(Outcome::Done)
            }
        });
        // Other agents' pending jobs belong to later steps and must not be executed here.
        t.db().write(move|tx|{tx.conn().execute("UPDATE background_jobs SET run_at='2099-01-01 00:00:00' WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id') NOT IN (SELECT id FROM agent_events WHERE agent_id=?)",[agent])?;Ok(())}).await.unwrap();
        let app = t.booted.app.clone();
        let runner = campfire_jobs::start(
            app.db.clone(),
            app.jobs.queue.clone(),
            registry,
            app.clone(),
            crate::queue::runner_config(&app.config),
        );
        crate::test_support::eventually("committed work webhook acknowledged",||async {app.db.read(move|c|Ok(c.query_row("SELECT COUNT(*)=0 FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id') IN (SELECT id FROM agent_events WHERE agent_id=?)",[agent],|r|r.get::<_,bool>(0))?)).await.unwrap()}).await;
        runner.shutdown(Duration::from_secs(2)).await;
        let requests=self.server.received().into_iter().map(|r|json!({"body":String::from_utf8(r.body.clone()).unwrap(),"timestamp":r.header("X-Smartfire-Timestamp"),"signature":r.header("X-Smartfire-Signature"),"content_type":r.header("Content-Type")})).collect::<Vec<_>>();
        json!({"queued":queued,"requests":requests})
    }
}

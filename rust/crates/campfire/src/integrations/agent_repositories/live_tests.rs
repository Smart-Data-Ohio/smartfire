//! The production seam calls the existing linked-account domain over actual TLS.
use super::*;
use crate::controllers::presenters::test_support::{BENDER, DAVID, SEED_NOW, TestApp};
use crate::integrations::{
    github::{accounts::Accounts, client::AppClient},
    net::Network,
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use serde_json::{Value, json};
use std::sync::Arc;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/agents_repository_adapter_contract.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn ws11_live_repository_adapter_matches_rails_allow_denial_disconnect_and_retry() {
    for status in [403, 404, 401, 500, 200] {
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            SEED_NOW.parse().unwrap(),
        ));
        let (app, _dir) = TestApp::boot_with_clock(clock)
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let (agent, thread) = super::tests::setup(&app.db, app.ar_encryption.clone()).await;
        let (server, roots) = FakeServer::start_named_tls_ws15e(
            vec![Route::new("GET", "api.github.com", "/repos/mixed/repo", status).body("{}")],
            vec!["api.github.com".into()],
        )
        .await;
        let net = Network {
            resolver: Arc::new(FakeResolver::new([(
                "api.github.com",
                vec!["93.184.216.34"],
            )])),
            dialer: Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: server.addr,
                dialed: Default::default(),
            }),
            tls: crate::integrations::net::tls_config(roots),
        };
        app.agent_repositories
            .install(Arc::new(Accounts::with_network(
                app.db.clone(),
                app.ar_encryption.clone(),
                AppClient::from_env(),
                net,
            )));
        let mut decisions = vec![];
        for _ in 0..2 {
            let access = app
                .agent_repositories
                .resolve_threads(&app.db, agent, vec![thread, thread])
                .await
                .unwrap();
            decisions.push(access.contains(&(DAVID, "mixed".into(), "repo".into())));
            let accessible = app
                .db
                .read(move |c| {
                    Ok(campfire_db::models::agent_payloads::work_payload(
                        c,
                        &campfire_db::ChannelThread::find(c, thread)?,
                        Some(DAVID),
                        &access,
                    )?["links"]
                        .clone())
                })
                .await
                .unwrap();
            assert_eq!(
                accessible,
                oracle()["cases"][status.to_string()]["links"],
                "Rails private/unknown redaction and public details"
            );
        }
        let reason = app
            .db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT disconnected_reason FROM github_connected_accounts WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, Option<String>>(0),
                )?)
            })
            .await
            .unwrap();
        let paths = server
            .received()
            .iter()
            .map(|r| r.target.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            json!({"decisions":decisions,"paths":paths,"disconnected_reason":reason}),
            {
                let mut expected = oracle()["cases"][status.to_string()].clone();
                expected.as_object_mut().unwrap().remove("links");
                expected
            }
        );
        assert_eq!(
            app.db
                .read(move |c| Ok(Agent::for_user(c, BENDER)?.unwrap().owner_id))
                .await
                .unwrap(),
            Some(DAVID)
        );
    }
}

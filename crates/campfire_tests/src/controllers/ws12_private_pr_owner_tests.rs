//! Both original private-PR branches, before and after the owner's connection.
use super::agent_http_tests::{AGENT, SECRET};
use super::presenters::test_support::Req;
use crate::integrations::{
    agent_repositories::{RepositoryReader, RepositoryRequest},
    net::BoxFuture,
};
use campfire_kit::Method;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct AllowOwner {
    calls: AtomicUsize,
}
impl RepositoryReader for AllowOwner {
    fn readable(&self, request: RepositoryRequest) -> BoxFuture<'_, campfire_db::Result<bool>> {
        assert_eq!(
            (request.account_id, request.user_id),
            (1996100000, 127326141)
        );
        assert_eq!(
            (request.owner.as_str(), request.repo.as_str()),
            ("acme", "secret")
        );
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(true) })
    }
}
#[tokio::test]
async fn ws12_private_pr_redacts_then_exposes_title_and_branch_for_connected_owner() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_private_pr_owner.json"
    ))
    .unwrap();
    let app = super::agent_reads_tests::prepare(&json!({"setup":oracle["setup"]})).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM github_connected_accounts WHERE user_id=127326141",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reader = Arc::new(AllowOwner {
        calls: AtomicUsize::new(0),
    });
    app.booted.app.agent_repositories.install(reader.clone());
    for step in oracle["steps"].as_array().unwrap() {
        let connected = step["name"] == "connected";
        if connected {
            let crypto = app.booted.app.ar_encryption.clone();
            app.db().write(move |tx| {
                tx.conn().execute("INSERT INTO github_connected_accounts(id,user_id,github_login,access_token,created_at,updated_at) VALUES(1996100000,127326141,'owner-gh',?,?,?)",rusqlite::params![crypto.encrypt("obviously-fake-owner-token"),tx.now(),tx.now()])?;
                assert_eq!(campfire_db::Agent::find(tx.conn(),AGENT)?.unwrap().owner_id,Some(127326141));
                Ok(())
            }).await.unwrap();
        }
        let response = app
            .anonymous()
            .send(
                Req::new(Method::GET, "/agents/work/1900700020")
                    .header("accept", "application/json")
                    .header("authorization", &["Bearer", SECRET].join(" ")),
            )
            .await;
        assert_eq!(
            response.status.as_u16() as u64,
            step["status"].as_u64().unwrap(),
            "{}",
            step["name"]
        );
        assert_eq!(
            response.text(),
            step["body"].as_str().unwrap(),
            "{}: complete private PR response",
            step["name"]
        );
        for key in ["content-type", "cache-control", "pragma", "location"] {
            assert_eq!(
                response.header(key),
                step["headers"][key].as_str(),
                "{}: {key}",
                step["name"]
            );
        }
        let body = response.json();
        let link = &body["links"][0];
        assert_eq!(
            link["title"],
            if connected {
                json!("Secret acquisition α & β")
            } else {
                Value::Null
            }
        );
        assert_eq!(link["pull_request"]["title"], link["title"]);
        assert_eq!(
            link["pull_request"]["head_branch"],
            if connected {
                json!("secret-branch")
            } else {
                Value::Null
            }
        );
        assert_eq!(
            link["pull_request"]["base_branch"],
            if connected {
                json!("main")
            } else {
                Value::Null
            }
        );
        if !connected {
            assert!(!response.text().contains("Secret acquisition"));
            assert!(!response.text().contains("secret-branch"));
        }
    }
    assert_eq!(reader.calls.load(Ordering::Relaxed), 1);
    println!(
        "WS12_PRIVATE_PR_OWNER 2 complete HTTP responses; negative and positive title/head/base branches; 0 mismatches"
    );
}

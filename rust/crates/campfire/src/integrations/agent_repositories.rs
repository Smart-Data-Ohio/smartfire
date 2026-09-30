//! Live owner-specific PR access, resolved without holding a database connection.
//! Delegates refresh/cache/401 policy to WS15g's existing Accounts domain.
use super::net::BoxFuture;
use campfire_db::models::agent_payloads::RepositoryAccess;
use campfire_db::{Agent, Connection, Database, Result};
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

/// The account belongs to the Agent's owner, never to its bot or the triggering actor.
#[derive(Clone, Debug)]
pub struct RepositoryRequest {
    pub account_id: i64,
    pub user_id: i64,
    pub owner: String,
    pub repo: String,
}
pub trait RepositoryReader: Send + Sync {
    fn readable(&self, request: RepositoryRequest) -> BoxFuture<'_, Result<bool>>;
}
impl RepositoryReader for super::github::accounts::Accounts {
    fn readable(&self, request: RepositoryRequest) -> BoxFuture<'_, Result<bool>> {
        Box::pin(async move {
            self.can_read_repository(request.account_id, &request.owner, &request.repo)
                .await
        })
    }
}
#[derive(Default)]
pub struct State {
    reader: RwLock<Option<Arc<dyn RepositoryReader>>>,
}
impl State {
    pub fn live(db: Database, crypto: Arc<rails_compat::ar_encryption::ArEncryption>) -> Self {
        let state = Self::default();
        state.install(Arc::new(super::github::accounts::Accounts::new(
            db,
            crypto,
            super::github::client::AppClient::from_env(),
        )));
        state
    }
    #[cfg(test)]
    fn installed(&self) -> bool {
        self.reader.read().unwrap().is_some()
    }
    pub fn install(&self, reader: Arc<dyn RepositoryReader>) {
        *self.reader.write().unwrap_or_else(|p| p.into_inner()) = Some(reader);
    }
    /// Shared by webhook, polling, context and work callers. No access is cached here;
    /// linking, refresh, ten-minute grants/denials and transport retry belong to WS15g.
    pub async fn resolve_threads(
        &self,
        db: &Database,
        agent_id: i64,
        thread_ids: Vec<i64>,
    ) -> Result<RepositoryAccess> {
        let reader = self
            .reader
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let Some(reader) = reader else {
            // Explicitly uninstalled states (tests/custom callers) still deny private access.
            return Ok(RepositoryAccess::default());
        };
        let requests = db
            .read(move |conn| requests(conn, agent_id, &thread_ids))
            .await?;
        let mut access = RepositoryAccess::default();
        for request in requests {
            if reader.readable(request.clone()).await? {
                let (account_id, user_id) = (request.account_id, request.user_id);
                let current=db.read(move |conn| {
                    if Agent::find(conn,agent_id)?.and_then(|a|a.owner_id)!=Some(user_id) {return Ok(false)}
                    let reason=conn.query_row("SELECT disconnected_reason FROM github_connected_accounts WHERE id=? AND user_id=?",params![account_id,user_id],|r|r.get::<_,Option<String>>(0)).optional()?;
                    Ok(reason.is_some_and(|reason|reason.as_deref().is_none_or(campfire_richtext::ruby::is_blank)))
                }).await?;
                if current {
                    access.insert((request.user_id, request.owner, request.repo));
                }
            }
        }
        Ok(access)
    }
}
fn requests(conn: &Connection, agent_id: i64, threads: &[i64]) -> Result<Vec<RepositoryRequest>> {
    let Some(user_id) = Agent::find(conn, agent_id)?.and_then(|a| a.owner_id) else {
        return Ok(vec![]);
    };
    let account = conn
        .query_row(
            "SELECT id,disconnected_reason FROM github_connected_accounts WHERE user_id=?",
            [user_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let Some((account_id, reason)) = account else {
        return Ok(vec![]);
    };
    if reason
        .as_deref()
        .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
    {
        return Ok(vec![]);
    }
    let mut repositories = BTreeSet::new();
    for thread_id in threads {
        let mut query = conn.prepare(
            "SELECT owner,repo FROM github_pull_requests WHERE (private IS NULL OR private!=0) AND id IN (SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? UNION SELECT github_pull_request_id FROM work_thread_links WHERE channel_thread_id=? AND kind='pull_request')",
        )?;
        for row in query.query_map(params![thread_id, thread_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (owner, repo) = row?;
            repositories.insert((owner.to_lowercase(), repo.to_lowercase()));
        }
    }
    Ok(repositories
        .into_iter()
        .map(|(owner, repo)| RepositoryRequest {
            account_id,
            user_id,
            owner,
            repo,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
    use campfire_db::{ChannelThread, NewChannelThread};
    use std::sync::Mutex;
    #[tokio::test]
    async fn ws11_live_repository_reader_is_installed_at_boot() {
        let test = TestApp::boot().await.expect("default seed");
        assert!(
            test.booted.app.agent_repositories.installed(),
            "production boot must install the linked-account reader"
        );
    }
    struct Reader {
        db: Database,
        calls: Mutex<Vec<RepositoryRequest>>,
        allow: bool,
    }
    impl RepositoryReader for Reader {
        fn readable(&self, request: RepositoryRequest) -> BoxFuture<'_, Result<bool>> {
            Box::pin(async move {
                // A real writer must proceed during the external call: no read/writer lease crosses it.
                self.db
                    .write(|tx| {
                        tx.conn()
                            .execute("UPDATE agents SET last_seen_at=?", [tx.now()])?;
                        Ok(())
                    })
                    .await?;
                self.calls.lock().unwrap().push(request);
                Ok(self.allow)
            })
        }
    }
    pub(super) async fn setup(
        db: &Database,
        crypto: Arc<rails_compat::ar_encryption::ArEncryption>,
    ) -> (i64, i64) {
        db.write(move |tx| {
            let agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
            tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?", params![DAVID,agent.id])?;
            tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,created_at,updated_at) VALUES (?,'ws11-owner',?,?,?)", params![DAVID,crypto.encrypt("ws11-public-fake-repository-token"),tx.now(),tx.now()])?;
            let thread=ChannelThread::create(tx, NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Private context".into()),..Default::default()})?;
            for (id,private) in [(900130001,Some(true)),(900130002,None),(900130003,Some(false))] {
                tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,private,created_at,updated_at) VALUES (?,'mixed','repo',?,'Private title',?,?,?)", params![id,id,private,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES (?,'pull_request',?,?,?,?)",params![thread.id,id,DAVID,tx.now(),tx.now()])?;
            }
            Ok((agent.id,thread.id))
        }).await.unwrap()
    }
    #[tokio::test]
    async fn ws11_repository_owner_access_is_live_deduplicated_and_outside_the_writer() {
        let test = TestApp::boot().await.expect("default seed");
        let db = &test.booted.app.db;
        let (agent, thread) = setup(db, test.booted.app.ar_encryption.clone()).await;
        let state = &test.booted.app.agent_repositories;
        let uninstalled = State::default();
        assert!(
            uninstalled
                .resolve_threads(db, agent, vec![thread])
                .await
                .unwrap()
                .is_empty()
        );
        let reader = Arc::new(Reader {
            db: db.clone(),
            calls: Mutex::new(vec![]),
            allow: true,
        });
        state.install(reader.clone());
        let access = state
            .resolve_threads(db, agent, vec![thread, thread])
            .await
            .unwrap();
        assert_eq!(
            access,
            RepositoryAccess::from([(DAVID, "mixed".into(), "repo".into())])
        );
        {
            let calls = reader.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].user_id, DAVID);
        }
        db.write(move |tx| {
            tx.conn()
                .execute("UPDATE agents SET owner_id=NULL WHERE id=?", [agent])?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            state
                .resolve_threads(db, agent, vec![thread])
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(reader.calls.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn ws11_repository_disconnected_accounts_and_denials_reveal_nothing() {
        let test = TestApp::boot().await.expect("default seed");
        let db = &test.booted.app.db;
        let (agent, thread) = setup(db, test.booted.app.ar_encryption.clone()).await;
        let state = &test.booted.app.agent_repositories;
        let reader = Arc::new(Reader {
            db: db.clone(),
            calls: Mutex::new(vec![]),
            allow: false,
        });
        state.install(reader.clone());
        assert!(
            state
                .resolve_threads(db, agent, vec![thread])
                .await
                .unwrap()
                .is_empty()
        );
        db.write(|tx| {
            tx.conn().execute(
                "UPDATE github_connected_accounts SET disconnected_reason='401' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            state
                .resolve_threads(db, agent, vec![thread])
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(reader.calls.lock().unwrap().len(), 1);
    }
}

#[cfg(test)]
mod live_tests;

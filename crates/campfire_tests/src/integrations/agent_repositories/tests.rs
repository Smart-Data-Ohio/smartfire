use super::*;
use campfire_app::net::BoxFuture;
use campfire_db::Result;
use campfire_db::Agent;
use campfire_db::Database;
use campfire_db::models::agent_payloads::RepositoryAccess;
use rusqlite::params;
use std::sync::Arc;
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
async fn ws11_repository_owner_access_is_live_and_outside_the_writer() {
    let test = TestApp::boot().await.expect("default seed");
    let db = &test.booted.app.db;
    let (agent, thread) = setup(db, test.booted.app.ar_encryption.clone()).await;
    let state = &test.booted.app.agent_repositories;
    // Explicitly detached registries still deny without external calls.
    let detached = State::default();
    assert!(
        detached
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
        assert_eq!(calls.len(), 2);
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
    assert_eq!(reader.calls.lock().unwrap().len(), 2);
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
    assert_eq!(reader.calls.lock().unwrap().len(), 2);
}

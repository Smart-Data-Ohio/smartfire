//! Live owner-specific PR access, resolved without holding a database connection.
//! Boot installs WS15g Accounts::can_read_repository (refresh/cache/401 policy).
use super::net::BoxFuture;
use campfire_db::models::agent_payloads::{RepositoryAccess, RepositoryEntry};
use campfire_db::{Agent, Connection, Database, Result};
use rusqlite::{OptionalExtension, params};
use std::collections::{HashMap, HashSet};
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
#[derive(Clone, Copy)]
enum Surface {
    Work,
    Messages,
    #[allow(
        dead_code,
        reason = "Combined thread seam retained for WS11-api source compatibility"
    )]
    Both,
}
impl State {
    /// Share the booted owner service, including its configured network.
    pub fn live(accounts: super::github::accounts::Accounts) -> Self {
        let state = Self::default();
        state.install(Arc::new(accounts));
        state
    }
    #[cfg(test)]
    fn installed(&self) -> bool {
        self.reader.read().unwrap().is_some()
    }
    pub fn install(&self, reader: Arc<dyn RepositoryReader>) {
        *self.reader.write().unwrap_or_else(|p| p.into_inner()) = Some(reader);
    }
    /// Compatibility seam for callers needing both kinds of thread context.
    /// Payload-specific callers use work/messages; polling uses event order.
    #[allow(
        dead_code,
        reason = "Combined thread seam retained for WS11-api source compatibility"
    )]
    pub async fn resolve_threads(
        &self,
        db: &Database,
        agent: i64,
        threads: Vec<i64>,
    ) -> Result<RepositoryAccess> {
        self.resolve(db, agent, threads, Surface::Both).await
    }
    pub async fn resolve_work(
        &self,
        db: &Database,
        agent: i64,
        threads: Vec<i64>,
    ) -> Result<RepositoryAccess> {
        self.resolve(db, agent, threads, Surface::Work).await
    }
    pub async fn resolve_messages(
        &self,
        db: &Database,
        agent: i64,
        threads: Vec<i64>,
    ) -> Result<RepositoryAccess> {
        self.resolve(db, agent, threads, Surface::Messages).await
    }
    /// Polling reads events in ledger order, including repeated work/thread
    /// occurrences. Each event gets its own decisions from the same batch.
    #[allow(
        dead_code,
        reason = "Polling REST/MCP caller is owned by WS11-api; batch seam is tested here"
    )]
    pub async fn resolve_events(
        &self,
        db: &Database,
        agent: i64,
        mut events: Vec<i64>,
    ) -> Result<RepositoryAccess> {
        events.sort_unstable();
        events.dedup();
        let requests = db
            .read(move |conn| {
                if events.is_empty() {
                    return Ok(vec![]);
                }
                let mut query = conn.prepare("SELECT e.id,e.event_type,e.metadata,m.thread_id FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.agent_id=? AND e.id IN (SELECT value FROM json_each(?)) ORDER BY e.id")?;
                let rows = query.query_map(params![agent,serde_json::json!(events).to_string()],|row| {
                    let kind:String=row.get(1)?;
                    let metadata=row.get::<_,Option<serde_json::Value>>(2)?.unwrap_or_default();
                    let work=campfire_db::models::agent_delivery::WORK_TYPES.contains(&kind.as_str());
                    let thread = if work {metadata.get("thread_id").map(campfire_db::models::agent_delivery::ruby_i64)} else {row.get(3)?};
                    Ok((row.get::<_,i64>(0)?,thread,work))
                })?.collect::<std::result::Result<Vec<_>,_>>()?;
                let work_threads=rows.iter().filter(|r|r.2).filter_map(|r|r.1).collect::<Vec<_>>();
                let message_threads=rows.iter().filter(|r|!r.2).filter_map(|r|r.1).collect::<Vec<_>>();
                let mut work:HashMap<i64,Vec<_>>=HashMap::new();
                let mut messages:HashMap<i64,Vec<_>>=HashMap::new();
                for (thread,entry,request) in requests_by_thread(conn,agent,&work_threads,Surface::Work)? {work.entry(thread).or_default().push((entry,request));}
                for (thread,entry,request) in requests_by_thread(conn,agent,&message_threads,Surface::Messages)? {messages.entry(thread).or_default().push((entry,request));}
                let mut result=Vec::new();
                for (event,thread,is_work) in rows {
                    if let Some(requests)=thread.and_then(|thread|if is_work {work.get(&thread)} else {messages.get(&thread)}) {
                        result.extend(requests.iter().cloned().map(|(entry,request)|(event,entry,request)));
                    }
                }
                Ok(result)
            })
            .await?;
        self.resolve_requests(db, agent, requests, true).await
    }
    async fn resolve(
        &self,
        db: &Database,
        agent: i64,
        threads: Vec<i64>,
        surface: Surface,
    ) -> Result<RepositoryAccess> {
        let requests = db
            .read(move |conn| requests(conn, agent, &threads, surface))
            .await?
            .into_iter()
            .map(|(entry, request)| (0, entry, request))
            .collect();
        self.resolve_requests(db, agent, requests, false).await
    }
    async fn resolve_requests(
        &self,
        db: &Database,
        agent: i64,
        requests: Vec<(i64, RepositoryEntry, RepositoryRequest)>,
        scoped: bool,
    ) -> Result<RepositoryAccess> {
        let reader = self
            .reader
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let Some(reader) = reader else {
            // Explicitly detached test states remain fail-closed. Production boot
            // installs the owner service before handling requests or jobs.
            return Ok(RepositoryAccess::default());
        };
        let mut access = RepositoryAccess::batch();
        if scoped {
            access.scope_events();
        }
        let identity = requests
            .first()
            .map(|(_, _, request)| (request.user_id, request.account_id));
        let mut guard = IdentityGuard::default();
        for (scope, entry, request) in requests {
            let (account, user) = (request.account_id, request.user_id);
            // Check before using even a cached allowance: a later occurrence of
            // the same repo must be redacted after this batch disconnects it.
            if !guard.current(db, agent, user, account).await? {
                break;
            }
            // Accounts owns the versioned cache. A batch-local cache would
            // reuse a denial after a transient error or an allowance after relink.
            let readable = reader.readable(request.clone()).await?;
            if readable && guard.current(db, agent, user, account).await? {
                let mut scoped_access = access.in_event(scope);
                scoped_access.allow_entry(entry, user, request.owner, request.repo);
                access = scoped_access;
            }
        }
        if let Some((user, account)) = identity {
            access = db
                .read(move |conn| {
                    access.seal(conn, agent, user, account)?;
                    Ok(access)
                })
                .await?;
        }
        Ok(access)
    }
}
// Cache only the current identity snapshot, never a repository permission result.
// A refresh/disconnect/relink or owner/grant write invalidates it even if an account
// changes and changes back during a network call. Never hold a reader across await.
#[derive(Default)]
struct IdentityGuard {
    cached: Option<(u64, i64, i64, i64, bool)>,
}
impl IdentityGuard {
    async fn current(
        &mut self,
        db: &Database,
        agent: i64,
        user: i64,
        account: i64,
    ) -> Result<bool> {
        let generation = db.write_generation();
        if generation.is_multiple_of(2)
            && let Some((version, a, u, c, allowed)) = self.cached
            && (version, a, u, c) == (generation, agent, user, account)
        {
            return Ok(allowed);
        }
        let snapshot = db.clone();
        let (before, allowed, after) = db
            .read(move |conn| {
                let before = snapshot.write_generation();
                let allowed = current(conn, agent, user, account)?;
                Ok((before, allowed, snapshot.write_generation()))
            })
            .await?;
        self.cached = if before == after && after.is_multiple_of(2) {
            Some((after, agent, user, account, allowed))
        } else {
            None
        };
        Ok(allowed)
    }
}
fn current(conn: &Connection, agent: i64, user: i64, account: i64) -> Result<bool> {
    let reason=conn.query_row(
        "SELECT c.disconnected_reason FROM agents a JOIN github_connected_accounts c ON c.user_id=a.owner_id WHERE a.id=? AND a.owner_id=? AND c.id=?",
        params![agent,user,account],|r|r.get::<_,Option<String>>(0)).optional()?;
    Ok(reason.is_some_and(|reason| {
        reason
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
    }))
}
fn requests(
    conn: &Connection,
    agent: i64,
    threads: &[i64],
    surface: Surface,
) -> Result<Vec<(RepositoryEntry, RepositoryRequest)>> {
    Ok(requests_by_thread(conn, agent, threads, surface)?
        .into_iter()
        .map(|(_, entry, request)| (entry, request))
        .collect())
}
fn requests_by_thread(
    conn: &Connection,
    agent: i64,
    threads: &[i64],
    surface: Surface,
) -> Result<Vec<(i64, RepositoryEntry, RepositoryRequest)>> {
    if threads.is_empty() {
        return Ok(vec![]);
    }
    let slots = std::iter::repeat_n("?", threads.len())
        .collect::<Vec<_>>()
        .join(",");
    let mut work: HashMap<i64, Vec<(i64, String, String)>> = HashMap::new();
    if matches!(surface, Surface::Work | Surface::Both) {
        let mut query=conn.prepare(&format!("SELECT w.channel_thread_id,w.id,p.owner,p.repo FROM work_thread_links w JOIN github_pull_requests p ON p.id=w.github_pull_request_id WHERE w.channel_thread_id IN ({slots}) AND w.kind='pull_request' AND (p.private IS NULL OR p.private!=0) ORDER BY w.id"))?;
        for row in query.query_map(rusqlite::params_from_iter(threads), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })? {
            let (thread, id, owner, repo) = row?;
            work.entry(thread).or_default().push((id, owner, repo));
        }
    }
    let mut messages: HashMap<i64, (String, String)> = HashMap::new();
    if matches!(surface, Surface::Messages | Surface::Both) {
        let mut query=conn.prepare(&format!("SELECT t.channel_thread_id,p.owner,p.repo FROM github_pull_request_threads t JOIN github_pull_requests p ON p.id=t.github_pull_request_id WHERE t.channel_thread_id IN ({slots}) AND (p.private IS NULL OR p.private!=0) ORDER BY t.id"))?;
        for row in query.query_map(rusqlite::params_from_iter(threads), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (thread, owner, repo) = row?;
            messages.entry(thread).or_insert((owner, repo));
        }
    }
    // Plain/public work needs no repository identity. Keep owner/account facts
    // current for actual private links, after loading their bounded candidate set.
    if work.is_empty() && messages.is_empty() {
        return Ok(vec![]);
    }
    let Some(user) = Agent::find(conn, agent)?.and_then(|a| a.owner_id) else {
        return Ok(vec![]);
    };
    let account = conn
        .query_row(
            "SELECT id FROM github_connected_accounts WHERE user_id=?",
            [user],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    let Some(account) = account else {
        return Ok(vec![]);
    };
    if !current(conn, agent, user, account)? {
        return Ok(vec![]);
    }
    let mut result = vec![];
    let mut seen = HashSet::new();
    for &thread in threads {
        if !seen.insert(thread) {
            continue;
        }
        for (id, owner, repo) in work.remove(&thread).unwrap_or_default() {
            result.push((
                thread,
                RepositoryEntry::WorkLink(id),
                RepositoryRequest {
                    account_id: account,
                    user_id: user,
                    owner: owner.to_lowercase(),
                    repo: repo.to_lowercase(),
                },
            ));
        }
        if let Some((owner, repo)) = messages.remove(&thread) {
            result.push((
                thread,
                RepositoryEntry::Thread(thread),
                RepositoryRequest {
                    account_id: account,
                    user_id: user,
                    owner: owner.to_lowercase(),
                    repo: repo.to_lowercase(),
                },
            ));
        }
    }
    Ok(result)
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
}

#[cfg(test)]
mod live_tests;

#[cfg(test)]
mod bot_plaintext_cases;

#[cfg(test)]
mod review_tests;

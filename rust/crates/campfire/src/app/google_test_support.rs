//! Observe real queue INSERTs even when a fast registered consumer deletes the row.
//! The observation shares the triggering transaction, so rollback removes it too.
use crate::controllers::presenters::test_support::TestApp;
pub async fn observe_jobs(a: &TestApp) {
    a.db().write(|tx|{tx.conn().execute_batch("CREATE TABLE ws14g_emitted_jobs(id INTEGER PRIMARY KEY AUTOINCREMENT,job_class TEXT NOT NULL,arguments TEXT NOT NULL); CREATE TRIGGER ws14g_observe_job AFTER INSERT ON background_jobs BEGIN INSERT INTO ws14g_emitted_jobs(job_class,arguments) VALUES(NEW.job_class,NEW.arguments); END;")?;Ok(())}).await.unwrap();
}

/// Wake on queue writes, then read committed state. No busy read loop competes
/// with the runner on CPU-limited CI. The writer barrier also fences DELETE's
/// pre-commit update hook, so a successful drain means the outcomes committed.
pub struct QueueDrain {
    changed: tokio::sync::mpsc::UnboundedReceiver<()>,
}
impl QueueDrain {
    pub async fn install(a: &TestApp) -> Self {
        let (send, changed) = tokio::sync::mpsc::unbounded_channel();
        a.db()
            .write(move |tx| {
                tx.conn()
                    .update_hook(Some(move |_, _: &str, table: &str, _| {
                        if table == "background_jobs" {
                            let _ = send.send(());
                        }
                    }));
                Ok(())
            })
            .await
            .unwrap();
        Self { changed }
    }
    pub async fn calendar(&mut self, a: &TestApp) {
        loop {
            // Consume old notifications before the writer barrier, never after it.
            while self.changed.try_recv().is_ok() {}
            let rows = a.db().write(|tx| Ok(tx.conn().prepare("SELECT job_class,status,attempts,last_error FROM background_jobs WHERE job_class IN ('Calendar::InboundSyncJob','Calendar::MeetLinkJob','Calendar::SyncEntryJob')")?.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<String>>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
            if rows.is_empty() {
                break;
            }
            assert!(
                rows.iter()
                    .all(|(_, status, attempts, _)| status != "failed" && *attempts <= 1),
                "Calendar drain failed or retried: {rows:?}"
            );
            self.changed.recv().await.expect("queue observer closed");
        }
        a.db()
            .write(|tx| {
                tx.conn()
                    .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>);
                Ok(())
            })
            .await
            .unwrap();
    }
}

impl QueueDrain {
    /// Fence update notifications and wait for the actual retry schedule or committed deletion.
    pub async fn cleanup_attempt(
        &mut self,
        a: &TestApp,
        attempt: u32,
    ) -> Option<campfire_db::Timestamp> {
        use rusqlite::OptionalExtension;
        loop {
            while self.changed.try_recv().is_ok() {}
            let row=a.db().write(|tx| Ok(tx.conn().query_row("SELECT status,attempts,run_at,last_error FROM background_jobs WHERE job_class='Calendar::DisconnectCleanupJob'",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?,r.get::<_,campfire_db::Timestamp>(2)?,r.get::<_,Option<String>>(3)?))).optional()?)).await.unwrap();
            match row {
                None => return None,
                Some((status, count, at, _)) if status == "ready" && count == attempt => {
                    return Some(at);
                }
                Some((status, _, _, error)) if status == "failed" => {
                    panic!("cleanup failed: {error:?}")
                }
                _ => {
                    self.changed.recv().await.expect("queue observer closed");
                }
            }
        }
    }
}

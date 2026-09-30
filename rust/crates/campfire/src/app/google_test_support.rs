//! Observe real queue INSERTs even when a fast registered consumer deletes the row.
//! The observation shares the triggering transaction, so rollback removes it too.
use crate::controllers::presenters::test_support::TestApp;
pub async fn observe_jobs(a: &TestApp) {
    a.db().write(|tx|{tx.conn().execute_batch("CREATE TABLE ws14g_emitted_jobs(id INTEGER PRIMARY KEY AUTOINCREMENT,job_class TEXT NOT NULL,arguments TEXT NOT NULL); CREATE TRIGGER ws14g_observe_job AFTER INSERT ON background_jobs BEGIN INSERT INTO ws14g_emitted_jobs(job_class,arguments) VALUES(NEW.job_class,NEW.arguments); END;")?;Ok(())}).await.unwrap();
}

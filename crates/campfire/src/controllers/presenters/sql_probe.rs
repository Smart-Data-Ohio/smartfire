//! Test-only statement tracing on the real writer and every pooled reader.
//! This observes executions (including cached statements), not SQL preparation.
use campfire_db::{Connection, Database};
use std::ffi::{CStr, c_void};
use std::sync::{Arc, Barrier, Mutex};
type Hook = Arc<dyn Fn(&str, bool) + Send + Sync>;
#[derive(Clone, Debug)]
pub(crate) struct Statement {
    pub sql: String,
    pub transaction: bool,
}
struct State {
    statements: Mutex<Vec<Statement>>,
    hook: Option<Hook>,
}
unsafe extern "C" fn trace(_: u32, data: *mut c_void, stmt: *mut c_void, _: *mut c_void) -> i32 {
    // SQLite calls this only while the owning connection executes a statement. The
    // boxed state stays alive until every connection has had its callback removed.
    let state = unsafe { &*data.cast::<State>() };
    let stmt = stmt.cast::<rusqlite::ffi::sqlite3_stmt>();
    let sql = unsafe { rusqlite::ffi::sqlite3_sql(stmt) };
    if sql.is_null() {
        return 0;
    }
    let sql = unsafe { CStr::from_ptr(sql) }
        .to_string_lossy()
        .into_owned();
    let transaction = unsafe {
        rusqlite::ffi::sqlite3_get_autocommit(rusqlite::ffi::sqlite3_db_handle(stmt)) == 0
    };
    state.statements.lock().unwrap().push(Statement {
        sql: sql.clone(),
        transaction,
    });
    if let Some(hook) = &state.hook {
        hook(&sql, transaction);
    }
    0
}
fn install(conn: &Connection, data: usize) {
    // These test connections are exclusively checked out. rusqlite doesn't expose
    // trace_v2 with &Connection; use the matching SQLite API without changing the DB seam.
    let result = unsafe {
        rusqlite::ffi::sqlite3_trace_v2(
            conn.handle(),
            if data == 0 {
                0
            } else {
                rusqlite::ffi::SQLITE_TRACE_STMT
            },
            if data == 0 { None } else { Some(trace) },
            data as *mut c_void,
        )
    };
    assert_eq!(result, rusqlite::ffi::SQLITE_OK);
}
async fn readers(db: &Database, count: usize, data: usize) {
    let barrier = Arc::new(Barrier::new(count));
    let mut tasks = Vec::new();
    for _ in 0..count {
        let db = db.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            db.read(move |conn| {
                install(conn, data);
                barrier.wait();
                Ok(())
            })
            .await
            .unwrap()
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
}
pub(crate) struct SqlProbe {
    db: Database,
    reader_count: usize,
    state: Option<Box<State>>,
}
impl SqlProbe {
    pub async fn start(db: &Database, reader_count: usize, hook: Option<Hook>) -> Self {
        let mut state = Box::new(State {
            statements: Mutex::new(Vec::new()),
            hook,
        });
        let data = (&mut *state as *mut State) as usize;
        readers(db, reader_count, data).await;
        db.write(move |tx| {
            install(tx.conn(), data);
            Ok(())
        })
        .await
        .unwrap();
        state.statements.lock().unwrap().clear();
        Self {
            db: db.clone(),
            reader_count,
            state: Some(state),
        }
    }
    pub async fn finish(mut self) -> Vec<Statement> {
        let statements = self
            .state
            .as_ref()
            .unwrap()
            .statements
            .lock()
            .unwrap()
            .clone();
        self.db
            .write(|tx| {
                install(tx.conn(), 0);
                Ok(())
            })
            .await
            .unwrap();
        readers(&self.db, self.reader_count, 0).await;
        self.state.take();
        statements
    }
}
impl Drop for SqlProbe {
    fn drop(&mut self) {
        // If an assertion panics, the connections may still execute their callbacks.
        // Retain the tiny diagnostic allocation in that failure path rather than
        // leaving dangling SQLite userdata. Successful tests detach and free it.
        if let Some(state) = self.state.take() {
            let _ = Box::leak(state);
        }
    }
}

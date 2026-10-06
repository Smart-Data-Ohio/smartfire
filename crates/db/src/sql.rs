use rusqlite::{Connection, Params, Row};

use crate::error::Result;

/// `Connection::execute` and `Connection::query_row` through the connection's statement cache,
/// so a query is compiled once per connection rather than on every call (Active Record keeps a
/// prepared-statement cache per connection too).
pub trait CachedStatements {
    fn execute_cached(&self, sql: &str, params: impl Params) -> rusqlite::Result<usize>;

    fn query_row_cached<T>(
        &self,
        sql: &str,
        params: impl Params,
        map: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T>;
}

impl CachedStatements for Connection {
    fn execute_cached(&self, sql: &str, params: impl Params) -> rusqlite::Result<usize> {
        self.prepare_cached(sql)?.execute(params)
    }

    fn query_row_cached<T>(
        &self,
        sql: &str,
        params: impl Params,
        map: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        self.prepare_cached(sql)?.query_row(params, map)
    }
}

/// `?, ?, ?`
pub fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

pub fn query_all<T>(
    conn: &Connection,
    sql: &str,
    params: impl Params,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, map)?;
    Ok(rows.collect::<rusqlite::Result<Vec<T>>>()?)
}

pub fn query_one<T>(
    conn: &Connection,
    sql: &str,
    params: impl Params,
    map: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Option<T>> {
    use rusqlite::OptionalExtension;
    let mut stmt = conn.prepare_cached(sql)?;
    Ok(stmt.query_row(params, map).optional()?)
}

pub fn count(conn: &Connection, sql: &str, params: impl Params) -> Result<i64> {
    Ok(conn.prepare_cached(sql)?.query_row(params, |r| r.get(0))?)
}

pub fn exists(conn: &Connection, sql: &str, params: impl Params) -> Result<bool> {
    Ok(conn.prepare_cached(sql)?.exists(params)?)
}

/// `SecureRandom.alphanumeric(n)`
pub fn alphanumeric(n: usize) -> String {
    #[cfg(test)]
    if let Some(value) = FIXTURE_ALPHANUMERIC.with(|fixture| {
        fixture.borrow_mut().as_mut().map(|(values, calls)| {
            calls.push(n);
            values.pop_front().expect("fixture entropy exhausted")
        })
    }) {
        return value;
    }

    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..n)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}

/// `SecureRandom.base58(n)`, as `has_secure_token` uses it.
pub fn base58(n: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut rng = rand::rng();
    (0..n)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}

/// `SecureRandom.uuid` / `Random.uuid`
pub fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
type FixtureAlphanumeric = (std::collections::VecDeque<String>, Vec<usize>);
#[cfg(test)]
thread_local! { static FIXTURE_ALPHANUMERIC: std::cell::RefCell<Option<FixtureAlphanumeric>> = const {std::cell::RefCell::new(None)}; }
#[cfg(test)]
pub(crate) fn with_fixture_alphanumeric<T>(
    tokens: &[&str],
    run: impl FnOnce() -> T,
) -> (T, Vec<usize>) {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            FIXTURE_ALPHANUMERIC.with(|v| {
                v.borrow_mut().take();
            });
        }
    }
    FIXTURE_ALPHANUMERIC.with(|v| {
        assert!(v.borrow().is_none());
        *v.borrow_mut() = Some((tokens.iter().map(|v| (*v).into()).collect(), vec![]));
    });
    let _clear = Clear;
    let value = run();
    let calls = FIXTURE_ALPHANUMERIC.with(|v| v.borrow().as_ref().unwrap().1.clone());
    (value, calls)
}

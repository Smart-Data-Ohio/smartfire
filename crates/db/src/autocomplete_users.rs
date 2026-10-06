//! Active user mention picker, capped in SQL, with uniqueness over the complete room scope.
use crate::{Connection, Result, User};
use rusqlite::{params_from_iter, types::Value};
use std::collections::HashSet;
fn scope(room: Option<i64>) -> (String, Vec<Value>) {
    if let Some(id) = room {
        ( "FROM users u JOIN memberships mem ON mem.user_id=u.id WHERE mem.room_id=? AND u.status=0".into(),vec![Value::Integer(id)])
    } else {
        ("FROM users u WHERE u.status=0".into(), vec![])
    }
}
pub fn count(conn: &Connection, room: Option<i64>, query: Option<&str>) -> Result<i64> {
    let (mut sql, mut values) = scope(room);
    if let Some(query) = query {
        sql.push_str(" AND u.name LIKE ?");
        values.push(Value::Text(format!("%{query}%")));
    }
    Ok(conn.query_row(
        &format!("SELECT COUNT(*) {sql}"),
        params_from_iter(values),
        |r| r.get(0),
    )?)
}
pub fn page(
    conn: &Connection,
    room: Option<i64>,
    query: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<User>, HashSet<String>)> {
    let (mut sql, mut values) = scope(room);
    if let Some(query) = query {
        sql.push_str(" AND u.name LIKE ?");
        values.push(Value::Text(format!("%{query}%")));
    }
    values.extend([Value::Integer(limit), Value::Integer(offset)]);
    let users = conn
        .prepare(&format!(
            "SELECT u.* {sql} ORDER BY LOWER(u.name) LIMIT ? OFFSET ?"
        ))?
        .query_map(params_from_iter(values), User::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if users.is_empty() {
        return Ok((users, HashSet::new()));
    }
    let (sql, mut values) = scope(room);
    values.extend(users.iter().map(|u| Value::Text(u.name.clone())));
    let unique = conn
        .prepare(&format!(
            "SELECT u.name {sql} AND u.name IN ({}) GROUP BY u.name HAVING COUNT(*)=1",
            vec!["?"; users.len()].join(",")
        ))?
        .query_map(params_from_iter(values), |r| r.get(0))?
        .collect::<rusqlite::Result<HashSet<String>>>()?;
    Ok((users, unique))
}

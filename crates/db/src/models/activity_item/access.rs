//! ActivityItem.accessible_to and ActivityItemsController's ordered/filter/cursor query.
use crate::sql::query_all;
use crate::{ActivityItem, Connection, Result, User};
use rusqlite::types::Value;

/// Unfiltered is useful for counts and source lookups; the inbox defaults belong to its controller.
#[derive(Clone, Debug, Default)]
pub struct ActivityQuery<'a> {
    pub state: Option<&'a str>,
    pub type_filter: Option<&'a str>,
    pub before: Option<&'a str>,
    pub limit: Option<usize>,
}

/// The badge count and its per-user ordering token from one SQLite statement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityUnread {
    pub count: i64,
    pub revision: i64,
}

fn unmuted_activity_sql() -> String {
    let room = "COALESCE(activity_messages.room_id, activity_saved_messages.room_id, activity_work_threads.room_id, activity_sla_nudges.room_id, activity_huddle_grants.room_id, activity_events.room_id)";
    format!(" AND {}", crate::models::notification_policy::unmuted_sql(room, "users.inbox_preferences"))
}

impl ActivityItem {
    pub fn unread_snapshot(conn: &Connection, user_id: i64) -> Result<ActivityUnread> {
        let count =
            include_str!("access.sql").replacen("SELECT activity_items.*", "SELECT COUNT(*)", 1)
                + " AND activity_items.read_at IS NULL AND activity_items.handled_at IS NULL"
                + &unmuted_activity_sql();
        let sql = format!("SELECT ({count}), activity_revision FROM users WHERE id = ?1");
        Ok(crate::sql::query_one(conn, &sql, [user_id], |row| {
            Ok(ActivityUnread {
                count: row.get(0)?,
                revision: row.get(1)?,
            })
        })?
        .unwrap_or_default())
    }

    pub fn accessible_to(conn: &Connection, user: &User) -> Result<Vec<Self>> {
        Self::query_accessible(conn, user, ActivityQuery::default())
    }

    pub fn find_accessible(conn: &Connection, user: &User, id: i64) -> Result<Option<Self>> {
        if !user.is_active() || user.is_bot() {
            return Ok(None);
        }
        let sql = format!("{} AND activity_items.id=?2", include_str!("access.sql"));
        crate::sql::query_one(conn, &sql, rusqlite::params![user.id, id], Self::from_row)
    }

    pub fn unread_count(conn: &Connection, user: &User) -> Result<i64> {
        if !user.is_active() || user.is_bot() {
            return Ok(0);
        }
        let sql =
            include_str!("access.sql").replacen("SELECT activity_items.*", "SELECT COUNT(*)", 1)
                + " AND activity_items.read_at IS NULL AND activity_items.handled_at IS NULL"
                + &unmuted_activity_sql();
        Ok(conn.query_row(&sql, [user.id], |row| row.get(0))?)
    }

    pub fn query_accessible(
        conn: &Connection,
        user: &User,
        query: ActivityQuery<'_>,
    ) -> Result<Vec<Self>> {
        if !user.is_active() || user.is_bot() {
            return Ok(Vec::new());
        }
        let key = match query
            .before
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|before| before.parse().ok())
        {
            Some(id) => Self::find_accessible(conn, user, id)?.map(|row| (row.updated_at, row.id)),
            None => None,
        };
        Self::query_keyed(conn, user, &query, key)
    }

    /// The single-page app's page: [`Self::query_accessible`] with `query.before` ignored, starting
    /// strictly after the sort key `after` (`updated_at`, `id`) in `updated_at DESC, id DESC`
    /// order, whether or not that row still exists.
    pub fn query_accessible_after(
        conn: &Connection,
        user: &User,
        query: ActivityQuery<'_>,
        after: Option<(crate::Timestamp, i64)>,
    ) -> Result<Vec<Self>> {
        if !user.is_active() || user.is_bot() {
            return Ok(Vec::new());
        }
        Self::query_keyed(conn, user, &query, after)
    }

    fn query_keyed(
        conn: &Connection,
        user: &User,
        query: &ActivityQuery<'_>,
        key: Option<(crate::Timestamp, i64)>,
    ) -> Result<Vec<Self>> {
        let mut sql = include_str!("access.sql").to_string();
        let mut values = vec![Value::Integer(user.id)];
        if let Some((updated_at, id)) = key {
            sql.push_str(" AND (activity_items.updated_at < ? OR (activity_items.updated_at = ? AND activity_items.id < ?))");
            values.extend([
                Value::Text(updated_at.to_db()),
                Value::Text(updated_at.to_db()),
                Value::Integer(id),
            ]);
        }
        match query.state {
            Some("unread") => sql.push_str(
                " AND activity_items.read_at IS NULL AND activity_items.handled_at IS NULL",
            ),
            Some("read") => sql.push_str(
                " AND activity_items.read_at IS NOT NULL AND activity_items.handled_at IS NULL",
            ),
            Some("handled") => sql.push_str(" AND activity_items.handled_at IS NOT NULL"),
            _ => (),
        }
        let types: &[&str] = match query.type_filter {
            Some("mentions") => &["mention", "reply", "keyword_alert"],
            Some("threads") => &[
                "thread_activity",
                "work_update",
                "work_assignment",
                "work_sla",
            ],
            Some("events") => &[
                "event_invitation",
                "event_update",
                "event_cancelled",
                "event_reminder",
            ],
            Some("agents") => &["agent_approval_request", "agent_budget_exceeded"],
            Some("github") => &["pr_review_request"],
            Some("huddles") => &["huddle_started", "huddle_missed"],
            Some("reminders") => &["message_reminder"],
            Some("security") => &["new_sign_in", "two_factor_lockout"],
            _ => &[],
        };
        if !types.is_empty() {
            sql.push_str(&format!(
                " AND activity_items.event_type IN ({})",
                crate::sql::placeholders(types.len())
            ));
            values.extend(types.iter().map(|value| Value::Text((*value).to_string())));
        }
        sql.push_str(" ORDER BY activity_items.updated_at DESC,activity_items.id DESC");
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            values.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));
        }
        query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Self::from_row,
        )
    }
}

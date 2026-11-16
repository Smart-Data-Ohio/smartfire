//! Shared sidebar and app-icon badge counts.

use std::collections::HashMap;

use crate::{Connection, Result, Timestamp};

/// A room's counts for the viewer, from [`notification_counts`].
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct RoomCounts {
    /// Root messages in the room's unread range (`unreadCount`), as `room_unread::first_unread`
    /// counts them.
    pub unread: i64,
    /// Unread `mention` inbox items about messages in the room (`mentionCount`).
    pub mentions: i64,
    /// The red pill (`notificationCount`).
    pub notifications: i64,
    /// The part of `notifications` in threads (`threadNotificationCount`).
    pub thread_notifications: i64,
}

/// Inside the room's unread range, as `room_unread::first_unread` draws it, for a message `m`
/// against its membership's bounds `b`: the room is unread, and the message follows the last read
/// root message (or the moment it went unread, when there's no read position to go by). The
/// leading `created_at` bound follows from each case and lets SQLite seek
/// `index_messages_on_room_thread_created` instead of walking the room's whole timeline.
const IN_UNREAD_RANGE: &str = r#""b"."unread_at" IS NOT NULL AND "m"."created_at" >= COALESCE("b"."read_created_at", "b"."unread_at") AND CASE WHEN "b"."read_id" IS NULL THEN "m"."created_at" >= "b"."unread_at" WHEN "b"."read_created_at" IS NOT NULL THEN ("m"."created_at", "m"."id") > ("b"."read_created_at", "b"."read_id") ELSE ("m"."created_at", "m"."id") > ("b"."unread_at", "b"."read_id") END"#;

/// Each room's [`RoomCounts`] for the viewer, in one statement (so one read snapshot) over all
/// their rooms, or only `room_id`. Rooms with nothing to count are left out.
///
/// `unread` is every root message in the room's unread range, as the timeline's divider counts.
///
/// The red number is what classic would have pushed to the viewer and is still unread:
/// `Notifications::Policy#push` (`crates/db/src/models/notification_policy.rs`), leaving out
/// quiet hours and do-not-disturb, which only hold delivery back.
///
/// - root messages count while they're in the room's unread range, once per message, so reading
///   the room clears them: in an `everything` room every one but system notes and the viewer's
///   own; in a `mentions` room those with an unread mention or reply inbox item; in a `muted`
///   room those with an unread mention;
/// - in threads the unit is the unread inbox item, while the thread is unread for the viewer
///   (`thread_memberships.unread_at`) and posted after they last read it
///   (`thread_memberships.last_read_message_id`, so a reply that makes the thread unread again
///   doesn't bring back pings already read): classic reads a thread apart from its room, so
///   reading the room leaves them and reading the thread clears them. Keyword alerts and `nothing`
///   rooms aside, the recorder writes a thread's items exactly where the policy pushes (a
///   mention when the viewer follows the thread, every reply when they follow it with
///   `everything` in an `everything` or `mentions` room). Thread activity is grouped, one item per followed thread that moves to the
///   newest reply (`ActivityItem` recorder), so any number of ordinary replies in a thread
///   count 1; a mention or reply to you there is its own item and counts on its own. In a
///   `muted` room only mentions push;
/// - keyword alerts alone never push, so their items don't count; `nothing` and `invisible`
///   rooms never push, so they count none.
///
/// Reading the inbox item clears a ping too, since a ping counts only while it's unread both in
/// the inbox and where it was posted.
pub fn notification_counts(
    conn: &Connection,
    user_id: i64,
    room_id: Option<i64>,
    now: Timestamp,
) -> Result<HashMap<i64, RoomCounts>> {
    let mut statement = conn.prepare_cached(&notification_counts_sql(room_id.is_some()))?;
    let map = |row: &rusqlite::Row<'_>| {
        Ok((
            row.get::<_, i64>(0)?,
            RoomCounts {
                unread: row.get(1)?,
                mentions: row.get(2)?,
                notifications: row.get(3)?,
                thread_notifications: row.get(4)?,
            },
        ))
    };
    let rows = match room_id {
        Some(room_id) => statement.query_map(rusqlite::params![user_id, now, room_id], map)?,
        None => statement.query_map(rusqlite::params![user_id, now], map)?,
    };
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The counts statement: `?1` is the viewer, `?2` is now, `?3` the one room when `one_room`. `roots` and
/// `pinged` are materialized, each read once through its own index (the room's unread range on
/// `index_messages_on_room_thread_created`, the viewer's unread pings on
/// `index_activity_items_on_unread_message_pings`); inlined, SQLite would walk the room's whole
/// history by `room_id` to find them.
fn notification_counts_sql(one_room: bool) -> String {
    let visible = crate::models::ignore_policy::visible_sql("?1", "m.creator_id");
    let visible = format!(
        "({visible}) AND (m.thread_id IS NULL OR {})",
        crate::channel_thread::readable_by_sql("m.thread_id", "?1")
    );
    let only = |column: &str| match one_room {
        true => format!(r#" AND {column} = ?3"#),
        false => String::new(),
    };
    let sql = format!(
        r#"WITH "bounds" AS (SELECT "ms"."room_id", "ms"."involvement", "ms"."unread_at", "ms"."last_read_message_id" AS "read_id", "read"."created_at" AS "read_created_at" FROM "memberships" "ms" LEFT JOIN "messages" "read" ON "read"."id" = "ms"."last_read_message_id" AND "read"."room_id" = "ms"."room_id" AND "read"."thread_id" IS NULL WHERE "ms"."user_id" = ?1 AND "ms"."involvement" IN ('everything', 'mentions', 'muted', 'nothing') AND "ms"."unread_at" IS NOT NULL{bounds_only}), "roots" AS MATERIALIZED (SELECT "m"."room_id", "m"."id", "b"."involvement" = 'everything' AND NOT "m"."system_note" AND "m"."creator_id" != ?1 AS "notifies" FROM "bounds" "b" INNER JOIN "messages" "m" ON "m"."room_id" = "b"."room_id" AND "m"."thread_id" IS NULL WHERE {IN_UNREAD_RANGE} AND {visible}), "pinged" AS MATERIALIZED (SELECT "m"."room_id", "m"."id", "m"."thread_id", "activity_items"."event_type" = 'mention' AS "mention" FROM "activity_items" INNER JOIN "messages" "m" ON "m"."id" = "activity_items"."source_id" WHERE "activity_items"."user_id" = ?1 AND "activity_items"."source_type" = 'Message' AND "activity_items"."read_at" IS NULL AND "activity_items"."event_type" IN ('mention', 'reply', 'thread_activity'){pinged_only} AND {visible}), "notifying" AS (SELECT "room_id", "id", 0 AS "thread" FROM "roots" WHERE "notifies" UNION SELECT "p"."room_id", "p"."id", "p"."thread_id" IS NOT NULL FROM "pinged" "p" INNER JOIN "memberships" "ms" ON "ms"."room_id" = "p"."room_id" AND "ms"."user_id" = ?1 WHERE ("ms"."involvement" IN ('everything', 'mentions') OR ("ms"."involvement" = 'muted' AND "p"."mention")) AND CASE WHEN "p"."thread_id" IS NULL THEN "p"."id" IN (SELECT "id" FROM "roots") ELSE EXISTS (SELECT 1 FROM "thread_memberships" "tm" WHERE "tm"."thread_id" = "p"."thread_id" AND "tm"."user_id" = ?1 AND "tm"."unread_at" IS NOT NULL AND ("tm"."last_read_message_id" IS NULL OR "p"."id" > "tm"."last_read_message_id")) END) SELECT "room_id", SUM("unread"), SUM("mentions"), SUM("notifications"), SUM("threads") FROM (SELECT "room_id", 1 AS "unread", 0 AS "mentions", 0 AS "notifications", 0 AS "threads" FROM "roots" UNION ALL SELECT "room_id", 0, "mention", 0, 0 FROM "pinged" UNION ALL SELECT "room_id", 0, 0, 1, "thread" FROM "notifying") GROUP BY "room_id""#,
        bounds_only = only(r#""ms"."room_id""#),
        pinged_only = only(r#""m"."room_id""#),
    );
    let preferences = "(SELECT inbox_preferences FROM users WHERE id=?1)";
    let effective = crate::models::notification_policy::involvement_sql("ms", preferences);
    let unmuted = crate::models::notification_policy::unmuted_sql("ms.room_id", preferences, "?2");
    sql.replace(
        r#""ms"."involvement", "ms"."unread_at""#,
        &format!(r#"({effective}) AS "involvement", "ms"."unread_at""#),
    )
    .replace(r#""ms"."involvement""#, &format!("({effective})"))
    .replace(
        r#"WHERE "ms"."user_id" = ?1"#,
        &format!(r#"WHERE "ms"."user_id" = ?1 AND {unmuted}"#),
    )
    .replace(r#"WHERE ("#, &format!("WHERE {unmuted} AND ("))
}

#[cfg(test)]
mod tests {
    use super::{Timestamp, notification_counts_sql};

    /// The counts read the unread range and the viewer's unread pings through their indexes,
    /// for every room and for one: never a room's whole history by `room_id`, nor every inbox
    /// item the viewer ever had (SQLite's plan with no statistics, as production runs).
    #[test]
    fn the_counts_never_walk_a_rooms_history_or_the_whole_inbox() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::schema::prepare(&mut conn, "production", &crate::SystemClock).unwrap();
        for one_room in [false, true] {
            let sql = format!("EXPLAIN QUERY PLAN {}", notification_counts_sql(one_room));
            let mut statement = conn.prepare(&sql).unwrap();
            let now = Timestamp::parse_db("2035-01-01T12:00:00Z").unwrap();
            let mut params = vec![
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Text(now.to_db()),
            ];
            if one_room {
                params.push(rusqlite::types::Value::Integer(2));
            }
            let plan: Vec<String> = statement
                .query_map(rusqlite::params_from_iter(params), |row| {
                    row.get::<_, String>(3)
                })
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            let text = plan.join("\n");
            assert!(
                !text.contains("index_messages_on_room_id "),
                "one_room={one_room}:\n{text}"
            );
            assert!(!text.contains("SCAN m"), "one_room={one_room}:\n{text}");
            assert!(
                !text.contains("SCAN activity_items"),
                "one_room={one_room}:\n{text}"
            );
            assert!(
                text.contains(
                    "SEARCH activity_items USING INDEX index_activity_items_on_unread_message_pings"
                ),
                "one_room={one_room}:\n{text}"
            );
            assert!(
                text.contains("SEARCH m USING INDEX index_messages_on_room_thread_created")
                    || text.contains(
                        "SEARCH m USING COVERING INDEX index_messages_on_room_thread_created"
                    ),
                "one_room={one_room}:\n{text}"
            );
            assert!(
                super::notification_counts(&conn, 1, one_room.then_some(2), now)
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

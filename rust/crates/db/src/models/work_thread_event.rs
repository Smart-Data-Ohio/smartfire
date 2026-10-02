//! app/models/work_thread_event.rb: immutable work history and after-create inbox fanout.
use crate::sql::{query_all, query_one};
use crate::{ActivityItem, ChannelThread, Errors, Result, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub const EVENT_TYPES: [&str; 4] = [
    "work_update",
    "work_assignment",
    "work_handoff",
    "result_updated",
];

#[derive(Debug, Clone)]
pub struct WorkThreadEvent {
    pub id: i64,
    pub channel_thread_id: i64,
    pub actor_id: Option<i64>,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub from_owner_id: Option<i64>,
    pub to_owner_id: Option<i64>,
    pub from_owner_name: Option<String>,
    pub to_owner_name: Option<String>,
    pub metadata: Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl WorkThreadEvent {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            channel_thread_id: row.get("channel_thread_id")?,
            actor_id: row.get("actor_id")?,
            event_type: row.get("event_type")?,
            from_status: row.get("from_status")?,
            to_status: row.get("to_status")?,
            from_owner_id: row.get("from_owner_id")?,
            to_owner_id: row.get("to_owner_id")?,
            from_owner_name: row.get("from_owner_name")?,
            to_owner_name: row.get("to_owner_name")?,
            metadata: row
                .get::<_, Option<Value>>("metadata")?
                .unwrap_or(Value::Null),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM work_thread_events WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_thread(conn: &Connection, thread_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM work_thread_events WHERE channel_thread_id=? ORDER BY created_at DESC,id DESC",
            [thread_id],
            Self::from_row,
        )
    }
    pub fn create_for_change(
        tx: &mut Tx<'_>,
        before: &ChannelThread,
        after: &ChannelThread,
        actor: Option<&User>,
        note: Option<&str>,
    ) -> Result<Option<Self>> {
        if before.work_status == after.work_status && before.work_owner_id == after.work_owner_id {
            return Ok(None);
        }
        let from = owner(tx.conn(), before.work_owner_id)?;
        let to = owner(tx.conn(), after.work_owner_id)?;
        let kind = if before.work_status == after.work_status {
            "work_assignment"
        } else {
            "work_update"
        };
        let metadata = json!({"before":{"status":before.work_status,"owner":snapshot(from.as_ref())},
            "after":{"status":after.work_status,"owner":snapshot(to.as_ref())},"actor":snapshot(actor),
            "note":note.filter(|value| !campfire_richtext::ruby::is_blank(value))});
        Self::insert(
            tx,
            after,
            actor,
            kind,
            before.work_status.as_deref(),
            from.as_ref(),
            to.as_ref(),
            metadata,
        )
        .map(Some)
    }
    pub fn create_for_result(
        tx: &mut Tx<'_>,
        thread: &ChannelThread,
        actor: &User,
        excerpt: &str,
    ) -> Result<Self> {
        let owner = owner(tx.conn(), thread.work_owner_id)?;
        Self::insert(
            tx,
            thread,
            Some(actor),
            "result_updated",
            thread.work_status.as_deref(),
            owner.as_ref(),
            owner.as_ref(),
            json!({"excerpt":excerpt,"actor":snapshot(Some(actor))}),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn insert(
        tx: &mut Tx<'_>,
        thread: &ChannelThread,
        actor: Option<&User>,
        kind: &str,
        from_status: Option<&str>,
        from_owner: Option<&User>,
        to_owner: Option<&User>,
        metadata: Value,
    ) -> Result<Self> {
        ChannelThread::find(tx.conn(), thread.id)?;
        let mut errors = Errors::default();
        if !EVENT_TYPES.contains(&kind) {
            errors.add("event_type", "is not included in the list");
        }
        errors.into_result()?;
        let id = tx.conn().query_row("INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id,from_owner_name,to_owner_name,metadata,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",
            params![thread.id,actor.map(|u|u.id),kind,from_status,thread.work_status,from_owner.map(|u|u.id),to_owner.map(|u|u.id),from_owner.map(|u|u.name.as_str()),to_owner.map(|u|u.name.as_str()),metadata,tx.now(),tx.now()], |r|r.get::<_,i64>(0))?;
        tx.after_commit_record("work_thread_events", id, move |tx| {
            let Some(event) = Self::find_by_id(tx.conn(), id)? else {
                return Ok(());
            };
            event.record_activity_items(tx)
        });
        Self::find_by_id(tx.conn(), id)?.ok_or(crate::Error::RecordNotFound("WorkThreadEvent"))
    }
    pub fn recipient_user_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        Ok(self
            .recipient_users(conn)?
            .into_iter()
            .map(|user| user.id)
            .collect())
    }

    // app/models/work_thread_event.rb memoizes the authorized roster for one fanout and
    // preloads users. Keep this snapshot local to the event; standalone recorder calls
    // still check the current source permissions.
    fn recipient_users(&self, conn: &Connection) -> Result<Vec<User>> {
        let thread = ChannelThread::find(conn, self.channel_thread_id)?;
        let memberships = crate::Membership::for_room(conn, thread.room_id)?;
        let followers = crate::ThreadMembership::for_thread(conn, thread.id)?;
        let memberships = memberships
            .iter()
            .map(|m| (m.user_id, m))
            .collect::<HashMap<_, _>>();
        let follower_involvements = followers
            .iter()
            .map(|m| (m.user_id, m.involvement))
            .collect::<HashMap<_, _>>();
        let mut candidates = vec![
            Some(thread.creator_id),
            self.from_owner_id,
            self.to_owner_id,
        ];
        if self.event_type != "result_updated" {
            // Preserve the Rails candidate order from ThreadMembership::for_thread.
            candidates.extend(
                followers
                    .iter()
                    .filter(|m| m.involvement == crate::ThreadInvolvement::Everything)
                    .map(|m| Some(m.user_id)),
            );
        }
        let mut seen = HashSet::new();
        let ids = candidates
            .into_iter()
            .flatten()
            .filter(|id| {
                Some(*id) != self.actor_id
                    && seen.insert(*id)
                    && memberships.get(id).is_some_and(|membership| {
                        !matches!(
                            membership.involvement,
                            Some(crate::Involvement::Invisible | crate::Involvement::Nothing)
                        )
                    })
                    && follower_involvements.get(id) != Some(&crate::ThreadInvolvement::Nothing)
            })
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut users = User::where_ids(conn, &ids)?
            .into_iter()
            .map(|user| (user.id, user))
            .collect::<HashMap<_, _>>();
        Ok(ids
            .into_iter()
            .filter_map(|id| users.remove(&id))
            .filter(|user| user.is_active() && !user.is_bot())
            .collect())
    }
    fn record_activity_items(&self, tx: &mut Tx<'_>) -> Result<()> {
        let kind = match self.event_type.as_str() {
            "result_updated" => "work_update",
            "work_handoff" => "work_assignment",
            value => value,
        };
        let agent_assignment = self.event_type == "work_assignment"
            && crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agents WHERE user_id=?",
                [self.actor_id],
            )?;
        for user in self.recipient_users(tx.conn())? {
            if agent_assignment {
                let raw: Option<Value> = tx.conn().query_row(
                    "SELECT inbox_preferences FROM users WHERE id=?",
                    [user.id],
                    |r| r.get(0),
                )?;
                if raw
                    .as_ref()
                    .and_then(|v| v.get("agent_work"))
                    .is_some_and(|v| {
                        matches!(v, Value::Bool(false))
                            || *v == json!(0)
                            || *v == json!("0")
                            || *v == json!("false")
                    })
                {
                    continue;
                }
            }
            // Rails commits each recipient separately after committing the work change.
            crate::database::run_write(tx.conn(), tx.env(), |tx| {
                ActivityItem::record_authorized_work_event(tx, &user, self, kind)?;
                Ok(())
            })?;
        }
        Ok(())
    }
}
fn owner(conn: &Connection, id: Option<i64>) -> Result<Option<User>> {
    id.map(|id| User::find_by_id(conn, id))
        .transpose()
        .map(Option::flatten)
}
fn snapshot(user: Option<&User>) -> Value {
    user.map(|u| json!({"id":u.id,"name":u.name,"status":u.status.name(),"role":u.role.name()}))
        .unwrap_or(Value::Null)
}

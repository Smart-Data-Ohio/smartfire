//! WorkThreadsController's unbounded workspace relation, independent of HTTP rendering.
use super::*;

/// Read-only page snapshot for the work index's JSON rows. These facts are not write
/// authority: mutations continue to check the live model policies in their transaction.
pub struct WorkReadFacts {
    rooms: HashMap<i64, Room>,
    users: HashMap<i64, User>,
    owners: WorkOwners,
    memberships: HashMap<i64, ThreadMembership>,
    message_counts: HashMap<i64, i64>,
    member_counts: HashMap<i64, i64>,
    viewer_rooms: HashSet<i64>,
    viewer: User,
    custom_icons: HashSet<String>,
}

pub struct WorkReadPermissions {
    pub settings: bool,
    pub lifecycle: bool,
    pub assignment: bool,
    pub manageable: bool,
}

impl WorkReadFacts {
    pub fn custom_icon(&self, name: &str) -> bool {
        self.custom_icons.contains(name)
    }

    pub fn room(&self, thread: &ChannelThread) -> Result<&Room> {
        self.rooms.get(&thread.room_id).or_not_found("Room")
    }

    pub fn user(&self, id: i64) -> Result<&User> {
        self.users.get(&id).or_not_found("User")
    }

    pub fn owner_active(&self, thread: &ChannelThread) -> bool {
        self.owners.available(thread)
    }

    pub fn membership(&self, thread: &ChannelThread) -> Option<&ThreadMembership> {
        self.memberships.get(&thread.id)
    }

    pub fn message_count(&self, thread: &ChannelThread) -> i64 {
        self.message_counts.get(&thread.id).copied().unwrap_or(0)
    }

    pub fn member_count(&self, thread: &ChannelThread) -> i64 {
        self.member_counts.get(&thread.id).copied().unwrap_or(0)
    }

    pub fn permissions(&self, thread: &ChannelThread) -> Result<WorkReadPermissions> {
        let lifecycle =
            self.viewer.is_administrator() || self.viewer.id == self.room(thread)?.creator_id;
        let settings = lifecycle || self.viewer.id == thread.creator_id;
        let viewable = self.viewer.is_active()
            && !self.viewer.is_bot()
            && self.viewer_rooms.contains(&thread.room_id);
        Ok(WorkReadPermissions {
            settings,
            lifecycle,
            assignment: viewable && settings,
            manageable: viewable && (settings || thread.work_owner_id == Some(self.viewer.id)),
        })
    }
}

impl ChannelThread {
    /// Preload every per-row fact once, including distinct rooms, owners and agent grants.
    pub fn work_read_facts(
        conn: &Connection,
        threads: &[Self],
        viewer: &User,
    ) -> Result<WorkReadFacts> {
        let ids = threads.iter().map(|thread| thread.id).collect::<Vec<_>>();
        let room_ids = threads
            .iter()
            .map(|thread| thread.room_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let user_ids = threads
            .iter()
            .flat_map(|thread| std::iter::once(thread.creator_id).chain(thread.work_owner_id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let rooms = Room::for_ids(conn, &room_ids)?
            .into_iter()
            .map(|room| (room.id, room))
            .collect();
        let users: HashMap<i64, User> = if user_ids.is_empty() {
            HashMap::new()
        } else {
            User::where_ids(conn, &user_ids)?
                .into_iter()
                .map(|user| (user.id, user))
                .collect()
        };
        let icon_names = users
            .values()
            .filter_map(|user| user.icon_name.as_ref())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let custom_icons = if icon_names.is_empty() {
            HashSet::new()
        } else {
            query_all(
                conn,
                "SELECT name FROM workspace_icons WHERE name IN (SELECT value FROM json_each(?))",
                [serde_json::json!(icon_names).to_string()],
                |row| row.get(0),
            )?
            .into_iter()
            .collect()
        };
        let owners = Self::work_owners(conn, threads)?;
        let (memberships, member_counts, viewer_rooms) = if ids.is_empty() {
            (HashMap::new(), HashMap::new(), HashSet::new())
        } else {
            let ids_json = serde_json::json!(ids).to_string();
            let memberships = query_all(conn,
                "SELECT * FROM thread_memberships WHERE user_id=? AND thread_id IN (SELECT value FROM json_each(?))",
                params![viewer.id, ids_json], ThreadMembership::from_row)?
                .into_iter().map(|member| (member.thread_id, member)).collect();
            let member_counts = query_all(conn,
                "SELECT thread_id,COUNT(*) FROM thread_memberships WHERE thread_id IN (SELECT value FROM json_each(?)) GROUP BY thread_id",
                [&ids_json], |row| Ok((row.get(0)?, row.get(1)?)))?.into_iter().collect();
            let viewer_rooms = Membership::for_user(conn, viewer.id)?
                .into_iter()
                .map(|member| member.room_id)
                .collect();
            (memberships, member_counts, viewer_rooms)
        };
        Ok(WorkReadFacts {
            rooms,
            users,
            owners,
            memberships,
            member_counts,
            viewer_rooms,
            message_counts: Self::board_reply_counts(conn, &ids)?,
            viewer: viewer.clone(),
            custom_icons,
        })
    }

    pub fn visible_work_threads(
        conn: &Connection,
        viewer: &User,
        state: &str,
    ) -> Result<Vec<Self>> {
        if !viewer.is_active() || viewer.is_bot() {
            return Ok(Vec::new());
        }
        let predicate = match state {
            "all" => "",
            "done" => " AND channel_threads.work_status='done'",
            "agents" => " AND channel_threads.work_owner_id IN (SELECT user_id FROM agents)",
            "boards" => {
                " AND channel_threads.room_id IN (SELECT id FROM rooms WHERE type='Rooms::Board')"
            }
            _ => " AND channel_threads.work_status!='done'",
        };
        query_all(
            conn,
            &format!(
                "SELECT channel_threads.* FROM channel_threads WHERE work_status IS NOT NULL AND room_id IN (SELECT room_id FROM memberships WHERE user_id=?){predicate} ORDER BY updated_at DESC,id DESC"
            ),
            [viewer.id],
            Self::from_row,
        )
    }
}

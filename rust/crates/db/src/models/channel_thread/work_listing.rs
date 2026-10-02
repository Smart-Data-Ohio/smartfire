//! WorkThreadsController's unbounded workspace relation, independent of HTTP rendering.
use super::*;

impl ChannelThread {
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

//! Read-side work policy and owner choices, shared by posts and human work controls.
use super::ChannelThread;
use crate::{Agent, Membership, Result, User};
use rusqlite::Connection;

impl ChannelThread {
    pub fn work_viewable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(user.is_active()
            && !user.is_bot()
            && Membership::find_by_room_and_user(conn, self.room_id, user.id)?.is_some())
    }

    pub fn work_manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(self.work_viewable_by(conn, user)?
            && (self.settings_manageable_by(conn, user)? || self.work_owner_id == Some(user.id)))
    }

    pub fn work_assignment_manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(self.work_viewable_by(conn, user)? && self.settings_manageable_by(conn, user)?)
    }

    pub fn work_owner_candidates_for(
        conn: &Connection,
        room_id: i64,
    ) -> Result<(Vec<User>, Vec<User>)> {
        let mut humans = Vec::new();
        let mut agents = Vec::new();
        for membership in Membership::for_room(conn, room_id)? {
            let user = User::find(conn, membership.user_id)?;
            if !user.is_active() {
                continue;
            }
            if user.is_bot() {
                if let Some(agent) = Agent::for_user(conn, user.id)?
                    && agent.active(conn)?
                    && agent.can(conn, "post_messages", Some(room_id))?
                {
                    agents.push(user);
                }
            } else {
                humans.push(user);
            }
        }
        humans.sort_by_key(|user| user.name.to_lowercase());
        agents.sort_by_key(|user| user.name.to_lowercase());
        Ok((humans, agents))
    }
}

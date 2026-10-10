//! User's hard destroy, preserving `dependent: :delete` versus `:destroy`.
//! WS13/14/15/16 provide their own dependency callbacks at these flagged phases.
use crate::sql::query_all;
use crate::{
    AgentGrant, Attachment, Boost, ChannelThread, Event, Message, MessagePin, Result, RoomCategory,
    ScheduledMessage, Session, TwoFactorCredential, Tx, User,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyPhase {
    /// Prepend huddle revocation; run remote preparations before entering this writer.
    BeforeDestroy,
    /// Google/Calendar/GitHub/Fizzy/Slack and event entries, in User declaration order.
    Connections,
}
impl User {
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_with_dependencies(tx, |tx, user, phase| {
            use crate::callbacks::Phase;
            let phases: &[Phase] = match phase {
                DependencyPhase::BeforeDestroy => &[Phase::UserHuddles],
                DependencyPhase::Connections => &[Phase::UserGoogleAccount, Phase::UserCalendarMeetingCache,
                    Phase::UserGoogleIdentity, Phase::UserGithubAccount, Phase::UserFizzyAccount,
                    Phase::UserSlackConnection, Phase::UserSlackImports, Phase::UserEventCalendarEntries],
            };
            for &phase in phases { tx.model_callback(phase, user.id)?; }
            Ok(())
        })
    }
    /// Shared hard-removal operation. Dependency adapters must do no network I/O here.
    /// A missing peer leaves its FK rows intact: failure rolls this entire model operation back.
    pub fn destroy_with_dependencies(
        &self,
        tx: &mut Tx<'_>,
        mut dependencies: impl FnMut(&mut Tx<'_>, &User, DependencyPhase) -> Result<()>,
    ) -> Result<()> {
        tx.savepoint(|tx| {
            AgentGrant::revoke_for_user(tx, self.id)?;
            dependencies(tx, self, DependencyPhase::BeforeDestroy)?;
            if let Some(attachment) = Attachment::find_for(tx.conn(), "User", self.id, "avatar")? {
                attachment.delete(tx)?;
                tx.emit_after_commit(Event::PurgeBlob {
                    blob_id: attachment.blob_id,
                });
            }
            // User::Bot registers these associations before User's core ones.
            // Do not call Agent::destroy: Rails deletes the Agent row and preserves its rows.
            tx.conn()
                .execute("DELETE FROM webhooks WHERE user_id=?", [self.id])?;
            tx.conn()
                .execute("DELETE FROM agents WHERE user_id=?", [self.id])?;
            tx.conn().execute(
                "DELETE FROM user_stars WHERE user_id=? OR starred_user_id=?",
                [self.id, self.id],
            )?;
            for table in ["keyword_alerts", "dnd_allowed_users"] {
                tx.conn()
                    .execute(&format!("DELETE FROM {table} WHERE user_id=?"), [self.id])?;
            }
            if let Some(credential) = TwoFactorCredential::for_user(tx.conn(), self.id)? {
                credential.destroy(tx)?;
            }
            tx.conn().execute(
                "DELETE FROM two_factor_remembered_devices WHERE user_id=?",
                [self.id],
            )?;
            tx.conn()
                .execute("DELETE FROM memberships WHERE user_id=?", [self.id])?;
            for message in Message::by_creator(tx.conn(), self.id)? {
                message.destroy(tx)?;
            }
            for id in query_all(
                tx.conn(),
                "SELECT id FROM channel_threads WHERE creator_id=? ORDER BY id",
                [self.id],
                |r| r.get::<_, i64>(0),
            )? {
                ChannelThread::find(tx.conn(), id)?.destroy(tx)?;
            }
            tx.conn()
                .execute("DELETE FROM thread_memberships WHERE user_id=?", [self.id])?;
            tx.conn()
                .execute("DELETE FROM push_subscriptions WHERE user_id=?", [self.id])?;
            dependencies(tx, self, DependencyPhase::Connections)?;
            for id in query_all(
                tx.conn(),
                "SELECT id FROM boosts WHERE booster_id=? ORDER BY id",
                [self.id],
                |r| r.get::<_, i64>(0),
            )? {
                Boost::find(tx.conn(), id)?.destroy(tx)?;
            }
            tx.conn()
                .execute("DELETE FROM poll_votes WHERE user_id=?", [self.id])?;
            // Owned-message pins disappeared above; only surviving messages broadcast here.
            for id in query_all(
                tx.conn(),
                "SELECT id FROM message_pins WHERE pinner_id=? ORDER BY id",
                [self.id],
                |r| r.get::<_, i64>(0),
            )? {
                MessagePin::find(tx.conn(), id)?.unpin(tx)?;
            }
            // Detach and purge scheduled files exactly as ScheduledMessage::destroy does.
            for id in query_all(
                tx.conn(),
                "SELECT id FROM scheduled_messages WHERE user_id=? ORDER BY id",
                [self.id],
                |r| r.get::<_, i64>(0),
            )? {
                ScheduledMessage::find(tx.conn(), id)?.replace_attachments(tx, &[])?;
            }
            for table in ["saved_items", "scheduled_messages", "searches"] {
                // delete_all intentionally retains a saved/scheduled source's inbox item.
                tx.conn()
                    .execute(&format!("DELETE FROM {table} WHERE user_id=?"), [self.id])?;
            }
            for id in query_all(
                tx.conn(),
                "SELECT id FROM room_categories WHERE user_id=? ORDER BY id",
                [self.id],
                |r| r.get::<_, i64>(0),
            )? {
                RoomCategory::find(tx.conn(), id)?.delete_rows(tx)?;
            }
            for session in Session::for_user(tx.conn(), self.id)? {
                session.destroy(tx)?;
            }
            for table in ["user_devices", "workspace_presence_leases", "bans"] {
                tx.conn()
                    .execute(&format!("DELETE FROM {table} WHERE user_id=?"), [self.id])?;
            }
            tx.conn()
                .execute("DELETE FROM users WHERE id=?", [self.id])?;
            Ok(())
        })
    }
}

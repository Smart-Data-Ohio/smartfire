//! Status-controller effects. Cache reconciliation, refresh enqueue and claims share the save's
//! transaction; pure broadcast facts leave it after commit. Calendar fetching belongs to WS14.
use super::UserStatusSettings;
use crate::{Broadcast, Event, Job, Result, Timestamp, Tx, WorkspacePresenceLease};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingRefreshJob {
    pub user_id: i64,
}
impl Job for MeetingRefreshJob {
    const CLASS: &'static str = "Calendar::MeetingRefreshJob";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBadgeBroadcast {
    pub user_id: i64,
    pub presence: String,
    pub status_text: Option<String>,
}
impl Broadcast for StatusBadgeBroadcast {
    const KIND: &'static str = "Calendar::MeetingDispatcher.broadcast_badges_for";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OooNoticeBroadcast {
    pub user_id: i64,
    pub name: String,
    pub visible: bool,
    pub until_date: Option<String>,
    pub note: Option<String>,
}
impl Broadcast for OooNoticeBroadcast {
    const KIND: &'static str = "Calendar::OooDispatcher.broadcast_ooo_for";
}

impl UserStatusSettings {
    /// Conditional boundary claim. A stale false claimant cannot clear a newly extended end.
    /// Like update_all, this does not refresh the loaded record's attributes or dirty tracking.
    pub fn claim_ooo_broadcast(
        &self,
        tx: &mut Tx<'_>,
        active: bool,
        now: Timestamp,
    ) -> Result<bool> {
        let affected = if active {
            tx.conn().execute("UPDATE users SET ooo_broadcast=1,updated_at=? WHERE id=? AND (ooo_broadcast IS NULL OR ooo_broadcast!=1)", rusqlite::params![tx.now(),self.user.id])?
        } else {
            tx.conn().execute("UPDATE users SET ooo_broadcast=0,ooo_until=NULL,ooo_note=NULL,updated_at=? WHERE id=? AND (ooo_broadcast IS NULL OR ooo_broadcast!=0) AND (ooo_until IS NULL OR ooo_until<=?)", rusqlite::params![tx.now(),self.user.id,now])?
        };
        Ok(affected == 1)
    }

    pub fn announce_badge(&self, tx: &mut Tx<'_>) -> Result<()> {
        Self::announce_badges_for(tx, std::slice::from_ref(self))
    }

    /// Dispatcher batches leases once, then emits every badge before any OOO notice.
    pub fn announce_badges_for(tx: &mut Tx<'_>, users: &[Self]) -> Result<()> {
        use crate::models::workspace_presence_lease::Presence;
        if users.is_empty() {
            return Ok(());
        }
        let now = tx.now();
        let ids: Vec<_> = users.iter().map(|user| user.user.id).collect();
        let leases = WorkspacePresenceLease::presence_by_user_id(tx.conn(), &ids, now)?;
        for user in users {
            let lease = leases
                .get(&user.user.id)
                .copied()
                .unwrap_or(Presence::Offline);
            let presence = match user.effective_presence(lease) {
                Presence::Online => "online",
                Presence::Idle => "idle",
                Presence::Offline => "offline",
                Presence::Dnd => "dnd",
            };
            tx.emit_after_commit(Event::broadcast(&StatusBadgeBroadcast {
                user_id: user.user.id,
                presence: presence.into(),
                status_text: user.status_text_display(now),
            }));
        }
        Ok(())
    }

    pub fn announce_ooo(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.announce_badge(tx)?;
        self.announce_ooo_notice(tx);
        Ok(())
    }

    pub fn announce_ooo_notice(&self, tx: &mut Tx<'_>) {
        let now = tx.now();
        tx.emit_after_commit(Event::broadcast(&OooNoticeBroadcast {
            user_id: self.user.id,
            name: self.user.name.clone(),
            visible: self.presence_setting != "invisible" && self.out_of_office(now),
            until_date: self.ooo_until_effective(now).map(|t| {
                t.jiff()
                    .to_zoned(self.zone())
                    .strftime("%B %d, %Y")
                    .to_string()
            }),
            note: self
                .ooo_note
                .clone()
                .filter(|n| self.manual_ooo_active(now) && !n.chars().all(char::is_whitespace)),
        }));
    }

    pub fn save_status(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let attributes = self.attributes();
        let changed = |key: &str| {
            attributes
                .iter()
                .zip(&self.original_attributes)
                .any(|((name, value), (_, old))| *name == key && value != old)
        };
        let meeting_changed = changed("meeting_status_enabled");
        let calendar_changed = changed("ooo_calendar_enabled");
        let manual_changed = changed("ooo_until") || changed("ooo_note");
        self.save(tx)?;
        // Rails invokes these in this order and can enqueue twice when both opt-ins turn on.
        if meeting_changed {
            if self.meeting_status_enabled {
                tx.emit_after_commit(Event::job(&MeetingRefreshJob {
                    user_id: self.user.id,
                }));
            } else if let Some(cache) = &mut self.meeting_cache {
                if self.ooo_calendar_enabled {
                    tx.conn().execute("UPDATE calendar_meeting_caches SET busy_intervals='[]',updated_at=? WHERE id=?",rusqlite::params![tx.now(),cache.id])?;
                    cache.busy_intervals = serde_json::json!([]);
                } else {
                    tx.conn()
                        .execute("DELETE FROM calendar_meeting_caches WHERE id=?", [cache.id])?;
                    // A destroyed has_one remains in Rails' loaded association. The following
                    // calendar reconciliation still sees it when both switches turn off.
                }
                self.announce_badge(tx)?;
            }
        }
        if calendar_changed {
            if self.ooo_calendar_enabled {
                tx.emit_after_commit(Event::job(&MeetingRefreshJob {
                    user_id: self.user.id,
                }));
            } else if let Some(cache) = &mut self.meeting_cache {
                if self.meeting_status_enabled {
                    tx.conn().execute("UPDATE calendar_meeting_caches SET ooo_intervals='[]',updated_at=? WHERE id=?",rusqlite::params![tx.now(),cache.id])?;
                    cache.ooo_intervals = serde_json::json!([]);
                } else {
                    tx.conn()
                        .execute("DELETE FROM calendar_meeting_caches WHERE id=?", [cache.id])?;
                }
                self.claim_ooo_broadcast(tx, self.out_of_office(tx.now()), tx.now())?;
                self.announce_ooo(tx)?;
            }
        }
        if manual_changed {
            self.claim_ooo_broadcast(tx, self.out_of_office(tx.now()), tx.now())?;
            self.announce_ooo(tx)?;
        }
        Ok(())
    }
}

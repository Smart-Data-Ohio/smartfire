//! Pending tag writes and `apply_board_tag_auto_assign`, after the original write commits.
use super::*;
use crate::{BoardTagAssignment, WorkThreadEvent};

impl ChannelThread {
    pub(super) fn save_with_tags(
        &mut self,
        tx: &mut Tx<'_>,
        changed: Self,
        names: Option<Vec<String>>,
    ) -> Result<()> {
        let room = Room::find(tx.conn(), self.room_id)?;
        if let Err(error) = changed
            .validate_for_save(
                tx.conn(),
                &room,
                names.as_deref(),
                changed.work_owner_id != self.work_owner_id,
            )?
            .into_result()
        {
            *self = changed;
            return Err(error);
        }
        if let Some(names) = &names {
            for tag in self.tags(tx.conn())? {
                if !names.contains(&tag.name) {
                    tag.destroy(tx)?;
                }
            }
        }
        tx.register_record("channel_threads", self.id);
        self.save_validated(tx, changed, &room)?;
        if let Some(names) = names {
            let existing = self.tag_names(tx.conn())?;
            let added = names
                .into_iter()
                .filter(|name| !existing.contains(name))
                .collect::<Vec<_>>();
            Self::register_tag_assignment(tx, self.id, added.clone(), self.work_owner_id);
            for name in added {
                ThreadTag::create_for_thread(tx, self, &name)?;
            }
        }
        Ok(())
    }

    pub(super) fn register_tag_assignment(
        tx: &mut Tx<'_>,
        id: i64,
        added: Vec<String>,
        owner_id: Option<i64>,
    ) {
        tx.after_commit_record_latest("board_tag_auto_assignment", id, move |tx| {
            let operation = (|| -> Result<()> {
                if owner_id.is_some() {
                    return Ok(());
                }
                let Some(thread) = Self::find_by_id(tx.conn(), id)? else {
                    return Ok(());
                };
                if added.is_empty()
                    || !thread.board_post(tx.conn())?
                    || thread.work_owner_id.is_some()
                {
                    return Ok(());
                }
                let rule = BoardTagAssignment::for_room(tx.conn(), thread.room_id)?
                    .into_iter()
                    .find(|rule| added.contains(&rule.tag));
                if let Some(rule) = rule {
                    // A failed callback leaves the original tag write committed. Its owner,
                    // audit, ledger and durable jobs succeed or roll back together.
                    crate::database::run_write(tx.conn(), tx.env(), |tx| {
                        let Some(mut fresh) = Self::find_by_id(tx.conn(), id)? else {
                            return Ok(());
                        };
                        if fresh.work_owner_id.is_some() || !fresh.board_post(tx.conn())? {
                            return Ok(());
                        }
                        let Some(user) = User::find_by_id(tx.conn(), rule.assignee_id)? else {
                            return Ok(());
                        };
                        if !crate::models::board_tag_assignment::eligible(
                            tx.conn(),
                            fresh.room_id,
                            &user,
                        )? {
                            return Ok(());
                        }
                        let before = fresh.clone();
                        let mut changed = fresh.clone();
                        changed.work_owner_id = Some(user.id);
                        fresh.save(tx, changed)?;
                        WorkThreadEvent::create_for_change(
                            tx,
                            &before,
                            &fresh,
                            None,
                            Some("Auto-assigned by board tag rule"),
                        )?;
                        crate::models::agent_work_events::record_owner_change(
                            tx,
                            &fresh,
                            None,
                            Some(user.id),
                            None,
                        )?;
                        Ok(())
                    })?;
                }
                Ok(())
            })();
            if let Err(error) = operation {
                tracing::error!(%error, thread_id=id, "Board tag auto-assign failed");
            }
            Ok(())
        });
    }
}

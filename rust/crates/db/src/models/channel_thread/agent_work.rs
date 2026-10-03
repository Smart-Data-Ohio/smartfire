//! The agent model mutations in `app/models/channel_thread.rb`. Services own membership
//! and capability checks; the model rechecks ownership against fresh rows under the writer lock.
use super::*;
use crate::models::{agent_work_events, audit_log};
use crate::{Agent, HandoffPackage, NewWorkHandoff, WorkHandoff, WorkThreadEvent};
use campfire_richtext::ruby::{is_blank, json_value_to_s, truncate};
use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct AgentWorkChanges {
    /// None means omitted; Some(Null) is an explicit blank and is invalid for status.
    pub work_status: Option<Value>,
    pub note: Option<Value>,
    pub tags: Option<Value>,
    pub run_url: Option<Value>,
}

/// `tag_names=` distinguishes a tag array from a comma-separated scalar.
pub fn tag_names_from_value(value: &Value) -> Vec<String> {
    let names: Vec<String> = match value {
        Value::Array(items) => items.iter().map(json_value_to_s).collect(),
        _ => json_value_to_s(value)
            .split(',')
            .map(str::to_owned)
            .collect(),
    };
    super::normalize_tag_names(&names)
}

fn invalid(attribute: &'static str, message: &str) -> Error {
    let mut errors = Errors::default();
    errors.add(attribute, message);
    Error::RecordInvalid(errors)
}
fn presence(value: &Value) -> Option<String> {
    let value = json_value_to_s(value);
    (!is_blank(&value)).then_some(value)
}
fn ensure_owned(thread: &ChannelThread, agent: &Agent) -> Result<()> {
    if thread.work() && thread.work_owner_id == Some(agent.user_id) {
        Ok(())
    } else {
        Err(Error::RecordNotFound(
            "Work thread is not owned by this agent",
        ))
    }
}

impl ChannelThread {
    pub fn update_work_by_agent(
        &mut self,
        tx: &mut Tx<'_>,
        agent: &Agent,
        changes: AgentWorkChanges,
    ) -> Result<()> {
        let status = changes.work_status.as_ref().and_then(presence);
        if changes.work_status.is_some()
            && !status
                .as_deref()
                .is_some_and(|s| WORK_STATUSES.contains(&s))
        {
            return Err(invalid("work_status", "is invalid"));
        }
        let note = changes.note.as_ref().and_then(presence);
        if note.as_ref().is_some_and(|n| n.chars().count() > 500) {
            return Err(invalid(
                "base",
                "Note is too long (maximum is 500 characters)",
            ));
        }
        if changes.work_status.is_none() && changes.tags.is_none() && changes.run_url.is_none() {
            return Err(invalid("work_status", "is invalid"));
        }
        let fresh = tx.savepoint(|tx| {
            let mut fresh = Self::find(tx.conn(), self.id)?;
            ensure_owned(&fresh, agent)?;
            if changes.tags.is_some() || changes.run_url.is_some() {
                let mut changed = fresh.clone();
                if let Some(url) = &changes.run_url {
                    changed.run_url = presence(url);
                }
                let names = changes.tags.as_ref().map(tag_names_from_value);
                fresh.save_with_tags(tx, changed, names)?;
            }
            if changes.work_status.is_some() && fresh.work_status != status {
                let before = fresh.clone();
                let mut changed = fresh.clone();
                changed.work_status = status;
                fresh.save(tx, changed)?;
                let actor = User::find(tx.conn(), agent.user_id)?;
                WorkThreadEvent::create_for_change(
                    tx,
                    &before,
                    &fresh,
                    Some(&actor),
                    note.as_deref(),
                )?;
            }
            Ok(fresh)
        })?;
        *self = fresh;
        Ok(())
    }

    pub fn update_result_by_agent(
        &mut self,
        tx: &mut Tx<'_>,
        agent: &Agent,
        markdown: &Value,
    ) -> Result<()> {
        ensure_owned(self, agent)?;
        let normalized = presence(markdown);
        // Rails checks the operation instance before reload, including a stale no-op.
        if normalized == self.result_markdown {
            return Ok(());
        }
        if normalized
            .as_ref()
            .is_some_and(|s| s.chars().count() > RESULT_LIMIT)
        {
            return Err(invalid(
                "result_markdown",
                "is too long (maximum is 20000 characters)",
            ));
        }
        let fresh = tx.savepoint(|tx| {
            let mut fresh = Self::find(tx.conn(), self.id)?;
            ensure_owned(&fresh, agent)?;
            let mut changed = fresh.clone();
            changed.result_markdown = normalized.clone();
            changed.result_updated_at = Some(tx.now());
            changed.result_updated_by_id = Some(agent.user_id);
            fresh.save(tx, changed)?;
            let actor = User::find(tx.conn(), agent.user_id)?;
            let excerpt = normalized
                .as_deref()
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect::<String>();
            WorkThreadEvent::create_for_result(tx, &fresh, &actor, &excerpt)?;
            Ok(fresh)
        })?;
        *self = fresh;
        Ok(())
    }

    /// Shared agent/human handoff writer. Human pages and links are the next WS12 slice.
    /// Callers validate the receiver with WorkHandoff::receiver_error before this operation.
    pub fn hand_off(
        &mut self,
        tx: &mut Tx<'_>,
        sender: &User,
        receiver: &Agent,
        package: HandoffPackage,
        context: &audit_log::Context,
    ) -> Result<WorkHandoff> {
        self.check_handoff_target(receiver)?;
        let (fresh,handoff) = tx.savepoint(|tx| {
            let mut fresh = Self::find(tx.conn(),self.id)?; fresh.check_handoff_target(receiver)?;
            if sender.is_bot() {
                if fresh.work_owner_id != Some(sender.id) { return Err(Error::RecordNotFound("Work thread is not owned by this agent")); }
            } else if !fresh.work_manageable_by(tx.conn(),sender)? {
                return Err(Error::Other(super::WORK_UPDATE_FORBIDDEN.into()));
            }
            let before = fresh.clone();
            let handoff = WorkHandoff::create(tx,NewWorkHandoff { channel_thread_id:fresh.id,
                sender_id:sender.id,receiver_agent_id:receiver.id,summary:package.summary.clone(),links:package.links,open_questions:package.open_questions })?;
            let mut changed = fresh.clone(); changed.work_owner_id = Some(receiver.user_id); fresh.save(tx,changed)?;
            WorkThreadEvent::create_for_handoff(tx,&before,&fresh,sender,&handoff)?;
            let payload = handoff.payload(tx.conn())?;
            agent_work_events::record_handoff(tx,&fresh,before.work_owner_id,receiver.id,Some(sender.id),payload)?;
            let from_owner = if before.work_owner_id == Some(sender.id) { Some(sender.clone()) } else { before.work_owner_id.map(|id|User::find_by_id(tx.conn(),id)).transpose()?.flatten() };
            let to_owner = User::find(tx.conn(),receiver.user_id)?;
            audit_log::AuditLog::record(tx,audit_log::NewAuditLog { action:"work.handoff".into(),actor:Some(sender.into()),
                target:Some(audit_log::Target {record_type:"ChannelThread".into(),id:fresh.id,label:Some(fresh.name.clone())}),
                changes:Some(json!({"from_owner":from_owner.map(|u|u.name),"to_owner":to_owner.name,"summary":truncate(&package.summary,200,"...")})),
                ..Default::default() },context)?;
            Ok((fresh,handoff))
        })?;
        *self = fresh;
        Ok(handoff)
    }
    fn check_handoff_target(&self, receiver: &Agent) -> Result<()> {
        if !self.work() {
            return Err(Error::RecordNotFound("Work thread is not tracked"));
        }
        if self.work_owner_id == Some(receiver.user_id) {
            return Err(invalid("work_owner", "is already the owner of this work"));
        }
        Ok(())
    }
}

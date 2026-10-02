//! Read-side work policy and owner choices, shared by posts and human work controls.
use crate::Room;
use super::ChannelThread;
use crate::{
    Agent, Error, Errors, Membership, NewChannelThread, NewMessage, Result, Tx, User,
    WorkThreadEvent,
};
use rusqlite::Connection;
use serde_json::Value;

pub const WORK_UPDATE_FORBIDDEN: &str = "You cannot manage work in this thread";

#[derive(Debug, Clone, Default)]
pub struct WorkChanges {
    /// None omits the field; Some(None) stops tracking.
    pub status: Option<Option<String>>,
    /// None omits the field. Ruby's blank value clears the assignment.
    pub owner_id: Option<Value>,
}

pub fn normalize_owner_id(value: &Value) -> Result<Option<i64>> {
    if match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(v) => campfire_richtext::ruby::is_blank(v),
        Value::Array(v) => v.is_empty(),
        Value::Object(v) => v.is_empty(),
        _ => false,
    } {
        return Ok(None);
    }
    let id = match value {
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|n| n.is_finite() && *n >= i64::MIN as f64 && *n < -(i64::MIN as f64))
                .map(|n| n.trunc() as i64)
        }),
        Value::String(s) => ruby_integer(s),
        _ => None,
    };
    id.map(Some)
        .ok_or_else(|| invalid("work_owner", "is invalid"))
}

fn ruby_integer(value: &str) -> Option<i64> {
    // Integer rejects NUL bytes, even though Ruby's String#strip removes them.
    if value.contains('\0') {
        return None;
    }
    let value = campfire_richtext::ruby::strip(value);
    let (negative, value) = if let Some(value) = value.strip_prefix('-') {
        (true, value)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let (radix, digits) = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (16, value)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        (2, value)
    } else if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        (8, value)
    } else if let Some(value) = value
        .strip_prefix("0d")
        .or_else(|| value.strip_prefix("0D"))
    {
        (10, value)
    } else if value.len() > 1 && value.starts_with('0') {
        (8, value)
    } else {
        (10, value)
    };
    if digits.is_empty()
        || digits.starts_with(['_', '+', '-'])
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return None;
    }
    let digits = digits.replace('_', "");
    let magnitude = i128::from_str_radix(&digits, radix).ok()?;
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

fn invalid(attribute: &'static str, message: &str) -> Error {
    let mut errors = Errors::default();
    errors.add(attribute, message);
    Error::RecordInvalid(errors)
}

impl ChannelThread {
    pub(super) fn validate_work_owner(&self, conn: &Connection) -> Result<Errors> {
        let mut errors = Errors::default();
        let Some(id) = self.work_owner_id else {
            return Ok(errors);
        };
        let owner = User::find_by_id(conn, id)?;
        let member = Membership::find_by_room_and_user(conn, self.room_id, id)?.is_some();
        if let Some(owner) = owner.as_ref().filter(|u| u.is_bot()) {
            let eligible = if let Some(agent) = Agent::for_user(conn, owner.id)? {
                agent.active(conn)?
                    && member
                    && agent.can(conn, "post_messages", Some(self.room_id))?
            } else {
                false
            };
            if !eligible {
                errors.add(
                    "work_owner",
                    "must be an active agent member of the parent room with permission to post",
                );
            }
        } else if !owner.is_some_and(|u| u.is_active()) || !member {
            errors.add(
                "work_owner",
                "must be an active human member of the parent room",
            );
        }
        Ok(errors)
    }

    pub fn create_board_post(
        tx: &mut Tx<'_>,
        mut attributes: NewChannelThread,
        first_message: Option<String>,
    ) -> Result<Self> {
        tx.savepoint(move |tx| {
            attributes.run_url = attributes
                .run_url
                .filter(|value| !campfire_richtext::ruby::is_blank(value));
            let creator_id = attributes.creator_id;
            let mut thread = Self::create(tx, attributes)?;
            crate::ThreadMembership::join(tx, thread.id, creator_id)?;
            let opener = first_message
                .filter(|message| {
                    !campfire_richtext::ruby::is_blank(campfire_richtext::ruby::strip(message))
                })
                .map(|message| {
                    thread.post_message_with_agent_delivery(
                        tx,
                        creator_id,
                        NewMessage {
                            markdown_source: Some(message),
                            board_post_opener: true,
                            ..Default::default()
                        },
                        true,
                    )
                })
                .transpose()?;
            if thread.work_owner_id.is_some() {
                let mut before = thread.clone();
                before.work_owner_id = None;
                let actor = User::find(tx.conn(), creator_id)?;
                WorkThreadEvent::create_for_change(tx, &before, &thread, Some(&actor), None)?;
                crate::models::agent_work_events::record_owner_change(
                    tx,
                    &thread,
                    None,
                    thread.work_owner_id,
                    Some(creator_id),
                )?;
            }
            if let Some(message) = opener {
                crate::models::agent_delivery::enqueue_for_message(tx, &message)?;
                for (membership, user) in
                    Membership::for_room_with_users(tx.conn(), thread.room_id)?
                {
                    let Some(user) = user else {
                        continue;
                    };
                    if !user.is_active()
                        || user.is_bot()
                        || user.id == creator_id
                        || membership.involvement == Some(crate::Involvement::Invisible)
                    {
                        continue;
                    }
                    if membership.involvement == Some(crate::Involvement::Everything)
                        || Some(user.id) == thread.work_owner_id
                    {
                        crate::ActivityItem::record_authorized_board_opener(tx, &user, &message)?;
                    }
                }
            }
            Ok(thread)
        })
    }

    pub fn update_work(
        &mut self,
        tx: &mut Tx<'_>,
        actor: &User,
        changes: WorkChanges,
    ) -> Result<()> {
        let id = self.id;
        let mut fresh = Self::find(tx.conn(), id)?;
        let result = tx.savepoint(|tx| {
            if !fresh.work_manageable_by(tx.conn(), actor)? {
                return Err(Error::Other(WORK_UPDATE_FORBIDDEN.into()));
            }
            let before = fresh.clone();
            let status = changes
                .status
                .unwrap_or_else(|| fresh.work_status.clone())
                .filter(|value| !campfire_richtext::ruby::is_blank(value));
            if status
                .as_deref()
                .is_some_and(|value| !super::WORK_STATUSES.contains(&value))
            {
                return Err(invalid("work_status", "is invalid"));
            }
            let owner = changes
                .owner_id
                .as_ref()
                .map(normalize_owner_id)
                .transpose()?
                .unwrap_or(fresh.work_owner_id);
            if (changes.owner_id.is_some() || before.work_status.is_some() != status.is_some())
                && !fresh.work_assignment_manageable_by(tx.conn(), actor)?
            {
                return Err(Error::Other(WORK_UPDATE_FORBIDDEN.into()));
            }
            if owner.is_some() && status.is_none() {
                return Err(invalid("work_owner", "requires work tracking"));
            }
            if before.work_status != status || before.work_owner_id != owner {
                let mut changed = fresh.clone();
                changed.work_status = status;
                changed.work_owner_id = owner;
                fresh.save(tx, changed)?;
                WorkThreadEvent::create_for_change(tx, &before, &fresh, Some(actor), None)?;
                crate::models::agent_work_events::record_owner_change(
                    tx,
                    &fresh,
                    before.work_owner_id,
                    owner,
                    Some(actor.id),
                )?;
            }
            Ok(())
        });
        *self = fresh;
        result
    }

    pub fn update_result(
        &mut self,
        tx: &mut Tx<'_>,
        actor: &User,
        markdown: Option<String>,
    ) -> Result<()> {
        if !self.work_manageable_by(tx.conn(), actor)? {
            return Err(Error::Other(WORK_UPDATE_FORBIDDEN.into()));
        }
        let normalized = markdown.filter(|value| !campfire_richtext::ruby::is_blank(value));
        // Rails checks its operation instance before reloading, then always records
        // the accepted edit under the fresh ownership check.
        if normalized == self.result_markdown {
            return Ok(());
        }
        if normalized
            .as_ref()
            .is_some_and(|value| value.chars().count() > super::RESULT_LIMIT)
        {
            return Err(invalid(
                "result_markdown",
                "is too long (maximum is 20000 characters)",
            ));
        }
        let fresh = tx.savepoint(|tx| {
            let mut fresh = Self::find(tx.conn(), self.id)?;
            if !fresh.work_manageable_by(tx.conn(), actor)? {
                return Err(Error::Other(WORK_UPDATE_FORBIDDEN.into()));
            }
            let mut changed = fresh.clone();
            changed.result_markdown = normalized.clone();
            changed.result_updated_at = Some(tx.now());
            changed.result_updated_by_id = Some(actor.id);
            fresh.save(tx, changed)?;
            let excerpt = normalized
                .as_deref()
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect::<String>();
            WorkThreadEvent::create_for_result(tx, &fresh, actor, &excerpt)?;
            Ok(fresh)
        })?;
        *self = fresh;
        Ok(())
    }
    pub fn work_viewable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(Self::work_viewable_with_membership(user, true)
            && Membership::find_by_room_and_user(conn, self.room_id, user.id)?.is_some())
    }

    pub fn work_viewable_with_membership(user: &User, member: bool) -> bool {
        user.is_active() && !user.is_bot() && member
    }
    pub fn work_manageable_in_room(&self, room: &Room, user: &User, member: bool) -> bool {
        Self::work_viewable_with_membership(user, member)
            && (self.settings_manageable_in_room(room, user) || self.work_owner_id == Some(user.id))
    }
    pub fn work_assignment_manageable_in_room(&self, room: &Room, user: &User, member: bool) -> bool {
        Self::work_viewable_with_membership(user, member) && self.settings_manageable_in_room(room, user)
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
        let members = Membership::for_room_with_users(conn, room_id)?;
        let bot_ids = members
            .iter()
            .filter_map(|(_, user)| {
                user.as_ref()
                    .filter(|user| user.is_active() && user.is_bot())
                    .map(|user| user.id)
            })
            .collect::<Vec<_>>();
        let agents_by_user = Agent::for_users(conn, &bot_ids)?
            .into_iter()
            .map(|agent| (agent.user_id, agent.id))
            .collect::<std::collections::HashMap<_, _>>();
        let requests = agents_by_user
            .values()
            .map(|&id| (id, Some(room_id)))
            .collect::<Vec<_>>();
        let capabilities = Agent::capabilities_for_rooms(conn, "post_messages", &requests)?;
        for (_, user) in members {
            let Some(user) = user else {
                continue;
            };
            if !user.is_active() {
                continue;
            }
            if user.is_bot() {
                if agents_by_user.get(&user.id).is_some_and(|agent| {
                    capabilities
                        .get(&(*agent, Some(room_id)))
                        .copied()
                        .unwrap_or(false)
                }) {
                    agents.push(user);
                }
            } else {
                humans.push(user);
            }
        }
        humans.sort_by_key(|user| rails_compat::unicode::downcase(&user.name));
        agents.sort_by_key(|user| rails_compat::unicode::downcase(&user.name));
        Ok((humans, agents))
    }
}

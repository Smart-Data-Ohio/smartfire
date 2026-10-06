//! AgentStep and Agents::Steps. Parent authorization, progress limits and changes
//! are shared by REST and MCP; WS11-ui/WS8bm render the parent-change requests.
use super::agent_payloads::{compact, json_time};
use super::agent_service::ServiceResult;
use crate::sql::{exists, query_one};
use crate::{ChannelThread, Connection, Errors, Event, Message, Result, Timestamp, Tx};
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const STATUSES: [&str; 4] = ["pending", "running", "done", "failed"];
#[derive(Debug, Clone)]
pub struct AgentStep {
    pub id: i64,
    pub agent_id: i64,
    pub message_id: Option<i64>,
    pub channel_thread_id: Option<i64>,
    pub name: String,
    pub status: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub duration_ms: Option<i64>,
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone)]
pub struct NewAgentStep {
    pub agent_id: i64,
    pub message_id: Option<i64>,
    pub channel_thread_id: Option<i64>,
    pub name: String,
    pub status: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub duration_ms: Option<i64>,
}
impl Default for NewAgentStep {
    fn default() -> Self {
        Self {
            agent_id: 0,
            message_id: None,
            channel_thread_id: None,
            name: String::new(),
            status: "running".into(),
            input_summary: None,
            output_summary: None,
            duration_ms: None,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct AgentStepChanges {
    pub name: Option<String>,
    pub status: Option<String>,
    pub input_summary: Option<Option<String>>,
    pub output_summary: Option<Option<String>>,
    pub duration_ms: Option<Option<i64>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepParentChange {
    pub message_id: Option<i64>,
    pub thread_id: Option<i64>,
}
impl crate::events::Broadcast for StepParentChange {
    const KIND: &'static str = "Agents::Steps#broadcast_parent";
}

fn work(thread: &ChannelThread) -> bool {
    thread
        .work_status
        .as_deref()
        .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
}
impl AgentStep {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            message_id: r.get("message_id")?,
            channel_thread_id: r.get("channel_thread_id")?,
            name: r.get("name")?,
            status: r.get("status")?,
            input_summary: r.get("input_summary")?,
            output_summary: r.get("output_summary")?,
            duration_ms: r.get("duration_ms")?,
            position: r.get("position")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_steps WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn validate(conn: &Connection, a: &NewAgentStep, exclude: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        let user = query_one(
            conn,
            "SELECT user_id FROM agents WHERE id=?",
            [a.agent_id],
            |r| r.get::<_, i64>(0),
        )?;
        if user.is_none() {
            errors.add("agent", "must exist");
        }
        if campfire_richtext::ruby::is_blank(&a.name) {
            errors.add("name", "can't be blank");
        }
        if a.name.chars().count() > 120 {
            errors.add("name", "is too long (maximum is 120 characters)");
        }
        if !STATUSES.contains(&a.status.as_str()) {
            errors.add("status", "is not included in the list");
        }
        for (field, text) in [
            ("input_summary", &a.input_summary),
            ("output_summary", &a.output_summary),
        ] {
            if text
                .as_ref()
                .is_some_and(|s| !campfire_richtext::ruby::is_blank(s) && s.chars().count() > 1000)
            {
                errors.add(field, "is too long (maximum is 1000 characters)");
            }
        }
        if a.duration_ms.is_some_and(|duration| duration < 0) {
            errors.add("duration_ms", "must be greater than or equal to 0");
        }
        if a.message_id.is_some() == a.channel_thread_id.is_some() {
            errors.add("base", "Step must belong to exactly one message or thread");
        }
        if let Some(message) = a
            .message_id
            .map(|id| Message::find_by_id(conn, id))
            .transpose()?
            .flatten()
            && Some(message.creator_id) != user
        {
            errors.add("message", "must be the agent's own message");
        }
        if let Some(thread) = a
            .channel_thread_id
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
            && (!work(&thread) || thread.work_owner_id != user)
        {
            errors.add("channel_thread", "must be work the agent owns");
        }
        if exclude.is_none() {
            let count: i64 = if let Some(message) = a.message_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM agent_steps WHERE message_id=?",
                    [message],
                    |r| r.get(0),
                )?
            } else if let Some(thread) = a.channel_thread_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM agent_steps WHERE channel_thread_id=?",
                    [thread],
                    |r| r.get(0),
                )?
            } else {
                0
            };
            if count >= 50 {
                errors.add("base", "Steps are limited to 50 per message or thread");
            }
        }
        Ok(errors)
    }
    pub fn create(tx: &Tx<'_>, a: NewAgentStep) -> Result<Self> {
        Self::validate(tx.conn(), &a, None)?.into_result()?;
        let position: Option<i64> = if let Some(message) = a.message_id {
            tx.conn().query_row(
                "SELECT MAX(position) FROM agent_steps WHERE message_id=?",
                [message],
                |r| r.get(0),
            )?
        } else {
            tx.conn().query_row(
                "SELECT MAX(position) FROM agent_steps WHERE channel_thread_id=?",
                [a.channel_thread_id],
                |r| r.get(0),
            )?
        };
        let position = position.unwrap_or(-1) + 1;
        let id=tx.conn().query_row("INSERT INTO agent_steps(agent_id,message_id,channel_thread_id,name,status,input_summary,output_summary,duration_ms,position,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?) RETURNING id",params![a.agent_id,a.message_id,a.channel_thread_id,a.name,a.status,a.input_summary,a.output_summary,a.duration_ms,position,tx.now(),tx.now()],|r|r.get(0))?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted step"))
    }
    fn attributes(&self) -> NewAgentStep {
        NewAgentStep {
            agent_id: self.agent_id,
            message_id: self.message_id,
            channel_thread_id: self.channel_thread_id,
            name: self.name.clone(),
            status: self.status.clone(),
            input_summary: self.input_summary.clone(),
            output_summary: self.output_summary.clone(),
            duration_ms: self.duration_ms,
        }
    }
    pub fn update(&mut self, tx: &Tx<'_>, changes: AgentStepChanges) -> Result<()> {
        self.update_with_input_errors(tx, changes, Errors::default())
    }

    /// Transport seam: preserve validation of raw numeric input before casting.
    pub fn update_with_input_errors(
        &mut self,
        tx: &Tx<'_>,
        changes: AgentStepChanges,
        input_errors: Errors,
    ) -> Result<()> {
        let before =
            Self::find(tx.conn(), self.id)?.ok_or(crate::Error::RecordNotFound("AgentStep"))?;
        let mut candidate = before.clone();
        macro_rules! assign {($($field:ident),*)=>{$(if let Some(value)=changes.$field {candidate.$field=value;})*};}
        assign!(name, status, input_summary, output_summary, duration_ms);
        let mut errors = Self::validate(tx.conn(), &candidate.attributes(), Some(self.id))?;
        errors.0.extend(input_errors.0);
        errors.into_result()?;
        if candidate.name != before.name
            || candidate.status != before.status
            || candidate.input_summary != before.input_summary
            || candidate.output_summary != before.output_summary
            || candidate.duration_ms != before.duration_ms
        {
            tx.conn().execute("UPDATE agent_steps SET name=?,status=?,input_summary=?,output_summary=?,duration_ms=?,updated_at=? WHERE id=?",params![candidate.name,candidate.status,candidate.input_summary,candidate.output_summary,candidate.duration_ms,tx.now(),self.id])?;
            candidate.updated_at = tx.now();
        }
        *self = candidate;
        Ok(())
    }
    pub fn payload(&self) -> Value {
        compact(
            json!({"id":self.id,"message_id":self.message_id,"thread_id":self.channel_thread_id,"name":self.name,"status":self.status,"input_summary":self.input_summary,"output_summary":self.output_summary,"duration_ms":self.duration_ms,"position":self.position,"created_at":json_time(self.created_at),"updated_at":json_time(self.updated_at)}),
        )
    }
    fn broadcast_parent(&self, tx: &mut Tx<'_>) -> Result<()> {
        let message = match self.message_id {
            Some(id) if exists(tx.conn(), "SELECT 1 FROM messages WHERE id=?", [id])? => Some(id),
            _ => None,
        };
        let thread = match (message, self.channel_thread_id) {
            (None, Some(id))
                if exists(tx.conn(), "SELECT 1 FROM channel_threads WHERE id=?", [id])? =>
            {
                Some(id)
            }
            _ => None,
        };
        if message.is_some() || thread.is_some() {
            tx.emit_after_commit(Event::broadcast(&StepParentChange {
                message_id: message,
                thread_id: thread,
            }));
        }
        Ok(())
    }
}

fn member(conn: &Connection, agent_id: i64, room_id: i64) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM agents a JOIN memberships m ON m.user_id=a.user_id JOIN rooms r ON r.id=m.room_id WHERE a.id=? AND r.id=? AND r.deleted_at IS NULL",
        params![agent_id, room_id],
    )
}
fn parent_room(conn: &Connection, a: &NewAgentStep) -> Result<Option<i64>> {
    let user = query_one(
        conn,
        "SELECT user_id FROM agents WHERE id=?",
        [a.agent_id],
        |r| r.get::<_, i64>(0),
    )?;
    if let Some(message_id) = a.message_id {
        Ok(Message::find_by_id(conn, message_id)?
            .filter(|m| Some(m.creator_id) == user)
            .map(|m| m.room_id))
    } else {
        Ok(a.channel_thread_id
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
            .filter(|t| t.work_owner_id == user)
            .map(|t| t.room_id))
    }
}
fn invalid(errors: Errors) -> ServiceResult {
    let error = crate::slash_commands::sentence(errors.full_messages());
    let mut fields = serde_json::Map::new();
    for (field, message) in errors.0 {
        fields
            .entry(field.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(message));
    }
    ServiceResult {
        payload: Some(json!({"errors":fields})),
        error: Some(error),
        status: 422,
    }
}
pub fn create(tx: &mut Tx<'_>, agent_id: i64, attributes: NewAgentStep) -> Result<ServiceResult> {
    create_with_input_errors(tx, agent_id, attributes, Errors::default())
}

/// Extra input errors are applied after parent authorization and before writing.
pub fn create_with_input_errors(
    tx: &mut Tx<'_>,
    agent_id: i64,
    mut attributes: NewAgentStep,
    input_errors: Errors,
) -> Result<ServiceResult> {
    attributes.agent_id = agent_id;
    if attributes.message_id.is_some() == attributes.channel_thread_id.is_some() {
        return Ok(ServiceResult::fail(
            "Exactly one of message_id or thread_id is required",
            422,
        ));
    };
    let missing = if attributes.message_id.is_some() {
        "Message not found"
    } else {
        "Work not found"
    };
    let Some(room) = parent_room(tx.conn(), &attributes)? else {
        return Ok(ServiceResult::fail(missing, 404));
    };
    if !member(tx.conn(), agent_id, room)? {
        return Ok(ServiceResult::fail(missing, 404));
    };
    let capability = if attributes.message_id.is_some() {
        "post_messages"
    } else {
        "manage_threads"
    };
    if !super::agent_access::capability_for_agent(tx.conn(), agent_id, capability, Some(room))? {
        return Ok(ServiceResult::fail(
            format!("Forbidden: agent lacks {capability} capability"),
            403,
        ));
    };
    if campfire_richtext::ruby::is_blank(&attributes.status) {
        attributes.status = "running".into();
    }
    attributes.input_summary = attributes
        .input_summary
        .filter(|s| !campfire_richtext::ruby::is_blank(s));
    attributes.output_summary = attributes
        .output_summary
        .filter(|s| !campfire_richtext::ruby::is_blank(s));
    let mut errors = AgentStep::validate(tx.conn(), &attributes, None)?;
    let position = errors
        .0
        .iter()
        .position(|(field, _)| *field == "base")
        .unwrap_or(errors.0.len());
    errors.0.splice(position..position, input_errors.0);
    if !errors.is_empty() {
        return Ok(invalid(errors));
    };
    let step = AgentStep::create(tx, attributes)?;
    step.broadcast_parent(tx)?;
    Ok(ServiceResult::ok(step.payload(), 201))
}
pub fn update(
    tx: &mut Tx<'_>,
    agent_id: i64,
    id: i64,
    changes: AgentStepChanges,
) -> Result<ServiceResult> {
    update_with_input_errors(tx, agent_id, id, changes, Errors::default())
}

/// Same seam as create, retaining live parent authorization on updates.
pub fn update_with_input_errors(
    tx: &mut Tx<'_>,
    agent_id: i64,
    id: i64,
    changes: AgentStepChanges,
    input_errors: Errors,
) -> Result<ServiceResult> {
    let Some(mut step) = AgentStep::find(tx.conn(), id)?.filter(|s| s.agent_id == agent_id) else {
        return Ok(ServiceResult::fail("Step not found", 404));
    };
    let capability = if step.message_id.is_some() {
        "post_messages"
    } else {
        "manage_threads"
    };
    let mut accessible = false;
    if let Some(room) = parent_room(tx.conn(), &step.attributes())? {
        let is_work = step.message_id.is_some()
            || step
                .channel_thread_id
                .map(|id| ChannelThread::find_by_id(tx.conn(), id))
                .transpose()?
                .flatten()
                .is_some_and(|t| work(&t));
        accessible = is_work
            && member(tx.conn(), agent_id, room)?
            && super::agent_access::capability_for_agent(
                tx.conn(),
                agent_id,
                capability,
                Some(room),
            )?;
    }
    if !accessible {
        return Ok(ServiceResult::fail(
            format!("Forbidden: agent lacks {capability} capability"),
            403,
        ));
    };
    if let Err(error) = step.update_with_input_errors(tx, changes, input_errors) {
        return match error {
            crate::Error::RecordInvalid(errors) => Ok(invalid(errors)),
            error => Err(error),
        };
    };
    step.broadcast_parent(tx)?;
    Ok(ServiceResult::ok(step.payload(), 200))
}

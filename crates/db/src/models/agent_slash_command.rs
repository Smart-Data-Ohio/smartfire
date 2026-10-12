//! AgentSlashCommand, Agents::SlashCommands and the agent branch of the composer
//! dispatcher. Registration policy and invocation stay shared by all transports.
use super::agent_delivery::{NewEvent, record_delivered};
use super::agent_service::ServiceResult;
use crate::sql::{exists, query_one};
use crate::{Connection, Errors, Result, Timestamp, Tx};
use campfire_slash_schema::{CommandOption, CommandSchema, CommandValues, OptionValue};
use rusqlite::{Row, params};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct AgentSlashCommand {
    pub id: i64,
    pub agent_id: i64,
    pub room_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub takes_arguments: bool,
    option_schema_json: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone)]
pub struct NewAgentSlashCommand {
    pub agent_id: i64,
    pub room_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub takes_arguments: bool,
    pub option_schema: Option<CommandSchema>,
}
impl Default for NewAgentSlashCommand {
    fn default() -> Self {
        Self {
            agent_id: 0,
            room_id: 0,
            name: String::new(),
            description: None,
            takes_arguments: true,
            option_schema: None,
        }
    }
}
fn normalized_name(name: &str) -> String {
    campfire_richtext::ruby::strip(name).to_lowercase()
}
fn normalized_description(description: Option<&str>) -> Option<String> {
    description
        .map(campfire_richtext::ruby::strip)
        .filter(|s| !campfire_richtext::ruby::is_blank(s))
        .map(str::to_owned)
}
impl AgentSlashCommand {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            room_id: r.get("room_id")?,
            name: r.get("name")?,
            description: r.get("description")?,
            takes_arguments: r.get("takes_arguments")?,
            option_schema_json: r.get("option_schema")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_slash_commands WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn find_by_name(conn: &Connection, room_id: i64, name: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_slash_commands WHERE room_id=? AND name=? LIMIT 1",
            params![room_id, normalized_name(name)],
            Self::from_row,
        )
    }
    pub fn validate(
        conn: &Connection,
        a: &NewAgentSlashCommand,
        exclude: Option<i64>,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if !exists(conn, "SELECT 1 FROM agents WHERE id=?", [a.agent_id])? {
            errors.add("agent", "must exist");
        }
        if !exists(conn, "SELECT 1 FROM rooms WHERE id=?", [a.room_id])? {
            errors.add("room", "must exist");
        }
        let name = normalized_name(&a.name);
        if campfire_richtext::ruby::is_blank(&name) {
            errors.add("name", "can't be blank");
        }
        if !(1..=32).contains(&name.len())
            || !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
        {
            errors.add("name", "is invalid");
        }
        if exists(
            conn,
            "SELECT 1 FROM agent_slash_commands WHERE room_id=? AND name=? AND (? IS NULL OR id!=?)",
            params![a.room_id, name, exclude, exclude],
        )? {
            errors.add("name", "has already been taken");
        }
        if normalized_description(a.description.as_deref()).is_some_and(|s| s.chars().count() > 140)
        {
            errors.add("description", "is too long (maximum is 140 characters)");
        }
        if !campfire_richtext::ruby::is_blank(&name)
            && crate::slash_commands::lookup(&name).is_some()
        {
            errors.add("name", "is already a built-in command");
        }
        if let Some(schema) = &a.option_schema {
            for (field, messages) in schema.validate() {
                for message in messages {
                    errors.add("option_schema", format!("{field} {message}"));
                }
            }
        }
        Ok(errors)
    }
    pub fn create(tx: &Tx<'_>, a: NewAgentSlashCommand) -> Result<Self> {
        Self::validate(tx.conn(), &a, None)?.into_result()?;
        let id=tx.conn().query_row("INSERT INTO agent_slash_commands(agent_id,room_id,name,description,takes_arguments,option_schema,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?) RETURNING id",params![a.agent_id,a.room_id,normalized_name(&a.name),normalized_description(a.description.as_deref()),a.takes_arguments,a.option_schema.as_ref().map(|s|serde_json::to_string(s).expect("validated schema")),tx.now(),tx.now()],|r|r.get(0))?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted command"))
    }
    pub fn payload(&self) -> serde_json::Value {
        let mut payload = json!({"name":self.name,"description":self.description,"room_id":self.room_id,"agent_id":self.agent_id,"takes_arguments":self.takes_arguments});
        if let Some(schema) = self.option_schema().ok().flatten() {
            payload["option_schema"] = json!(schema);
        }
        payload
    }
    pub fn option_schema(&self) -> Result<Option<CommandSchema>> {
        parse_schema(self.id, self.option_schema_json.as_deref())
    }
}

fn authorized_room(tx: &Tx<'_>, agent_id: i64, room_id: i64) -> Result<Option<ServiceResult>> {
    if exists(
        tx.conn(),
        "SELECT 1 FROM agents WHERE id=? AND kind='personal'",
        [agent_id],
    )? {
        return Ok(Some(ServiceResult::fail(
            "Not available to personal agents",
            403,
        )));
    }
    if !exists(
        tx.conn(),
        "SELECT 1 FROM rooms r JOIN memberships m ON m.room_id=r.id JOIN agents a ON a.user_id=m.user_id WHERE r.id=? AND a.id=? AND r.deleted_at IS NULL",
        params![room_id, agent_id],
    )? {
        return Ok(Some(ServiceResult::fail("Room not found", 404)));
    };
    if !super::agent_access::capability_for_agent(
        tx.conn(),
        agent_id,
        "post_messages",
        Some(room_id),
    )? {
        return Ok(Some(ServiceResult::fail(
            "Forbidden: agent lacks post_messages capability",
            403,
        )));
    };
    Ok(None)
}
pub fn register(
    tx: &Tx<'_>,
    agent_id: i64,
    room_id: i64,
    name: &str,
    description: Option<&str>,
    takes_arguments: Option<bool>,
) -> Result<ServiceResult> {
    register_with_schema(
        tx,
        agent_id,
        room_id,
        name,
        description,
        takes_arguments,
        None,
    )
}

pub fn register_with_schema(
    tx: &Tx<'_>,
    agent_id: i64,
    room_id: i64,
    name: &str,
    description: Option<&str>,
    takes_arguments: Option<bool>,
    option_schema: Option<Value>,
) -> Result<ServiceResult> {
    if let Some(denial) = authorized_room(tx, agent_id, room_id)? {
        return Ok(denial);
    };
    let name = normalized_name(name);
    let existing = AgentSlashCommand::find_by_name(tx.conn(), room_id, &name)?;
    if existing.as_ref().is_some_and(|c| c.agent_id != agent_id) {
        return Ok(ServiceResult::fail(
            format!("“/{name}” is already registered in this room"),
            422,
        ));
    };
    let update_schema = option_schema.is_some();
    let schema = match option_schema {
        None | Some(Value::Null) => None,
        Some(value) => {
            let schema: CommandSchema = match serde_json::from_value(value) {
                Ok(schema) => schema,
                Err(error) => {
                    return Ok(schema_invalid(std::collections::BTreeMap::from([(
                        "option_schema".into(),
                        vec![error.to_string()],
                    )])));
                }
            };
            let errors = schema
                .validate()
                .into_iter()
                .map(|(field, messages)| (format!("option_schema.{field}"), messages))
                .collect::<campfire_slash_schema::FieldErrors>();
            if !errors.is_empty() {
                return Ok(schema_invalid(errors));
            }
            Some(schema)
        }
    };
    let option_schema_json = if update_schema {
        schema
            .as_ref()
            .map(|s| serde_json::to_string(s).expect("validated schema"))
    } else {
        existing.as_ref().and_then(|c| c.option_schema_json.clone())
    };
    let attributes = NewAgentSlashCommand {
        agent_id,
        room_id,
        name,
        description: normalized_description(description),
        option_schema: schema,
        takes_arguments: takes_arguments
            .unwrap_or_else(|| existing.as_ref().is_none_or(|c| c.takes_arguments)),
    };
    let errors =
        AgentSlashCommand::validate(tx.conn(), &attributes, existing.as_ref().map(|c| c.id))?;
    if !errors.is_empty() {
        return Ok(ServiceResult::fail(
            crate::slash_commands::sentence(errors.full_messages()),
            422,
        ));
    };
    let command = if let Some(existing) = existing {
        if attributes.description != existing.description
            || attributes.takes_arguments != existing.takes_arguments
            || option_schema_json != existing.option_schema_json
        {
            tx.conn().execute("UPDATE agent_slash_commands SET description=?,takes_arguments=?,option_schema=?,updated_at=? WHERE id=?",params![attributes.description,attributes.takes_arguments,option_schema_json,tx.now(),existing.id])?;
        }
        AgentSlashCommand::find(tx.conn(), existing.id)?.expect("updated command")
    } else {
        AgentSlashCommand::create(tx, attributes)?
    };
    Ok(ServiceResult::ok(command.payload(), 201))
}
pub fn unregister(tx: &Tx<'_>, agent_id: i64, room_id: i64, name: &str) -> Result<ServiceResult> {
    if let Some(denial) = authorized_room(tx, agent_id, room_id)? {
        return Ok(denial);
    };
    let Some(command) = AgentSlashCommand::find_by_name(tx.conn(), room_id, name)?
        .filter(|c| c.agent_id == agent_id)
    else {
        return Ok(ServiceResult::fail("Command not found", 404));
    };
    tx.conn()
        .execute("DELETE FROM agent_slash_commands WHERE id=?", [command.id])?;
    Ok(ServiceResult::ok(
        json!({"unregistered":true,"name":command.name,"room_id":room_id}),
        200,
    ))
}

pub fn invoke(
    tx: &mut Tx<'_>,
    context: &crate::slash_commands::Context,
    name: &str,
    args: &str,
) -> Result<Option<crate::slash_commands::CommandResult>> {
    invoke_with_options(tx, context, name, args, None)
}

pub fn invoke_with_options(
    tx: &mut Tx<'_>,
    context: &crate::slash_commands::Context,
    name: &str,
    args: &str,
    options: Option<&CommandValues>,
) -> Result<Option<crate::slash_commands::CommandResult>> {
    let Some(command) = AgentSlashCommand::find_by_name(tx.conn(), context.room_id, name)? else {
        return Ok(None);
    };
    let available = exists(
        tx.conn(),
        "SELECT 1 FROM agents a JOIN users u ON u.id=a.user_id JOIN memberships m ON m.user_id=a.user_id JOIN rooms r ON r.id=m.room_id WHERE a.id=? AND a.kind!='personal' AND r.id=? AND r.deleted_at IS NULL AND a.suspended_at IS NULL AND u.status=0",
        params![command.agent_id, context.room_id],
    )? && super::agent_access::capability_for_agent(
        tx.conn(),
        command.agent_id,
        "post_messages",
        Some(context.room_id),
    )?;
    if !available {
        return Ok(Some(crate::slash_commands::CommandResult::error(format!(
            "“/{}” is no longer available.",
            command.name
        ))));
    };
    let resolved = if let Some(schema) = command.option_schema()? {
        let values = options.cloned().unwrap_or_default();
        let errors = schema.validate_values(&values);
        if !errors.is_empty() {
            return Err(crate::Error::FieldValidation(errors));
        }
        for option in schema
            .options_for(values.subcommand.as_deref())
            .expect("validated subcommand")
        {
            let name = option.attributes().0;
            if let Some(OptionValue::Integer(id)) = values.values.get(name) {
                match option {
                    CommandOption::User { .. } => {
                        crate::User::find_active(tx.conn(), *id)?;
                    }
                    CommandOption::Room { .. }
                        if crate::Room::find_for_user(tx.conn(), context.user_id, *id)?
                            .is_none() =>
                    {
                        return Err(crate::Error::RecordNotFound("Room"));
                    }
                    _ => {}
                }
            }
        }
        Some(values)
    } else {
        None
    };
    let count:i64=tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND room_id=? AND event_type='slash_command' AND created_at>=? AND outcome IN ('pending','delivered','acknowledged')",params![command.agent_id,context.room_id,tx.now().ago(jiff::SignedDuration::from_mins(1))],|r|r.get(0))?;
    if count >= 20 {
        return Ok(Some(crate::slash_commands::CommandResult::error(format!(
            "“/{}” is receiving too many invocations right now. Try again in a minute.",
            command.name
        ))));
    };
    let mut metadata = json!({"command":command.name,"arguments":args});
    if let Some(options) = resolved {
        metadata["options"] = json!(options);
    }
    if let Some(thread_id) = context.thread_id {
        metadata["thread_id"] = json!(thread_id);
    }
    record_delivered(
        tx,
        NewEvent {
            agent_id: command.agent_id,
            room_id: Some(context.room_id),
            actor_id: Some(context.user_id),
            event_type: "slash_command".into(),
            chain_id: Some(uuid::Uuid::new_v4().to_string()),
            metadata,
            ..Default::default()
        },
    )?;
    let user: String = tx.conn().query_row(
        "SELECT u.* FROM users u JOIN agents a ON a.user_id=u.id WHERE a.id=?",
        [command.agent_id],
        |r| Ok(crate::User::from_row(r)?.display_name().to_owned()),
    )?;
    Ok(Some(crate::slash_commands::CommandResult::ephemeral(
        format!("Sent to {user}"),
    )))
}

pub(crate) fn schema_from_row(
    row: &Row<'_>,
    column: &str,
) -> rusqlite::Result<Option<CommandSchema>> {
    let json: Option<String> = row.get(column)?;
    Ok(parse_schema(row.get("command_id")?, json.as_deref())
        .ok()
        .flatten())
}
fn parse_schema(command_id: i64, json: Option<&str>) -> Result<Option<CommandSchema>> {
    json.map(|json| {
        serde_json::from_str(json).map_err(|error| {
            tracing::warn!(command_id, %error, "Unreadable slash command option schema");
            crate::Error::FieldValidation(std::collections::BTreeMap::from([(
                "options".into(),
                vec!["command option schema is unavailable".into()],
            )]))
        })
    })
    .transpose()
}
fn schema_invalid(fields: campfire_slash_schema::FieldErrors) -> ServiceResult {
    ServiceResult {
        payload: Some(json!({"errors":fields})),
        error: Some("Invalid option schema".into()),
        status: 422,
    }
}

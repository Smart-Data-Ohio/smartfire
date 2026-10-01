//! Thin JSON adapters for WS11's shared services. Rails agents/* controllers are
//! the reference; the MCP adapter uses the same operations and minute buckets.
use campfire_db::models::{agent_event_access, agent_event_polling, agent_service::ServiceResult};
use campfire_kit::{Ctx, Param, Result, StatusCode, format, halt};
use serde_json::{Value, json};

use super::presenters::page::db_error;
pub mod mcp;
pub mod approvals;
pub mod pending;
pub mod conversations;
pub mod integrations;

pub async fn me(c: &mut Ctx) -> Result {
    concerns::before_actions(c, concerns::Before::default().allow_agent_access()).await?;
    agent_api::no_store(c);
    let user_id = concerns::require_current_user(c)?.id;
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let payload = c
        .app()
        .db
        .read(move |conn| {
            campfire_db::Agent::for_user(conn, user_id)?
                .map(|agent| profile_payload(conn, &agent, now))
                .transpose()
        })
        .await
        .map_err(db_error)?;
    Ok(match payload {
        Some(payload) => c.render(StatusCode::OK, &format::JSON, payload.to_string()),
        None => c.head(StatusCode::NOT_FOUND),
    })
}

pub async fn update_me(c: &mut Ctx) -> Result {
    concerns::before_actions(c, concerns::Before::default().allow_agent_access()).await?;
    agent_api::no_store(c);
    let Some(identity) = c
        .current::<concerns::CurrentAgent>()
        .copied()
        .filter(|_| concerns::authenticated_by(c) == concerns::AuthenticatedBy::AgentToken)
    else {
        return Ok(c.render(
            StatusCode::FORBIDDEN,
            &format::JSON,
            json!({"error":"Forbidden: Bearer agent token required"}).to_string(),
        ));
    };
    let status = field(c, "status");
    let note = field(c, "status_note");
    let presence = field(c, "working_presence");
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::find(tx.conn(), identity.agent_id)?
                .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            if let Some(status) = status {
                agent.status = attribute_string(Some(&status)).unwrap_or_default();
            }
            if let Some(note) = note {
                agent.status_note = attribute_string(Some(&note));
            }
            if let Some(presence) = presence {
                let text = text(Some(&presence));
                agent.assign_working_presence(text.as_deref(), tx.now());
            }
            match agent.save(tx) {
                Ok(()) => Ok(ServiceResult::ok(
                    profile_payload(tx.conn(), &agent, tx.now())?,
                    200,
                )),
                Err(campfire_db::Error::RecordInvalid(errors)) => Ok(ServiceResult::fail(
                    campfire_db::slash_commands::sentence(errors.full_messages()),
                    422,
                )),
                Err(error) => Err(error),
            }
        })
        .await
        .map_err(db_error)?;
    render_result(c, result)
}

fn profile_payload(
    conn: &campfire_db::Connection,
    agent: &campfire_db::Agent,
    now: campfire_db::Timestamp,
) -> campfire_db::Result<Value> {
    let name = campfire_db::User::find_by_id(conn, agent.user_id)?
        .ok_or(campfire_db::Error::RecordNotFound("User"))?
        .name;
    let owner = agent
        .owner_id
        .map(|id| campfire_db::User::find_by_id(conn, id))
        .transpose()?
        .flatten()
        .map(|user| json!({"id":user.id,"name":user.name}));
    let mut payload = json!({"id":agent.id,"kind":agent.kind.name(),"name":name,"user_id":agent.user_id,"owner":owner,"provider":agent.provider,"runtime":agent.runtime,"description":agent.description,"status":agent.status,"status_note":agent.status_note,"status_changed_at":agent.status_changed_at.map(json_time),"last_seen_at":agent.last_seen_at.map(json_time),"working_presence":agent.working_presence_text(now)});
    payload
        .as_object_mut()
        .unwrap()
        .retain(|_, value| !value.is_null());
    Ok(payload)
}
fn json_time(time: campfire_db::Timestamp) -> String {
    format!(
        "{}.{:03}Z",
        time.jiff().strftime("%Y-%m-%dT%H:%M:%S"),
        time.subsec_microsecond() / 1000
    )
}
use crate::app::AppCtx;
use crate::concerns::{self, agent_api};

pub async fn events(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, false).await?;
    ensure_anywhere(c, identity.agent_id, "read_messages").await?;
    agent_api::throttle(c, 120, "agents/events", "index")?;
    agent_api::no_store(c);
    let page = poll(c, identity.agent_id, field(c, "since"), field(c, "limit")).await?;
    c.set_header("x-smartfire-next-since", &page["next_since"].to_string());
    let payload = if c.param_str("envelope") == Some("1") {
        page
    } else {
        page["events"].clone()
    };
    Ok(c.render(StatusCode::OK, &format::JSON, payload.to_string()))
}

pub async fn poll(
    c: &Ctx,
    agent_id: i64,
    since: Option<Value>,
    limit: Option<Value>,
) -> Result<Value> {
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let since_id = since.as_ref().map_or(0, ruby_i64);
    let page_limit = limit.as_ref().filter(|v| !mcp::blank(v)).map(ruby_i64);
    let events = c.app().db.read(move |conn| {
        Ok(campfire_db::models::agent_event_access::readable_page(conn, agent_id, since_id, page_limit)?.into_iter().map(|event| event.id).collect())
    }).await.map_err(db_error)?;
    let access = c.app().agent_repositories.resolve_events(&c.app().db, agent_id, events).await.map_err(db_error)?;
    super::messages::present(c, move |presenter| {
        agent_event_polling::poll(
            presenter.conn,
            agent_id,
            since.as_ref(),
            limit.as_ref(),
            now,
            &access,
            |message| presenter.agent_message_payload(message),
        )
    })
    .await
}

pub async fn ack(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, false).await?;
    agent_api::throttle(c, 120, "agents/events", "ack")?;
    agent_api::no_store(c);
    let id = id(c, "id");
    let result = c
        .app()
        .db
        .write(move |tx| acknowledge(tx, identity.agent_id, id))
        .await
        .map_err(db_error)?;
    render_result(c, result)
}

pub fn acknowledge(
    tx: &campfire_db::Tx<'_>,
    agent_id: i64,
    id: i64,
) -> campfire_db::Result<ServiceResult> {
    Ok(match agent_event_access::acknowledge(tx, agent_id, id)? {
        agent_event_access::Acknowledgment::Acknowledged { id } => {
            ServiceResult::ok(json!({"id":id,"outcome":"acknowledged"}), 200)
        }
        agent_event_access::Acknowledgment::NotFound => ServiceResult::fail("Event not found", 404),
        agent_event_access::Acknowledgment::Forbidden => {
            ServiceResult::fail("Forbidden: agent lacks read_messages capability", 403)
        }
    })
}

pub async fn ensure_anywhere(c: &mut Ctx, agent_id: i64, capability: &'static str) -> Result<()> {
    let allowed = c
        .app()
        .db
        .read(move |conn| {
            campfire_db::models::agent_access::has_capability_anywhere(conn, agent_id, capability)
        })
        .await
        .map_err(db_error)?;
    if !allowed {
        return halt(c.render(
            StatusCode::FORBIDDEN,
            &format::JSON,
            json!({"error":format!("Forbidden: agent lacks {capability} capability")}).to_string(),
        ));
    }
    Ok(())
}

pub async fn create_step(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, true).await?;
    agent_api::throttle(c, 60, "agents/steps", "create")?;
    agent_api::no_store(c);
    let fields = fields(c);
    let result = c
        .app()
        .db
        .write(move |tx| create_step_service(tx, identity.agent_id, &fields))
        .await
        .map_err(db_error)?;
    render_result(c, result)
}
pub async fn update_step(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, true).await?;
    agent_api::throttle(c, 60, "agents/steps", "update")?;
    agent_api::no_store(c);
    let fields = fields(c);
    let id = id(c, "id");
    let result = c
        .app()
        .db
        .write(move |tx| update_step_service(tx, identity.agent_id, id, &fields))
        .await
        .map_err(db_error)?;
    render_result(c, result)
}

pub fn create_step_service(
    tx: &mut campfire_db::Tx<'_>,
    agent_id: i64,
    fields: &Value,
) -> campfire_db::Result<ServiceResult> {
    use campfire_db::models::agent_step::{self, NewAgentStep};
    let (duration_ms, input_errors) = duration_input(fields.get("duration_ms"));
    agent_step::create_with_input_errors(
        tx,
        agent_id,
        NewAgentStep {
            message_id: integer(fields.get("message_id")),
            channel_thread_id: integer(fields.get("thread_id")),
            name: attribute_string(fields.get("name")).unwrap_or_default(),
            status: attribute_string(fields.get("status")).unwrap_or_else(|| "running".into()),
            input_summary: attribute_string(fields.get("input_summary")),
            output_summary: attribute_string(fields.get("output_summary")),
            duration_ms,
            ..Default::default()
        },
        input_errors,
    )
}
pub fn update_step_service(
    tx: &mut campfire_db::Tx<'_>,
    agent_id: i64,
    id: i64,
    fields: &Value,
) -> campfire_db::Result<ServiceResult> {
    use campfire_db::models::agent_step::{self, AgentStepChanges};
    let (duration_ms, input_errors) = duration_input(fields.get("duration_ms"));
    agent_step::update_with_input_errors(
        tx,
        agent_id,
        id,
        AgentStepChanges {
            name: fields
                .get("name")
                .map(|v| attribute_string(Some(v)).unwrap_or_default()),
            status: fields
                .get("status")
                .map(|v| attribute_string(Some(v)).unwrap_or_default()),
            input_summary: fields
                .get("input_summary")
                .map(|v| attribute_string(Some(v))),
            output_summary: fields
                .get("output_summary")
                .map(|v| attribute_string(Some(v))),
            duration_ms: fields.get("duration_ms").map(|_| duration_ms),
        },
        input_errors,
    )
}

pub async fn register_command(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, true).await?;
    agent_api::throttle(c, 60, "agents/slash_commands", "create")?;
    let room = id(c, "room_id");
    let name = c
        .params
        .get("name")
        .and_then(Param::to_s)
        .unwrap_or_default();
    let description = c.params.get("description").and_then(Param::to_s);
    let takes_arguments = c
        .params
        .get("takes_arguments")
        .and_then(Param::to_s)
        .as_deref()
        .and_then(campfire_db::account::cast_boolean);
    let result = c
        .app()
        .db
        .write(move |tx| {
            campfire_db::models::agent_slash_command::register(
                tx,
                identity.agent_id,
                room,
                &name,
                description.as_deref(),
                takes_arguments,
            )
        })
        .await
        .map_err(db_error)?;
    render_result(c, result)
}
pub async fn unregister_command(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, true).await?;
    agent_api::throttle(c, 60, "agents/slash_commands", "destroy")?;
    let room = id(c, "room_id");
    let name = c.param_str("name").unwrap_or_default().to_owned();
    let result = c
        .app()
        .db
        .write(move |tx| {
            campfire_db::models::agent_slash_command::unregister(tx, identity.agent_id, room, &name)
        })
        .await
        .map_err(db_error)?;
    render_result(c, result)
}

pub fn render_result(c: &mut Ctx, result: ServiceResult) -> Result {
    let status = StatusCode::from_u16(result.status).map_err(campfire_kit::Error::internal)?;
    if result.status == 404 {
        return Ok(c.head(status));
    }
    let body = if result.is_ok() {
        result.payload.unwrap_or(Value::Null)
    } else {
        result.failure_body()
    };
    Ok(c.render(status, &format::JSON, body.to_string()))
}
fn field(c: &Ctx, key: &str) -> Option<Value> {
    c.params.get(key).map(Param::to_json)
}
fn fields(c: &Ctx) -> Value {
    Value::Object(
        [
            "message_id",
            "thread_id",
            "name",
            "status",
            "input_summary",
            "output_summary",
            "duration_ms",
        ]
        .into_iter()
        .filter_map(|key| {
            c.params
                .get(key)
                .filter(|p| !matches!(p, Param::Array(_) | Param::Hash(_)))
                .map(|p| (key.to_owned(), p.to_json()))
        })
        .collect(),
    )
}
fn id(c: &Ctx, key: &str) -> i64 {
    c.params
        .get(key)
        .map(Param::to_json)
        .as_ref()
        .map_or(0, ruby_i64)
}
pub fn integer(value: Option<&Value>) -> Option<i64> {
    value
        .filter(|v| !v.is_null() && !v.as_str().is_some_and(|s| s.is_empty()))
        .map(ruby_i64)
}
pub fn text(value: Option<&Value>) -> Option<String> {
    value.filter(|v| !v.is_null()).map(|v| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| ruby_inspect(v))
    })
}

pub fn ruby_i64(value: &Value) -> i64 {
    match value {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Value::String(s) => concerns::ruby_to_i(s),
        _ => 0,
    }
}

fn attribute_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::Bool(true)) => Some("t".into()),
        Some(Value::Bool(false)) => Some("f".into()),
        _ => text(value),
    }
}

fn duration_input(value: Option<&Value>) -> (Option<i64>, campfire_db::Errors) {
    let mut errors = campfire_db::Errors::default();
    let invalid = match value {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => None,
        Some(Value::Number(_)) => Some("must be an integer"),
        Some(Value::String(s)) => {
            if s.trim().parse::<f64>().is_err() {
                Some("is not a number")
            } else {
                let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    Some("must be an integer")
                } else {
                    None
                }
            }
        }
        _ => Some("is not a number"),
    };
    if let Some(message) = invalid {
        errors.add("duration_ms", message);
        (None, errors)
    } else {
        (integer(value), errors)
    }
}

pub fn ruby_inspect(value: &Value) -> String {
    match value {
        Value::Null => "nil".into(),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(ruby_inspect)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{} => {}", json!(key), ruby_inspect(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => value.to_string(),
    }
}

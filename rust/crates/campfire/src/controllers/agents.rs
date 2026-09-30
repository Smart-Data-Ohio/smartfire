//! Thin JSON adapters for WS11's shared services. Rails agents/* controllers are
//! the reference; the MCP adapter uses the same operations and minute buckets.
use campfire_db::models::{agent_event_access, agent_event_polling, agent_service::ServiceResult};
use campfire_kit::{Ctx, Param, Result, StatusCode, format, halt};
use serde_json::{Value, json};

use super::presenters::page::db_error;
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
    super::messages::present(c, move |presenter| {
        // WS11/WS15g supplies live private-repository access decisions. The default
        // set reveals only public fields, matching the current delivery adapter.
        agent_event_polling::poll(
            presenter.conn,
            agent_id,
            since.as_ref(),
            limit.as_ref(),
            now,
            &Default::default(),
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
    agent_step::create(
        tx,
        agent_id,
        NewAgentStep {
            message_id: integer(fields.get("message_id")),
            channel_thread_id: integer(fields.get("thread_id")),
            name: text(fields.get("name")).unwrap_or_default(),
            status: text(fields.get("status")).unwrap_or_else(|| "running".into()),
            input_summary: text(fields.get("input_summary")),
            output_summary: text(fields.get("output_summary")),
            duration_ms: integer(fields.get("duration_ms")),
            ..Default::default()
        },
    )
}
pub fn update_step_service(
    tx: &mut campfire_db::Tx<'_>,
    agent_id: i64,
    id: i64,
    fields: &Value,
) -> campfire_db::Result<ServiceResult> {
    use campfire_db::models::agent_step::{self, AgentStepChanges};
    agent_step::update(
        tx,
        agent_id,
        id,
        AgentStepChanges {
            name: fields
                .get("name")
                .map(|v| text(Some(v)).unwrap_or_default()),
            status: fields
                .get("status")
                .map(|v| text(Some(v)).unwrap_or_default()),
            input_summary: fields.get("input_summary").map(|v| text(Some(v))),
            output_summary: fields.get("output_summary").map(|v| text(Some(v))),
            duration_ms: fields.get("duration_ms").map(|v| integer(Some(v))),
        },
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
            .unwrap_or_else(|| v.to_string())
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

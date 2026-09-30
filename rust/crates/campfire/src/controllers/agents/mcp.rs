//! Agents::McpController: stateless Streamable HTTP, with Rails protocol metadata.
//! Tool operations call WS11's shared services. Unported operations deliberately
//! produce Internal error; they are tracked as partial in the WS11-api report.
use std::sync::LazyLock;

use base64::{Engine, engine::general_purpose::STANDARD};
use campfire_db::models::agent_service::ServiceResult;
use campfire_kit::{Ctx, Result, StatusCode, format};
use serde_json::{Value, json};

use super::{acknowledge, create_step_service, poll, ruby_i64, text, update_step_service};
use crate::app::AppCtx;
use crate::concerns::agent_api;

static METADATA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("mcp_metadata.json")).expect("vendored MCP metadata")
});
const MODERN: &str = "2026-07-28";
const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";

pub async fn method_not_allowed(c: &mut Ctx) -> Result {
    agent_api::require_token(c, true).await?;
    Ok(rpc_error(
        c,
        Value::Null,
        -32600,
        "Only POST is supported on the MCP endpoint",
        None,
        StatusCode::METHOD_NOT_ALLOWED,
    ))
}

pub async fn create(c: &mut Ctx) -> Result {
    // The unparsed kit adapter bounds/spools the complete upload before here.
    // Rails parses application/json params before authentication callbacks.
    let limit = c
        .kit()
        .config()
        .max_body_bytes
        .unwrap_or(usize::MAX)
        .min(campfire_kit::body::MAX_BUFFERED_BODY);
    let raw = c.read_body(limit + 1).await;
    if raw.len() > limit {
        return Err(campfire_kit::Error::Status(StatusCode::PAYLOAD_TOO_LARGE));
    }
    let parsed = serde_json::from_slice::<Value>(&raw);
    if c.request.media_type().as_deref() == Some("application/json")
        && !raw.is_empty()
        && parsed.is_err()
    {
        return Ok(rpc_error(
            c,
            Value::Null,
            -32700,
            "Parse error",
            None,
            StatusCode::BAD_REQUEST,
        ));
    }
    let identity = agent_api::require_token(c, true).await?;
    agent_api::no_store(c);
    agent_api::throttle(c, 600, "agents/mcp", "create")?;
    if invalid_origin(c) {
        return Ok(rpc_error(
            c,
            Value::Null,
            -32600,
            "Invalid origin",
            None,
            StatusCode::FORBIDDEN,
        ));
    }
    let envelope = match parsed {
        Ok(value) => value,
        Err(_) => {
            return Ok(rpc_error(
                c,
                Value::Null,
                -32700,
                "Parse error",
                None,
                StatusCode::BAD_REQUEST,
            ));
        }
    };
    let id = envelope.get("id").cloned().unwrap_or(Value::Null);
    if !envelope.is_object() || envelope["jsonrpc"] != "2.0" || !envelope["method"].is_string() {
        return Ok(rpc_error(
            c,
            id,
            -32600,
            "Invalid request",
            None,
            StatusCode::BAD_REQUEST,
        ));
    }
    if envelope.get("id").is_none() {
        return Ok(c.head(StatusCode::ACCEPTED));
    }
    let method = envelope["method"].as_str().unwrap();
    let params = envelope
        .get("params")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let version = match version(c, method, &params) {
        Ok(version) => version,
        Err((code, message, data)) => {
            return Ok(rpc_error(
                c,
                id,
                code,
                &message,
                data,
                StatusCode::BAD_REQUEST,
            ));
        }
    };
    let modern = version == MODERN;
    if modern && let Some(message) = header_mismatch(c, method, &params) {
        return Ok(rpc_error(
            c,
            id,
            -32020,
            &message,
            None,
            StatusCode::BAD_REQUEST,
        ));
    }
    match method {
        "initialize" => Ok(rpc_result(
            c,
            id,
            json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":METADATA["server_name"],"version":METADATA["server_version"]},"instructions":METADATA["instructions"]}),
        )),
        "server/discover" => Ok(rpc_result(
            c,
            id,
            modern_result(
                json!({"supportedVersions":METADATA["versions"],"capabilities":{"tools":{}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":METADATA["server_name"],"version":METADATA["server_version"]}},"instructions":METADATA["instructions"]}),
                modern,
            ),
        )),
        "tools/list" => {
            let tools:Vec<_>=METADATA["tools"].as_array().unwrap().iter().map(|tool|json!({"name":tool["name"],"description":tool["description"],"inputSchema":tool["inputSchema"]})).collect();
            Ok(rpc_result(
                c,
                id,
                modern_result(json!({"tools":tools}), modern),
            ))
        }
        "tools/call" => call_tool(c, id, &params, identity.agent_id, modern).await,
        "ping" => Ok(rpc_result(c, id, modern_result(json!({}), modern))),
        notification if notification.starts_with("notifications/") => {
            Ok(rpc_result(c, id, json!({})))
        }
        _ => Ok(rpc_error(
            c,
            id,
            -32601,
            &format!("Method not found: {method}"),
            None,
            StatusCode::NOT_FOUND,
        )),
    }
}

fn invalid_origin(c: &Ctx) -> bool {
    let Some(origin) = c
        .request
        .origin()
        .filter(|s| !s.chars().all(char::is_whitespace))
    else {
        return false;
    };
    if !origin.is_ascii()
        || origin
            .chars()
            .any(|c| c.is_ascii_whitespace() || c.is_ascii_control())
        || (!origin.contains("://") && !origin.starts_with("//"))
    {
        return true;
    }
    // Ruby URI preserves an encoded host; WHATWG URL decodes it. Such an
    // authority cannot equal request.host, so don't normalize it into a match.
    let authority = origin
        .strip_prefix("//")
        .unwrap_or_else(|| origin.split_once("://").unwrap().1)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    if host.contains('%') {
        return true;
    }
    // URI.parse also accepts a network-path reference with an authority.
    let origin = if origin.starts_with("//") {
        format!("http:{origin}")
    } else {
        origin.to_owned()
    };
    url::Url::parse(&origin)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_none_or(|host| !host.eq_ignore_ascii_case(&c.request.host()))
}

type VersionError = (i64, String, Option<Value>);
fn version(c: &Ctx, method: &str, params: &Value) -> std::result::Result<Value, VersionError> {
    let header = c
        .request
        .header("mcp-protocol-version")
        .filter(|s| !s.chars().all(char::is_whitespace))
        .map(|s| json!(s));
    let meta = params
        .get("_meta")
        .filter(|v| v.is_object())
        .and_then(|m| m.get(VERSION_KEY))
        .filter(|v| !blank(v))
        .cloned();
    if let (Some(header), Some(meta)) = (&header, &meta)
        && header != meta
    {
        return Err((
            -32020,
            format!(
                "Header mismatch: MCP-Protocol-Version header value {} does not match body value {}",
                inspect(header),
                inspect(meta)
            ),
            None,
        ));
    }
    let requested = header
        .or(meta)
        .or_else(|| {
            (method == "initialize")
                .then(|| params.get("protocolVersion").filter(|v| !blank(v)).cloned())
                .flatten()
        })
        .unwrap_or_else(|| json!("2025-03-26"));
    if !METADATA["versions"]
        .as_array()
        .unwrap()
        .contains(&requested)
    {
        return Err((
            -32022,
            "Unsupported protocol version".into(),
            Some(json!({"supported":METADATA["versions"],"requested":requested})),
        ));
    }
    Ok(requested)
}

fn header_mismatch(c: &Ctx, method: &str, params: &Value) -> Option<String> {
    let Some(header) = c
        .request
        .header("mcp-method")
        .filter(|s| !s.chars().all(char::is_whitespace))
    else {
        return Some("Header mismatch: Mcp-Method header is required".into());
    };
    if header != method {
        return Some(format!(
            "Header mismatch: Mcp-Method header value {} does not match body value {}",
            inspect(&json!(header)),
            inspect(&json!(method))
        ));
    }
    if method != "tools/call" {
        return None;
    }
    let Some(name) = c
        .request
        .header("mcp-name")
        .filter(|s| !s.chars().all(char::is_whitespace))
    else {
        return Some("Header mismatch: Mcp-Name header is required".into());
    };
    let decoded = decode_header(name);
    let Some(decoded) = decoded else {
        return Some("Header mismatch: Mcp-Name header value is malformed".into());
    };
    let expected = text(params.get("name")).unwrap_or_default();
    (decoded != expected).then(|| {
        format!(
            "Header mismatch: Mcp-Name header value {} does not match body value {}",
            inspect(&json!(name)),
            inspect(&json!(expected))
        )
    })
}
fn decode_header(value: &str) -> Option<String> {
    if let Some(inner) = value
        .strip_prefix("=?base64?")
        .and_then(|s| s.strip_suffix("?="))
        .filter(|s| !s.is_empty() && !s.contains('\n'))
    {
        String::from_utf8(STANDARD.decode(inner).ok()?).ok()
    } else {
        Some(value.into())
    }
}

async fn call_tool(c: &mut Ctx, id: Value, params: &Value, agent_id: i64, modern: bool) -> Result {
    let name = params.get("name").unwrap_or(&Value::Null);
    let Some(tool) = METADATA["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| &tool["name"] == name)
    else {
        return Ok(rpc_error(
            c,
            id,
            -32602,
            &format!("Unknown tool: {}", inspect(name)),
            None,
            StatusCode::OK,
        ));
    };
    let args = params.get("arguments").unwrap_or(&Value::Null);
    let tool_name = name.as_str().unwrap();
    if let Some(throttle) = tool["throttle"].as_array() {
        let charges = if tool_name == "ack_events" {
            let ids = args.get("event_ids").and_then(Value::as_array);
            if ids.is_some_and(|ids| ids.len() > 100) {
                return Ok(rpc_error(
                    c,
                    id,
                    -32602,
                    "event_ids must contain at most 100 ids",
                    None,
                    StatusCode::OK,
                ));
            }
            ids.map_or(1, |ids| ids.len().max(1))
        } else {
            1
        };
        for _ in 0..charges {
            if let Some(seconds) = agent_api::retry_after(
                c,
                throttle[0].as_u64().unwrap(),
                throttle[1].as_str().unwrap(),
                throttle[2].as_str().unwrap(),
            ) {
                c.set_header("retry-after", &seconds.to_string());
                return Ok(rpc_result(
                    c,
                    id,
                    modern_result(
                        json!({"content":[{"type":"text","text":format!("rate_limited: retry after {seconds} seconds")}],"structuredContent":{"error":"rate_limited","retry_after":seconds},"isError":true}),
                        modern,
                    ),
                ));
            }
        }
    }
    if !args.is_null() && !args.is_object() {
        return Ok(rpc_error(
            c,
            id,
            -32602,
            "arguments must be an object",
            None,
            StatusCode::OK,
        ));
    }
    let args = if args.is_null() {
        json!({})
    } else {
        args.clone()
    };
    let result = execute(c, agent_id, tool_name, args).await;
    match result {
        Ok(result) => {
            let body = if result.is_ok() {
                let payload = result.payload.unwrap_or(Value::Null);
                json!({"content":[{"type":"text","text":payload.to_string()}],"structuredContent":payload,"isError":false})
            } else {
                let error = result.error.unwrap_or_default();
                let mut structured = json!({"error":error,"status":status_name(result.status)});
                if let Some(payload) = result.payload {
                    for key in ["cap", "limit", "retry_after"] {
                        if let Some(value) = payload.get(key) {
                            structured[key] = value.clone();
                        }
                    }
                }
                json!({"content":[{"type":"text","text":error}],"structuredContent":structured,"isError":true})
            };
            Ok(rpc_result(c, id, modern_result(body, modern)))
        }
        Err(ToolError::Invalid(message)) => {
            Ok(rpc_error(c, id, -32602, &message, None, StatusCode::OK))
        }
        Err(ToolError::Internal(error)) => {
            tracing::error!(%error,tool=tool_name,"MCP tools/call failed");
            Ok(rpc_error(
                c,
                id,
                -32603,
                "Internal error",
                None,
                StatusCode::OK,
            ))
        }
    }
}

enum ToolError {
    Invalid(String),
    Internal(campfire_kit::Error),
}
impl From<campfire_kit::Error> for ToolError {
    fn from(error: campfire_kit::Error) -> Self {
        Self::Internal(error)
    }
}
fn required<'a>(args: &'a Value, key: &str) -> std::result::Result<&'a Value, ToolError> {
    args.get(key)
        .filter(|v| !blank(v))
        .ok_or_else(|| ToolError::Invalid(format!("Missing required argument: {key}")))
}
async fn execute(
    c: &Ctx,
    agent_id: i64,
    name: &str,
    args: Value,
) -> std::result::Result<ServiceResult, ToolError> {
    use super::super::presenters::page::db_error;
    match name {
        "poll_events" => {
            let allowed = c
                .app()
                .db
                .read(move |conn| {
                    campfire_db::models::agent_access::has_capability_anywhere(
                        conn,
                        agent_id,
                        "read_messages",
                    )
                })
                .await
                .map_err(db_error)?;
            if !allowed {
                return Ok(ServiceResult::fail(
                    "Forbidden: agent lacks read_messages capability",
                    403,
                ));
            }
            if args
                .get("since")
                .is_some_and(|v| !v.is_null() && !v.is_string() && !v.is_number())
                || args
                    .get("limit")
                    .is_some_and(|v| !blank(v) && !v.is_string() && !v.is_number())
            {
                return Err(ToolError::Internal(campfire_kit::Error::internal(
                    anyhow::anyhow!("Ruby to_i unavailable for event cursor"),
                )));
            }
            Ok(ServiceResult::ok(
                poll(
                    c,
                    agent_id,
                    args.get("since").cloned(),
                    args.get("limit").cloned(),
                )
                .await?,
                200,
            ))
        }
        "ack_events" => {
            let ids = args
                .get("event_ids")
                .and_then(Value::as_array)
                .filter(|ids| !ids.is_empty())
                .ok_or_else(|| ToolError::Invalid("event_ids must be a non-empty array".into()))?
                .clone();
            let result = c
                .app()
                .db
                .write(move |tx| {
                    if !campfire_db::models::agent_access::has_capability_anywhere(
                        tx.conn(),
                        agent_id,
                        "read_messages",
                    )? {
                        return Ok(ServiceResult::fail(
                            "Forbidden: agent lacks read_messages capability",
                            403,
                        ));
                    }
                    let mut results = vec![];
                    for id in ids {
                        let result = acknowledge(tx, agent_id, ruby_i64(&id))?;
                        results.push(if result.is_ok() {
                            json!({"id":id,"outcome":"acknowledged"})
                        } else {
                            json!({"id":id,"error":result.error})
                        });
                    }
                    Ok(ServiceResult::ok(json!({"results":results}), 200))
                })
                .await
                .map_err(db_error)?;
            Ok(result)
        }
        "add_step" => {
            required(&args, "name")?;
            Ok(c.app()
                .db
                .write(move |tx| create_step_service(tx, agent_id, &args))
                .await
                .map_err(db_error)?)
        }
        "update_step" => {
            let id = ruby_i64(required(&args, "step_id")?);
            Ok(c.app()
                .db
                .write(move |tx| update_step_service(tx, agent_id, id, &args))
                .await
                .map_err(db_error)?)
        }
        "register_slash_command" | "unregister_slash_command" => {
            let room = ruby_i64(required(&args, "room_id")?);
            let command = text(Some(required(&args, "name")?)).unwrap_or_default();
            let register = name == "register_slash_command";
            Ok(c.app()
                .db
                .write(move |tx| {
                    if register {
                        let description = text(args.get("description"));
                        let takes = text(args.get("takes_arguments"))
                            .as_deref()
                            .and_then(campfire_db::account::cast_boolean);
                        campfire_db::models::agent_slash_command::register(
                            tx,
                            agent_id,
                            room,
                            &command,
                            description.as_deref(),
                            takes,
                        )
                    } else {
                        campfire_db::models::agent_slash_command::unregister(
                            tx, agent_id, room, &command,
                        )
                    }
                })
                .await
                .map_err(db_error)?)
        }
        "set_presence" => Ok(c
            .app()
            .db
            .write(move |tx| {
                if let Some(value) = args.get("text") {
                    let text = text(Some(value));
                    campfire_db::models::agent_working_presence::set(tx, agent_id, text.as_deref())
                } else {
                    let agent = campfire_db::Agent::find(tx.conn(), agent_id)?
                        .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
                    Ok(ServiceResult::ok(
                        json!({"working_presence":agent.working_presence_text(tx.now())}),
                        200,
                    ))
                }
            })
            .await
            .map_err(db_error)?),
        _ => Err(ToolError::Internal(campfire_kit::Error::internal(
            anyhow::anyhow!("agent tool service not yet ported: {name}"),
        ))),
    }
}

fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => campfire_richtext::ruby::is_blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
        _ => false,
    }
}
fn inspect(value: &Value) -> String {
    super::ruby_inspect(value)
}
fn modern_result(mut result: Value, modern: bool) -> Value {
    if modern {
        result["resultType"] = json!("complete");
    }
    result
}
fn status_name(status: u16) -> &'static str {
    match status {
        200 => "ok",
        201 => "created",
        400 => "bad_request",
        401 => "unauthorized",
        403 => "forbidden",
        404 => "not_found",
        422 => "unprocessable_entity",
        429 => "too_many_requests",
        _ => "internal_server_error",
    }
}
fn rpc_result(c: &mut Ctx, id: Value, result: Value) -> campfire_kit::Response {
    c.render(
        StatusCode::OK,
        &format::JSON,
        json!({"jsonrpc":"2.0","id":id,"result":result}).to_string(),
    )
}
fn rpc_error(
    c: &mut Ctx,
    id: Value,
    code: i64,
    message: &str,
    data: Option<Value>,
    status: StatusCode,
) -> campfire_kit::Response {
    let mut error = json!({"code":code,"message":message});
    if let Some(data) = data {
        error["data"] = data;
    }
    c.render(
        status,
        &format::JSON,
        json!({"jsonrpc":"2.0","id":id,"error":error}).to_string(),
    )
}

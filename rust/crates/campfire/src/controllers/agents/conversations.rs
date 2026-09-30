//! REST/MCP presentation around WS11's context, Posting and DirectMessages services.
use super::{integer, ruby_i64, text};
use crate::app::AppCtx;
use crate::concerns;
use crate::controllers::{messages, presenters::page::db_error};
use campfire_db::models::{
    agent_access, agent_context, agent_direct_messages, agent_posting,
    agent_service::ServiceResult, audit_log,
};
use campfire_db::{Agent, NewMessage, Room};
use campfire_kit::{Ctx, Result};
use serde_json::{Value, json};

fn present_id(args: &Value, key: &str) -> Option<i64> {
    args.get(key)
        .filter(|v| !super::mcp::blank(v))
        .map(ruby_i64)
}

pub async fn context(c: &Ctx, agent_id: i64, args: Value) -> Result<ServiceResult> {
    let now = campfire_db::Timestamp::from_jiff(c.now());
    messages::present(c, move |presenter| {
        agent_context::build(
            presenter.conn,
            agent_id,
            present_id(&args, "message_id"),
            present_id(&args, "thread_id"),
            args.get("limit"),
            now,
            |m| presenter.agent_message_payload(m),
        )
    })
    .await
}

fn scalar<'a>(fields: &'a Value, key: &str) -> Option<&'a Value> {
    fields.get(key).filter(|v| !v.is_array() && !v.is_object())
}
fn attributes(args: &Value) -> NewMessage {
    let fields = args
        .get("message")
        .filter(|v| v.is_object())
        .unwrap_or(args);
    NewMessage {
        room_id: args.get("room_id").map_or(0, ruby_i64),
        thread_id: present_id(args, "thread_id"),
        client_message_id: super::attribute_string(scalar(fields, "client_message_id")),
        body: text(scalar(fields, "body")),
        markdown_source: super::attribute_string(scalar(fields, "markdown_source")),
        reply_to_message_id: integer(scalar(fields, "reply_to_message_id")),
        reply_notify_author: scalar(fields, "reply_notify_author")
            .and_then(|v| text(Some(v)))
            .as_deref()
            .and_then(campfire_db::account::cast_boolean),
        ..Default::default()
    }
}
fn drive(args: &Value) -> agent_posting::DriveInput {
    let fields = args
        .get("message")
        .filter(|v| v.is_object())
        .unwrap_or(args);
    match fields.get("drive_file_ids") {
        None => agent_posting::DriveInput::Absent,
        Some(Value::Array(ids)) => {
            let mut result = Vec::new();
            for id in ids {
                let id = text(Some(id)).unwrap_or_default();
                let id = campfire_richtext::ruby::strip(&id).to_owned();
                if !campfire_richtext::ruby::is_blank(&id) && !result.contains(&id) {
                    result.push(id);
                }
            }
            agent_posting::DriveInput::Ids(result)
        }
        _ => agent_posting::DriveInput::Invalid,
    }
}
async fn canonical(c: &Ctx, args: &Value) -> Result<NewMessage> {
    let mut a = attributes(args);
    if a.markdown_source.is_none()
        && let Some(body) = a.body.take()
    {
        a.body = Some(messages::canonicalize_body(c.app(), body, Some(c.request.host())).await?);
    }
    Ok(a)
}

pub async fn post(c: &Ctx, agent_id: i64, args: Value) -> Result<ServiceResult> {
    let a = canonical(c, &args).await?;
    let drive = drive(&args);
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let agent = Agent::find(tx.conn(), agent_id)?
                .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            if Room::find_for_user(tx.conn(), agent.user_id, a.room_id)?.is_none() {
                return Ok(agent_posting::PostResult::Denied(ServiceResult::fail(
                    "Room not found",
                    404,
                )));
            }
            if !agent_access::capability_for_agent(
                tx.conn(),
                agent_id,
                "post_messages",
                Some(a.room_id),
            )? {
                return Ok(agent_posting::PostResult::Denied(ServiceResult::fail(
                    "Forbidden: agent lacks post_messages capability",
                    403,
                )));
            }
            match agent_posting::post_service(tx, agent_id, a, drive) {
                Err(campfire_db::Error::RecordNotFound(_)) => {
                    Ok(agent_posting::PostResult::Denied(ServiceResult::fail(
                        "Reply target not found",
                        404,
                    )))
                }
                result => result,
            }
        })
        .await
        .map_err(db_error)?;
    match outcome {
        agent_posting::PostResult::Denied(result) => Ok(result),
        agent_posting::PostResult::Posted(message) => {
            messages::present(c, move |p| {
                let mut payload = p.agent_message_payload(&message)?;
                payload["thread_id"] = json!(message.thread_id);
                Ok(ServiceResult::ok(payload, 201))
            })
            .await
        }
    }
}

pub async fn dm(c: &Ctx, agent_id: i64, args: Value) -> Result<ServiceResult> {
    let a = canonical(c, &args).await?;
    let drive = drive(&args);
    let target = args.get("user_id").map_or(0, ruby_i64);
    let audit = audit_log::Context {
        actor: concerns::current_user(c).map(Into::into),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_owned),
    };
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            agent_direct_messages::open_and_post(tx, agent_id, target, a, drive, &audit)
        })
        .await
        .map_err(db_error)?;
    match outcome {
        agent_direct_messages::DirectMessageResult::Denied(result) => Ok(result),
        agent_direct_messages::DirectMessageResult::Posted {room,message} => messages::present(c,move |p| {
            Ok(ServiceResult::ok(json!({"room":{"id":room.id,"name":room.name,"direct":true},"message":p.agent_message_payload(&message)?,"thread_id":message.thread_id}),201))
        }).await,
    }
}

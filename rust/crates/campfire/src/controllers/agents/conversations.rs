//! REST/MCP presentation around WS11's context, Posting and DirectMessages services.
use super::{ruby_i64, text};
use crate::app::AppCtx;
use crate::concerns;
use crate::controllers::{messages, presenters::page::db_error};
use campfire_db::models::{
    agent_access, agent_context, agent_direct_messages, agent_posting,
    agent_service::ServiceResult, agent_streaming, audit_log,
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

// Strong parameters filter REST attributes; MCP passes its documented slice directly.
fn fields(args: &Value, rest: bool) -> &Value {
    if rest {
        args.get("message")
            .filter(|v| v.is_object())
            .unwrap_or(args)
    } else {
        args
    }
}
fn attribute<'a>(fields: &'a Value, key: &str, rest: bool) -> Option<&'a Value> {
    fields
        .get(key)
        .filter(|v| !rest || (!v.is_array() && !v.is_object()))
}
fn attributes(args: &Value, rest: bool, dm: bool) -> NewMessage {
    let fields = fields(args, rest);
    let replies = !dm || (rest && args.get("message").is_some_and(Value::is_object));
    let reply = attribute(fields, "reply_to_message_id", rest).filter(|v| !super::mcp::blank(v));
    NewMessage {
        room_id: args.get("room_id").map_or(0, ruby_i64),
        thread_id: present_id(args, "thread_id"),
        client_message_id: super::attribute_string(attribute(fields, "client_message_id", rest)),
        body: text(attribute(fields, "body", rest)),
        markdown_source: super::attribute_string(attribute(fields, "markdown_source", rest)),
        reply_to_message_id: if replies { reply.map(ruby_i64) } else { None },
        reply_notify_author: if replies {
            attribute(fields, "reply_notify_author", rest)
                .and_then(|v| text(Some(v)))
                .as_deref()
                .and_then(campfire_db::account::cast_boolean)
        } else {
            None
        },
        ..Default::default()
    }
}
fn drive(args: &Value, rest: bool) -> agent_posting::DriveInput {
    let fields = fields(args, rest);
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
async fn canonical(c: &Ctx, args: &Value, rest: bool, dm: bool) -> Result<NewMessage> {
    let mut a = attributes(args, rest, dm);
    if a.markdown_source.is_none()
        && let Some(body) = a.body.take()
    {
        a.body = Some(messages::canonicalize_body(c.app(), body, Some(c.request.host())).await?);
    }
    Ok(a)
}

pub async fn post(c: &Ctx, agent_id: i64, args: Value, rest: bool) -> Result<ServiceResult> {
    start(c, agent_id, args, false, rest).await
}
async fn start(
    c: &Ctx,
    agent_id: i64,
    args: Value,
    streaming: bool,
    rest: bool,
) -> Result<ServiceResult> {
    if rest && args.get("message").is_some_and(|v| !v.is_object()) {
        // Rails params.require(:message).permit raises for non-Parameters values.
        return Err(campfire_kit::Error::internal(anyhow::anyhow!(
            "message does not support permit"
        )));
    }
    let a = if streaming {
        let mut a = attributes(&args, rest, false);
        a.body = None;
        a
    } else {
        canonical(c, &args, rest, false).await?
    };
    let drive = drive(&args, rest);
    let assignment = if rest && !streaming {
        let params = c
            .params
            .get("message")
            .and_then(campfire_kit::Param::as_hash)
            .unwrap_or(&c.params);
        messages::attachment_assignment(
            &params.permit(&campfire_kit::permit_keys(&["attachment"])),
        )?
    } else {
        None
    };
    // Rails checks replay/budget/Drive before assigning an attachment. Repeat the same
    // permission and service checks in the writer after staging, so a revoked grant wins.
    if assignment.is_some() {
        let (attributes, drive) = (a.clone(), drive.clone());
        let early = c
            .app()
            .db
            .write(move |tx| {
                if let Some(result) = post_permission(tx, agent_id, attributes.room_id)? {
                    return Ok(Some(result));
                }
                match agent_posting::preflight_service(tx, agent_id, &attributes, &drive) {
                    Err(campfire_db::Error::RecordNotFound(_)) => Ok(Some(reply_not_found())),
                    result => result,
                }
            })
            .await
            .map_err(db_error)?;
        if let Some(result) = early {
            return present_post(c, result, 201, false).await;
        }
    }
    let attachment = match assignment {
        Some(assignment) => Some(assignment.stage(c.app()).await?),
        None => None,
    };
    if matches!(
        attachment,
        Some(crate::controllers::presenters::attachments::Assignment::Invalid)
    ) {
        return Err(campfire_kit::Error::internal(anyhow::anyhow!(
            "Could not find or build blob: expected attachable"
        )));
    }
    let (mut outcome, blob) = c
        .app()
        .db
        .write(move |tx| {
            if let Some(result) = post_permission(tx, agent_id, a.room_id)? {
                return Ok((result, None));
            }
            let mut blob = None;
            let result = if streaming {
                agent_streaming::start(tx, agent_id, a)
            } else {
                agent_posting::post_service_with_preparation(tx, agent_id, a, drive, |tx, a| {
                    if let Some(attachment) = attachment {
                        blob = messages::attachment_blob(tx, attachment)?;
                        a.attachment_blob_id = blob.as_ref().map(|blob| blob.id);
                        if let Some(blob) = &blob {
                            crate::controllers::presenters::attachments::enqueue_analysis(tx, blob);
                        }
                    }
                    Ok(())
                })
            };
            let result = match result {
                Err(campfire_db::Error::RecordNotFound(_)) => reply_not_found(),
                result => result?,
            };
            if matches!(result, agent_posting::PostResult::Denied(_)) {
                blob = None;
            }
            Ok((result, blob))
        })
        .await
        .map_err(db_error)?;
    if let Some(blob) = blob {
        messages::process_attachment(c.app(), blob).await?;
        if let agent_posting::PostResult::Posted(message) = &mut outcome {
            let id = message.id;
            **message = c
                .app()
                .db
                .read(move |conn| campfire_db::Message::find(conn, id))
                .await
                .map_err(db_error)?;
        }
    }
    present_post(c, outcome, 201, streaming).await
}
fn reply_not_found() -> agent_posting::PostResult {
    agent_posting::PostResult::Denied(ServiceResult::fail("Reply target not found", 404))
}
fn post_permission(
    tx: &campfire_db::Tx<'_>,
    agent_id: i64,
    room_id: i64,
) -> campfire_db::Result<Option<agent_posting::PostResult>> {
    let agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let denial = if Room::find_for_user(tx.conn(), agent.user_id, room_id)?.is_none() {
        Some(ServiceResult::fail("Room not found", 404))
    } else if !agent_access::capability_for_agent(
        tx.conn(),
        agent_id,
        "post_messages",
        Some(room_id),
    )? {
        Some(ServiceResult::fail(
            "Forbidden: agent lacks post_messages capability",
            403,
        ))
    } else {
        None
    };
    Ok(denial.map(agent_posting::PostResult::Denied))
}
async fn present_post(
    c: &Ctx,
    outcome: agent_posting::PostResult,
    status: u16,
    streaming: bool,
) -> Result<ServiceResult> {
    match outcome {
        agent_posting::PostResult::Denied(result) => Ok(result),
        agent_posting::PostResult::Posted(message) => {
            messages::present(c, move |p| {
                let mut payload = p.agent_message_payload(&message)?;
                payload["thread_id"] = json!(message.thread_id);
                if streaming {
                    payload["streaming"] = json!(message.streaming);
                }
                Ok(ServiceResult::ok(payload, status))
            })
            .await
        }
    }
}

pub async fn dm(c: &Ctx, agent_id: i64, args: Value, rest: bool) -> Result<ServiceResult> {
    let a = canonical(c, &args, rest, true).await?;
    let drive = drive(&args, rest);
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

pub async fn stream(
    c: &Ctx,
    agent_id: i64,
    operation: &str,
    args: Value,
    rest: bool,
) -> Result<ServiceResult> {
    if operation == "start_stream" {
        return start(c, agent_id, args, true, rest).await;
    }
    let id = present_id(&args, "message_id").unwrap_or(0);
    let append = text(args.get("append"));
    let markdown = text(args.get("markdown_source"));
    let finalize = operation == "finalize_stream";
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            if finalize {
                agent_streaming::finalize(tx, agent_id, id)
            } else {
                agent_streaming::update(tx, agent_id, id, append.as_deref(), markdown.as_deref())
            }
        })
        .await
        .map_err(db_error)?;
    present_post(c, outcome, 200, true).await
}

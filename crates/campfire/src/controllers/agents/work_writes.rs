//! Agents::{Posts,Work}Controller and MCP use the merged WS12 agent work services.
//! Only adapter coercion, board authorization and payload rendering live here.
use super::{attribute_string, mcp::blank, reads::lookup_ids, ruby_i64};
use crate::{app::AppCtx, concerns, controllers::presenters::page::db_error};
use campfire_db::models::{
    agent_access, agent_reading, agent_payloads, agent_service::ServiceResult, agent_work, audit_log,
};
use campfire_db::{Agent, AgentWorkChanges, ChannelThread, HandoffPackage, WorkHandoff};
use campfire_kit::{Ctx, Result};
use campfire_richtext::ruby::json_value_to_s;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

enum Written {
    Denied(ServiceResult),
    Saved {
        id: i64,
        status: u16,
        handoff: Option<WorkHandoff>,
    },
}
fn saved(outcome: agent_work::Outcome<ChannelThread>) -> Written {
    match outcome {
        agent_work::Outcome::Success { payload, status } => Written::Saved {
            id: payload.id,
            status,
            handoff: None,
        },
        agent_work::Outcome::Denied(denial) => Written::Denied(denial),
    }
}

pub(super) async fn operation(
    c: &Ctx,
    agent_id: i64,
    operation: &str,
    args: Value,
    rest: bool,
) -> Result<ServiceResult> {
    let operation = operation.to_owned();
    let audit = audit_log::Context {
        actor: concerns::current_user(c).map(Into::into),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_owned),
    };
    let written=c.app().db.write(move |tx| {
        let agent=Agent::find(tx.conn(),agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
        if operation=="create_board_post" {
            // Active Record's association find_by accepts IN arrays. Resolve one
            // current member room; the model service retains its budget/validation order.
            let ids=lookup_ids(&args["room_id"]);
            let Some(room)=agent_reading::member_room_by_ids(tx.conn(),agent.user_id,&ids)? else {return Ok(Written::Denied(ServiceResult::fail("Room not found",404)));};
            let room_id=room.id;
            let facts=agent_access::capabilities_for_user_in_room(tx.conn(),agent.user_id,room_id,&["post_messages","manage_threads"])?;
            for capability in ["post_messages","manage_threads"] {
                if facts[capability]!=Some(true) {
                    return Ok(Written::Denied(ServiceResult::fail(format!("Forbidden: agent lacks {capability} capability"),403)));
                }
            }
            if !room.board() {return Ok(Written::Denied(ServiceResult::fail("Room is not a board",422)));}
            let input=agent_work::BoardPostInput {
                title:attribute_string(args.get("title")),
                // ChannelThread passes first_message to Message's string column;
                // Rails casts false to "f", rather than calling false.to_s.
                body:attribute_string(args.get("body")),
                tags:args.get("tags").cloned(),
                work_status:attribute_string(args.get("work_status").filter(|v|!blank(v))),
                run_url:attribute_string(args.get("run_url").filter(|v|!blank(v))),
                owner_id:args.get("owner_id").cloned(),
            };
            return agent_work::create_board_post(tx,&agent,&room,input).map(saved);
        }
        // Scalar ids go straight to the service. Arrays select the first row in
        // Rails' owned-work association; fresh membership/grant checks stay in WS12.
        let raw=&args["work_id"];
        let id=if raw.is_array() {
            tx.conn().query_row("SELECT id FROM channel_threads WHERE work_status IS NOT NULL AND work_owner_id=? AND id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1",params![agent.user_id,json!(lookup_ids(raw)).to_string()],|row|row.get::<_,i64>(0)).optional()?.unwrap_or(0)
        } else {lookup_ids(raw).first().copied().unwrap_or(0)};
        match operation.as_str() {
            "update_work"|"update_board_post"=>agent_work::update_work(tx,&agent,id,AgentWorkChanges {
                work_status:args.get("work_status").cloned(),note:args.get("note").cloned(),
                tags:args.get("tags").cloned(),run_url:args.get("run_url").cloned(),
            }).map(saved),
            "set_result"=>agent_work::set_result(tx,&agent,id,args.get("markdown")).map(saved),
            "handoff_work"=>{
                let raw=&args["receiver_agent_id"];
                let receiver=if raw.is_array() {
                    tx.conn().query_row("SELECT id FROM agents WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1",[json!(lookup_ids(raw)).to_string()],|row|row.get::<_,i64>(0)).optional()?.unwrap_or(0)
                }else {lookup_ids(raw).first().copied().unwrap_or_else(||ruby_i64(raw))};
                let package=HandoffPackage {
                    summary:if rest {json_value_to_s(&args["summary"])} else {attribute_string(args.get("summary")).unwrap_or_default()},
                    links:args["links"].clone(),open_questions:args["open_questions"].clone(),
                };
                Ok(match agent_work::handoff_work(tx,&agent,id,receiver,package,&audit)? {
                    agent_work::Outcome::Success{payload,status}=>Written::Saved{id:payload.thread.id,status,handoff:Some(payload.handoff)},
                    agent_work::Outcome::Denied(denial)=>Written::Denied(denial),
                })
            }
            _=>Err(campfire_db::Error::Other(format!("Unknown work operation: {operation}"))),
        }
    }).await.map_err(db_error)?;
    let Written::Saved {
        id,
        status,
        handoff,
    } = written
    else {
        let Written::Denied(denial) = written else {
            unreachable!()
        };
        return Ok(denial);
    };
    let access = c
        .app()
        .agent_repositories
        .resolve_work(&c.app().db, agent_id, vec![id])
        .await
        .map_err(db_error)?;
    let payload = c
        .app()
        .db
        .read(move |conn| {
            // Tag assignment has its own after_commit write. Render the committed row.
            let thread = ChannelThread::find(conn, id)?;
            // The live batch already names the owner of each allowance and
            // work_payloads revalidates its account snapshot during rendering.
            // With no allowance there is no owner-specific data to expose.
            let owner = access.iter().next().map(|(owner, _, _)| *owner);
            let mut payload =
                agent_payloads::work_payloads(conn, &[thread], owner, &access)?.remove(0);
            if let Some(handoff) = handoff {
                payload["handoff"] = handoff.payload(conn)?;
            }
            Ok(payload)
        })
        .await
        .map_err(db_error)?;
    Ok(ServiceResult::ok(payload, status))
}

//! REST/MCP readers over the shared models and owner-specific WorkPayload adapter.
use super::{mcp::blank, ruby_i64, text};
use crate::{
    app::AppCtx,
    controllers::{messages, presenters::page::db_error},
};
use campfire_db::models::{agent_access, agent_payloads, agent_reading, agent_service::ServiceResult};
use campfire_db::{Agent, AgentGrant, ChannelThread, Message, Room};
use campfire_kit::{Ctx, Result};
use serde_json::{Value, json};

pub async fn operation(c: &Ctx, agent_id: i64, op: &str, args: Value) -> Result<ServiceResult> {
    match op {
        "list_rooms" => {
            let payload = c.app().db.read(move |conn| {
                let agent = Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
                let mut rooms = Room::for_user(conn, agent.user_id)?;
                // SQLite LOWER(name) lowercases ASCII only, and sorts NULL before text.
                rooms.sort_by_key(|room| room.name.as_ref().map(|name| name.to_ascii_lowercase()));
                if !agent.legacy_capabilities(conn)? {
                    let grants = AgentGrant::all_active(conn)?.into_iter().filter(|grant|grant.agent_id==agent_id && grant.capability!="dm_anyone").map(|grant|grant.room_id).collect::<Vec<_>>();
                    let active = agent.active(conn)?;
                    rooms.retain(|room|active && (grants.contains(&None) || grants.contains(&Some(room.id))));
                }
                Ok(json!(rooms.into_iter().map(|room|json!({"id":room.id,"name":room.name,"type":room.room_type.class_name().strip_prefix("Rooms::").unwrap().to_ascii_lowercase(),"board":room.board(),"direct":room.direct()})).collect::<Vec<_>>()))
            }).await.map_err(db_error)?;
            Ok(ServiceResult::ok(payload, 200))
        }
        "read_messages" => history(c, agent_id, args).await,
        _ => work(c, agent_id, op, args).await,
    }
}
// Active Record find_by(id:) flattens IN lists. Other scalar shapes cannot match
// an integer key; unlike Ruby to_i they are not page-limit conversion errors.
pub(super) fn lookup_ids(value: &Value) -> Vec<i64> {
    fn collect(value: &Value, ids: &mut Vec<i64>) {
        match value {
            Value::Array(values) => {
                for value in values {
                    collect(value, ids);
                }
            }
            Value::Number(_) | Value::String(_) => ids.push(ruby_i64(value)),
            Value::Bool(value) => ids.push(i64::from(*value)),
            _ => {}
        }
    }
    let mut ids = Vec::new();
    collect(value, &mut ids);
    ids.sort_unstable();
    ids.dedup();
    ids
}
async fn history(c: &Ctx, agent_id: i64, args: Value) -> Result<ServiceResult> {
    // Agents::Reading resolves the conversation and its current read grant before
    // limit conversion, then resolves each cursor inside that conversation.
    messages::present(c,move |p| {
        let room_given=args.get("room_id").is_some_and(|v| !blank(v));
        let thread_given=args.get("thread_id").is_some_and(|v| !blank(v));
        if room_given && thread_given {return Ok(ServiceResult::fail("Pass only one of room_id, thread_id",422));}
        if !room_given && !thread_given {return Ok(ServiceResult::fail("room_id or thread_id is required",422));}
        let agent=Agent::find(p.conn,agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
        let (room,thread) = if room_given {
            let ids=lookup_ids(&args["room_id"]);
            let room=Room::for_user(p.conn,agent.user_id)?.into_iter().filter(|room|ids.contains(&room.id)).min_by_key(|room|room.id);
            let Some(room)=room else {return Ok(ServiceResult::fail("Room not found",404));};
            (room,None)
        }else{
            let mut thread=None;
            for id in lookup_ids(&args["thread_id"]) {
                if let Some(found)=ChannelThread::find_by_id(p.conn,id)? {thread=Some(found);break;}
            }
            let Some(thread)=thread else {return Ok(ServiceResult::fail("Thread not found",404));};
            let Some(room)=Room::find_for_user(p.conn,agent.user_id,thread.room_id)? else {return Ok(ServiceResult::fail("Thread not found",404));};
            (room,Some(thread.id))
        };
        if !agent_access::capability_for_agent(p.conn,agent_id,"read_messages",Some(room.id))? {
            return Ok(ServiceResult::fail("Forbidden: agent lacks read_messages capability",403));
        }
        let limit=args.get("limit").filter(|value|!blank(value));
        if limit.is_some_and(|value|!value.is_string() && !value.is_number()) {
            return Err(campfire_db::Error::Other("Reading limit does not support to_i".into()));
        }
        let limit=limit.map_or(50,ruby_i64).clamp(1,100);
        let (scope,conversation)=match thread {Some(thread)=>("thread_id=?",thread),None=>("room_id=? AND thread_id IS NULL",room.id)};
        let mut anchors=[None,None];
        for (i,key) in ["before","after"].into_iter().enumerate() {
            if let Some(value)=args.get(key).filter(|value|!blank(value)) {
                for id in lookup_ids(value) {
                    if Message::find_by_id(p.conn,id)?.is_some_and(|m|m.room_id==room.id && m.thread_id==thread) {anchors[i]=Some(id);break;}
                }
                if anchors[i].is_none() {return Ok(ServiceResult::fail("Message not found",404));}
            }
        }
        let [before,after]=anchors;
        let records=agent_reading::message_window(p.conn,room.id,thread,before,after,false,limit)?;
        let ids=records.iter().map(|m|m.id).collect::<Vec<_>>();
        let exists=|predicate:&str,cursor:Option<i64>|->campfire_db::Result<bool> {
            match cursor {Some(cursor)=>Ok(p.conn.query_row(&format!("SELECT EXISTS(SELECT 1 FROM messages WHERE {scope} AND id{predicate}?)"),[conversation,cursor],|row|row.get(0))?),None=>Ok(false)}
        };
        let first=ids.first().copied();let last=ids.last().copied();
        let before=exists("<",first)?;let after=exists(">",last)?;
        let payloads=p.agent_message_payloads(&records)?;
        Ok(ServiceResult::ok(json!({"messages":payloads,"before":first,"after":last,"has_more_before":before,"has_more_after":after}),200))
    }).await
}
async fn work(c: &Ctx, agent_id: i64, op: &str, args: Value) -> Result<ServiceResult> {
    let op = op.to_owned();
    let single = op == "get_work";
    let records = c
        .app()
        .db
        .read(move |conn| {
            let agent =
                Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            let threads = if single {
                let ids=args.get("work_id").map(lookup_ids).unwrap_or_default();
                agent_reading::thread_by_ids(conn,&ids)?.into_iter().collect()
            } else if op == "list_board_posts" {
                let status=text(args.get("status")).unwrap_or_default();
                let status=campfire_richtext::ruby::strip(&status);
                let status=if status.is_empty(){"open"}else{status};
                let owner=text(args.get("owner")).unwrap_or_default();
                let owner=campfire_richtext::ruby::strip(&owner);
                let tag=text(args.get("tag")).unwrap_or_default();
                let tag=campfire_richtext::ruby::strip(&tag).to_lowercase();
                agent_reading::board_posts(conn,args.get("room_id").map_or(0,ruby_i64),agent.user_id,status,owner,&tag)?
            } else {
                let rooms=Room::for_user(conn,agent.user_id)?.into_iter().map(|r|r.id).collect::<Vec<_>>();
                let allowed=agent_access::capabilities_for_users_in_rooms(conn,&[agent.user_id],&rooms,"read_messages")?;
                let rooms=rooms.into_iter().filter(|room|allowed.contains(&(agent.user_id,*room))).collect::<Vec<_>>();
                agent_reading::owned_work(conn,agent.user_id,&rooms)?
            };
            Ok(threads)
        })
        .await
        .map_err(db_error)?;
    let ids = records.iter().map(|thread| thread.id).collect();
    let access = c
        .app()
        .agent_repositories
        .resolve_work(&c.app().db, agent_id, ids)
        .await
        .map_err(db_error)?;
    let payload = c
        .app()
        .db
        .read(move |conn| {
            let owner = Agent::find(conn, agent_id)?.and_then(|agent| agent.owner_id);
            let values = agent_payloads::work_payloads(conn,&records,owner,&access)?;
            Ok(if single {
                values.into_iter().next().unwrap_or(Value::Null)
            } else {
                values.into()
            })
        })
        .await
        .map_err(db_error)?;
    Ok(ServiceResult::ok(payload, 200))
}

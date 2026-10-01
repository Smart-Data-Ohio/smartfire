//! REST/MCP readers over the shared models and owner-specific WorkPayload adapter.
use super::{mcp::blank, ruby_i64, text};
use crate::{app::AppCtx, controllers::{messages, presenters::page::db_error}};
use campfire_db::{Agent, AgentGrant, ChannelThread, Message, Room};
use campfire_db::models::{agent_access, agent_payloads, agent_service::ServiceResult};
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
            Ok(ServiceResult::ok(payload,200))
        }
        "read_messages" => history(c, args).await,
        _ => work(c, agent_id, op, args).await,
    }
}
async fn history(c: &Ctx, args: Value) -> Result<ServiceResult> {
    let limit = args.get("limit").filter(|value|!blank(value));
    if limit.is_some_and(|value|!value.is_string() && !value.is_number()) {
        return Err(campfire_kit::Error::internal(anyhow::anyhow!("Reading limit does not support to_i")));
    }
    let limit = limit.map_or(50,ruby_i64).clamp(1,100);
    messages::present(c,move |p| {
        let thread = args.get("thread_id").filter(|value|!blank(value)).map(ruby_i64);
        let (scope,conversation) = match thread {
            Some(thread) => ("thread_id=?",thread),
            None => ("room_id=? AND thread_id IS NULL",args.get("room_id").map_or(0,ruby_i64)),
        };
        let before = args.get("before").filter(|value|!blank(value)).map(ruby_i64);
        let after = args.get("after").filter(|value|!blank(value)).map(ruby_i64);
        let mut statement=p.conn.prepare(&format!("SELECT id FROM messages WHERE {scope} AND (? IS NULL OR id<?) AND (? IS NULL OR id>?) ORDER BY id DESC LIMIT ?"))?;
        let mut ids=statement.query_map(rusqlite::params![conversation,before,before,after,after,limit],|row|row.get::<_,i64>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
        ids.reverse();
        let exists = |predicate:&str,cursor:Option<i64>| -> campfire_db::Result<bool> {
            match cursor {
                Some(cursor)=>Ok(p.conn.query_row(&format!("SELECT EXISTS(SELECT 1 FROM messages WHERE {scope} AND id{predicate}?)"),[conversation,cursor],|row|row.get(0))?),
                None=>Ok(false),
            }
        };
        let first=ids.first().copied(); let last=ids.last().copied();
        let before=exists("<",first)?;let after=exists(">",last)?;
        let payloads=ids.into_iter().map(|id|p.agent_message_payload(&Message::find(p.conn,id)?)).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(ServiceResult::ok(json!({"messages":payloads,"before":first,"after":last,"has_more_before":before,"has_more_after":after}),200))
    }).await
}
async fn work(c: &Ctx, agent_id: i64, op: &str, args: Value) -> Result<ServiceResult> {
    let op=op.to_owned();
    let single=op=="get_work";
    let records=c.app().db.read(move |conn| {
        let agent=Agent::find(conn,agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
        let mut threads=if single {
            ChannelThread::find_by_id(conn,args.get("work_id").map_or(0,ruby_i64))?.into_iter().collect::<Vec<_>>()
        } else if op=="list_board_posts" {
            let mut threads=ChannelThread::for_room(conn,args.get("room_id").map_or(0,ruby_i64))?;
            let status=text(args.get("status")).unwrap_or_default();let status=campfire_richtext::ruby::strip(&status);
            let status=if status.is_empty(){"open"}else{status};
            let owner=text(args.get("owner")).unwrap_or_default();let owner=campfire_richtext::ruby::strip(&owner);
            let tag=text(args.get("tag")).unwrap_or_default();let tag=campfire_richtext::ruby::strip(&tag).to_lowercase();
            let mut filtered=Vec::new();
            for thread in threads.drain(..) {
                let status_matches=match status {"all"=>true,"open"=>thread.work_status.as_deref().is_some_and(|status|status!="done"),status=>thread.work_status.as_deref()==Some(status)};
                let owner_matches=match owner {""=>true,"me"=>thread.work_owner_id==Some(agent.user_id),"agents"=>thread.work_owner_id.map(|id|Agent::for_user(conn,id)).transpose()?.flatten().is_some(),owner=>thread.work_owner_id==Some(super::ruby_i64(&json!(owner)))};
                if status_matches && owner_matches && (tag.is_empty() || thread.tag_names(conn)?.contains(&tag)){filtered.push(thread);}
            }
            filtered
        } else {
            let rooms=Room::for_user(conn,agent.user_id)?.into_iter().map(|room|room.id).collect::<Vec<_>>();
            let mut threads=ChannelThread::for_rooms(conn,&rooms)?;
            let mut filtered=Vec::new();
            for thread in threads.drain(..) {
                if thread.work() && thread.work_owner_id==Some(agent.user_id) && agent_access::capability_for_agent(conn,agent_id,"read_messages",Some(thread.room_id))?{filtered.push(thread);}
            }
            filtered.sort_by(|a,b|b.updated_at.cmp(&a.updated_at).then_with(||b.id.cmp(&a.id)));
            filtered
        };
        threads.truncate(100);
        Ok(threads)
    }).await.map_err(db_error)?;
    let ids=records.iter().map(|thread|thread.id).collect();
    let access=c.app().agent_repositories.resolve_work(&c.app().db,agent_id,ids).await.map_err(db_error)?;
    let payload=c.app().db.read(move |conn| {
        let owner=Agent::find(conn,agent_id)?.and_then(|agent|agent.owner_id);
        let values=records.into_iter().map(|thread|agent_payloads::work_payload(conn,&thread,owner,&access)).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(if single {values.into_iter().next().unwrap_or(Value::Null)}else{values.into()})
    }).await.map_err(db_error)?;
    Ok(ServiceResult::ok(payload,200))
}

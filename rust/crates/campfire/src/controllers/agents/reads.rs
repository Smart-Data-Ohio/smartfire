//! REST/MCP readers over the shared models and owner-specific WorkPayload adapter.
use super::{mcp::blank, ruby_i64, text};
use crate::{
    app::AppCtx,
    controllers::{messages, presenters::page::db_error},
};
use campfire_db::models::{
    agent_access, agent_payloads, agent_reading, agent_service::ServiceResult,
};
use campfire_db::{Agent, AgentGrant, Room};
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
// Active Record's ArrayHandler casts each top-level candidate separately.
// After compacting nils, a singleton predicate delegates back to ArrayHandler;
// nested values in a multi-candidate IN predicate serialize independently.
pub(super) fn lookup_ids(value: &Value) -> Vec<i64> {
    fn cast(value: &Value) -> Option<i64> {
        match value {
            Value::Number(n) => n.as_i64().or_else(|| {
                if n.is_u64() {
                    return None;
                }
                n.as_f64()
                    .filter(|n| *n >= i64::MIN as f64 && *n < i64::MAX as f64)
                    .map(|n| n as i64)
            }),
            Value::String(s) => {
                // Ruby's decimal to_i also accepts vertical tab and 0d/0D.
                // Active Model rejects nonnumeric strings and out-of-range IDs.
                let s = s.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
                let bytes = s.as_bytes();
                let mut offset = usize::from(s.starts_with(['+', '-']));
                let mut decimal = s[..offset].to_owned();
                let numeric = bytes.get(offset).is_some_and(u8::is_ascii_digit);
                if s[offset..].starts_with("0d") || s[offset..].starts_with("0D") {
                    offset += 2;
                }
                let mut digits = 0;
                while let Some(byte) = bytes.get(offset) {
                    if byte.is_ascii_digit() {
                        decimal.push(char::from(*byte));
                        digits += 1;
                        offset += 1;
                    } else if *byte == b'_'
                        && digits > 0
                        && bytes.get(offset + 1).is_some_and(u8::is_ascii_digit)
                    {
                        offset += 1;
                    } else {
                        break;
                    }
                }
                if digits > 0 {
                    decimal.parse().ok()
                } else {
                    // A numeric prefix with no decimal digits still casts to 0
                    // (e.g. 0d or 0d_1), rather than a nonnumeric NULL predicate.
                    numeric.then_some(0)
                }
            }
            Value::Bool(value) => Some(i64::from(*value)),
            _ => None,
        }
    }
    let mut value = value;
    while let Value::Array(values) = value {
        let mut present = values.iter().filter(|v| !v.is_null());
        if let Some(first) = present.next()
            && present.next().is_none()
        {
            value = first;
        } else {
            break;
        }
    }
    let mut ids = match value {
        Value::Array(values) => values.iter().filter_map(cast).collect(),
        other => cast(other).into_iter().collect::<Vec<_>>(),
    };
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
            let thread=agent_reading::thread_by_ids(p.conn,&lookup_ids(&args["thread_id"]))?;
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
                anchors[i]=agent_reading::conversation_anchor(p.conn,room.id,thread,&lookup_ids(value))?;
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
                let ids = args.get("work_id").map(lookup_ids).unwrap_or_default();
                agent_reading::thread_by_ids(conn, &ids)?
                    .into_iter()
                    .collect()
            } else if op == "list_board_posts" {
                let status = text(args.get("status")).unwrap_or_default();
                let status = campfire_richtext::ruby::strip(&status);
                let status = if status.is_empty() { "open" } else { status };
                let owner = text(args.get("owner")).unwrap_or_default();
                let owner = campfire_richtext::ruby::strip(&owner);
                let tag = text(args.get("tag")).unwrap_or_default();
                let tag = campfire_richtext::ruby::strip(&tag).to_lowercase();
                agent_reading::board_posts(
                    conn,
                    args.get("room_id").map_or(0, ruby_i64),
                    agent.user_id,
                    status,
                    owner,
                    &tag,
                )?
            } else {
                let rooms = Room::for_user(conn, agent.user_id)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<_>>();
                let allowed = agent_access::capabilities_for_users_in_rooms(
                    conn,
                    &[agent.user_id],
                    &rooms,
                    "read_messages",
                )?;
                let rooms = rooms
                    .into_iter()
                    .filter(|room| allowed.contains(&(agent.user_id, *room)))
                    .collect::<Vec<_>>();
                agent_reading::owned_work(conn, agent.user_id, &rooms)?
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
            let values = agent_payloads::work_payloads(conn, &records, owner, &access)?;
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

#[cfg(test)]
mod id_cast_tests {
    use serde_json::{Value, json};

    #[test]
    fn ws11_next3_active_record_integer_candidates() {
        let vector: Value =
            serde_json::from_str(include_str!("../../../../../vectors/agent_id_casting.json"))
                .unwrap();
        for case in vector["cases"].as_array().unwrap() {
            assert_eq!(
                json!(super::lookup_ids(&case["input"])),
                case["candidates"],
                "{}",
                case["input"]
            );
        }
    }
    #[test]
    fn ws11_next4_numeric_boundaries_match_rails_predicates() {
        let vector: Value = serde_json::from_str(include_str!(
            "../../../../../vectors/next4_numeric_ids.json"
        )).unwrap();
        let allowed: Vec<i64> = vector["allowed"].as_array().unwrap().iter()
            .map(|v| v.as_i64().unwrap()).collect();
        let mut mismatches = Vec::new();
        for case in vector["cases"].as_array().unwrap() {
            // Deserialize raw input, rather than reserializing a rounded Value.
            let input: Value = serde_json::from_str(case["input_json"].as_str().unwrap()).unwrap();
            let selected: Vec<i64> = super::lookup_ids(&input).into_iter()
                .filter(|id| allowed.contains(id)).collect();
            if json!(selected) != case["selected"] {
                mismatches.push(format!("input={} actual={:?} expected={}",
                    case["input_json"], selected, case["selected"]));
            }
        }
        println!("WS11 next4 numeric IDs: {} cases; {} mismatches",
            vector["cases"].as_array().unwrap().len(), mismatches.len());
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }
    #[test]
    fn pr214_scalar_array_corpus_matches_rails_predicates() {
        let vector: Value =
            serde_json::from_str(include_str!("../../../../../vectors/pr214_id_corpus.json"))
                .unwrap();
        let allowed = vector["allowed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect::<Vec<_>>();
        let mut mismatches = Vec::new();
        for case in vector["cases"].as_array().unwrap() {
            let selected = super::lookup_ids(&case["input"])
                .into_iter()
                .filter(|id| allowed.contains(id))
                .collect::<Vec<_>>();
            if json!(selected) != case["selected"] {
                mismatches.push(format!(
                    "form={} position={} input={} actual={:?} expected={}",
                    case["form"], case["position"], case["input"], selected, case["selected"]
                ));
            }
        }
        println!(
            "PR214 ID corpus: {} scalar forms; {} positions; {} cases; {} mismatches",
            vector["scalar_forms"],
            vector["positions"],
            vector["cases"].as_array().unwrap().len(),
            mismatches.len()
        );
        assert!(
            mismatches.is_empty(),
            "{}",
            mismatches
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

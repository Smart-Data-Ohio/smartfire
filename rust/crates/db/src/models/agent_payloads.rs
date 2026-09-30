//! Agent::Delivery payloads and Agents::WorkPayload. These readers never perform HTTP.
//! Private repository decisions must be resolved by the GitHub domain before taking a DB
//! transaction; until WS15g provides that caller seam, production carries the public fields.
use super::agent_delivery::{AgentEvent, MESSAGE_TYPES, WORK_TYPES};
use crate::rich_text::RichText;
use crate::sql::{query_all, query_one};
use crate::{ChannelThread, Connection, Message, Result, Room, Timestamp, User, Webhook};
use serde_json::{Value, json};
use std::collections::HashSet;

/// Owner-specific decisions supplied by GithubConnectedAccount#can_read_repository?.
pub type RepositoryAccess = HashSet<(i64, String, String)>;

pub enum Payload {
    Ready {
        body: String,
        sync_message: Option<Box<Message>>,
    },
    Unavailable(String),
}

pub fn build(
    conn: &Connection,
    rich_text: &dyn RichText,
    event: &AgentEvent,
    access: &RepositoryAccess,
) -> Result<Payload> {
    let (user_id,owner_id,name,owner):(i64,Option<i64>,String,Option<String>)=conn.query_row(
        "SELECT a.user_id,a.owner_id,u.name,o.name FROM agents a JOIN users u ON u.id=a.user_id LEFT JOIN users o ON o.id=a.owner_id WHERE a.id=?",
        [event.agent_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    let agent = json!({"id":event.agent_id,"name":name,"owner":owner,"delivery_id":event.id});
    let metadata = &event.metadata;
    let data = if MESSAGE_TYPES.contains(&event.event_type.as_str()) {
        let Some(message) = event
            .message_id
            .map(|id| Message::find_by_id(conn, id))
            .transpose()?
            .flatten()
        else {
            return Ok(Payload::Unavailable("Message no longer available".into()));
        };
        let Some(webhook) = Webhook::find_by_user(conn, user_id)? else {
            return Ok(Payload::Unavailable("Webhook no longer available".into()));
        };
        let pr = pull_request_for_message(conn, &message, owner_id, access)?;
        let body =
            webhook.payload_for_agent(conn, rich_text, &message, event.agent_id, event.id, &pr)?;
        return Ok(Payload::Ready {
            body,
            sync_message: Some(Box::new(message)),
        });
    } else if event.event_type == "approval_decided" {
        let mut approval = event
            .agent_approval_id
            .map(|id| approval_payload(conn, id))
            .transpose()?
            .flatten();
        if approval.is_none()
            && let Some(id) = metadata
                .get("approval_id")
                .map(super::agent_delivery::ruby_i64)
        {
            approval = approval_payload(conn, id)?;
        }
        let Some(approval) = approval else {
            return Ok(Payload::Unavailable("Approval no longer available".into()));
        };
        json!({"agent":agent,"approval":approval})
    } else if event.event_type == "slash_command" {
        let actor = event
            .actor_id
            .map(|id| {
                query_one(conn, "SELECT id,name FROM users WHERE id=?", [id], |r| {
                    Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?}))
                })
            })
            .transpose()?
            .flatten();
        let room = event
            .room_id
            .map(|id| {
                query_one(conn, "SELECT id,name FROM rooms WHERE id=?", [id], |r| {
                    Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,Option<String>>(1)?}))
                })
            })
            .transpose()?
            .flatten();
        compact(
            json!({"agent":agent,"event_type":event.event_type,"user":actor,"room":room,"thread_id":metadata.get("thread_id"),"command":{"name":metadata.get("command"),"arguments":metadata.get("arguments")}}),
        )
    } else if ["github_action_completed", "fizzy_action_completed"]
        .contains(&event.event_type.as_str())
    {
        let mut data = json!({"agent":agent});
        data[if event.event_type == "github_action_completed" {
            "github_action"
        } else {
            "fizzy_action"
        }] = compact(
            json!({"approval_id":metadata.get("approval_id"),"action":metadata.get("action"),"status":metadata.get("status"),"url":metadata.get("url"),"message":metadata.get("message")}),
        );
        data
    } else if WORK_TYPES.contains(&event.event_type.as_str()) {
        let thread = metadata
            .get("thread_id")
            .map(super::agent_delivery::ruby_i64)
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten();
        let work = if let Some(thread) = thread {
            let mut work = work_payload(conn, &thread, owner_id, access)?;
            work["thread_id"] = json!(thread.id);
            work["status"] = json!(thread.work_status);
            work["assigned_by"] = metadata.get("assigned_by").cloned().unwrap_or(Value::Null);
            Some(work)
        } else {
            metadata
                .get("work_snapshot")
                .filter(|v| !v.is_null() && **v != Value::Bool(false))
                .cloned()
        };
        let Some(work) = work else {
            return Ok(Payload::Unavailable("Thread no longer available".into()));
        };
        compact(
            json!({"agent":agent,"event_type":event.event_type,"work":work,"handoff":if event.event_type=="work_handed_off" {metadata.get("handoff")} else {None}}),
        )
    } else {
        return Ok(Payload::Unavailable(format!(
            "Event type {} has no webhook payload",
            event.event_type
        )));
    };
    Ok(Payload::Ready {
        body: rails_json(&data),
        sync_message: None,
    })
}

fn approval_payload(conn: &Connection, id: i64) -> Result<Option<Value>> {
    query_one(
        conn,
        "SELECT a.id,a.status,u.name,a.decision_note FROM agent_approvals a LEFT JOIN users u ON u.id=a.decided_by_id WHERE a.id=?",
        [id],
        |r| {
            Ok(
                json!({"approval_id":r.get::<_,i64>(0)?,"status":r.get::<_,String>(1)?,"decided_by":r.get::<_,Option<String>>(2)?,"note":r.get::<_,Option<String>>(3)?}),
            )
        },
    )
}
fn compact(mut value: Value) -> Value {
    if let Some(map) = value.as_object_mut() {
        map.retain(|_, v| !v.is_null());
    }
    value
}
pub fn rails_json(value: &Value) -> String {
    value
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}
fn json_time(time: Timestamp) -> String {
    format!(
        "{}.{:03}Z",
        time.jiff().strftime("%Y-%m-%dT%H:%M:%S"),
        time.subsec_microsecond() / 1000
    )
}

pub fn work_payload(
    conn: &Connection,
    thread: &ChannelThread,
    owner_id: Option<i64>,
    access: &RepositoryAccess,
) -> Result<Value> {
    let room = Room::find(conn, thread.room_id)?;
    let owner = thread
        .work_owner_id
        .map(|id| User::find_by_id(conn, id))
        .transpose()?
        .flatten();
    Ok(
        json!({"id":thread.id,"room_id":thread.room_id,"board_id":room.board().then_some(room.id),"board_name":if room.board() {room.name.as_deref()} else {None},"title":thread.name,"work_status":thread.work_status,"owner":owner.map(|u|json!({"id":u.id,"name":u.name,"agent":u.is_bot()})),"tags":thread.tag_names(conn)?,"result":thread.result_markdown,"result_updated_at":thread.result_updated_at.map(json_time),"run_url":thread.run_url,"url":format!("/rooms/{}?thread={}",thread.room_id,thread.id),"updated_at":json_time(thread.updated_at),"links":work_links(conn,thread,owner_id,access)?}),
    )
}
pub fn pull_request_for_message(
    conn: &Connection,
    message: &Message,
    owner_id: Option<i64>,
    access: &RepositoryAccess,
) -> Result<Value> {
    let pr=message.thread_id.map(|id|query_one(conn,"SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? LIMIT 1",[id],|r|r.get::<_,i64>(0))).transpose()?.flatten();
    Ok(pr
        .map(|id| pull_request_payload(conn, id, owner_id, access))
        .transpose()?
        .flatten()
        .unwrap_or(Value::Null))
}
fn pull_request_payload(
    conn: &Connection,
    id: i64,
    owner_id: Option<i64>,
    access: &RepositoryAccess,
) -> Result<Option<Value>> {
    query_one(
        conn,
        "SELECT * FROM github_pull_requests WHERE id=?",
        [id],
        |r| {
            let owner: String = r.get("owner")?;
            let repo: String = r.get("repo")?;
            let number: i64 = r.get("number")?;
            let public = r.get::<_, Option<bool>>("private")? == Some(false);
            let visible = public
                || owner_id.is_some_and(|id| {
                    access.contains(&(id, owner.to_lowercase(), repo.to_lowercase()))
                });
            let url = r
                .get::<_, Option<String>>("html_url")?
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}/pull/{number}"));
            Ok(
                json!({"url":url,"owner":owner,"repo":repo,"number":number,"title":if visible {r.get::<_,Option<String>>("title")?} else {None},"state":r.get::<_,Option<String>>("state")?,"head_branch":if visible {r.get::<_,Option<String>>("head_branch")?} else {None},"base_branch":if visible {r.get::<_,Option<String>>("base_branch")?} else {None},"review_decision":r.get::<_,Option<String>>("review_decision")?,"checks_state":r.get::<_,Option<String>>("check_status")?}),
            )
        },
    )
}
fn work_links(
    conn: &Connection,
    thread: &ChannelThread,
    owner_id: Option<i64>,
    access: &RepositoryAccess,
) -> Result<Vec<Value>> {
    let rows = query_all(
        conn,
        "SELECT kind,url,title,github_pull_request_id,event_id FROM work_thread_links WHERE channel_thread_id=? ORDER BY id",
        [thread.id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        },
    )?;
    let mut links = vec![];
    for (kind, url, title, pr, event) in rows {
        match kind.as_str() {
            "drive_file"=>links.push(json!({"kind":kind,"url":url,"title":title,"pull_request":null,"event":null})),
            "pull_request"=>if let Some(pr)=pr.map(|id|pull_request_payload(conn,id,owner_id,access)).transpose()?.flatten() {links.push(json!({"kind":kind,"url":pr["url"],"title":pr["title"],"pull_request":pr,"event":null}));},
            "event"=>if let Some(event)=event.map(|id|query_one(conn,"SELECT id,title,starts_at,ends_at,cancelled_at FROM events WHERE id=?",[id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"starts_at":r.get::<_,Option<Timestamp>>(2)?.map(json_time),"ends_at":r.get::<_,Option<Timestamp>>(3)?.map(json_time),"cancelled":r.get::<_,Option<Timestamp>>(4)?.is_some(),"url":format!("/rooms/{}/events/{id}",thread.room_id)})))).transpose()?.flatten() {links.push(json!({"kind":kind,"url":event["url"],"title":event["title"],"pull_request":null,"event":event}));},
            _=>{},
        }
    }
    Ok(links)
}

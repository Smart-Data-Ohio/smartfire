//! Agents::Reactions transport adapter over the shared Boost and icon APIs.
use super::{ruby_i64, text};
use crate::app::AppCtx;
use crate::controllers::presenters::page::db_error;
use campfire_db::models::{agent_access, agent_service::ServiceResult};
use campfire_db::{Agent, Boost, Message, Room};
use campfire_kit::{Ctx, Result};
use serde_json::{Value, json};

// find_by(id: Array) binds a flattened IN list; it does not call Array#to_i.
fn lookup_ids(value: &Value, ids: &mut Vec<i64>) -> campfire_db::Result<()> {
    match value {
        Value::Array(values) => {
            for value in values {
                lookup_ids(value, ids)?;
            }
        }
        Value::Number(_) | Value::String(_) => ids.push(ruby_i64(value)),
        Value::Object(_) => {
            return Err(campfire_db::Error::Other(
                "can't cast Hash for message id".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(super) async fn operation(c: &Ctx, agent_id: i64, args: Value) -> Result<ServiceResult> {
    let result=c.app().db.write(move |tx| {
        let agent=Agent::find(tx.conn(),agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
        let mut ids=vec![];
        lookup_ids(&args["message_id"],&mut ids)?;
        ids.sort_unstable();
        let message=campfire_db::models::agent_reading::message_by_ids(tx.conn(),&ids)?;
        let Some(message)=message else {return Ok(ServiceResult::fail("Message not found",404));};
        if Room::find_for_user(tx.conn(),agent.user_id,message.room_id)?.is_none() {
            return Ok(ServiceResult::fail("Message not found",404));
        }
        if !agent_access::capability_for_agent(tx.conn(),agent_id,"react",Some(message.room_id))? {
            return Ok(ServiceResult::fail("Forbidden: agent lacks react capability",403));
        }
        let content=text(args.get("content")).unwrap_or_default();
        let content=tx.rich_text().resolve_boost_content(tx.conn(),&content).map_err(campfire_db::Error::Other)?;
        if campfire_richtext::ruby::is_blank(&content) {return Ok(ServiceResult::fail("Reaction content can't be blank",422));}
        if let Some(boost)=Boost::for_message(tx.conn(),message.id)?.into_iter().find(|boost|boost.booster_id==agent.user_id && boost.content==content) {
            return Ok(ServiceResult::ok(json!({"id":boost.id,"message_id":message.id,"content":boost.content,"created":false}),200));
        }
        let boost=Boost::create(tx,message.id,agent.user_id,&content)?;
        Ok(ServiceResult::ok(json!({"id":boost.id,"message_id":message.id,"content":boost.content,"created":true}),200))
    }).await.map_err(db_error)?;
    if let Some(payload) = result
        .payload
        .as_ref()
        .filter(|payload| payload["created"] == true)
    {
        let id = payload["message_id"]
            .as_i64()
            .expect("created reaction message id");
        let message = c
            .app()
            .db
            .read(move |conn| Message::find(conn, id))
            .await
            .map_err(db_error)?;
        crate::controllers::messages::boosts::broadcast_reactions(c, &message).await?;
    }
    Ok(result)
}

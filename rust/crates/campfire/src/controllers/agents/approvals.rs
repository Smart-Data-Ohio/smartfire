//! Agents::ApprovalsController. Parsing belongs here; lifecycle remains WS11.
use campfire_db::models::{agent_approvals, agent_service::ServiceResult};
use campfire_kit::{Ctx, Result};
use serde_json::{Value, json};
use super::{attribute_string, field, id, integer, render_result, text};
use crate::app::AppCtx;
use crate::concerns::{CurrentAgent, agent_api};
use crate::controllers::presenters::page::db_error;

async fn before(c: &mut Ctx, limit: u64, action: &str) -> Result<CurrentAgent> {
    let identity=agent_api::require_token(c,false).await?;
    agent_api::throttle(c,limit,"agents/approvals",action)?;
    agent_api::no_store(c);
    Ok(identity)
}
pub async fn index(c:&mut Ctx)->Result {
    let identity=before(c,120,"index").await?;
    let status=text(field(c,"status").as_ref());
    let result=c.app().db.write(move |tx|agent_approvals::list(tx,identity.agent_id,status.as_deref())).await.map_err(db_error)?;
    render_result(c,result)
}
pub async fn show(c:&mut Ctx)->Result {
    let identity=before(c,120,"show").await?;
    let id=id(c,"id");
    let result=c.app().db.write(move |tx|agent_approvals::show(tx,identity.agent_id,id)).await.map_err(db_error)?;
    render_result(c,result)
}
pub async fn destroy(c:&mut Ctx)->Result {
    let identity=before(c,60,"destroy").await?;
    let id=id(c,"id");
    let result=c.app().db.write(move |tx|agent_approvals::cancel(tx,identity.agent_id,id)).await.map_err(db_error)?;
    render_result(c,result)
}
pub async fn create(c:&mut Ctx)->Result {
    let identity=before(c,60,"create").await?;
    let body:Value=serde_json::from_slice(c.request.raw_post()).unwrap_or(Value::Null);
    let nested=field(c,"approval").filter(Value::is_object).unwrap_or_else(||json!({}));
    let mut fields=json!({});
    for key in ["summary","room_id","external_id","expires_at"] {
        fields[key]=nested.get(key).filter(|v|!super::mcp::blank(v)).cloned()
            .or_else(||field(c,key).filter(|v|!super::mcp::blank(v)))
            .or_else(||body.get(key).filter(|v|!super::mcp::blank(v)).cloned()).unwrap_or(Value::Null);
    }
    fields["action"]=nested.get("action").filter(|v|!super::mcp::blank(v)).cloned()
        .or_else(||body.get("action").filter(|v|!super::mcp::blank(v)).cloned())
        .or_else(||field(c,"approval_action").filter(|v|!super::mcp::blank(v))).unwrap_or(Value::Null);
    fields["payload"]=nested.get("payload").or_else(||body.get("payload")).cloned().or_else(||field(c,"payload")).unwrap_or(Value::Null);
    fields["expires_in"]=[nested.get("expires_in").cloned(),nested.get("expires_in_seconds").cloned(),field(c,"expires_in"),field(c,"expires_in_seconds"),body.get("expires_in").cloned(),body.get("expires_in_seconds").cloned()].into_iter().flatten().find(|v|!super::mcp::blank(v)).unwrap_or(Value::Null);
    let result=create_operation(c,identity,fields).await?;
    render_result(c,result)
}
pub async fn create_operation(c:&Ctx, identity:CurrentAgent, fields:Value)->Result<ServiceResult> {
    c.app().db.write(move |tx| {
        let expires_in=fields.get("expires_in").filter(|v|!super::mcp::blank(v));
        let expires_at=fields.get("expires_at").filter(|v|!super::mcp::blank(v));
        let mut input_error=None;
        let deadline=if let Some(value)=expires_in {
            let seconds=match value {
                Value::Number(n)=>n.as_i64().or_else(||n.as_f64().map(|f|f as i64)),
                Value::String(s)=>s.trim().parse::<i64>().ok(), _=>None,
            };
            seconds.and_then(|s|tx.now().jiff().checked_add(jiff::SignedDuration::from_secs(s)).ok()).map(campfire_db::Timestamp::from_jiff).or_else(||{input_error=Some("Invalid expires_in".into());None})
        } else if let Some(value)=expires_at {
            let string=text(Some(value)).unwrap_or_default();
            string.parse::<jiff::Timestamp>().ok().map(campfire_db::Timestamp::from_jiff).or_else(||campfire_db::Timestamp::parse_db(&string)).or_else(||{input_error=Some("Invalid expires_at".into());None})
        } else {None};
        let payload=fields.get("payload").filter(|v|!v.is_null()).map(|v|if v.is_object()||v.is_array(){v.to_string()}else{text(Some(v)).unwrap_or_default()});
        agent_approvals::create_with_input_error(tx,identity.agent_id,agent_approvals::ApprovalRequest {
            room_id:integer(fields.get("room_id").filter(|v|!super::mcp::blank(v))),
            action:attribute_string(fields.get("action")).unwrap_or_default(),
            summary:attribute_string(fields.get("summary")).unwrap_or_default(),
            payload,external_id:attribute_string(fields.get("external_id")),expires_at:deadline,
        },Some(identity.credential_id),input_error)
    }).await.map_err(db_error)
}

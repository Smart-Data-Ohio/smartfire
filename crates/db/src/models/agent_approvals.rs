//! Agents::Approvals. Transport adapters parse request fields into this typed input;
//! authorization, replay, budgets, settlement and payloads live here for REST and MCP.
use super::agent_access::{capability_for_agent, has_capability_anywhere};
use super::agent_approval::STATUSES;
use super::agent_posting::{Cap, check_budget};
use super::agent_service::ServiceResult;
use crate::sql::{exists, query_all};
use crate::{AgentApproval, NewApproval, Result, Timestamp, Tx};
use rusqlite::params;

#[derive(Debug, Default, Clone)]
pub struct ApprovalRequest {
    pub room_id: Option<i64>,
    pub action: String,
    pub summary: String,
    pub payload: Option<String>,
    pub external_id: Option<String>,
    pub expires_at: Option<Timestamp>,
}
fn forbidden() -> ServiceResult {
    ServiceResult::fail("Forbidden: agent lacks external_action capability", 403)
}
fn missing() -> ServiceResult {
    ServiceResult::fail("Approval not found", 404)
}
fn resolved_room(tx: &Tx<'_>, room_id: Option<i64>) -> Result<Option<i64>> {
    Ok(match room_id {
        Some(id) if exists(tx.conn(), "SELECT 1 FROM rooms WHERE id=?", [id])? => Some(id),
        _ => None,
    })
}
pub fn list(tx: &mut Tx<'_>, agent_id: i64, status: Option<&str>) -> Result<ServiceResult> {
    if !has_capability_anywhere(tx.conn(), agent_id, "external_action")? {
        return Ok(forbidden());
    };
    let status = status.filter(|s| STATUSES.contains(s));
    let mut approvals = query_all(
        tx.conn(),
        "SELECT * FROM agent_approvals WHERE agent_id=?1 AND
        (?3 IS NULL OR (?3='pending' AND status='pending' AND expires_at>?2)
         OR (?3='expired' AND (status='expired' OR (status='pending' AND expires_at<=?2)))
         OR (?3 NOT IN ('pending','expired') AND status=?3)) ORDER BY id DESC LIMIT 100",
        params![agent_id, tx.now(), status],
        AgentApproval::from_row,
    )?;
    let mut payloads = vec![];
    for approval in &mut approvals {
        approval.expire_if_due(tx)?;
        payloads.push(approval.payload(tx.conn(), tx.now())?);
    }
    Ok(ServiceResult::ok(payloads.into(), 200))
}
pub fn show(tx: &mut Tx<'_>, agent_id: i64, id: i64) -> Result<ServiceResult> {
    let Some(mut approval) = AgentApproval::find(tx.conn(), id)?.filter(|a| a.agent_id == agent_id)
    else {
        return Ok(missing());
    };
    if !capability_for_agent(
        tx.conn(),
        agent_id,
        "external_action",
        resolved_room(tx, approval.room_id)?,
    )? {
        return Ok(forbidden());
    };
    approval.expire_if_due(tx)?;
    Ok(ServiceResult::ok(
        approval.payload(tx.conn(), tx.now())?,
        200,
    ))
}
pub fn create(
    tx: &mut Tx<'_>,
    agent_id: i64,
    request: ApprovalRequest,
    credential_id: Option<i64>,
) -> Result<ServiceResult> {
    create_with_input_error(tx, agent_id, request, credential_id, None)
}

/// Transport seam: resolve raw expiry errors after authorization, replay and budget.
pub fn create_with_input_error(
    tx: &mut Tx<'_>, agent_id: i64, request: ApprovalRequest,
    credential_id: Option<i64>, input_error: Option<String>,
) -> Result<ServiceResult> {
    if let Some(room) = request.room_id
        && !exists(
            tx.conn(),
            "SELECT 1 FROM rooms r JOIN memberships m ON m.room_id=r.id JOIN agents a ON a.user_id=m.user_id WHERE r.id=? AND a.id=? AND r.deleted_at IS NULL",
            params![room, agent_id],
        )?
    {
        return Ok(ServiceResult::fail("Room not found", 404));
    };
    if !capability_for_agent(tx.conn(), agent_id, "external_action", request.room_id)? {
        return Ok(forbidden());
    };
    if let Some(external) = &request.external_id
        && !campfire_richtext::ruby::is_blank(external)
        && let Some(mut existing) =
            AgentApproval::find_by_external_id(tx.conn(), agent_id, external)?
    {
        existing.expire_if_due(tx)?;
        return Ok(ServiceResult::ok(existing.created_payload(tx.now()), 200));
    }
    if request.action.starts_with("github.") {
        return Ok(ServiceResult::fail(
            "github.* actions are requested through /rooms/:room_id/agents/github/pull_request_actions",
            422,
        ));
    };
    if request.action.starts_with("fizzy.") {
        return Ok(ServiceResult::fail(
            "fizzy.* actions are requested through /agents/fizzy/card_actions",
            422,
        ));
    };
    if let Some(denial) = check_budget(tx, agent_id, Cap::ExternalActions)? {
        return Ok(ServiceResult::budget(denial));
    };
    if let Some(error) = input_error { return Ok(ServiceResult::fail(error, 422)); }
    let attributes = NewApproval {
        agent_id,
        room_id: request.room_id,
        agent_credential_id: credential_id,
        action: request.action,
        summary: request.summary,
        payload: request.payload,
        external_id: request.external_id,
        expires_at: request.expires_at,
        ..Default::default()
    };
    let errors = AgentApproval::validate(tx.conn(), &attributes, tx.now(), None)?;
    if !errors.is_empty() {
        return Ok(ServiceResult::fail(
            crate::slash_commands::sentence(errors.full_messages()),
            422,
        ));
    };
    let approval = AgentApproval::create(tx, attributes)?;
    Ok(ServiceResult::ok(approval.created_payload(tx.now()), 201))
}
pub fn cancel(tx: &mut Tx<'_>, agent_id: i64, id: i64) -> Result<ServiceResult> {
    let Some(mut approval) = AgentApproval::find(tx.conn(), id)?.filter(|a| a.agent_id == agent_id)
    else {
        return Ok(missing());
    };
    if !capability_for_agent(
        tx.conn(),
        agent_id,
        "external_action",
        resolved_room(tx, approval.room_id)?,
    )? {
        return Ok(forbidden());
    };
    let errors = approval.cancel_by_agent(tx)?;
    if !errors.is_empty() {
        return Ok(ServiceResult::fail(
            crate::slash_commands::sentence(errors.full_messages()),
            422,
        ));
    };
    Ok(ServiceResult::ok(
        approval.payload(tx.conn(), tx.now())?,
        200,
    ))
}

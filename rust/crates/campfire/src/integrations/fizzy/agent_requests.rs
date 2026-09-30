//! Agents::FizzyCardActions. WS11's authenticated REST/MCP callers share this writer API.
//! Constructing a request cannot call Fizzy. The executable payload and identity snapshot
//! are built here; WS11's human decision calls agent_job::enqueue_approved in its write.
use super::{accounts::Account, agent_action::Action, agent_reads::ReadResult};
use campfire_db::{
    ActivityItem, Errors, Event, Result, Timestamp, Tx, User, broadcasts::Broadcast,
};
use campfire_richtext::ruby::json_value_to_s;
use jiff::{SignedDuration, tz::TimeZone};
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

struct Approval {
    id: i64,
    status: String,
    expires: Timestamp,
}
fn iso(time: Timestamp) -> String {
    format!(
        "{}.{:03}Z",
        time.jiff().strftime("%Y-%m-%dT%H:%M:%S"),
        time.subsec_microsecond() / 1000
    )
}
fn payload(approval: &Approval) -> Value {
    json!({"id":approval.id,"status":approval.status,"expires_at":iso(approval.expires)})
}
fn sentence(messages: Vec<String>) -> String {
    match messages.len() {
        0 => String::new(),
        1 => messages[0].clone(),
        2 => messages.join(" and "),
        n => format!("{}, and {}", messages[..n - 1].join(", "), messages[n - 1]),
    }
}
fn preference(raw: Option<String>) -> bool {
    let raw: Value = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    !matches!(&raw["agent_approvals"], Value::Bool(false))
        && raw["agent_approvals"].as_f64() != Some(0.0)
        && raw["agent_approvals"] != json!("0")
        && raw["agent_approvals"] != json!("false")
}
fn fan_out(tx: &mut Tx<'_>, owner: i64, approval: i64) -> Result<()> {
    let recipients: Vec<(i64,Option<String>)> = tx.conn().prepare("SELECT id,inbox_preferences FROM users WHERE status=0 AND role!=2 AND (id=? OR role=1)")?
        .query_map([owner],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    for (recipient, prefs) in recipients {
        if preference(prefs) {
            ActivityItem::refresh_unread(
                tx,
                recipient,
                "AgentApproval",
                approval,
                "agent_approval_request",
            )?;
        }
    }
    Ok(())
}
fn expire(tx: &mut Tx<'_>, approval: &mut Approval) -> Result<()> {
    if approval.status == "pending" && approval.expires <= tx.now() {
        tx.conn().execute(
            "UPDATE agent_approvals SET status='expired',updated_at=? WHERE id=?",
            params![tx.now(), approval.id],
        )?;
        approval.status = "expired".into();
        let items:Vec<(i64,i64)>=tx.conn().prepare("UPDATE activity_items SET read_at=COALESCE(read_at,?),handled_at=?,updated_at=? WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL RETURNING id,user_id")?
            .query_map(params![tx.now(),tx.now(),tx.now(),approval.id],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        for (id, user) in items {
            if User::find_by_id(tx.conn(), user)?.is_some_and(|u| u.is_active() && !u.is_bot()) {
                tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
                    stream: format!("user_{user}_activity"),
                    payload: json!({"activityItemId":id}),
                }));
            }
        }
    }
    Ok(())
}
/// Date.current and end_of_day use the request's isolated Rails zone, including DST.
fn day_window(now: Timestamp, zone: &TimeZone) -> Result<(String, Timestamp, Timestamp, i64)> {
    let day = now.jiff().to_zoned(zone.clone()).date();
    let tomorrow = day
        .tomorrow()
        .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    let midnight = |date: jiff::civil::Date| {
        date.at(0, 0, 0, 0)
            .to_zoned(zone.clone())
            .map(|z| Timestamp::from_jiff(z.timestamp()))
            .map_err(|e| campfire_db::Error::Other(e.to_string()))
    };
    let start = midnight(day)?;
    let end = midnight(tomorrow)?;
    let retry = ((end.as_microsecond() - now.as_microsecond() - 1) / 1_000_000).max(1);
    Ok((day.to_string(), start, end, retry))
}
fn budget(
    tx: &mut Tx<'_>,
    agent: i64,
    owner: i64,
    limit: Option<i64>,
    zone: &TimeZone,
) -> Result<Option<ReadResult>> {
    let Some(limit) = limit else { return Ok(None) };
    let (day, start, end, retry) = day_window(tx.now(), zone)?;
    let count: i64 = tx.conn().query_row(
        "SELECT COUNT(*) FROM agent_approvals WHERE agent_id=? AND created_at>=? AND created_at<?",
        params![agent, start, end],
        |r| r.get(0),
    )?;
    if count < limit {
        return Ok(None);
    };
    let notice:Option<i64>=tx.conn().query_row("INSERT INTO agent_budget_notices (agent_id,cap,day,created_at,updated_at) VALUES (?,'external_actions',?,?,?) ON CONFLICT(agent_id,cap,day) DO NOTHING RETURNING id",params![agent,day,tx.now(),tx.now()],|r|r.get(0)).optional()?;
    if let Some(notice) = notice
        && User::find_by_id(tx.conn(), owner)?.is_some_and(|u| u.is_active() && !u.is_bot())
    {
        ActivityItem::refresh_unread(tx, owner, "AgentBudgetNotice", notice, "agent_budget_exceeded")?;
    }

    let message = format!("Daily external action budget exceeded ({limit}/day)");
    Ok(Some(ReadResult {
        status: 429,
        payload: Some(
            json!({"error":message,"cap":"external_actions","limit":limit,"retry_after":retry}),
        ),
        error: Some(message),
    }))
}
/// The caller has authenticated the agent and supplied its request-local time zone.
/// All authority, replay, budget, approval and inbox writes share this transaction.
pub fn create(
    tx: &mut Tx<'_>,
    crypto: &ArEncryption,
    agent: i64,
    mut fields: Value,
    credential: Option<i64>,
    zone: &TimeZone,
) -> Result<ReadResult> {
    let state=tx.conn().query_row("SELECT g.owner_id,g.daily_external_action_cap,(g.suspended_at IS NULL AND u.status=0 AND (g.owner_id IS NULL OR EXISTS(SELECT 1 FROM users owner WHERE owner.id=g.owner_id AND owner.status=0)) AND EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=g.id AND capability='external_action' AND room_id IS NULL AND revoked_at IS NULL)) FROM agents g JOIN users u ON u.id=g.user_id WHERE g.id=?",[agent],|r|Ok((r.get::<_,Option<i64>>(0)?,r.get::<_,Option<i64>>(1)?,r.get::<_,bool>(2)?))).optional()?;
    let Some((owner, limit, true)) = state else {
        return Ok(ReadResult::fail(
            403,
            "Forbidden: agent lacks external_action capability",
        ));
    };
    let Some(owner) = owner else {
        return Ok(ReadResult::fail(422, "Agent has no owner recorded"));
    };
    let Some(account) = Account::for_user(tx.conn(), owner)? else {
        return Ok(ReadResult::fail(
            422,
            "Agent owner has no usable Fizzy account",
        ));
    };
    if account.usable_token(tx, crypto)?.is_none() {
        return Ok(ReadResult::fail(
            422,
            "Agent owner has no usable Fizzy account",
        ));
    };
    // Rails' external_id replay relation cannot quote a Hash. This precedes action
    // validation and budgeting, and each HTTP surface maps the exception itself.
    if fields["external_id"].is_object() && !super::blank(&fields["external_id"]) {
        return Err(campfire_db::Error::Other("can't quote Hash".into()));
    }
    let external =
        (!super::blank(&fields["external_id"])).then(|| json_value_to_s(&fields["external_id"]));
    if let Some(external) = &external {
        let existing=tx.conn().query_row("SELECT id,status,expires_at FROM agent_approvals WHERE agent_id=? AND external_id=? LIMIT 1",params![agent,external],|r|Ok(Approval{id:r.get(0)?,status:r.get(1)?,expires:r.get(2)?})).optional()?;
        if let Some(mut existing) = existing {
            expire(tx, &mut existing)?;
            return Ok(ReadResult::ok(payload(&existing)));
        }
    }
    if super::blank(&fields["account_id"]) {
        fields["account_id"] = json!(account.account_id)
    }
    let action = Action::from_payload(fields);
    let errors = action.errors();
    if !errors.is_empty() {
        let mut hash = serde_json::Map::<String, Value>::new();
        for (key, message) in &errors.0 {
            hash.entry(*key)
                .or_insert(json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!(message));
        }
        return Ok(ReadResult {
            status: 422,
            payload: Some(json!({"errors":hash})),
            error: Some(sentence(errors.full_messages())),
        });
    }
    if let Some(denial) = budget(tx, agent, owner, limit, zone)? {
        return Ok(denial);
    }
    let summary = action.summary().expect("valid action has a summary");
    let stored = action.payload_json();
    let mut errors = Errors::default();
    if summary.chars().count() > 500 {
        errors.add("summary", "is too long (maximum is 500 characters)")
    }
    if stored.len() > 4096 {
        errors.add("payload", "is too large (maximum is 4 KB)")
    }
    if let Some(credential) = credential {
        let exists: bool = tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM agent_credentials WHERE id=?)",
            [credential],
            |r| r.get(0),
        )?;
        if !exists {
            errors.add("agent_credential", "must exist")
        }
    }
    if !errors.is_empty() {
        return Ok(ReadResult::fail(422, &sentence(errors.full_messages())));
    }
    let now = tx.now();
    let expires = now.since(SignedDuration::from_secs(86400));
    let id=tx.conn().query_row("INSERT INTO agent_approvals (agent_id,agent_credential_id,action,summary,payload,external_id,fizzy_connected_account_id,fizzy_user_id,fizzy_user_name,expires_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",params![agent,credential,action.action_name(),summary,stored,external,account.id,account.fizzy_user_id,account.fizzy_user_name,expires,now,now],|r|r.get(0))?;
    fan_out(tx, owner, id)?;
    let mut result = ReadResult::ok(payload(&Approval {
        id,
        status: "pending".into(),
        expires,
    }));
    result.status = 202;
    Ok(result)
}
#[cfg(test)]
mod tests;

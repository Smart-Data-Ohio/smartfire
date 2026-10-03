//! AgentEvent and Agent::Delivery. Ledger changes and durable jobs share the writer transaction.
use crate::sql::{CachedStatements, exists, query_all, query_one};
use crate::{Connection, Errors, Event, Job, Message, Result, Room, Timestamp, Tx};
use jiff::SignedDuration;
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub const MESSAGE_TYPES: [&str; 3] = ["mention", "direct_message", "reply"];
pub const WORK_TYPES: [&str; 3] = ["work_assigned", "work_unassigned", "work_handed_off"];
pub const ALWAYS_READABLE: [&str; 3] = [
    "approval_decided",
    "github_action_completed",
    "fizzy_action_completed",
];
pub const DELIVERABLE_TYPES: [&str; 10] = [
    "mention",
    "direct_message",
    "reply",
    "approval_decided",
    "github_action_completed",
    "fizzy_action_completed",
    "work_assigned",
    "work_unassigned",
    "work_handed_off",
    "slash_command",
];
pub const MAX_ATTEMPTS: i64 = 5;
pub const RATE_LIMIT: i64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryJob {
    pub event_id: i64,
}
impl Job for DeliveryJob {
    const CLASS: &'static str = "Agent::DeliveryJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWebhookJob {
    pub event_id: i64,
    pub attempt: Option<i64>,
}
impl Job for EventWebhookJob {
    const CLASS: &'static str = "Agent::EventWebhookJob";
}

#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: i64,
    pub agent_id: i64,
    pub room_id: Option<i64>,
    pub message_id: Option<i64>,
    pub actor_id: Option<i64>,
    pub agent_approval_id: Option<i64>,
    pub agent_credential_id: Option<i64>,
    pub event_type: String,
    pub outcome: Option<String>,
    pub chain_id: Option<String>,
    pub metadata: Value,
    pub hop: i64,
    pub detail: Option<String>,
    pub created_at: Timestamp,
    pub webhook_status: String,
    pub webhook_attempts: i64,
    pub webhook_next_attempt_at: Option<Timestamp>,
    pub webhook_last_error: Option<String>,
}
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NewEvent {
    pub agent_id: i64,
    pub room_id: Option<i64>,
    pub message_id: Option<i64>,
    pub actor_id: Option<i64>,
    pub agent_approval_id: Option<i64>,
    pub agent_credential_id: Option<i64>,
    pub event_type: String,
    pub outcome: Option<String>,
    pub chain_id: Option<String>,
    pub metadata: Value,
    pub hop: i64,
    pub detail: Option<String>,
}
impl AgentEvent {
    pub(crate) fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            room_id: r.get("room_id")?,
            message_id: r.get("message_id")?,
            actor_id: r.get("actor_id")?,
            agent_approval_id: r.get("agent_approval_id")?,
            agent_credential_id: r.get("agent_credential_id")?,
            event_type: r.get("event_type")?,
            outcome: r.get("outcome")?,
            chain_id: r.get("chain_id")?,
            metadata: r
                .get::<_, Option<Value>>("metadata")?
                .unwrap_or(Value::Null),
            hop: r.get("hop")?,
            detail: r.get("detail")?,
            created_at: r.get("created_at")?,
            webhook_status: r.get("webhook_status")?,
            webhook_attempts: r.get("webhook_attempts")?,
            webhook_next_attempt_at: r.get("webhook_next_attempt_at")?,
            webhook_last_error: r.get("webhook_last_error")?,
        })
    }
    /// The management history's bounded 50+1 window, with complete rows.
    pub fn history_page(
        conn: &Connection,
        agent_id: i64,
        outcome: Option<&str>,
        offset: i64,
    ) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM agent_events WHERE agent_id=? AND (? IS NULL OR outcome=?) ORDER BY id DESC LIMIT 51 OFFSET ?",
            params![agent_id, outcome, outcome, offset],
            Self::from_row,
        )
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_events WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn create(tx: &Tx<'_>, a: NewEvent) -> Result<Self> {
        Self::create_record(tx, a, false)
    }
    /// Thread's after_destroy_commit keeps its already-loaded belongs_to Agent,
    /// even if the outer transaction deleted that row. Rails has no ledger FK.
    /// This is only for an identity captured by the deletion callback; ordinary
    /// event creation still validates an agent_id against the database.
    pub(crate) fn create_captured(tx: &Tx<'_>, a: NewEvent) -> Result<Self> {
        Self::create_record(tx, a, true)
    }
    fn create_record(tx: &Tx<'_>, mut a: NewEvent, captured_agent: bool) -> Result<Self> {
        let mut errors = Errors::default();
        if !DELIVERABLE_TYPES.contains(&a.event_type.as_str())
            && ![
                "posted",
                "delivery_suppressed_rate_limit",
                "delivery_suppressed_hop_limit",
                "delivery_suppressed_revoked",
            ]
            .contains(&a.event_type.as_str())
        {
            if campfire_richtext::ruby::is_blank(&a.event_type) {
                errors.add("event_type", "can't be blank");
            }
            errors.add("event_type", "is not included in the list");
        }
        if a.outcome
            .as_deref()
            .is_some_and(|o| !["pending", "delivered", "acknowledged", "suppressed"].contains(&o))
        {
            errors.add("outcome", "is not included in the list");
        }
        for (table, field, id) in [("agents", "agent", Some(a.agent_id))] {
            if let Some(id) = id
                && !captured_agent
                && !exists(
                    tx.conn(),
                    &format!("SELECT 1 FROM {table} WHERE id=?"),
                    [id],
                )?
            {
                errors.add(field, "must exist");
            }
        }
        errors.into_result()?;
        if a.hop == 0
            && let Some(hop) = a.metadata.get("hop")
        {
            a.hop = ruby_i64(hop);
        }
        let event=tx.conn().query_row_cached("INSERT INTO agent_events (agent_id,room_id,message_id,actor_id,agent_approval_id,agent_credential_id,event_type,outcome,chain_id,metadata,hop,detail,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING *",
            params![a.agent_id,a.room_id,a.message_id,a.actor_id,a.agent_approval_id,a.agent_credential_id,a.event_type,a.outcome,a.chain_id,(!a.metadata.is_null()).then_some(&a.metadata),a.hop,a.detail,tx.now()],Self::from_row)?;
        Ok(event)
    }
    /// The unfiltered ledger association. Polling must use agent_event_access,
    /// which checks live access before applying its page limit.
    pub fn for_agent(conn: &Connection, agent_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM agent_events WHERE agent_id=? ORDER BY id",
            [agent_id],
            Self::from_row,
        )
    }
    pub fn deliverable_for_agent(conn: &Connection, agent_id: i64) -> Result<Vec<Self>> {
        Self::of_types(conn, agent_id, &DELIVERABLE_TYPES)
    }
    pub fn message_deliverable_for_agent(conn: &Connection, agent_id: i64) -> Result<Vec<Self>> {
        Self::of_types(conn, agent_id, &MESSAGE_TYPES)
    }
    fn of_types(conn: &Connection, agent_id: i64, types: &[&str]) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM agent_events WHERE agent_id=? AND event_type IN (SELECT value FROM json_each(?)) ORDER BY id",
            params![agent_id, json!(types)],
            Self::from_row,
        )
    }
    /// Low-level acknowledged! model transition. HTTP/polling callers keep the
    /// authorization gate in agent_event_access::acknowledge.
    pub fn acknowledge(&mut self, tx: &Tx<'_>) -> Result<()> {
        if self.outcome.as_deref() != Some("acknowledged") {
            let mut errors = Errors::default();
            if !DELIVERABLE_TYPES.contains(&self.event_type.as_str())
                && ![
                    "posted",
                    "delivery_suppressed_rate_limit",
                    "delivery_suppressed_hop_limit",
                    "delivery_suppressed_revoked",
                ]
                .contains(&self.event_type.as_str())
            {
                if campfire_richtext::ruby::is_blank(&self.event_type) {
                    errors.add("event_type", "can't be blank");
                }
                errors.add("event_type", "is not included in the list");
            }
            if !exists(
                tx.conn(),
                "SELECT 1 FROM agents WHERE id=?",
                [self.agent_id],
            )? {
                errors.add("agent", "must exist");
            }
            errors.into_result()?;
            tx.conn().execute(
                "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
                [self.id],
            )?;
            self.outcome = Some("acknowledged".into());
        }
        Ok(())
    }
    pub fn hop(&self) -> i64 {
        if self.hop != 0 {
            self.hop
        } else {
            self.metadata.get("hop").map_or(0, ruby_i64)
        }
    }
}
pub fn ruby_i64(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|n| n as i64))
            .unwrap_or(0),
        Value::String(s) => {
            let s = s.trim_start();
            let end = s
                .char_indices()
                .find(|(i, c)| !c.is_ascii_digit() && !(*i == 0 && (*c == '-' || *c == '+')))
                .map_or(s.len(), |(i, _)| i);
            s[..end].parse().unwrap_or(0)
        }
        _ => 0,
    }
}

fn has_webhook(tx: &Tx<'_>, agent_id: i64) -> Result<bool> {
    exists(
        tx.conn(),
        "SELECT 1 FROM webhooks JOIN agents ON agents.user_id=webhooks.user_id WHERE agents.id=?",
        [agent_id],
    )
}

/// Non-message callbacks commit their delivered ledger row and optional webhook
/// job with the originating write. Webhook completion never changes polling state.
pub fn record_delivered(tx: &mut Tx<'_>, mut attributes: NewEvent) -> Result<AgentEvent> {
    attributes.outcome = Some("delivered".into());
    let event = create_delivered(tx, attributes)?;
    enqueue_delivered_webhook(tx, &event);
    Ok(event)
}

pub fn create_delivered(tx: &Tx<'_>, mut attributes: NewEvent) -> Result<AgentEvent> {
    attributes.outcome = Some("delivered".into());
    let mut event = AgentEvent::create(tx, attributes)?;
    if has_webhook(tx, event.agent_id)? {
        let next_attempt_at = tx.now();
        tx.conn().execute(
            "UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=?",
            params![next_attempt_at, event.id],
        )?;
        event.webhook_status = "pending".into();
        event.webhook_next_attempt_at = Some(next_attempt_at);
    }
    Ok(event)
}

pub fn enqueue_delivered_webhook(tx: &mut Tx<'_>, event: &AgentEvent) {
    if event.webhook_status == "pending" {
        tx.emit_after_commit(Event::job(&EventWebhookJob {
            event_id: event.id,
            attempt: Some(event.webhook_attempts),
        }));
    }
}
fn rate_limited(tx: &Tx<'_>, agent_id: i64, room_id: i64, exclude: Option<i64>) -> Result<bool> {
    let n:i64=tx.conn().query_row_cached("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND room_id=? AND event_type IN ('mention','direct_message','reply') AND outcome IN ('pending','delivered','acknowledged') AND created_at>=? AND (? IS NULL OR id!=?)",
        params![agent_id,room_id,tx.now().ago(SignedDuration::from_mins(1)),exclude,exclude],|r|r.get(0))?;
    Ok(n >= RATE_LIMIT)
}
/// Called from Message's create chain, including thread replies and synchronous webhook replies.
pub fn enqueue_for_message(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    if message.system_note || message.streaming {
        return Ok(());
    }
    let room = Room::find(tx.conn(), message.room_id)?;
    let sender: Option<i64> = query_one(
        tx.conn(),
        "SELECT id FROM agents WHERE user_id=?",
        [message.creator_id],
        |r| r.get(0),
    )?;
    let (hop, trigger) =
        super::bot_webhook_fanout::hop_and_chain_for_message_sender(tx, message, sender)?;
    let chain = trigger.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let metadata = json!({"hop":hop,"thread_id":message.thread_id});
    if let Some(agent_id) = sender {
        AgentEvent::create(
            tx,
            NewEvent {
                agent_id,
                room_id: Some(room.id),
                message_id: Some(message.id),
                actor_id: Some(message.creator_id),
                event_type: "posted".into(),
                outcome: Some("delivered".into()),
                chain_id: Some(chain.clone()),
                metadata: metadata.clone(),
                ..Default::default()
            },
        )?;
    }
    let members = query_all(
        tx.conn(),
        "SELECT agents.id,agents.user_id FROM agents JOIN memberships ON memberships.user_id=agents.user_id WHERE memberships.room_id=? ORDER BY agents.id",
        [room.id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )?;
    let mut recipients: Vec<(i64, &str)> = Vec::new();
    if room.direct() {
        recipients.extend(
            members
                .iter()
                .filter(|(_, user_id)| *user_id != message.creator_id)
                .map(|(agent_id, _)| (*agent_id, "direct_message")),
        );
    } else {
        // Hash insertion order follows the mentions, then a reply. Replacing a mentioned
        // recipient with a reply keeps its position in Rails' hash.
        for user in message.mentionees(tx.conn(), tx.rich_text())? {
            if user.id != message.creator_id
                && let Some((agent_id, _)) = members.iter().find(|(_, id)| *id == user.id)
                && !recipients.iter().any(|(id, _)| id == agent_id)
            {
                recipients.push((*agent_id, "mention"));
            }
        }
        if let Some(reply) = message
            .reply_to_message_id
            .map(|id| Message::find_by_id(tx.conn(), id))
            .transpose()?
            .flatten()
            && reply.creator_id != message.creator_id
            && let Some((agent_id, _)) = members.iter().find(|(_, id)| *id == reply.creator_id)
        {
            if let Some((_, kind)) = recipients.iter_mut().find(|(id, _)| id == agent_id) {
                *kind = "reply";
            } else {
                recipients.push((*agent_id, "reply"));
            }
        }
    }
    for (agent_id, kind) in recipients {
        let (kind, outcome, detail) = if hop >= 3 {
            (
                "delivery_suppressed_hop_limit",
                "suppressed",
                Some(format!("Hop limit reached (hop {hop})")),
            )
        } else if rate_limited(tx, agent_id, room.id, None)? {
            (
                "delivery_suppressed_rate_limit",
                "suppressed",
                Some(format!("Rate limit exceeded ({RATE_LIMIT} per minute)")),
            )
        } else {
            (kind, "pending", None)
        };
        let e = AgentEvent::create(
            tx,
            NewEvent {
                agent_id,
                room_id: Some(room.id),
                message_id: Some(message.id),
                actor_id: Some(message.creator_id),
                event_type: kind.into(),
                outcome: Some(outcome.into()),
                chain_id: Some(chain.clone()),
                metadata: json!({"hop":hop}),
                detail,
                ..Default::default()
            },
        )?;
        if outcome == "pending" {
            tx.emit_after_commit(Event::job(&DeliveryJob { event_id: e.id }));
        }
    }
    Ok(())
}
fn enqueue_webhook(tx: &mut Tx<'_>, e: &AgentEvent, wait: Duration) {
    tx.emit_after_commit(Event::job_in(
        wait,
        &EventWebhookJob {
            event_id: e.id,
            attempt: Some(e.webhook_attempts),
        },
    ));
}
/// DeliveryJob's pending outcome claim. An ack changes polling state and still owes a webhook.
pub fn perform_delivery(tx: &mut Tx<'_>, event_id: i64) -> Result<()> {
    let Some(e) = AgentEvent::find(tx.conn(), event_id)? else {
        return Ok(());
    };
    if !MESSAGE_TYPES.contains(&e.event_type.as_str()) {
        return Ok(());
    }
    if e.outcome.as_deref() == Some("acknowledged") {
        if has_webhook(tx,e.agent_id)? && tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=? AND outcome='acknowledged' AND webhook_status='none'",params![tx.now(),e.id])?==1 {enqueue_webhook(tx,&e,Duration::ZERO);}
        return Ok(());
    }
    if e.outcome.as_deref() != Some("pending") {
        return Ok(());
    }
    let available = e
        .message_id
        .map(|id| Message::find_by_id(tx.conn(), id))
        .transpose()?
        .flatten()
        .is_some()
        && e.room_id
            .map(|id| {
                exists(
                    tx.conn(),
                    "SELECT 1 FROM rooms WHERE id=? AND deleted_at IS NULL",
                    [id],
                )
            })
            .transpose()?
            .unwrap_or(false);
    if !available {
        tx.conn().execute("UPDATE agent_events SET outcome='suppressed',detail='Message no longer available' WHERE id=? AND outcome='pending'",[e.id])?;
        return Ok(());
    }
    let room = e.room_id.unwrap();
    let user: i64 =
        tx.conn()
            .query_row_cached("SELECT user_id FROM agents WHERE id=?", [e.agent_id], |r| {
                r.get(0)
            })?;
    let revoked =
        !exists(
            tx.conn(),
            "SELECT 1 FROM memberships WHERE user_id=? AND room_id=?",
            params![user, room],
        )? || super::agent_access::capability_for_user(tx.conn(), user, "read_messages", room)?
            != Some(true);
    let suppressed = if revoked {
        Some((
            "delivery_suppressed_revoked",
            "Grant revoked or room access removed".to_string(),
        ))
    } else if e.hop() >= 3 {
        Some((
            "delivery_suppressed_hop_limit",
            format!("Hop limit reached (hop {})", e.hop()),
        ))
    } else if rate_limited(tx, e.agent_id, room, Some(e.id))? {
        Some((
            "delivery_suppressed_rate_limit",
            format!("Rate limit exceeded ({RATE_LIMIT} per minute)"),
        ))
    } else {
        None
    };
    if let Some((kind, detail)) = suppressed {
        if tx.conn().execute("UPDATE agent_events SET outcome='suppressed',detail=? WHERE id=? AND outcome='pending'",params![detail,e.id])?==1 {
            AgentEvent::create(tx,NewEvent {agent_id:e.agent_id,room_id:e.room_id,message_id:e.message_id,actor_id:e.actor_id,event_type:kind.into(),outcome:Some("suppressed".into()),detail:Some(detail),chain_id:e.chain_id.clone(),metadata:json!({"hop":e.hop()}),..Default::default()})?;
        }
    } else {
        let configured = has_webhook(tx, e.agent_id)?;
        if tx.conn().execute("UPDATE agent_events SET outcome='delivered',webhook_status=?,webhook_next_attempt_at=? WHERE id=? AND outcome='pending'",params![if configured {"pending"} else {"none"},configured.then_some(tx.now()),e.id])?==1 && configured {enqueue_webhook(tx,&e,Duration::ZERO);}
    }
    Ok(())
}
/// The attempt CAS is independent of polling outcome. None supports old queued job arguments.
pub fn claim_webhook(tx: &Tx<'_>, job: &EventWebhookJob) -> Result<Option<AgentEvent>> {
    let Some(e) = AgentEvent::find(tx.conn(), job.event_id)? else {
        return Ok(None);
    };
    if e.webhook_status == "delivered" {
        return Ok(None);
    }
    if !has_webhook(tx, e.agent_id)? {
        tx.conn().execute(
            "UPDATE agent_events SET webhook_status='none' WHERE id=?",
            [e.id],
        )?;
        return Ok(None);
    }
    let attempt = job.attempt.unwrap_or(e.webhook_attempts);
    if tx.conn().execute("UPDATE agent_events SET webhook_attempts=?,webhook_next_attempt_at=? WHERE id=? AND webhook_attempts=? AND webhook_status='pending'",params![attempt+1,tx.now(),e.id,attempt])?==0 {return Ok(None);}
    AgentEvent::find(tx.conn(), e.id)
}
pub enum AttemptOutcome {
    Delivered,
    Permanent(String),
    Retry(String, Option<Duration>),
}
pub fn finish_webhook(tx: &mut Tx<'_>, e: &AgentEvent, result: AttemptOutcome) -> Result<()> {
    let (status, attempts, error, next, wait) = match result {
        AttemptOutcome::Delivered => (
            "delivered",
            e.webhook_attempts,
            None,
            e.webhook_next_attempt_at,
            None,
        ),
        AttemptOutcome::Permanent(error) => (
            "failed",
            e.webhook_attempts - 1,
            Some(error),
            e.webhook_next_attempt_at,
            None,
        ),
        AttemptOutcome::Retry(error, _) if e.webhook_attempts >= MAX_ATTEMPTS => (
            "failed",
            e.webhook_attempts,
            Some(error),
            e.webhook_next_attempt_at,
            None,
        ),
        AttemptOutcome::Retry(error, hint) => {
            let wait =
                hint.unwrap_or_else(|| Duration::from_secs(e.webhook_attempts.pow(4) as u64 + 2));
            (
                "pending",
                e.webhook_attempts,
                Some(error),
                Some(
                    tx.now()
                        .ago(-SignedDuration::from_micros(wait.as_micros() as i64)),
                ),
                Some(wait),
            )
        }
    };
    let error = error.map(|s| {
        if s.chars().count() > 500 {
            s.chars().take(497).collect::<String>() + "..."
        } else {
            s
        }
    });
    tx.conn().execute("UPDATE agent_events SET webhook_status=?,webhook_attempts=?,webhook_last_error=?,webhook_next_attempt_at=? WHERE id=?",params![status,attempts,error,next,e.id])?;
    if let Some(wait) = wait {
        enqueue_webhook(tx, e, wait);
    }
    Ok(())
}
/// Snapshots a sweep's candidates; each is committed separately so one queue failure does
/// not prevent recovery of later rows. Webhook attempts still claim against the snapshot.
#[derive(Debug, Clone)]
pub enum Recovery {
    Delivery(i64),
    Webhook {
        event_id: i64,
        attempt: i64,
        next: Option<Timestamp>,
    },
}
pub fn recovery_candidates(conn: &Connection, now: Timestamp) -> Result<Vec<Recovery>> {
    let grace = now.ago(SignedDuration::from_mins(2));
    let mut rows = query_all(
        conn,
        "SELECT id FROM agent_events WHERE outcome='pending' AND event_type IN ('mention','direct_message','reply') AND created_at<? ORDER BY id",
        [grace],
        |r| Ok(Recovery::Delivery(r.get(0)?)),
    )?;
    rows.extend(query_all(conn,"SELECT id,webhook_attempts,webhook_next_attempt_at FROM agent_events WHERE webhook_status='pending' AND webhook_attempts<5 AND ((webhook_next_attempt_at IS NULL AND created_at<?) OR webhook_next_attempt_at<?) ORDER BY id",params![grace,grace],|r|Ok(Recovery::Webhook {event_id:r.get(0)?,attempt:r.get(1)?,next:r.get(2)?}))?);
    Ok(rows)
}
pub fn fail_exhausted(tx: &Tx<'_>, now: Timestamp) -> Result<()> {
    let cutoff = now.ago(SignedDuration::from_mins(7));
    tx.conn().execute("UPDATE agent_events SET webhook_status='failed',webhook_last_error='Delivery attempts exhausted without a recorded outcome' WHERE webhook_status='pending' AND webhook_attempts>=5 AND ((webhook_next_attempt_at IS NULL AND created_at<?) OR webhook_next_attempt_at<?)",params![cutoff,cutoff])?;
    Ok(())
}
pub fn recover_one(tx: &mut Tx<'_>, candidate: Recovery) -> Result<()> {
    match candidate {
        Recovery::Delivery(event_id) => tx.emit_after_commit(Event::job(&DeliveryJob { event_id })),
        Recovery::Webhook {
            event_id,
            attempt,
            next,
        } => {
            tx.emit_after_commit(Event::job(&EventWebhookJob {
                event_id,
                attempt: Some(attempt),
            }));
            tx.conn().execute("UPDATE agent_events SET webhook_next_attempt_at=? WHERE id=? AND webhook_status='pending' AND webhook_attempts=? AND webhook_next_attempt_at IS ?",params![tx.now(),event_id,attempt,next])?;
        }
    }
    Ok(())
}

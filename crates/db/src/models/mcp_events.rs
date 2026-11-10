//! Credential-owned MCP webhook subscriptions and their durable delivery ledger.
use crate::sql::{exists, query_all, query_one};
use crate::{Agent, AgentCredential, Connection, Event, Job, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rand::RngCore;
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arguments {
    #[serde(skip_serializing_if = "Option::is_none")]
    room_ids: Option<Vec<i64>>,
}
impl Arguments {
    pub fn parse(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        if object.keys().any(|key| key != "roomIds") {
            return None;
        }
        let room_ids = match object.get("roomIds") {
            None => None,
            Some(value) => {
                let values = value.as_array()?;
                if !(1..=50).contains(&values.len()) {
                    return None;
                }
                let mut ids = values
                    .iter()
                    .map(|v| v.as_i64().filter(|n| *n > 0))
                    .collect::<Option<Vec<_>>>()?;
                ids.sort_unstable();
                if ids.windows(2).any(|pair| pair[0] == pair[1]) {
                    return None;
                }
                Some(ids)
            }
        };
        Some(Self { room_ids })
    }
    fn matches(&self, room: Option<i64>) -> bool {
        self.room_ids
            .as_ref()
            .is_none_or(|ids| room.is_some_and(|id| ids.binary_search(&id).is_ok()))
    }
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        let value: Value =
            serde_json::from_str(&r.get::<_, String>("arguments")?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Self::parse(&value).ok_or(rusqlite::Error::InvalidQuery)
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub agent_id: i64,
    pub credential_id: i64,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub previous_secret: Option<String>,
    pub previous_secret_expires_at: Option<Timestamp>,
    pub refresh_before: Timestamp,
    pub watermark: i64,
    pub last_delivery_at: Option<Timestamp>,
    pub last_error: Option<String>,
    pub failed_since: Option<Timestamp>,
}
impl Subscription {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            credential_id: r.get("agent_credential_id")?,
            name: r.get("name")?,
            url: r.get("url")?,
            secret: r.get("secret")?,
            previous_secret: r.get("previous_secret")?,
            previous_secret_expires_at: r.get("previous_secret_expires_at")?,
            refresh_before: r.get("refresh_before")?,
            watermark: r.get("watermark_event_id")?,
            last_delivery_at: r.get("last_delivery_at")?,
            last_error: r.get("last_error")?,
            failed_since: r.get("failed_since")?,
        })
    }
    pub fn find(conn: &Connection, id: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM mcp_event_subscriptions WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn result(&self, now: Timestamp, truncated: bool) -> Value {
        json!({"id":self.id,"refreshBefore":self.refresh_before.jiff().to_string(),"cursor":self.watermark.to_string(),"truncated":truncated,"deliveryStatus":{"active":self.refresh_before>now,"lastDeliveryAt":self.last_delivery_at.map(|t|t.jiff().to_string()),"lastError":self.last_error,"failedSince":self.failed_since.map(|t|t.jiff().to_string())}})
    }
}

pub fn subscription_id(credential: i64, url: &str, name: &str, arguments: &Arguments) -> String {
    let key = json!([credential, url, name, arguments]).to_string();
    hex::encode(&Sha256::digest(key.as_bytes())[..16])
}
pub fn nonce(prefix: &str) -> String {
    let mut bytes = [0; 24];
    rand::rng().fill_bytes(&mut bytes);
    format!("{prefix}{}", hex::encode(bytes))
}

/// OAuth token refresh does not change the credential that owns a subscription.
pub fn principal_live(
    conn: &Connection,
    credential: i64,
    now: Timestamp,
    origin: &str,
) -> Result<bool> {
    let Some(c) = AgentCredential::find(conn, credential)?.filter(|c| c.active(now)) else {
        return Ok(false);
    };
    let Some(a) = Agent::find(conn, c.agent_id)? else {
        return Ok(false);
    };
    if !a.active(conn)? {
        return Ok(false);
    }
    let connection = query_one(
        conn,
        "SELECT id FROM oauth_connections WHERE agent_credential_id=?",
        [credential],
        |r| r.get::<_, i64>(0),
    )?;
    match connection {
        Some(id) => super::oauth::OAuthConnection::find(conn, id)?
            .map_or(Ok(false), |c| c.live(conn, now, origin)),
        None => Ok(true),
    }
}
pub fn can_subscribe(conn: &Connection, agent: i64, arguments: &Arguments) -> Result<bool> {
    let Some(rooms) = &arguments.room_ids else {
        return super::agent_access::has_capability_anywhere(conn, agent, "read_messages");
    };
    let capability = super::agent_access::event_capability_sql("?1", "r.id", "'read_messages'");
    for room in rooms {
        if !exists(
            conn,
            &format!(
                "SELECT 1 FROM rooms r JOIN memberships member ON member.room_id=r.id JOIN agents a ON a.user_id=member.user_id WHERE a.id=?1 AND r.id=?2 AND ({capability})"
            ),
            params![agent, room],
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn endpoint_verified(conn: &Connection, credential: i64, url: &str, now: Timestamp) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM mcp_event_subscriptions WHERE agent_credential_id=? AND url=? AND verified_at>?",
        params![credential, url, now.ago(SignedDuration::from_hours(24))],
    )
}
pub fn at_limit(conn: &Connection, credential: i64, id: &str) -> Result<bool> {
    Ok(!exists(
        conn,
        "SELECT 1 FROM mcp_event_subscriptions WHERE id=?",
        [id],
    )? && conn.query_row(
        "SELECT COUNT(*) FROM mcp_event_subscriptions WHERE agent_credential_id=?",
        [credential],
        |r| r.get::<_, i64>(0),
    )? >= 20)
}

pub struct Grant {
    pub agent: i64,
    pub credential: i64,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub arguments: Arguments,
    pub ttl_ms: i64,
    pub cursor: Option<i64>,
    pub max_age_ms: i64,
    pub verified_at: Option<Timestamp>,
}
pub enum GrantResult {
    Paused,
    Granted(Box<Subscription>, bool),
    Forbidden,
    Limit,
}
pub fn subscribe(
    tx: &mut Tx<'_>,
    grant: Grant,
    encryption: &rails_compat::ar_encryption::ArEncryption,
) -> Result<GrantResult> {
    if Agent::find(tx.conn(), grant.agent)?.is_some_and(|a| a.paused()) { return Ok(GrantResult::Paused); }
    let id = subscription_id(grant.credential, &grant.url, &grant.name, &grant.arguments);
    if !can_subscribe(tx.conn(), grant.agent, &grant.arguments)?
        || !principal_live(
            tx.conn(),
            grant.credential,
            tx.now(),
            &tx.env().default_url_origin,
        )?
    {
        return Ok(GrantResult::Forbidden);
    }
    if at_limit(tx.conn(), grant.credential, &id)? {
        return Ok(GrantResult::Limit);
    }
    if let Some(verified_at) = grant.verified_at {
        tx.conn().execute(
            "UPDATE mcp_event_subscriptions SET verified_at=? WHERE agent_credential_id=? AND url=?",
            params![verified_at, grant.credential, grant.url],
        )?;
    }
    let verified_at = match grant.verified_at {
        Some(at) => Some(at),
        None => query_one(
            tx.conn(),
            "SELECT MAX(verified_at) FROM mcp_event_subscriptions WHERE agent_credential_id=? AND url=?",
            params![grant.credential, grant.url],
            |r| r.get::<_, Option<Timestamp>>(0),
        )?.flatten(),
    };
    let old = Subscription::find(tx.conn(), &id)?;
    let encrypted = encryption.encrypt(&grant.secret);
    let current: i64 = tx.conn().query_row(
        "SELECT COALESCE(MAX(id),0) FROM agent_events WHERE agent_id=?",
        [grant.agent],
        |r| r.get(0),
    )?;
    let refresh = tx.now().since(SignedDuration::from_millis(grant.ttl_ms));
    let watermark = grant.cursor.unwrap_or(current);
    if let Some(old) = &old {
        let changed = encryption
            .decrypt(&old.secret)
            .map_err(|e| crate::Error::Other(e.to_string()))?
            != grant.secret;
        tx.conn().execute("UPDATE mcp_event_subscriptions SET secret=?,previous_secret=CASE WHEN ? THEN secret ELSE previous_secret END,previous_secret_expires_at=CASE WHEN ? THEN ? ELSE previous_secret_expires_at END,refresh_before=?,watermark_event_id=MIN(watermark_event_id,?),updated_at=? WHERE id=?",params![encrypted,changed,changed,tx.now().since(SignedDuration::from_mins(5)),refresh,grant.cursor.unwrap_or(old.watermark),tx.now(),id])?;
    } else {
        tx.conn().execute("INSERT INTO mcp_event_subscriptions(id,agent_id,agent_credential_id,name,arguments,url,secret,refresh_before,verified_at,watermark_event_id,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![id,grant.agent,grant.credential,grant.name,json!(grant.arguments).to_string(),grant.url,encrypted,refresh,verified_at,watermark,tx.now(),tx.now()])?;
    }
    let mut truncated = false;
    if let Some(cursor) = grant.cursor {
        let cutoff = tx.now().ago(SignedDuration::from_millis(grant.max_age_ms));
        let auth = super::agent_event_access::authorized_sql("e", "m");
        let events = query_all(
            tx.conn(),
            &format!(
                "SELECT e.id,e.created_at,e.room_id FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.agent_id=? AND e.event_type=? AND e.id>? AND e.outcome IN ('delivered','acknowledged') AND ({auth}) ORDER BY e.id"
            ),
            params![grant.agent, grant.name, cursor],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Timestamp>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                ))
            },
        )?;
        for (event, at, room) in events {
            if !grant.arguments.matches(room) {
                continue;
            }
            if at < cutoff {
                truncated = true;
                tx.conn().execute(
                    "INSERT INTO mcp_event_deliveries(subscription_id,agent_event_id,webhook_id,status,next_attempt_at,created_at,updated_at,last_error) VALUES(?,?,?,'suppressed',?,?,?,'replay_age') ON CONFLICT(subscription_id,agent_event_id) DO NOTHING",
                    params![id,event,nonce("msg_"),tx.now(),tx.now(),tx.now()],
                )?;
            } else {
                enqueue(tx, &id, event)?;
            }
        }
    }
    Ok(GrantResult::Granted(
        Box::new(Subscription::find(tx.conn(), &id)?.expect("saved subscription")),
        truncated,
    ))
}
pub fn unsubscribe(
    tx: &Tx<'_>,
    credential: i64,
    url: &str,
    name: &str,
    arguments: &Arguments,
) -> Result<bool> {
    Ok(tx.conn().execute(
        "DELETE FROM mcp_event_subscriptions WHERE id=? AND agent_credential_id=?",
        params![
            subscription_id(credential, url, name, arguments),
            credential
        ],
    )? == 1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryJob {
    pub delivery_id: i64,
    pub attempt: i64,
}
impl Job for DeliveryJob {
    const CLASS: &'static str = "Agent::McpEventDeliveryJob";
}
#[derive(Debug, Clone)]
pub struct Delivery {
    pub id: i64,
    pub subscription_id: String,
    pub event_id: i64,
    pub webhook_id: String,
    pub body: Option<String>,
    pub attempts: i64,
}
impl Delivery {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            subscription_id: r.get("subscription_id")?,
            event_id: r.get("agent_event_id")?,
            webhook_id: r.get("webhook_id")?,
            body: r.get("body")?,
            attempts: r.get("attempts")?,
        })
    }
}
fn enqueue(tx: &mut Tx<'_>, subscription: &str, event: i64) -> Result<()> {
    let id = query_one(
        tx.conn(),
        "INSERT INTO mcp_event_deliveries(subscription_id,agent_event_id,webhook_id,status,next_attempt_at,created_at,updated_at) VALUES(?,?,?,'pending',?,?,?) ON CONFLICT(subscription_id,agent_event_id) DO UPDATE SET status='pending',attempts=0,next_attempt_at=excluded.next_attempt_at,last_error=NULL,updated_at=excluded.updated_at WHERE mcp_event_deliveries.status='expired' OR (mcp_event_deliveries.status='suppressed' AND mcp_event_deliveries.last_error='replay_age') RETURNING id",
        params![
            subscription,
            event,
            nonce("msg_"),
            tx.now(),
            tx.now(),
            tx.now()
        ],
        |r| r.get::<_, i64>(0),
    )?;
    if let Some(delivery_id) = id {
        tx.emit_after_commit(Event::job(&DeliveryJob {
            delivery_id,
            attempt: 0,
        }));
    }
    Ok(())
}
pub fn fan_out(tx: &mut Tx<'_>, event: &super::agent_delivery::AgentEvent) -> Result<()> {
    if Agent::find(tx.conn(), event.agent_id)?.is_some_and(|a| a.paused()) { return Ok(()); }
    let ids = query_all(
        tx.conn(),
        "SELECT id,arguments FROM mcp_event_subscriptions WHERE agent_id=? AND name=? AND refresh_before>? AND verified_at IS NOT NULL",
        params![event.agent_id, event.event_type, tx.now()],
        |r| Ok((r.get::<_, String>(0)?, Arguments::from_row(r)?)),
    )?;
    for (id, arguments) in ids {
        if arguments.matches(event.room_id) {
            enqueue(tx, &id, event.id)?;
        }
    }
    Ok(())
}
pub fn claim(tx: &Tx<'_>, job: &DeliveryJob) -> Result<Option<Delivery>> {
    query_one(
        tx.conn(),
        "UPDATE mcp_event_deliveries SET attempts=attempts+1,next_attempt_at=?,updated_at=? WHERE id=? AND status='pending' AND attempts=? AND attempts<5 AND next_attempt_at<=? RETURNING *",
        params![tx.now(), tx.now(), job.delivery_id, job.attempt, tx.now()],
        Delivery::from_row,
    )
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    ConnectionRefused,
    Timeout,
    TlsError,
    Http4xx,
    Http5xx,
    ChallengeFailed,
}
impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionRefused => "connection_refused",
            Self::Timeout => "timeout",
            Self::TlsError => "tls_error",
            Self::Http4xx => "http_4xx",
            Self::Http5xx => "http_5xx",
            Self::ChallengeFailed => "challenge_failed",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Termination {
    AccessRevoked,
}

pub enum DeliveryOutcome {
    Delivered,
    Retry(Category, Option<Duration>),
    Permanent(Category),
    Terminate(Termination),
}
pub fn expire(tx: &Tx<'_>, d: &Delivery) -> Result<()> {
    tx.conn().execute("UPDATE mcp_event_deliveries SET status='expired',updated_at=? WHERE id=? AND attempts=? AND status='pending'",params![tx.now(),d.id,d.attempts])?;
    Ok(())
}
pub fn suppress(tx: &Tx<'_>, d: &Delivery) -> Result<()> {
    tx.conn().execute("UPDATE mcp_event_deliveries SET status='suppressed',updated_at=? WHERE id=? AND attempts=? AND status='pending'",params![tx.now(),d.id,d.attempts])?;
    advance(tx, &d.subscription_id)
}
pub fn suppress_access_changed(tx: &Tx<'_>, d: &Delivery) -> Result<()> {
    tx.conn().execute("UPDATE mcp_event_deliveries SET status='suppressed',last_error='access_changed',updated_at=? WHERE id=? AND attempts=? AND status='pending'",params![tx.now(),d.id,d.attempts])?;
    advance(tx, &d.subscription_id)
}
pub fn delivery_cursor(
    conn: &Connection,
    subscription: &str,
    settling: Option<i64>,
) -> Result<i64> {
    Ok(query_one(conn,
        "WITH unsettled_obligations AS (
            SELECT agent_event_id AS event_id FROM mcp_event_deliveries
            WHERE subscription_id=?1 AND status IN ('pending','expired') AND id IS NOT ?2
            UNION ALL
            SELECT e.id FROM agent_events e JOIN mcp_event_subscriptions s ON s.id=?1
            WHERE e.agent_id=s.agent_id AND e.event_type=s.name AND e.id>s.watermark_event_id
              AND e.outcome IN ('pending','delivered','acknowledged')
              AND (json_type(s.arguments,'$.roomIds') IS NULL OR e.room_id IN (SELECT value FROM json_each(s.arguments,'$.roomIds')))
              AND NOT EXISTS (SELECT 1 FROM mcp_event_deliveries d WHERE d.subscription_id=s.id AND d.agent_event_id=e.id)
        )
        SELECT COALESCE((SELECT MIN(event_id)-1 FROM unsettled_obligations),
            MAX(watermark_event_id,COALESCE((SELECT MAX(agent_event_id) FROM mcp_event_deliveries
                WHERE subscription_id=?1 AND (status NOT IN ('pending','expired') OR id=?2)),watermark_event_id)))
        FROM mcp_event_subscriptions WHERE id=?1",
        params![subscription,settling],|r|r.get(0))?.unwrap_or(0))
}
fn advance(tx: &Tx<'_>, id: &str) -> Result<()> {
    tx.conn().execute(
        "UPDATE mcp_event_subscriptions SET watermark_event_id=? WHERE id=?",
        params![delivery_cursor(tx.conn(), id, None)?, id],
    )?;
    Ok(())
}
pub fn finish(tx: &mut Tx<'_>, d: &Delivery, outcome: DeliveryOutcome) -> Result<()> {
    let (status, error, wait) = match outcome {
        DeliveryOutcome::Delivered => ("delivered", None, None),
        DeliveryOutcome::Permanent(c) => ("failed", Some(c), None),
        DeliveryOutcome::Retry(c, _) if d.attempts >= 5 => ("failed", Some(c), None),
        DeliveryOutcome::Retry(c, hint) => {
            let backoff = [30, 120, 300, 480][(d.attempts - 1) as usize];
            let wait = hint
                .unwrap_or(Duration::from_secs(backoff))
                .min(Duration::from_secs(480));
            ("pending", Some(c), Some(wait))
        }
        DeliveryOutcome::Terminate(_) => {
            tx.conn().execute(
                "DELETE FROM mcp_event_subscriptions WHERE id=?",
                [&d.subscription_id],
            )?;
            return Ok(());
        }
    };
    let next = tx.now().since(SignedDuration::from_secs(
        wait.unwrap_or_default().as_secs() as i64,
    ));
    let changed=tx.conn().execute("UPDATE mcp_event_deliveries SET status=?,last_error=?,next_attempt_at=?,updated_at=? WHERE id=? AND attempts=? AND status='pending'",params![status,error.map(Category::as_str),next,tx.now(),d.id,d.attempts])?;
    if changed == 0 {
        return Ok(());
    }
    if let Some(error) = error {
        tx.conn().execute("UPDATE mcp_event_subscriptions SET last_error=?,failed_since=COALESCE(failed_since,?),updated_at=? WHERE id=?",params![error.as_str(),tx.now(),tx.now(),d.subscription_id])?;
    } else {
        tx.conn().execute("UPDATE mcp_event_subscriptions SET last_delivery_at=?,last_error=NULL,failed_since=NULL,updated_at=? WHERE id=?",params![tx.now(),tx.now(),d.subscription_id])?;
    }
    if let Some(wait) = wait {
        tx.emit_after_commit(Event::job_in(
            wait,
            &DeliveryJob {
                delivery_id: d.id,
                attempt: d.attempts,
            },
        ));
    }
    advance(tx, &d.subscription_id)
}
pub fn recovery_candidates(conn: &Connection, now: Timestamp) -> Result<Vec<DeliveryJob>> {
    query_all(
        conn,
        "SELECT id,attempts FROM mcp_event_deliveries WHERE status='pending' AND attempts<5 AND next_attempt_at<? ORDER BY id",
        [now.ago(SignedDuration::from_mins(2))],
        |r| {
            Ok(DeliveryJob {
                delivery_id: r.get(0)?,
                attempt: r.get(1)?,
            })
        },
    )
}
pub fn recover(tx: &mut Tx<'_>, job: DeliveryJob) -> Result<()> {
    if tx.conn().execute("UPDATE mcp_event_deliveries SET next_attempt_at=? WHERE id=? AND status='pending' AND attempts=? AND next_attempt_at<?",params![tx.now(),job.delivery_id,job.attempt,tx.now().ago(SignedDuration::from_mins(2))])?==1 {tx.emit_after_commit(Event::job(&job));}
    Ok(())
}
pub fn cleanup(tx: &Tx<'_>) -> Result<()> {
    let cutoff = tx.now().ago(SignedDuration::from_mins(2));
    let ids = query_all(
        tx.conn(),
        "SELECT DISTINCT subscription_id FROM mcp_event_deliveries WHERE status='pending' AND attempts>=5 AND next_attempt_at<?",
        [cutoff],
        |r| r.get::<_, String>(0),
    )?;
    tx.conn().execute("UPDATE mcp_event_deliveries SET status='failed',last_error='timeout',updated_at=? WHERE status='pending' AND attempts>=5 AND next_attempt_at<?",params![tx.now(),cutoff])?;
    for id in ids {
        tx.conn().execute("UPDATE mcp_event_subscriptions SET last_error='timeout',failed_since=COALESCE(failed_since,?),updated_at=? WHERE id=?",params![tx.now(),tx.now(),id])?;
        advance(tx, &id)?;
    }
    tx.conn().execute(
        "DELETE FROM mcp_event_subscriptions WHERE refresh_before<?",
        [tx.now().ago(SignedDuration::from_hours(24))],
    )?;
    tx.conn().execute("DELETE FROM mcp_event_deliveries WHERE status IN ('delivered','failed','suppressed') AND updated_at<?",[tx.now().ago(SignedDuration::from_hours(7*24))])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_migration_preserves_delivery_history_and_sequence() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/20261109120000_mcp_event_webhooks.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/20261109130000_mcp_event_delivery_body.sql"
        ))
        .unwrap();
        for (id, status) in [
            (1, "pending"),
            (2, "delivered"),
            (3, "failed"),
            (4, "suppressed"),
        ] {
            conn.execute("INSERT INTO mcp_event_deliveries(id,subscription_id,agent_event_id,webhook_id,status,attempts,next_attempt_at,created_at,updated_at,body) VALUES(?,'subscription',?,?,?,2,'2026-11-09','2026-11-09','2026-11-09','exact cached bytes')",params![id,id,format!("msg_{id}"),status]).unwrap();
        }
        conn.execute(
            "UPDATE sqlite_sequence SET seq=100 WHERE name='mcp_event_deliveries'",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "UPDATE mcp_event_deliveries SET status='expired' WHERE id=1",
                []
            )
            .is_err()
        );
        conn.execute_batch(include_str!(
            "../../migrations/20261109150000_mcp_delivery_expired.sql"
        ))
        .unwrap();
        let rows: Vec<(i64, String, String, i64)> = query_all(
            &conn,
            "SELECT id,status,body,attempts FROM mcp_event_deliveries ORDER BY id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
        assert_eq!(
            rows,
            [
                (1, "pending"),
                (2, "delivered"),
                (3, "failed"),
                (4, "suppressed")
            ]
            .map(|(id, status)| (id, status.into(), "exact cached bytes".into(), 2))
        );
        conn.execute(
            "UPDATE mcp_event_deliveries SET status='expired' WHERE id=1",
            [],
        )
        .unwrap();
        let sequence: i64 = conn
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name='mcp_event_deliveries'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sequence, 100);
    }

    #[test]
    fn expiry_migration_does_not_add_a_sequence_to_an_unused_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE agents(id INTEGER PRIMARY KEY); CREATE TABLE agent_credentials(id INTEGER PRIMARY KEY); CREATE TABLE agent_events(id INTEGER PRIMARY KEY);").unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/20261109120000_mcp_event_webhooks.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/20261109130000_mcp_event_delivery_body.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/20261109150000_mcp_delivery_expired.sql"
        ))
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_sequence WHERE name='mcp_event_deliveries'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}

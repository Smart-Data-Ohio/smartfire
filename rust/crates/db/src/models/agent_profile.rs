//! Read-only Agent association seam for the management pages. Lifecycle writes remain WS11.
use crate::sql::query_one;
use crate::{Connection, Result, Timestamp};

#[derive(Clone, Debug)]
pub struct AgentProfile {
    pub id: i64,
    pub owner_id: Option<i64>,
    pub kind: String,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub suspended_at: Option<Timestamp>,
    pub encrypted_webhook_signing_secret: Option<String>,
}
impl AgentProfile {
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agents WHERE user_id=? LIMIT 1",
            [user_id],
            |r| {
                Ok(Self {
                    id: r.get("id")?,
                    owner_id: r.get("owner_id")?,
                    kind: r.get("kind")?,
                    provider: r.get("provider")?,
                    runtime: r.get("runtime")?,
                    description: r.get("description")?,
                    daily_message_cap: r.get("daily_message_cap")?,
                    daily_board_post_cap: r.get("daily_board_post_cap")?,
                    daily_external_action_cap: r.get("daily_external_action_cap")?,
                    suspended_at: r.get("suspended_at")?,
                    encrypted_webhook_signing_secret: r.get("webhook_signing_secret")?,
                })
            },
        )
    }
}

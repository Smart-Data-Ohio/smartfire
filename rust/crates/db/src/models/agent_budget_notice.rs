//! app/models/agent_budget_notice.rb read-side owner API.
//! Recording, uniqueness and inbox fanout remain with agent_posting::check_budget.
use crate::sql::{query_all, query_one};
use crate::{Agent, Connection, Error, Result, Timestamp, User};
use jiff::civil::Date;
use rusqlite::Row;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBudgetNotice {
    pub id: i64,
    pub agent_id: i64,
    pub cap: String,
    pub day: Date,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl AgentBudgetNotice {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let day: String = row.get("day")?;
        Ok(Self {
            id: row.get("id")?,
            agent_id: row.get("agent_id")?,
            cap: row.get("cap")?,
            day: day.parse().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    row.as_ref().column_index("day").unwrap(),
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_budget_notices WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.ok_or(Error::RecordNotFound("AgentBudgetNotice"))
    }
    /// One bounded-bind query for exactly the authorized page's source IDs.
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        query_all(
            conn,
            "SELECT * FROM agent_budget_notices WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id",
            [serde_json::json!(ids).to_string()],
            Self::from_row,
        )
    }
    pub fn agent(&self, conn: &Connection) -> Result<Agent> {
        Agent::find(conn, self.agent_id)?.ok_or(Error::RecordNotFound("Agent"))
    }
    pub fn cap_label(&self) -> &str {
        match self.cap.as_str() {
            "messages" => "messages",
            "board_posts" => "board posts",
            "external_actions" => "external actions",
            other => other,
        }
    }
    pub fn budget_limit(&self, conn: &Connection) -> Result<Option<i64>> {
        self.budget_limit_for_agent(&self.agent(conn)?)
    }
    pub fn budget_limit_for_agent(&self, agent: &Agent) -> Result<Option<i64>> {
        if agent.id != self.agent_id {
            return Err(Error::RecordNotFound("Agent"));
        }
        Ok(match self.cap.as_str() {
            "messages" => agent.daily_message_cap,
            "board_posts" => agent.daily_board_post_cap,
            "external_actions" => agent.daily_external_action_cap,
            _ => return Err(Error::Other("Unknown agent budget cap".into())),
        })
    }
    /// Rails keeps an existing owner even when inactive or a bot. Only the
    /// ownerless fallback filters active human administrators.
    pub fn activity_recipient_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        let agent = self.agent(conn)?;
        if let Some(id) = agent.owner_id
            && User::find_by_id(conn, id)?.is_some()
        {
            return Ok(vec![id]);
        }
        query_all(
            conn,
            "SELECT id FROM users WHERE status=0 AND role=1 ORDER BY id",
            [],
            |r| r.get(0),
        )
    }
    pub fn recipient_user_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        self.activity_recipient_ids(conn)
    }
}

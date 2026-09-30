//! AgentGrant validations and revocation. Administrative authorization lives in callers.
use super::agent_access::CAPABILITIES;
use crate::sql::{exists, query_one};
use crate::{Connection, Errors, Result, Timestamp, Tx};
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct AgentGrant {
    pub id: i64,
    pub agent_id: i64,
    pub room_id: Option<i64>,
    pub capability: String,
    pub granted_by_id: i64,
    pub revoked_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone, Default)]
pub struct NewGrant {
    pub agent_id: i64,
    pub room_id: Option<i64>,
    pub capability: String,
    pub granted_by_id: i64,
    pub revoked_at: Option<Timestamp>,
}
#[derive(Default, Debug, Clone)]
pub struct GrantChanges {
    pub agent_id: Option<i64>,
    pub room_id: Option<Option<i64>>,
    pub capability: Option<String>,
    pub granted_by_id: Option<i64>,
    pub revoked_at: Option<Option<Timestamp>>,
}
impl AgentGrant {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            room_id: r.get("room_id")?,
            capability: r.get("capability")?,
            granted_by_id: r.get("granted_by_id")?,
            revoked_at: r.get("revoked_at")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_grants WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn validate(conn: &Connection, a: &NewGrant, exclude: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if !exists(conn, "SELECT 1 FROM agents WHERE id=?", [a.agent_id])? {
            errors.add("agent", "must exist");
        }
        if !exists(conn, "SELECT 1 FROM users WHERE id=?", [a.granted_by_id])? {
            errors.add("granted_by", "must exist");
        }
        let blank = campfire_richtext::ruby::is_blank(&a.capability);
        if blank {
            errors.add("capability", "can't be blank");
        }
        if !CAPABILITIES.contains(&a.capability.as_str()) {
            errors.add("capability", "is not included in the list");
        }
        // Rails room_id? queries an integer attribute: zero is false.
        if let Some(id) = a.room_id
            && id != 0
            && !exists(conn, "SELECT 1 FROM rooms WHERE id=?", [id])?
        {
            errors.add("room", "must be an existing room");
        }
        if a.revoked_at.is_none()
            && !blank
            && exists(
                conn,
                "SELECT 1 FROM agent_grants WHERE agent_id=? AND capability=? AND room_id IS ? AND revoked_at IS NULL AND (? IS NULL OR id!=?)",
                params![a.agent_id, a.capability, a.room_id, exclude, exclude],
            )?
        {
            errors.add("capability", "has already been granted");
        }
        if a.capability == "dm_anyone" && a.room_id.is_some() {
            errors.add(
                "room",
                "must be blank: dm_anyone is granted workspace-wide only",
            );
        }
        Ok(errors)
    }
    pub fn create(tx: &Tx<'_>, a: NewGrant) -> Result<Self> {
        Self::validate(tx.conn(), &a, None)?.into_result()?;
        let id=tx.conn().query_row("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?) RETURNING id",params![a.agent_id,a.room_id,a.capability,a.granted_by_id,a.revoked_at,tx.now(),tx.now()],|r|r.get(0))?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted grant"))
    }
    pub fn update(&mut self, tx: &Tx<'_>, changes: GrantChanges) -> Result<()> {
        let mut candidate =
            Self::find(tx.conn(), self.id)?.ok_or(crate::Error::RecordNotFound("AgentGrant"))?;
        macro_rules! assign {($($field:ident),*)=>{$(if let Some(value)=changes.$field {candidate.$field=value;})*};}
        assign!(agent_id, room_id, capability, granted_by_id, revoked_at);
        Self::validate(
            tx.conn(),
            &NewGrant {
                agent_id: candidate.agent_id,
                room_id: candidate.room_id,
                capability: candidate.capability.clone(),
                granted_by_id: candidate.granted_by_id,
                revoked_at: candidate.revoked_at,
            },
            Some(self.id),
        )?
        .into_result()?;
        let before = Self::find(tx.conn(), self.id)?.expect("loaded row");
        let mut sets: Vec<(&str, Box<dyn rusqlite::ToSql + '_>)> = vec![];
        macro_rules! changed {($($field:ident),*)=>{$(if candidate.$field!=before.$field {sets.push((stringify!($field),Box::new(&candidate.$field)));})*};}
        changed!(agent_id, room_id, capability, granted_by_id, revoked_at);
        if !sets.is_empty() {
            sets.push(("updated_at", Box::new(tx.now())));
            let columns = sets
                .iter()
                .map(|(field, _)| format!("{field}=?"))
                .collect::<Vec<_>>()
                .join(",");
            let mut values: Vec<&dyn rusqlite::ToSql> =
                sets.iter().map(|(_, value)| value.as_ref()).collect();
            values.push(&self.id);
            tx.conn().execute(
                &format!("UPDATE agent_grants SET {columns} WHERE id=?"),
                values.as_slice(),
            )?;
            candidate.updated_at = tx.now();
        }
        drop(sets);
        *self = candidate;
        Ok(())
    }
    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM agent_grants WHERE id=?", [self.id])?;
        Ok(())
    }
    pub fn revoke(&mut self, tx: &Tx<'_>) -> Result<()> {
        if self.revoked_at.is_some() {
            return Ok(());
        }
        Self::validate(
            tx.conn(),
            &NewGrant {
                agent_id: self.agent_id,
                room_id: self.room_id,
                capability: self.capability.clone(),
                granted_by_id: self.granted_by_id,
                revoked_at: Some(tx.now()),
            },
            Some(self.id),
        )?
        .into_result()?;
        tx.conn().execute(
            "UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE id=?",
            params![tx.now(), tx.now(), self.id],
        )?;
        self.revoked_at = Some(tx.now());
        self.updated_at = tx.now();
        Ok(())
    }
    pub fn revoke_for_agent(tx: &Tx<'_>, id: i64) -> Result<usize> {
        Ok(tx.conn().execute("UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE agent_id=? AND revoked_at IS NULL",params![tx.now(),tx.now(),id])?)
    }
    pub fn revoke_for_user(tx: &Tx<'_>, user_id: i64) -> Result<usize> {
        Ok(tx.conn().execute("UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE agent_id IN (SELECT id FROM agents WHERE user_id=?) AND revoked_at IS NULL",params![tx.now(),tx.now(),user_id])?)
    }
    pub fn revoke_for_membership(tx: &Tx<'_>, user_id: i64, room_id: i64) -> Result<usize> {
        Ok(tx.conn().execute("UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE agent_id IN (SELECT id FROM agents WHERE user_id=?) AND room_id=? AND revoked_at IS NULL",params![tx.now(),tx.now(),user_id,room_id])?)
    }
    pub fn revoke_for_room(tx: &Tx<'_>, room_id: i64) -> Result<usize> {
        Ok(tx.conn().execute("UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE room_id=? AND revoked_at IS NULL",params![tx.now(),tx.now(),room_id])?)
    }
}

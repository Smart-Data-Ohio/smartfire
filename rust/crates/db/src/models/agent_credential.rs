//! AgentCredential: digest-only reveal-once creation, validation, auth and usage stamps.
use crate::sql::{exists, query_one};
use crate::user::digest_bot_token;
use crate::{Connection, Errors, Result, Timestamp, Tx};
use rand::RngCore;
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct AgentCredential {
    pub id: i64,
    pub agent_id: i64,
    pub created_by_id: i64,
    pub name: String,
    pub token_digest: String,
    pub token_last_four: String,
    pub expires_at: Option<Timestamp>,
    /// Non-time SQLite values retained by Rails' DateTime deserialize.
    pub raw_expires_at: Option<rusqlite::types::Value>,
    pub revoked_at: Option<Timestamp>,
    pub last_used_at: Option<Timestamp>,
    pub last_used_ip: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
/// Public reveal-once creation facts; the plaintext secret is returned separately.
#[derive(Debug, Clone)]
pub struct IssuedCredential {
    pub id: i64,
    pub name: String,
    pub token_last_four: String,
}
#[derive(Default, Debug, Clone)]
pub struct NewCredential {
    pub agent_id: i64,
    pub created_by_id: i64,
    pub name: String,
    pub token_digest: String,
    pub token_last_four: String,
    pub expires_at: Option<Timestamp>,
    pub revoked_at: Option<Timestamp>,
}
#[derive(Default, Debug, Clone)]
pub struct CredentialChanges {
    pub agent_id: Option<i64>,
    pub created_by_id: Option<i64>,
    pub name: Option<String>,
    pub token_digest: Option<String>,
    pub token_last_four: Option<String>,
    pub expires_at: Option<Option<Timestamp>>,
    pub revoked_at: Option<Option<Timestamp>>,
    pub last_used_at: Option<Option<Timestamp>>,
    pub last_used_ip: Option<Option<String>>,
}
impl AgentCredential {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            agent_id: r.get("agent_id")?,
            created_by_id: r.get("created_by_id")?,
            name: r.get("name")?,
            token_digest: r.get("token_digest")?,
            token_last_four: r.get("token_last_four")?,
            expires_at: match r.get_ref("expires_at")? {
                rusqlite::types::ValueRef::Integer(_) | rusqlite::types::ValueRef::Real(_) => None,
                _ => r.get("expires_at")?,
            },
            raw_expires_at: match r.get_ref("expires_at")? {
                rusqlite::types::ValueRef::Integer(_) | rusqlite::types::ValueRef::Real(_) => {
                    Some(r.get("expires_at")?)
                }
                _ => None,
            },
            revoked_at: r.get("revoked_at")?,
            last_used_at: r.get("last_used_at")?,
            last_used_ip: r.get("last_used_ip")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agent_credentials WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn validate(conn: &Connection, a: &NewCredential, exclude: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if !exists(conn, "SELECT 1 FROM agents WHERE id=?", [a.agent_id])? {
            errors.add("agent", "must exist");
        }
        if !exists(conn, "SELECT 1 FROM users WHERE id=?", [a.created_by_id])? {
            errors.add("created_by", "must exist");
        }
        for (field, text) in [("name", &a.name), ("token_digest", &a.token_digest)] {
            if campfire_richtext::ruby::is_blank(text) {
                errors.add(field, "can't be blank");
            }
        }
        if exists(
            conn,
            "SELECT 1 FROM agent_credentials WHERE token_digest=? AND (? IS NULL OR id!=?)",
            params![a.token_digest, exclude, exclude],
        )? {
            errors.add("token_digest", "has already been taken");
        }
        if campfire_richtext::ruby::is_blank(&a.token_last_four) {
            errors.add("token_last_four", "can't be blank");
        }
        Ok(errors)
    }
    fn insert_with_expiry(
        tx: &Tx<'_>,
        a: &NewCredential,
        expiry: &dyn rusqlite::ToSql,
    ) -> Result<i64> {
        Self::validate(tx.conn(), a, None)?.into_result()?;
        Ok(tx.conn().query_row("INSERT INTO agent_credentials(agent_id,created_by_id,name,token_digest,token_last_four,expires_at,revoked_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?) RETURNING id",params![a.agent_id,a.created_by_id,a.name,a.token_digest,a.token_last_four,expiry,a.revoked_at,tx.now(),tx.now()],|r|r.get(0))?)
    }
    pub fn create(tx: &Tx<'_>, a: NewCredential) -> Result<Self> {
        let id = Self::insert_with_expiry(tx, &a, &a.expires_at)?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted credential"))
    }

    /// Rails' datetime type retains numeric/true input when serializing a save.
    /// Return issuance facts without coercing that raw SQLite value to Timestamp.
    /// All credentials use the same validation, secret generation and INSERT.
    pub fn create_with_raw_expiry_secret(
        tx: &Tx<'_>,
        agent_id: i64,
        name: &str,
        created_by_id: i64,
        expires_at: rusqlite::types::Value,
    ) -> Result<(IssuedCredential, String)> {
        let (attributes, secret) = Self::secret_attributes(agent_id, name, created_by_id);
        let id = Self::insert_with_expiry(tx, &attributes, &expires_at)?;
        Ok((
            IssuedCredential {
                id,
                name: attributes.name,
                token_last_four: attributes.token_last_four,
            },
            secret,
        ))
    }

    fn secret_attributes(agent_id: i64, name: &str, created_by_id: i64) -> (NewCredential, String) {
        let mut bytes = [0; 32];
        rand::rng().fill_bytes(&mut bytes);
        let secret = hex::encode(bytes);
        let digest = digest_bot_token(&secret);
        (
            NewCredential {
                agent_id,
                created_by_id,
                name: name.into(),
                token_last_four: digest[..4].into(),
                token_digest: digest,
                ..Default::default()
            },
            secret,
        )
    }
    pub fn create_with_secret(
        tx: &Tx<'_>,
        agent_id: i64,
        name: &str,
        created_by_id: i64,
        expires_at: Option<Timestamp>,
    ) -> Result<(Self, String)> {
        let (mut attributes, secret) = Self::secret_attributes(agent_id, name, created_by_id);
        attributes.expires_at = expires_at;
        Ok((Self::create(tx, attributes)?, secret))
    }
    pub fn authenticate(conn: &Connection, secret: &str, now: Timestamp) -> Result<Option<Self>> {
        if campfire_richtext::ruby::is_blank(secret) {
            return Ok(None);
        }
        query_one(
            conn,
            "SELECT * FROM agent_credentials WHERE token_digest=? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?) LIMIT 1",
            params![
                digest_bot_token(campfire_richtext::ruby::strip(secret)),
                now
            ],
            Self::from_row,
        )
    }
    pub fn active(&self, now: Timestamp) -> bool {
        self.revoked_at.is_none() && self.expires_at.is_none_or(|t| t > now)
    }
    pub fn update(&mut self, tx: &Tx<'_>, changes: CredentialChanges) -> Result<()> {
        let mut candidate = Self::find(tx.conn(), self.id)?
            .ok_or(crate::Error::RecordNotFound("AgentCredential"))?;
        macro_rules! assign {($($field:ident),*)=>{$(if let Some(value)=changes.$field {candidate.$field=value;})*};}
        assign!(
            agent_id,
            created_by_id,
            name,
            token_digest,
            token_last_four,
            expires_at,
            revoked_at,
            last_used_at,
            last_used_ip
        );
        Self::validate(
            tx.conn(),
            &NewCredential {
                agent_id: candidate.agent_id,
                created_by_id: candidate.created_by_id,
                name: candidate.name.clone(),
                token_digest: candidate.token_digest.clone(),
                token_last_four: candidate.token_last_four.clone(),
                expires_at: candidate.expires_at,
                revoked_at: candidate.revoked_at,
            },
            Some(self.id),
        )?
        .into_result()?;
        let before = Self::find(tx.conn(), self.id)?.expect("loaded row");
        let mut sets: Vec<(&str, Box<dyn rusqlite::ToSql + '_>)> = vec![];
        macro_rules! changed {($($field:ident),*)=>{$(if candidate.$field!=before.$field {sets.push((stringify!($field),Box::new(&candidate.$field)));})*};}
        changed!(
            agent_id,
            created_by_id,
            name,
            token_digest,
            token_last_four,
            expires_at,
            revoked_at,
            last_used_at,
            last_used_ip
        );
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
                &format!("UPDATE agent_credentials SET {columns} WHERE id=?"),
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
            .execute("DELETE FROM agent_credentials WHERE id=?", [self.id])?;
        Ok(())
    }
    pub fn revoke(&mut self, tx: &Tx<'_>) -> Result<()> {
        if self.revoked_at.is_some() {
            return Ok(());
        }
        Self::validate(
            tx.conn(),
            &NewCredential {
                agent_id: self.agent_id,
                created_by_id: self.created_by_id,
                name: self.name.clone(),
                token_digest: self.token_digest.clone(),
                token_last_four: self.token_last_four.clone(),
                expires_at: self.expires_at,
                revoked_at: Some(tx.now()),
            },
            Some(self.id),
        )?
        .into_result()?;
        tx.conn().execute(
            "UPDATE agent_credentials SET revoked_at=?,updated_at=? WHERE id=?",
            params![tx.now(), tx.now(), self.id],
        )?;
        self.revoked_at = Some(tx.now());
        self.updated_at = tx.now();
        Ok(())
    }
    pub fn record_use(&mut self, tx: &Tx<'_>, ip: Option<&str>) -> Result<()> {
        let now = tx.now();
        let cutoff = now.ago(jiff::SignedDuration::from_mins(1));
        if tx.conn().execute("UPDATE agent_credentials SET last_used_at=?,last_used_ip=?,updated_at=? WHERE id=? AND (last_used_at IS NULL OR last_used_at<=?)",params![now,ip,now,self.id,cutoff])?>0 {self.last_used_at=Some(now);self.last_used_ip=ip.map(str::to_owned);self.updated_at=now;}
        Ok(())
    }
}

use crate::sql::{query_all, query_one};
use crate::{Connection, Errors, NewUser, Result, Timestamp, Tx, User};
use jiff::SignedDuration;
use rand::RngCore;
use rusqlite::{Row, params};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteExpiry {
    ThirtyMinutes,
    OneHour,
    SixHours,
    TwelveHours,
    OneDay,
    SevenDays,
    Never,
}

impl InviteExpiry {
    fn seconds(self) -> Option<i64> {
        match self {
            Self::ThirtyMinutes => Some(1800),
            Self::OneHour => Some(3600),
            Self::SixHours => Some(21600),
            Self::TwelveHours => Some(43200),
            Self::OneDay => Some(86400),
            Self::SevenDays => Some(604800),
            Self::Never => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteState {
    Active,
    Expired,
    Exhausted,
    Revoked,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInvite {
    pub id: i64,
    pub creator_id: i64,
    pub creator_name: String,
    pub token_digest: String,
    pub expires_at: Option<Timestamp>,
    pub max_uses: Option<i64>,
    pub uses: i64,
    pub revoked_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

const SELECT: &str =
    "SELECT i.*, u.name AS creator_name FROM workspace_invites i JOIN users u ON u.id=i.creator_id";

impl WorkspaceInvite {
    fn row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            creator_id: row.get("creator_id")?,
            creator_name: row.get("creator_name")?,
            token_digest: row.get("token_digest")?,
            expires_at: row.get("expires_at")?,
            max_uses: row.get("max_uses")?,
            uses: row.get("uses")?,
            revoked_at: row.get("revoked_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn digest(token: &str) -> String {
        hex::encode(Sha256::digest(token.as_bytes()))
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(conn, &format!("{SELECT} WHERE i.id=?"), [id], Self::row)
    }

    pub fn find_by_token(conn: &Connection, token: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            &format!("{SELECT} WHERE i.token_digest=?"),
            [Self::digest(token)],
            Self::row,
        )
    }

    pub fn list(conn: &Connection) -> Result<Vec<Self>> {
        query_all(conn, &format!("{SELECT} ORDER BY i.id DESC"), [], Self::row)
    }

    /// The token is returned only here; neither the row nor its audit entry stores it.
    pub fn create(
        tx: &Tx<'_>,
        creator_id: i64,
        expiry: InviteExpiry,
        max_uses: Option<i64>,
    ) -> Result<(Self, String)> {
        let mut errors = Errors::default();
        if max_uses.is_some_and(|limit| ![1, 5, 10, 25, 50, 100].contains(&limit)) {
            errors.add("max_uses", "must be 1, 5, 10, 25, 50, 100 or unlimited");
        }
        errors.into_result()?;
        let mut bytes = [0; 32];
        rand::rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);
        let expires_at = expiry
            .seconds()
            .map(|seconds| tx.now().since(SignedDuration::from_secs(seconds)));
        tx.conn().execute(
            "INSERT INTO workspace_invites(creator_id,token_digest,expires_at,max_uses,created_at,updated_at) VALUES(?,?,?,?,?,?)",
            params![creator_id, Self::digest(&token), expires_at, max_uses, tx.now(), tx.now()],
        )?;
        let invite = Self::find(tx.conn(), tx.conn().last_insert_rowid())?
            .ok_or(crate::Error::RecordNotFound("WorkspaceInvite"))?;
        Ok((invite, token))
    }

    pub fn state(&self, now: Timestamp) -> InviteState {
        if self.revoked_at.is_some() {
            InviteState::Revoked
        } else if self.expires_at.is_some_and(|expiry| expiry <= now) {
            InviteState::Expired
        } else if self.max_uses.is_some_and(|limit| self.uses >= limit) {
            InviteState::Exhausted
        } else {
            InviteState::Active
        }
    }

    pub fn revoke(tx: &Tx<'_>, id: i64) -> Result<Option<Self>> {
        tx.conn().execute(
            "UPDATE workspace_invites SET revoked_at=?,updated_at=? WHERE id=? AND revoked_at IS NULL",
            params![tx.now(), tx.now(), id],
        )?;
        Self::find(tx.conn(), id)
    }

    /// `Database::write` holds BEGIN IMMEDIATE across availability, use consumption and user
    /// creation. Propagate creation errors so the enclosing transaction rolls the use back.
    pub fn redeem(tx: &mut Tx<'_>, token: &str, attributes: NewUser) -> Result<Option<User>> {
        if !tx.in_transaction() {
            return Err(crate::Error::Other(
                "Invite redemption requires a write transaction".into(),
            ));
        }
        let Some(invite) = Self::find_by_token(tx.conn(), token)? else {
            return Ok(None);
        };
        if invite.state(tx.now()) != InviteState::Active {
            return Ok(None);
        }
        tx.conn().execute(
            "UPDATE workspace_invites SET uses=uses+1,updated_at=? WHERE id=?",
            params![tx.now(), invite.id],
        )?;
        User::create(tx, attributes).map(Some)
    }
}

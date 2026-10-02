//! `app/models/google_identity.rb` and `google/sign_in/account_linker.rb`.
//! Only verified OIDC claims belong here. Calendar grants are never sign-in identities.
use campfire_richtext::ruby::{is_blank, json_value_to_s, strip};
use rails_compat::unicode::downcase;
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Map, Value};

use crate::{Connection, Error, NewUser, Result, Role, Status, Timestamp, Tx, User};

#[derive(Debug, Clone)]
pub struct GoogleIdentity {
    pub id: i64,
    pub user_id: i64,
    pub subject: String,
    pub email: String,
    pub domain: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    Existing,
    Linked,
    Provisioned,
}
#[derive(Debug)]
pub struct Resolution {
    pub user: User,
    pub kind: ResolutionKind,
}

impl GoogleIdentity {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            subject: row.get("subject")?,
            email: row.get("email")?,
            domain: row.get("domain")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn for_subject(conn: &Connection, subject: &str) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT * FROM google_identities WHERE subject=? LIMIT 1",
                [subject],
                Self::from_row,
            )
            .optional()?)
    }
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT * FROM google_identities WHERE user_id=? LIMIT 1",
                [user_id],
                Self::from_row,
            )
            .optional()?)
    }
    fn create(tx: &Tx<'_>, user_id: i64, subject: &str, email: &str, domain: &str) -> Result<Self> {
        let mut errors = crate::Errors::default();
        if is_blank(subject) {
            errors.add("subject", "can't be blank");
        }
        if is_blank(email) {
            errors.add("email", "can't be blank");
        }
        if Self::for_subject(tx.conn(), subject)?.is_some() {
            errors.add("subject", "has already been taken");
        }
        if Self::for_user(tx.conn(), user_id)?.is_some() {
            errors.add("user_id", "has already been taken");
        }
        if User::find_by_id(tx.conn(), user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        errors.into_result()?;
        let now = tx.now();
        tx.conn().execute("INSERT INTO google_identities(user_id,subject,email,domain,created_at,updated_at) VALUES(?,?,?,?,?,?)",
            params![user_id,subject,email,domain,now,now])?;
        Ok(Self::for_user(tx.conn(), user_id)?.expect("inserted identity"))
    }
    fn update_claims(&mut self, tx: &Tx<'_>, email: &str, domain: &str) -> Result<()> {
        // Active Record partial updates leave updated_at alone when neither claim changed.
        if self.email != email || self.domain.as_deref() != Some(domain) {
            let now = tx.now();
            tx.conn().execute(
                "UPDATE google_identities SET email=?,domain=?,updated_at=? WHERE id=?",
                params![email, domain, now, self.id],
            )?;
            self.email = email.into();
            self.domain = Some(domain.into());
            self.updated_at = now;
        }
        Ok(())
    }

    /// Immutable subject wins before ANY email lookup. Call in the transaction that also
    /// creates the first-factor session/pending challenge and its audits.
    pub fn resolve(tx: &mut Tx<'_>, claims: &Map<String, Value>) -> Result<Resolution> {
        let (subject, email, domain) = fields(claims)?;
        if let Some(mut identity) = Self::for_subject(tx.conn(), &subject)? {
            let user = User::find(tx.conn(), identity.user_id)?;
            ensure_eligible(tx.conn(), &user)?;
            identity.update_claims(tx, &email, &domain)?;
            return Ok(Resolution {
                user,
                kind: ResolutionKind::Existing,
            });
        }
        let mut query = tx
            .conn()
            .prepare("SELECT * FROM users WHERE LOWER(email_address)=?")?;
        let users = query
            .query_map([downcase(&email)], User::from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(query);
        if users.len() > 1 {
            return reject("ambiguous");
        }
        if let Some(user) = users.into_iter().next() {
            ensure_eligible(tx.conn(), &user)?;
            if Self::for_user(tx.conn(), user.id)?.is_some_and(|i| i.subject != subject) {
                return reject("subject_mismatch");
            }
            let (allowed, changed): (bool, Option<String>) = tx.conn().query_row(
                "SELECT google_email_link_allowed,email_self_changed_at FROM users WHERE id=?",
                [user.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if !allowed || changed.is_some() {
                return reject("admin_link_required");
            }
            Self::create(tx, user.id, &subject, &email, &domain)?;
            return Ok(Resolution {
                user,
                kind: ResolutionKind::Linked,
            });
        }
        if deactivated_predecessor(tx.conn(), &email)? {
            return reject("deactivated");
        }
        // Tx uses an immediate SQLite writer lock. Subject lookup, email checks, and creation
        // serialize with other resolvers and deactivation, so no lost creation race is possible.
        let user = User::create(
            tx,
            NewUser {
                name: display_name(claims, &email),
                email_address: Some(email.clone()),
                role: Role::Member,
                ..Default::default()
            },
        )?;
        Self::create(tx, user.id, &subject, &email, &domain)?;
        Ok(Resolution {
            user,
            kind: ResolutionKind::Provisioned,
        })
    }

    /// The signed-in member proves account ownership independently of its local email address.
    pub fn link_to_user(tx: &Tx<'_>, claims: &Map<String, Value>, user_id: i64) -> Result<Self> {
        let (subject, email, domain) = fields(claims)?;
        let user = User::find(tx.conn(), user_id)?;
        ensure_eligible(tx.conn(), &user)?;
        if let Some(mut identity) = Self::for_subject(tx.conn(), &subject)? {
            if identity.user_id != user_id {
                return reject("subject_taken");
            }
            identity.update_claims(tx, &email, &domain)?;
            return Ok(identity);
        }
        if Self::for_user(tx.conn(), user_id)?.is_some() {
            return reject("already_linked");
        }
        Self::create(tx, user_id, &subject, &email, &domain)
    }

    pub fn unlink(tx: &Tx<'_>, user_id: i64) -> Result<bool> {
        Ok(tx
            .conn()
            .execute("DELETE FROM google_identities WHERE user_id=?", [user_id])?
            == 1)
    }
}

fn reject<T>(reason: &'static str) -> Result<T> {
    Err(Error::GoogleSignInRejected(reason))
}
fn claim(claims: &Map<String, Value>, key: &str) -> String {
    json_value_to_s(claims.get(key).unwrap_or(&Value::Null))
}
fn fields(claims: &Map<String, Value>) -> Result<(String, String, String)> {
    let subject = claim(claims, "sub");
    let email = strip(&claim(claims, "email")).to_owned();
    let domain = downcase(strip(&claim(claims, "hd")));
    if is_blank(&subject) || is_blank(&email) {
        return reject("bad_token");
    }
    Ok((subject, email, domain))
}
fn ensure_eligible(conn: &Connection, user: &User) -> Result<()> {
    match user.status {
        Status::Deactivated => return reject("deactivated"),
        Status::Banned => return reject("banned"),
        Status::Active => {}
    }
    let agent: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM agents WHERE user_id=?)",
        [user.id],
        |r| r.get(0),
    )?;
    if user.role == Role::Bot || agent {
        return reject("ineligible");
    }
    Ok(())
}
fn deactivated_predecessor(conn: &Connection, email: &str) -> Result<bool> {
    let Some((local, domain)) = email.split_once('@') else {
        return Ok(false);
    };
    if is_blank(local) || is_blank(domain) {
        return Ok(false);
    }
    let escape = |s: &str| {
        downcase(s)
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    };
    let pattern = format!("{}-deactivated-%@{}", escape(local), escape(domain));
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM users WHERE status=1 AND LOWER(email_address) LIKE ? ESCAPE '\\')", [pattern], |r| r.get(0))?)
}
fn display_name(claims: &Map<String, Value>, email: &str) -> String {
    let name = strip(&claim(claims, "name")).to_owned();
    if !is_blank(&name) {
        return name;
    }
    let parts: Vec<String> = ["given_name", "family_name"]
        .iter()
        .map(|key| strip(&claim(claims, key)).to_owned())
        .filter(|s| !is_blank(s))
        .collect();
    if !parts.is_empty() {
        parts.join(" ")
    } else {
        email.split('@').next().unwrap_or("").into()
    }
}

#[cfg(test)]
mod tests;

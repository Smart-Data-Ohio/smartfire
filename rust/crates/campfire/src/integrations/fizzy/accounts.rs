//! `FizzyConnectedAccount`: encrypted per-user PAT, Rails validations and disconnection state.
use campfire_db::{Connection, Errors, Result, Tx};
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::{OptionalExtension, params};
pub const UNREADABLE_TOKEN_REASON: &str = "The stored token could not be read; link it again";
pub const REJECTED_TOKEN_REASON: &str = "Fizzy rejected the linked token (401)";
// Ciphertext and plaintext credentials must never be Debug/Serialize fields.
#[derive(Clone)]
pub struct Account {
    pub id: i64,
    pub user_id: i64,
    pub account_id: String,
    #[allow(dead_code, reason = "Fizzy profile/agent controller consumers remain staged")]
    pub account_name: Option<String>,
    #[allow(dead_code, reason = "Fizzy profile/agent controller consumers remain staged")]
    pub fizzy_user_id: Option<String>,
    #[allow(dead_code, reason = "Fizzy profile/agent controller consumers remain staged")]
    pub fizzy_user_name: Option<String>,
    pub disconnected_reason: Option<String>,
    access_token: String,
}
#[allow(dead_code, reason = "Fizzy connection controller input remains staged")]
pub struct Input<'a> {
    pub user_id: i64,
    pub account_id: &'a str,
    pub account_name: Option<&'a str>,
    pub fizzy_user_id: Option<&'a str>,
    pub fizzy_user_name: Option<&'a str>,
    pub token: &'a str,
}
impl Account {
    pub fn for_user(conn: &Connection, user: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT * FROM fizzy_connected_accounts WHERE user_id=?",
                [user],
                |r| {
                    Ok(Self {
                        id: r.get("id")?,
                        user_id: r.get("user_id")?,
                        account_id: r.get("fizzy_account_id")?,
                        account_name: r.get("fizzy_account_name")?,
                        fizzy_user_id: r.get("fizzy_user_id")?,
                        fizzy_user_name: r.get("fizzy_user_name")?,
                        disconnected_reason: r.get("disconnected_reason")?,
                        access_token: r.get("access_token")?,
                    })
                },
            )
            .optional()?)
    }
    pub fn connected(&self) -> bool {
        self.disconnected_reason
            .as_deref()
            .is_none_or(|s| s.chars().all(char::is_whitespace))
    }
    pub fn usable_token(&self, tx: &Tx<'_>, crypto: &ArEncryption) -> Result<Option<String>> {
        let deactivated: bool = tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id=? AND status=1)", [self.user_id], |r| r.get(0),
        )?;
        if deactivated {
            self.mark_disconnected(tx, "Account deactivated")?;
            return Ok(None);
        }
        if !self.connected() {
            return Ok(None);
        }
        match crypto.decrypt(&self.access_token) {
            Ok(token) => Ok((!token.chars().all(char::is_whitespace)).then_some(token)),
            Err(_) => {
                self.mark_disconnected(tx, UNREADABLE_TOKEN_REASON)?;
                Ok(None)
            }
        }
    }
    pub fn mark_disconnected(&self, tx: &Tx<'_>, reason: &str) -> Result<()> {
        validate(tx, Some(self.id), self.user_id, &self.account_id)?;
        tx.conn().execute(
            "UPDATE fizzy_connected_accounts SET disconnected_reason=?1,updated_at=?2 WHERE id=?3",
            params![reason, tx.now(), self.id],
        )?;
        Ok(())
    }
    #[allow(dead_code, reason = "Fizzy connection controller consumer remains staged")]
    pub fn create(tx: &Tx<'_>, crypto: &ArEncryption, input: &Input<'_>) -> Result<Self> {
        Self::save(tx, crypto, input, false)
    }
    #[allow(dead_code, reason = "Fizzy connection controller consumer remains staged")]
    pub fn relink(tx: &Tx<'_>, crypto: &ArEncryption, input: &Input<'_>) -> Result<Self> {
        Self::save(tx, crypto, input, true)
    }
    #[allow(dead_code, reason = "Used by staged Fizzy connection writes")]
    fn save(tx: &Tx<'_>, crypto: &ArEncryption, input: &Input<'_>, relink: bool) -> Result<Self> {
        let existing = Self::for_user(tx.conn(), input.user_id)?;
        let id = if relink {
            existing.as_ref().map(|a| a.id)
        } else {
            None
        };
        validate(tx, id, input.user_id, input.account_id)?;
        let token = crypto.encrypt(input.token);
        tx.conn().execute("INSERT INTO fizzy_connected_accounts (user_id,fizzy_account_id,fizzy_account_name,fizzy_user_id,fizzy_user_name,access_token,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?7) ON CONFLICT(user_id) DO UPDATE SET fizzy_account_id=excluded.fizzy_account_id,fizzy_account_name=excluded.fizzy_account_name,fizzy_user_id=excluded.fizzy_user_id,fizzy_user_name=excluded.fizzy_user_name,access_token=excluded.access_token,disconnected_reason=NULL,updated_at=excluded.updated_at",params![input.user_id,input.account_id,input.account_name,input.fizzy_user_id,input.fizzy_user_name,token,tx.now()])?;
        Ok(Self::for_user(tx.conn(), input.user_id)?.expect("saved account"))
    }
    /// Disconnect clears only this viewer's payloads; references/cards are shared.
    #[allow(dead_code, reason = "Fizzy connection controller consumer remains staged")]
    pub fn disconnect(tx: &Tx<'_>, user: i64) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM fizzy_card_caches WHERE user_id=?", [user])?;
        tx.conn().execute(
            "DELETE FROM fizzy_connected_accounts WHERE user_id=?",
            [user],
        )?;
        Ok(())
    }

}

fn validate(tx: &Tx<'_>, id: Option<i64>, user: i64, account: &str) -> Result<()> {
    let mut errors = Errors::default();
    if !tx.conn().query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?)",
        [user],
        |r| r.get::<_, bool>(0),
    )? {
        errors.add("user", "must exist");
    }
    if tx.conn().query_row(
        "SELECT EXISTS(SELECT 1 FROM fizzy_connected_accounts WHERE user_id=?1 AND id!=?2)",
        params![user, id.unwrap_or(0)],
        |r| r.get::<_, bool>(0),
    )? {
        errors.add("user_id", "has already been taken");
    }
    if account.chars().all(char::is_whitespace) {
        errors.add("fizzy_account_id", "can't be blank");
    }
    errors.into_result()
}

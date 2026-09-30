//! The four `app/models/two_factor_*.rb` models and `User::TwoFactor`.
//! These methods require the writer's transaction, equivalent to Rails' row locks on SQLite.
//! Controllers own authentication, session binding, audit records and lockout notifications.

use jiff::SignedDuration;
use rails_compat::{ar_encryption::ArEncryption, totp};
use rand::RngCore;
use rusqlite::{Connection, Row, params};
use sha2::{Digest, Sha256};

use crate::error::OptionalExt;
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::{Error, Errors, Result, Timestamp, Tx, User};

pub const BACKUP_CODE_COUNT: usize = 10;
pub const FAILURES_BEFORE_LOCKOUT: i64 = 5;
pub const SETUP_TTL: SignedDuration = SignedDuration::from_secs(30 * 60);
pub const REMEMBER_FOR: SignedDuration = SignedDuration::from_secs(30 * 24 * 60 * 60);

#[derive(Clone)]
pub struct TwoFactorCredential {
    pub id: i64,
    pub user_id: i64,
    encrypted_secret: String,
    pub confirmed_at: Option<Timestamp>,
    pub last_totp_at: Option<i64>,
    pub consecutive_failures: i64,
    pub lockout_count: i64,
    pub locked_until: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeFailure {
    Failed,
    Locked,
}

impl TwoFactorCredential {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            encrypted_secret: row.get("secret")?,
            confirmed_at: row.get("confirmed_at")?,
            last_totp_at: row.get("last_totp_at")?,
            consecutive_failures: row.get("consecutive_failures")?,
            lockout_count: row.get("lockout_count")?,
            locked_until: row.get("locked_until")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            "SELECT * FROM two_factor_credentials WHERE id = ?",
            [id],
            Self::from_row,
        )?
        .or_not_found("TwoFactorCredential")
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM two_factor_credentials WHERE user_id = ?",
            [user_id],
            Self::from_row,
        )
    }

    pub fn create(
        tx: &mut Tx<'_>,
        encryption: &ArEncryption,
        user_id: i64,
        secret: &str,
    ) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if !user_exists(tx.conn(), user_id)? {
            errors.add("user", "must exist");
        }
        if Self::for_user(tx.conn(), user_id)?.is_some() {
            errors.add("user_id", "has already been taken");
        }
        if blank(secret) {
            errors.add("secret", "can't be blank");
        }
        errors.into_result()?;
        let now = tx.now();
        // ROTP::Base32.random_base32 produces an ASCII-8BIT Ruby string, not UTF-8.
        let encrypted = encryption.encrypt_with_encoding(secret.as_bytes(), "ASCII-8BIT");
        let id: i64 = tx.conn().query_row_cached("INSERT INTO two_factor_credentials (user_id, secret, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id",
            params![user_id, encrypted, now, now], |row| row.get(0))?;
        Self::find(tx.conn(), id)
    }

    pub fn secret(&self, encryption: &ArEncryption) -> Result<String> {
        decrypt(encryption, &self.encrypted_secret)
    }

    pub fn enabled(&self) -> bool {
        self.confirmed_at.is_some()
    }

    pub fn provisioning_uri(
        &self,
        encryption: &ArEncryption,
        email_address: &str,
    ) -> Result<String> {
        Ok(totp::provisioning_uri(
            &self.secret(encryption)?,
            email_address,
        ))
    }

    pub fn formatted_secret(&self, encryption: &ArEncryption) -> Result<String> {
        let secret = self.secret(encryption)?;
        let mut groups = Vec::new();
        for line in secret.split('\n') {
            let characters: Vec<_> = line.chars().collect();
            groups.extend(
                characters
                    .chunks(4)
                    .map(|chunk| chunk.iter().collect::<String>()),
            );
        }
        Ok(groups.join(" "))
    }

    /// Reload under the write lock: stale clones can't replay an already accepted step.
    pub fn verify_code(
        &mut self,
        tx: &mut Tx<'_>,
        encryption: &ArEncryption,
        code: &str,
    ) -> Result<bool> {
        require_transaction(tx)?;
        if code.chars().all(totp::ruby_code_whitespace) {
            return Ok(false);
        }
        *self = Self::find(tx.conn(), self.id)?;
        let Some(at) = matched_step(
            &decrypt_totp_secret(encryption, &self.encrypted_secret)?,
            code,
            tx.now(),
            self.last_totp_at,
        )?
        else {
            return Ok(false);
        };
        tx.conn().execute_cached(
            "UPDATE two_factor_credentials SET last_totp_at = ?, updated_at = ? WHERE id = ?",
            params![at, tx.now(), self.id],
        )?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(true)
    }

    /// Like Rails' model method, the caller must obtain the current session's `valid_for`
    /// setup secret. Expiry and session ownership are controller checks, not model checks.
    pub fn confirm_with_setup_secret(
        &mut self,
        tx: &mut Tx<'_>,
        encryption: &ArEncryption,
        setup: &TwoFactorSetupSecret,
        code: &str,
    ) -> Result<bool> {
        require_transaction(tx)?;
        if code.chars().all(totp::ruby_code_whitespace) {
            return Ok(false);
        }
        *self = Self::find(tx.conn(), self.id)?;
        let secret = decrypt_totp_secret(encryption, &setup.encrypted_secret)?;
        let Some(at) = matched_step(&secret, code, tx.now(), self.last_totp_at)? else {
            return Ok(false);
        };
        let now = tx.now();
        let encrypted = encryption.encrypt_with_encoding(secret.as_bytes(), "ASCII-8BIT");
        tx.conn().execute_cached("UPDATE two_factor_credentials SET secret = ?, confirmed_at = ?, last_totp_at = ?, updated_at = ? WHERE id = ?", params![encrypted, now, at, now, self.id])?;
        tx.conn().execute_cached(
            "DELETE FROM two_factor_setup_secrets WHERE id = ?",
            [setup.id],
        )?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(true)
    }

    pub fn locked_out(&self, now: Timestamp) -> bool {
        self.locked_until.is_some_and(|until| until > now)
    }

    pub fn lockout_duration_for(level: i64) -> SignedDuration {
        // Ruby Array#fetch also accepts negative indices; its fallback is 15 minutes.
        let seconds = match level {
            1 | -2 => 60,
            2 | -1 => 300,
            _ => 900,
        };
        SignedDuration::from_secs(seconds)
    }

    pub fn register_challenge_failure(&mut self, tx: &mut Tx<'_>) -> Result<ChallengeFailure> {
        require_transaction(tx)?;
        *self = Self::find(tx.conn(), self.id)?;
        if self.locked_out(tx.now()) {
            return Ok(ChallengeFailure::Failed);
        }
        let failures = self
            .consecutive_failures
            .checked_add(1)
            .ok_or_else(|| Error::Other("two-factor failure counter out of range".into()))?;
        let outcome = if failures >= FAILURES_BEFORE_LOCKOUT {
            let level = self
                .lockout_count
                .checked_add(1)
                .ok_or_else(|| Error::Other("two-factor lockout counter out of range".into()))?;
            let until = tx.now().since(Self::lockout_duration_for(level));
            tx.conn().execute_cached("UPDATE two_factor_credentials SET consecutive_failures = 0, lockout_count = ?, locked_until = ?, updated_at = ? WHERE id = ?", params![level, until, tx.now(), self.id])?;
            ChallengeFailure::Locked
        } else {
            tx.conn().execute_cached("UPDATE two_factor_credentials SET consecutive_failures = ?, updated_at = ? WHERE id = ?", params![failures, tx.now(), self.id])?;
            ChallengeFailure::Failed
        };
        *self = Self::find(tx.conn(), self.id)?;
        Ok(outcome)
    }

    pub fn register_challenge_success(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute_cached("UPDATE two_factor_credentials SET consecutive_failures = 0, lockout_count = 0, locked_until = NULL, updated_at = ? WHERE id = ?", params![tx.now(), self.id])?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(())
    }

    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute_cached(
            "DELETE FROM two_factor_backup_codes WHERE two_factor_credential_id = ?",
            [self.id],
        )?;
        tx.conn()
            .execute_cached("DELETE FROM two_factor_credentials WHERE id = ?", [self.id])?;
        Ok(())
    }
}

fn matched_step(
    secret: &str,
    code: &str,
    now: Timestamp,
    after: Option<i64>,
) -> Result<Option<i64>> {
    // Ruby rescues ArgumentError (negative time), but not an invalid base32 secret.
    match totp::verify_code(secret, code, now.as_second(), after) {
        Ok(at) => Ok(at),
        Err(totp::InvalidTotp::Time) => Ok(None),
        Err(totp::InvalidTotp::Secret) => Err(Error::Other("invalid TOTP secret".into())),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TwoFactorBackupCode {
    pub id: i64,
    pub two_factor_credential_id: i64,
    pub code_digest: String,
    pub used_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl TwoFactorBackupCode {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            two_factor_credential_id: row.get("two_factor_credential_id")?,
            code_digest: row.get("code_digest")?,
            used_at: row.get("used_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn normalize(code: &str) -> String {
        code.chars()
            .filter(|c| !totp::ruby_code_whitespace(*c) && *c != '-')
            .flat_map(char::to_lowercase)
            .collect()
    }

    pub fn digest(code: &str) -> String {
        hex::encode(Sha256::digest(Self::normalize(code).as_bytes()))
    }

    pub fn generate_code() -> String {
        sql::alphanumeric(10).to_ascii_lowercase()
    }

    pub fn for_credential(conn: &Connection, credential_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM two_factor_backup_codes WHERE two_factor_credential_id = ?",
            [credential_id],
            Self::from_row,
        )
    }

    pub fn create(tx: &mut Tx<'_>, credential_id: i64, code_digest: &str) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if query_one(
            tx.conn(),
            "SELECT id FROM two_factor_credentials WHERE id = ?",
            [credential_id],
            |r| r.get::<_, i64>(0),
        )?
        .is_none()
        {
            errors.add("two_factor_credential", "must exist");
        }
        if blank(code_digest) {
            errors.add("code_digest", "can't be blank");
        }
        if query_one(
            tx.conn(),
            "SELECT id FROM two_factor_backup_codes WHERE code_digest = ?",
            [code_digest],
            |r| r.get::<_, i64>(0),
        )?
        .is_some()
        {
            errors.add("code_digest", "has already been taken");
        }
        errors.into_result()?;
        let now = tx.now();
        let id = tx.conn().query_row_cached("INSERT INTO two_factor_backup_codes (two_factor_credential_id, code_digest, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id", params![credential_id, code_digest, now, now], |r| r.get::<_, i64>(0))?;
        query_one(
            tx.conn(),
            "SELECT * FROM two_factor_backup_codes WHERE id = ?",
            [id],
            Self::from_row,
        )?
        .or_not_found("TwoFactorBackupCode")
    }

    pub fn regenerate_set(tx: &mut Tx<'_>, credential_id: i64) -> Result<Vec<String>> {
        require_transaction(tx)?;
        let codes: Vec<_> = (0..BACKUP_CODE_COUNT)
            .map(|_| Self::generate_code())
            .collect();
        tx.conn().execute_cached(
            "DELETE FROM two_factor_backup_codes WHERE two_factor_credential_id = ?",
            [credential_id],
        )?;
        for code in &codes {
            Self::create(tx, credential_id, &Self::digest(code))?;
        }
        Ok(codes)
    }

    /// The conditional update both checks and spends the code.
    pub fn consume(tx: &mut Tx<'_>, credential_id: i64, code: &str) -> Result<bool> {
        require_transaction(tx)?;
        if blank(&Self::normalize(code)) {
            return Ok(false);
        }
        Ok(tx.conn().execute_cached("UPDATE two_factor_backup_codes SET used_at = ?, updated_at = ? WHERE two_factor_credential_id = ? AND code_digest = ? AND used_at IS NULL", params![tx.now(), tx.now(), credential_id, Self::digest(code)])? == 1)
    }
}

#[derive(Clone)]
pub struct TwoFactorSetupSecret {
    pub id: i64,
    pub session_id: i64,
    encrypted_secret: String,
    pub expires_at: Timestamp,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl TwoFactorSetupSecret {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            encrypted_secret: row.get("secret")?,
            expires_at: row.get("expires_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn issue_for(tx: &mut Tx<'_>, encryption: &ArEncryption, session_id: i64) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if query_one(
            tx.conn(),
            "SELECT id FROM sessions WHERE id = ?",
            [session_id],
            |r| r.get::<_, i64>(0),
        )?
        .is_none()
        {
            errors.add("session", "must exist");
        }
        errors.into_result()?;
        let now = tx.now();
        let encrypted =
            encryption.encrypt_with_encoding(totp::generate_secret().as_bytes(), "ASCII-8BIT");
        // Rotating a concurrent issuance keeps one row, as Rails' RecordNotUnique retry does.
        tx.conn().execute_cached("INSERT INTO two_factor_setup_secrets (session_id, secret, expires_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(session_id) DO UPDATE SET secret = excluded.secret, expires_at = excluded.expires_at, updated_at = excluded.updated_at", params![session_id, encrypted, now.since(SETUP_TTL), now, now])?;
        Self::for_session(tx.conn(), session_id)?.or_not_found("TwoFactorSetupSecret")
    }

    fn for_session(conn: &Connection, session_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM two_factor_setup_secrets WHERE session_id = ?",
            [session_id],
            Self::from_row,
        )
    }

    pub fn valid_for(conn: &Connection, session_id: i64, now: Timestamp) -> Result<Option<Self>> {
        Ok(Self::for_session(conn, session_id)?.filter(|s| !s.expired(now)))
    }

    pub fn secret(&self, encryption: &ArEncryption) -> Result<String> {
        decrypt(encryption, &self.encrypted_secret)
    }

    pub fn expired(&self, now: Timestamp) -> bool {
        self.expires_at <= now
    }

    pub fn extend_expiry(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        require_transaction(tx)?;
        self.expires_at = tx.now().since(SETUP_TTL);
        self.updated_at = tx.now();
        tx.conn().execute_cached(
            "UPDATE two_factor_setup_secrets SET expires_at = ?, updated_at = ? WHERE id = ?",
            params![self.expires_at, self.updated_at, self.id],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TwoFactorRememberedDevice {
    pub id: i64,
    pub user_id: i64,
    pub token_digest: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: Timestamp,
    pub last_used_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl TwoFactorRememberedDevice {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            token_digest: row.get("token_digest")?,
            user_agent: row.get("user_agent")?,
            ip_address: row.get("ip_address")?,
            expires_at: row.get("expires_at")?,
            last_used_at: row.get("last_used_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn digest(token: &str) -> String {
        hex::encode(Sha256::digest(token.as_bytes()))
    }

    pub fn create_for(
        tx: &mut Tx<'_>,
        user_id: i64,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(Self, String)> {
        require_transaction(tx)?;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);
        let device = Self::create(
            tx,
            user_id,
            &Self::digest(&token),
            user_agent,
            ip_address,
            tx.now().since(REMEMBER_FOR),
        )?;
        Ok((device, token))
    }

    pub fn create(
        tx: &mut Tx<'_>,
        user_id: i64,
        token_digest: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        expires_at: Timestamp,
    ) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if !user_exists(tx.conn(), user_id)? {
            errors.add("user", "must exist");
        }
        if blank(token_digest) {
            errors.add("token_digest", "can't be blank");
        }
        if query_one(
            tx.conn(),
            "SELECT id FROM two_factor_remembered_devices WHERE token_digest = ?",
            [token_digest],
            |r| r.get::<_, i64>(0),
        )?
        .is_some()
        {
            errors.add("token_digest", "has already been taken");
        }
        errors.into_result()?;
        let now = tx.now();
        let ua = user_agent.map(truncate_user_agent);
        let id: i64 = tx.conn().query_row_cached("INSERT INTO two_factor_remembered_devices (user_id, token_digest, user_agent, ip_address, expires_at, last_used_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id", params![user_id, token_digest, ua, ip_address, expires_at, now, now, now], |r| r.get(0))?;
        query_one(
            tx.conn(),
            "SELECT * FROM two_factor_remembered_devices WHERE id = ?",
            [id],
            Self::from_row,
        )?
        .or_not_found("TwoFactorRememberedDevice")
    }

    pub fn find_valid(
        tx: &mut Tx<'_>,
        token: Option<&str>,
        user_id: Option<i64>,
    ) -> Result<Option<Self>> {
        require_transaction(tx)?;
        let Some(token) = token.filter(|token| !blank(token)) else {
            return Ok(None);
        };
        let Some(user_id) = user_id else {
            return Ok(None);
        };
        let digest =
            Self::digest(token.trim_matches(|c| totp::ruby_code_whitespace(c) || c == '\0'));
        let Some(mut device) = query_one(
            tx.conn(),
            "SELECT * FROM two_factor_remembered_devices WHERE token_digest = ? AND user_id = ?",
            params![digest, user_id],
            Self::from_row,
        )?
        else {
            return Ok(None);
        };
        if device.expired(tx.now()) {
            return Ok(None);
        }
        let now = tx.now();
        tx.conn().execute_cached("UPDATE two_factor_remembered_devices SET last_used_at = ?, updated_at = ? WHERE id = ?", params![now, now, device.id])?;
        device.last_used_at = Some(now);
        device.updated_at = now;
        Ok(Some(device))
    }

    pub fn expired(&self, now: Timestamp) -> bool {
        self.expires_at <= now
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM two_factor_remembered_devices WHERE user_id = ? ORDER BY last_used_at DESC, id DESC",
            [user_id],
            Self::from_row,
        )
    }

    pub fn revoke(tx: &mut Tx<'_>, user_id: i64, id: i64) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute_cached(
            "DELETE FROM two_factor_remembered_devices WHERE user_id = ? AND id = ?",
            params![user_id, id],
        )?;
        Ok(())
    }

    pub fn revoke_all(tx: &mut Tx<'_>, user_id: i64) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute_cached(
            "DELETE FROM two_factor_remembered_devices WHERE user_id = ?",
            [user_id],
        )?;
        Ok(())
    }
}

fn truncate_user_agent(ua: &str) -> String {
    if ua.chars().count() <= 512 {
        ua.to_string()
    } else {
        format!("{}...", ua.chars().take(509).collect::<String>())
    }
}

fn blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

fn user_exists(conn: &Connection, id: i64) -> Result<bool> {
    Ok(
        query_one(conn, "SELECT id FROM users WHERE id = ?", [id], |r| {
            r.get::<_, i64>(0)
        })?
        .is_some(),
    )
}

fn decrypt(encryption: &ArEncryption, ciphertext: &str) -> Result<String> {
    encryption
        .decrypt(ciphertext)
        .map_err(|_| Error::Other("two-factor secret can't be decrypted".into()))
}

fn decrypt_totp_secret(encryption: &ArEncryption, ciphertext: &str) -> Result<String> {
    let decoded = encryption
        .decrypt_bytes(ciphertext)
        .map_err(|_| Error::Other("two-factor secret can't be decrypted".into()))?;
    // Ruby upcase only changes ASCII for ASCII-8BIT, the encoding of generated secrets.
    // Non-ASCII bytes then fail ROTP's base32 lookup rather than becoming Unicode letters.
    if decoded.encoding != "UTF-8" && !decoded.bytes.is_ascii() {
        return Err(Error::Other("invalid TOTP secret".into()));
    }
    String::from_utf8(decoded.bytes).map_err(|_| Error::Other("invalid TOTP secret".into()))
}

fn require_transaction(tx: &Tx<'_>) -> Result<()> {
    if tx.in_transaction() {
        Ok(())
    } else {
        Err(Error::Other(
            "two-factor writes require a transaction".into(),
        ))
    }
}

impl User {
    pub fn requires_two_factor(&self) -> bool {
        self.is_active() && !self.is_bot()
    }

    pub fn two_factor_enabled(&self, conn: &Connection) -> Result<bool> {
        Ok(TwoFactorCredential::for_user(conn, self.id)?.is_some_and(|c| c.enabled()))
    }

    pub fn reset_two_factor(&self, tx: &mut Tx<'_>) -> Result<()> {
        require_transaction(tx)?;
        if let Some(credential) = TwoFactorCredential::for_user(tx.conn(), self.id)? {
            credential.destroy(tx)?;
        }
        TwoFactorRememberedDevice::revoke_all(tx, self.id)
    }
}

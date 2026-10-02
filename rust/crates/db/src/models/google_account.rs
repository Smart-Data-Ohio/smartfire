//! `app/models/google_account.rb`: the optional Calendar/Drive grant, separate from identity.
use crate::{Connection, Result, Timestamp, Tx, User};
use rails_compat::{
    Secrets,
    ar_encryption::ArEncryption,
    calendar_credentials::{self, Snapshot},
};
use rusqlite::{OptionalExtension, Row, params};
pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const UNREADABLE_TOKEN_REASON: &str = "The stored token could not be read; reconnect";
#[derive(Clone)]
pub struct GoogleAccount {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    pub scopes: Option<String>,
    pub disconnected_reason: Option<String>,
    pub access_token_expires_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    access_token: Option<String>,
    refresh_token: Option<String>,
}
// Deliberately no Debug: neither encrypted nor plaintext tokens belong in logs.
pub struct ConnectionGrant {
    pub user_id: i64,
    pub email: String,
    pub access_token: Option<String>,
    pub access_token_expires_at: Option<Timestamp>,
    pub refresh_token: Option<String>,
    pub scopes: Option<String>,
}
impl GoogleAccount {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            user_id: r.get("user_id")?,
            email: r.get("email")?,
            scopes: r.get("scopes")?,
            disconnected_reason: r.get("disconnected_reason")?,
            access_token: r.get("access_token")?,
            refresh_token: r.get("refresh_token")?,
            access_token_expires_at: r.get("access_token_expires_at")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT * FROM google_accounts WHERE user_id=?",
                [user_id],
                Self::from_row,
            )
            .optional()?)
    }
    /// Calendar::PushChannel.includes(user: :google_account), in bounded find_each batches.
    pub fn for_users(conn: &Connection, user_ids: &[i64]) -> Result<Vec<Self>> {
        let mut accounts = Vec::new();
        for ids in user_ids.chunks(1000) {
            accounts.extend(crate::sql::query_all(
                conn,
                &format!(
                    "SELECT * FROM google_accounts WHERE user_id IN ({})",
                    crate::sql::placeholders(ids.len())
                ),
                rusqlite::params_from_iter(ids),
                Self::from_row,
            )?);
        }
        Ok(accounts)
    }
    pub fn connected(&self) -> bool {
        self.disconnected_reason
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
    }
    pub fn calendar(&self) -> bool {
        self.scopes
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
            || self
                .scopes
                .as_deref()
                .is_some_and(|s| s.split_ascii_whitespace().any(|s| s == CALENDAR_SCOPE))
    }
    pub fn drive(&self) -> bool {
        self.scopes
            .as_deref()
            .is_some_and(|s| s.split_ascii_whitespace().any(|s| s == DRIVE_SCOPE))
    }
    pub fn access_token(
        &self,
        enc: &ArEncryption,
    ) -> std::result::Result<Option<String>, rails_compat::ar_encryption::DecryptionError> {
        self.access_token
            .as_deref()
            .map(|s| enc.decrypt(s))
            .transpose()
    }
    pub fn refresh_token(
        &self,
        enc: &ArEncryption,
    ) -> std::result::Result<Option<String>, rails_compat::ar_encryption::DecryptionError> {
        self.refresh_token
            .as_deref()
            .map(|s| enc.decrypt(s))
            .transpose()
    }
    pub fn usable(&mut self, tx: &Tx<'_>, enc: &ArEncryption) -> Result<bool> {
        if !self.connected() {
            return Ok(false);
        }
        match self.refresh_token(enc) {
            Ok(token) => Ok(token
                .as_deref()
                .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))),
            Err(_) => {
                self.mark_disconnected(tx, UNREADABLE_TOKEN_REASON)?;
                Ok(false)
            }
        }
    }
    pub fn access_token_expired(
        &self,
        enc: &ArEncryption,
        now: Timestamp,
    ) -> std::result::Result<bool, rails_compat::ar_encryption::DecryptionError> {
        Ok(self
            .access_token(enc)?
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
            || self.access_token_expires_at.is_none_or(|at| at <= now))
    }
    pub fn cleanup_snapshot(&self, secrets: &Secrets, now: jiff::Timestamp) -> Option<String> {
        let enc = ArEncryption::new(secrets);
        let refresh_token = self.refresh_token(&enc).ok()?;
        if refresh_token
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
        {
            return None;
        }
        Some(calendar_credentials::encrypt(
            secrets,
            &Snapshot {
                access_token: self.access_token(&enc).ok()?,
                refresh_token,
                access_token_expires_at: self.access_token_expires_at.map(Timestamp::jiff),
            },
            now,
        ))
    }
    fn validate(tx: &Tx<'_>, user_id: i64, email: &str) -> Result<()> {
        let mut errors = crate::Errors::default();
        if User::find_by_id(tx.conn(), user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if campfire_richtext::ruby::is_blank(email) {
            errors.add("email", "can't be blank");
        }
        errors.into_result()
    }
    /// Callback assignment preserves an existing refresh token/scope when Google omits them.
    pub fn save_connection(
        tx: &Tx<'_>,
        enc: &ArEncryption,
        grant: ConnectionGrant,
    ) -> Result<Self> {
        Self::validate(tx, grant.user_id, &grant.email)?;
        let old = Self::for_user(tx.conn(), grant.user_id)?;
        if let Some(account) = old.as_ref() {
            let refresh_unchanged = grant
                .refresh_token
                .as_deref()
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
                .is_none_or(|s| account.refresh_token(enc).ok().flatten().as_deref() == Some(s));
            let scopes_unchanged = grant
                .scopes
                .as_deref()
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
                .is_none_or(|s| account.scopes.as_deref() == Some(s));
            if account.email == grant.email
                && account.access_token(enc).ok().as_ref() == Some(&grant.access_token)
                && account.access_token_expires_at == grant.access_token_expires_at
                && account.disconnected_reason.is_none()
                && refresh_unchanged
                && scopes_unchanged
            {
                return Ok(account.clone());
            }
        }
        let refresh = grant
            .refresh_token
            .as_deref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .map(|s| enc.encrypt(s))
            .or_else(|| old.as_ref().and_then(|a| a.refresh_token.clone()));
        let scopes = grant
            .scopes
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .or_else(|| old.as_ref().and_then(|a| a.scopes.clone()));
        let access = grant.access_token.as_deref().map(|s| enc.encrypt(s));
        let now = tx.now();
        tx.conn().execute("INSERT INTO google_accounts(user_id,email,access_token,access_token_expires_at,refresh_token,scopes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET email=excluded.email,access_token=excluded.access_token,access_token_expires_at=excluded.access_token_expires_at,refresh_token=excluded.refresh_token,scopes=excluded.scopes,disconnected_reason=NULL,updated_at=excluded.updated_at",params![grant.user_id,grant.email,access,grant.access_token_expires_at,refresh,scopes,now,now])?;
        Ok(Self::for_user(tx.conn(), grant.user_id)?.expect("saved account"))
    }
    /// A refresh replaces only access and expiry; GoogleAccount never rotates refresh here.
    pub fn refresh_access(
        &mut self,
        tx: &Tx<'_>,
        enc: &ArEncryption,
        token: Option<&str>,
        expires_at: Timestamp,
    ) -> Result<()> {
        Self::validate(tx, self.user_id, &self.email)?;
        // Active Record compares plaintext to this instance's snapshot. A delayed
        // refresh may change only expiry, so it must not overwrite a newer token.
        let access_changed =
            self.access_token(enc).ok().as_ref() != Some(&token.map(str::to_owned));
        let expiry_changed = self.access_token_expires_at != Some(expires_at);
        if !access_changed && !expiry_changed {
            return Ok(());
        }
        let now = tx.now();
        let encrypted = access_changed
            .then(|| token.map(|s| enc.encrypt(s)))
            .flatten();
        let mut columns = Vec::new();
        let mut values: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if access_changed {
            columns.push("access_token=?");
            values.push(&encrypted);
        }
        if expiry_changed {
            columns.push("access_token_expires_at=?");
            values.push(&expires_at);
        }
        if self.updated_at != now {
            columns.push("updated_at=?");
            values.push(&now);
        }
        values.push(&self.id);
        tx.conn().execute(
            &format!(
                "UPDATE google_accounts SET {} WHERE id=?",
                columns.join(",")
            ),
            rusqlite::params_from_iter(values),
        )?;
        if access_changed {
            self.access_token = encrypted;
        }
        if expiry_changed {
            self.access_token_expires_at = Some(expires_at);
        }
        self.updated_at = now;
        Ok(())
    }
    pub fn mark_disconnected(&mut self, tx: &Tx<'_>, reason: &str) -> Result<()> {
        Self::validate(tx, self.user_id, &self.email)?;
        if self.disconnected_reason.as_deref() != Some(reason) {
            let now = tx.now();
            tx.conn().execute(
                "UPDATE google_accounts SET disconnected_reason=?,updated_at=? WHERE id=?",
                params![reason, now, self.id],
            )?;
            self.disconnected_reason = Some(reason.into());
            self.updated_at = now;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewUser, tests::TestDb};
    fn grant(tx: &mut Tx<'_>) -> Result<ConnectionGrant> {
        let user = User::create(
            tx,
            NewUser {
                name: "Alice".into(),
                email_address: Some("alice@smartdata.net".into()),
                ..Default::default()
            },
        )?;
        Ok(ConnectionGrant {
            user_id: user.id,
            email: "alice@smartdata.net".into(),
            access_token: Some("FAKE-access".into()),
            access_token_expires_at: Some(tx.now()),
            refresh_token: Some("FAKE-refresh".into()),
            scopes: None,
        })
    }
    #[test]
    fn reads_rails_written_google_columns() {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../vectors/rails_compat_smartfire.json"
        ))
        .unwrap();
        let secrets = Secrets::new(v["secret_key_base"].as_str().unwrap());
        let enc = ArEncryption::new(&secrets);
        let db = TestDb::new();
        db.write(move |tx| {
            let g = grant(tx)?;
            let mut a = GoogleAccount::save_connection(tx, &enc, g)?;
            for fixture in v["ar_encryption"]["encryptions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["model"] == "GoogleAccount")
            {
                if fixture["attribute"] == "access_token" {
                    a.access_token = Some(fixture["ciphertext"].as_str().unwrap().into());
                    assert_eq!(
                        a.access_token(&enc).unwrap().as_deref(),
                        fixture["value"].as_str()
                    );
                } else {
                    a.refresh_token = Some(fixture["ciphertext"].as_str().unwrap().into());
                    assert_eq!(
                        a.refresh_token(&enc).unwrap().as_deref(),
                        fixture["value"].as_str()
                    );
                }
            }
            Ok(())
        });
    }
    #[test]
    fn credentials_are_encrypted_and_cleanup_expires_in_one_day() {
        let db = TestDb::new();
        let secrets = Secrets::new("FAKE-connection-secret");
        let enc = ArEncryption::new(&secrets);
        db.write(move |tx| {
            let grant = grant(tx)?;
            let mut account = GoogleAccount::save_connection(tx, &enc, grant)?;
            assert_eq!(
                account.refresh_token(&enc).unwrap().as_deref(),
                Some("FAKE-refresh")
            );
            assert!(account.usable(tx, &enc)?);
            assert!(account.access_token_expired(&enc, tx.now()).unwrap());
            assert!(account.refresh_token.as_deref().unwrap().starts_with('{'));
            assert!(
                !account
                    .refresh_token
                    .as_deref()
                    .unwrap()
                    .contains("FAKE-refresh")
            );
            let blob = account.cleanup_snapshot(&secrets, tx.now().jiff()).unwrap();
            let snapshot =
                calendar_credentials::decrypt_snapshot(&secrets, &blob, tx.now().jiff()).unwrap();
            assert_eq!(snapshot.refresh_token.as_deref(), Some("FAKE-refresh"));
            assert!(
                calendar_credentials::decrypt(
                    &secrets,
                    &blob,
                    tx.now().jiff() + jiff::SignedDuration::from_hours(24)
                )
                .is_none()
            );
            Ok(())
        });
    }
    #[test]
    fn unreadable_grant_disconnects_without_disclosing_tokens_and_reconnect_heals_it() {
        let db = TestDb::new();
        let enc = ArEncryption::new(&Secrets::new("FAKE-original-key"));
        let wrong = ArEncryption::new(&Secrets::new("FAKE-rotated-key"));
        db.write(move |tx| {
            let mut grant = grant(tx)?;
            let id = grant.user_id;
            let mut account = GoogleAccount::save_connection(tx, &enc, grant)?;
            assert!(!account.usable(tx, &wrong)?);
            assert_eq!(
                account.disconnected_reason.as_deref(),
                Some(UNREADABLE_TOKEN_REASON)
            );
            assert!(
                account
                    .cleanup_snapshot(&Secrets::new("FAKE-rotated-key"), tx.now().jiff())
                    .is_none()
            );
            grant = ConnectionGrant {
                user_id: id,
                email: account.email.clone(),
                access_token: Some("FAKE-new-access".into()),
                access_token_expires_at: None,
                refresh_token: Some("FAKE-new-refresh".into()),
                scopes: Some(DRIVE_SCOPE.into()),
            };
            let account = GoogleAccount::save_connection(tx, &enc, grant)?;
            assert!(account.connected());
            assert!(account.drive());
            assert!(!account.calendar());
            Ok(())
        });
    }
    #[test]
    fn omitted_refresh_scope_are_preserved_and_rows_validate_association_presence_uniqueness() {
        let db = TestDb::new();
        let enc = ArEncryption::new(&Secrets::new("FAKE-key"));
        db.write(move |tx| {
            let mut g = grant(tx)?;
            g.scopes = Some(format!("{CALENDAR_SCOPE} {DRIVE_SCOPE}"));
            let a = GoogleAccount::save_connection(tx, &enc, g)?;
            let g = ConnectionGrant {
                user_id: a.user_id,
                email: a.email.clone(),
                access_token: None,
                access_token_expires_at: None,
                refresh_token: None,
                scopes: None,
            };
            let a = GoogleAccount::save_connection(tx, &enc, g)?;
            assert_eq!(
                a.refresh_token(&enc).unwrap().as_deref(),
                Some("FAKE-refresh")
            );
            assert!(a.calendar() && a.drive());
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM google_accounts WHERE user_id=?",
                    [a.user_id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            for (id, email) in [(-1, "alice@smartdata.net"), (a.user_id, " ")] {
                let g = ConnectionGrant {
                    user_id: id,
                    email: email.into(),
                    access_token: None,
                    access_token_expires_at: None,
                    refresh_token: None,
                    scopes: None,
                };
                assert!(matches!(
                    GoogleAccount::save_connection(tx, &enc, g),
                    Err(crate::Error::RecordInvalid(_))
                ));
            }
            Ok(())
        });
    }
}

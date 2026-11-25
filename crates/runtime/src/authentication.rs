//! Transactional auth operations shared by HTML and future JSON controllers.
//! Matches `app/controllers/two_factor/{setups,challenges,backup_codes,remembered_devices}_controller.rb`.
//! No request, cookies or rendering enter this layer.
use campfire_db::models::audit_log::{Actor, AuditLog, Context, NewAuditLog, Target, pair};
use campfire_db::user::email_addresses::EmailAddress;
use campfire_db::{
    ActivityItem, ChallengeFailure, Database, PasswordDigest, Result, Session, TwoFactorBackupCode,
    TwoFactorCredential, TwoFactorRememberedDevice, TwoFactorSetupSecret, Tx, User, UserChanges,
};
use rails_compat::{ar_encryption::ArEncryption, totp};
use rusqlite::OptionalExtension;
use serde_json::json;

#[cfg(feature = "test-support")]
pub mod test_hooks {
    use std::{cell::RefCell, future::Future, pin::Pin};

    type Hook = RefCell<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>;

    tokio::task_local! {
        pub static BEFORE_ENROLLMENT_CONFIRM: Hook;
        pub static BEFORE_SESSION_WRITE: Hook;
    }

    pub async fn run(key: &'static tokio::task::LocalKey<Hook>) {
        if let Ok(Some(hook)) = key.try_with(|hook| hook.borrow_mut().take()) {
            hook.await;
        }
    }
}

/// `app/controllers/users/profiles_controller.rb`: security audits and the rollback marker
/// commit with the user update.
/// Credential changes require the original session and digest, and email submissions must still
/// agree with the address snapshot used to authorize them.
/// Returns false without writing when that authorization has gone stale.
pub fn update_profile(
    tx: &mut Tx<'_>,
    user: &mut User,
    mut changes: UserChanges,
    audit_email_change: bool,
    session_id: Option<i64>,
    legacy_raw_password_present: bool,
    context: &Context,
) -> Result<bool> {
    let writes_digest = changes.password_digest.is_some();
    let submitted_email = changes.email_address.as_ref();
    let writes_email = submitted_email.is_some_and(|email| *email != user.email_address);
    let writes_credentials = writes_email || writes_digest || audit_email_change;
    if (submitted_email.is_some() || writes_credentials)
        && !profile_write_authorized(
            tx,
            user,
            session_id,
            submitted_email.is_some(),
            writes_credentials,
        )?
    {
        return Ok(false);
    }
    let previous_digest = writes_digest.then(|| user.password_digest.clone());
    let previous_email = user.email_address.clone();
    if audit_email_change {
        changes.email_self_changed_at = Some(tx.now());
    }
    user.update(tx, changes)?;
    if audit_email_change {
        audit_primary_email_change(tx, user, previous_email.as_deref(), context)?;
    }
    // The retained form also audits non-scalar input that strong parameters discard.
    if legacy_raw_password_present
        || previous_digest.is_some_and(|digest| digest != user.password_digest)
    {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "user.password.change".into(),
                actor: Some(Actor::from(&*user)),
                target: Some(Target::from(&*user)),
                ..Default::default()
            },
            context,
        )?;
        TwoFactorRememberedDevice::revoke_all(tx, user.id)?;
    }
    Ok(true)
}

fn profile_write_authorized(
    tx: &Tx<'_>,
    user: &User,
    session_id: Option<i64>,
    check_email_snapshot: bool,
    writes_credentials: bool,
) -> Result<bool> {
    let current: Option<(Option<String>, Option<String>, bool)> = tx
        .conn()
        .query_row(
            "SELECT email_address, password_digest,
                    EXISTS(SELECT 1 FROM sessions WHERE user_id = users.id AND id = ?2)
             FROM users WHERE id = ?1 AND status = 0",
            rusqlite::params![user.id, session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    Ok(current.is_some_and(|(email, digest, has_session)| {
        (!check_email_snapshot || email == user.email_address)
            && (!writes_credentials || (has_session && digest == user.password_digest))
    }))
}

/// Recheck the session and password observed before authorizing a credential write.
pub fn credential_write_authorized(
    tx: &Tx<'_>,
    user: &User,
    session_id: Option<i64>,
) -> Result<bool> {
    profile_write_authorized(tx, user, session_id, false, true)
}

fn audit_primary_email_change(
    tx: &Tx<'_>,
    user: &User,
    previous_email: Option<&str>,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "user.email.change".into(),
            actor: Some(Actor::from(user)),
            target: Some(Target::from(user)),
            changes: Some(
                json!({"email_address": pair(json!(previous_email), json!(user.email_address))}),
            ),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

/// `Accounts::UsersController#update`: only a successful, changed role writes a row.
pub fn update_role(
    tx: &mut Tx<'_>,
    user: &mut User,
    role: campfire_db::Role,
    context: &Context,
) -> Result<()> {
    let before = user.role;
    user.update(
        tx,
        UserChanges {
            role: Some(role),
            ..Default::default()
        },
    )?;
    if user.role != before {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "user.role.change".into(),
                target: Some(Target::from(&*user)),
                changes: Some(json!({"role": pair(json!(before.name()), json!(user.role.name()))})),
                ..Default::default()
            },
            context,
        )?;
    }
    Ok(())
}

/// `Accounts::UsersController#destroy`: snapshot the label before deactivation rewrites email.
pub fn deactivate_user(tx: &mut Tx<'_>, user: &mut User, context: &Context) -> Result<()> {
    let before = user.status;
    let target = Target::from(&*user);
    user.deactivate_with_audit(tx, context)?;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "user.deactivate".into(),
            target: Some(target),
            changes: Some(json!({"status": pair(json!(before.name()), json!("deactivated"))})),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

/// `Users::BansController`: both actions accept every status; replays write no audit row.
pub fn set_user_banned(
    tx: &mut Tx<'_>,
    user: &mut User,
    banned: bool,
    context: &Context,
) -> Result<()> {
    let before = user.status;
    if banned {
        user.ban_with_audit(tx, context)?;
    } else {
        user.unban(tx)?;
    }
    if user.status != before {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: if banned { "user.ban" } else { "user.unban" }.into(),
                target: Some(Target::from(&*user)),
                changes: Some(
                    json!({"status": pair(json!(before.name()), json!(user.status.name()))}),
                ),
                ..Default::default()
            },
            context,
        )?;
    }
    Ok(())
}

/// `Authentication#start_new_session_for` and `NewSignInAlert`: the durable mail decision is
/// committed with the new session, device and inbox row. Pending first factors never call this.
pub fn start_session(
    tx: &mut Tx<'_>,
    user_id: i64,
    attributes: campfire_db::NewSession<'_>,
    notify: bool,
) -> Result<Session> {
    let session = Session::start_with(tx, user_id, attributes)?;
    if campfire_db::UserDevice::record_sign_in(
        tx,
        user_id,
        session.device_id.as_deref(),
        session.user_agent.as_deref(),
    )? == campfire_db::DeviceSignIn::NewDevice
    {
        let item = ActivityItem::refresh_unread(tx, user_id, "Session", session.id, "new_sign_in")?;
        if notify {
            campfire_mail::jobs::new_sign_in_alert_later(tx, item.id);
        }
    }
    Ok(session)
}

pub fn device_description(session: &Session) -> String {
    let ua = session.user_agent.as_deref().unwrap_or("");
    let platform = crate::concerns::platform::ApplicationPlatform::new(Some(ua));
    let browser = if ua.contains("Edg/") {
        "Edge"
    } else if platform.chrome() {
        "Chrome"
    } else if platform.firefox() {
        "Firefox"
    } else if platform.safari() {
        "Safari"
    } else {
        "Unknown browser"
    };
    let os = platform
        .operating_system()
        .filter(|s| !s.chars().all(char::is_whitespace))
        .unwrap_or("Unknown device".into());
    format!("{browser} on {os}")
}

pub fn visible_sessions(
    conn: &campfire_db::Connection,
    user: &User,
    timeout: jiff::SignedDuration,
    now: campfire_db::Timestamp,
) -> Result<Vec<Session>> {
    let mut sessions = Session::for_user(conn, user.id)?;
    sessions.retain(|s| !crate::concerns::session_expired(s, user, timeout, now));
    sessions.sort_by_key(|s| std::cmp::Reverse(s.last_active_at));
    Ok(sessions)
}

pub fn revoke_session(
    tx: &mut Tx<'_>,
    user: &User,
    session: &Session,
    context: &Context,
) -> Result<()> {
    session.destroy(tx)?;
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "session.revoke".into(),
            target: Some(Target::from(user)),
            changes: Some(json!({"revoked_session_id":session.id})),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

pub fn revoke_other_sessions(
    tx: &mut Tx<'_>,
    user: &User,
    current_id: i64,
    context: &Context,
) -> Result<usize> {
    let others = Session::for_user(tx.conn(), user.id)?
        .into_iter()
        .filter(|s| s.id != current_id)
        .collect::<Vec<_>>();
    if others.is_empty() {
        return Ok(0);
    }
    for session in &others {
        session.destroy(tx)?;
    }
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "session.revoke_others".into(),
            target: Some(Target::from(user)),
            changes: Some(json!({"count":others.len()})),
            ..Default::default()
        },
        context,
    )?;
    Ok(others.len())
}

pub fn reset_two_factor(tx: &mut Tx<'_>, user: &User, context: &Context) -> Result<()> {
    user.reset_two_factor(tx)?;
    for session in Session::for_user(tx.conn(), user.id)? {
        session.destroy(tx)?;
    }
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "two_factor.reset".into(),
            target: Some(Target::from(user)),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

/// Who can reset a password once an address has matched: active people with a password.
pub fn can_reset_password(user: &User) -> bool {
    user.is_active()
        && !user.is_bot()
        && user
            .password_digest
            .as_deref()
            .is_some_and(|digest| !campfire_richtext::ruby::is_blank(digest))
}

pub fn request_email_address_confirmation(
    tx: &mut Tx<'_>,
    user: &User,
    email_address: String,
) -> Result<()> {
    let cursor = EmailAddress::confirmation_cursor(tx.conn())?;
    campfire_mail::jobs::email_address_confirmation_later(tx, user, email_address, cursor);
    Ok(())
}

pub fn audit_email_address(
    tx: &Tx<'_>,
    user: &User,
    action: &str,
    email_address: &str,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            actor: Some(Actor::from(user)),
            target: Some(Target::from(user)),
            changes: Some(json!({ "email_address": email_address })),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

pub fn promote_email_address(
    tx: &mut Tx<'_>,
    user: &mut User,
    id: i64,
    session_id: Option<i64>,
    mail_available: bool,
    context: &Context,
) -> Result<bool> {
    if !profile_write_authorized(tx, user, session_id, true, true)? {
        return Ok(false);
    }
    let previous = user.promote_email_address(tx, id)?;
    audit_primary_email_change(tx, user, previous.as_deref(), context)?;
    if mail_available
        && let Some(address) = previous
            .as_deref()
            .and_then(campfire_mail::outbound::bare_confirmation_address)
        && EmailAddress::is_available(tx.conn(), address)?
    {
        request_email_address_confirmation(tx, user, address.to_owned())?;
    }
    campfire_db::user::profile_settings::bump_revision(tx, user.id)?;
    Ok(true)
}

/// "Forgot password?": queue the email. Delivery decides whether `email_address` gets one, so
/// the request can answer independently of whether the account exists.
pub fn request_password_reset(tx: &mut Tx<'_>, email_address: String) {
    campfire_mail::jobs::password_reset_later(tx, email_address);
}

/// A reset link's new password: saved with its audit, then every remembered device and session
/// of the user's goes, as a password change and an administrator's two-step reset do.
pub fn reset_password(
    tx: &mut Tx<'_>,
    user: &mut User,
    digest: PasswordDigest,
    context: &Context,
) -> Result<()> {
    user.update(
        tx,
        UserChanges {
            password_digest: Some(digest),
            ..Default::default()
        },
    )?;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "user.password.reset".into(),
            actor: Some(Actor::from(&*user)),
            target: Some(Target::from(&*user)),
            ..Default::default()
        },
        context,
    )?;
    TwoFactorRememberedDevice::revoke_all(tx, user.id)?;
    for session in Session::for_user(tx.conn(), user.id)? {
        session.destroy(tx)?;
    }
    user.reset_remote_connections(tx);
    Ok(())
}

pub enum Enrollment {
    SessionGone,
    Enabled,
    Wrong,
    Confirmed {
        codes: Vec<String>,
        signed_out: usize,
        session: Box<Session>,
    },
}

/// `SetupsController#create` commits each model operation separately. Confirmation checks the
/// session and consumes its setup secret atomically before enrollment issues backup codes.
pub async fn enroll(
    db: &Database,
    user: User,
    session_id: i64,
    encryption: std::sync::Arc<ArEncryption>,
    code: String,
) -> Result<Enrollment> {
    let user_id = user.id;
    let creation_encryption = encryption.clone();
    let enrollment = db
        .write(move |tx| {
            let session = match Session::find(tx.conn(), session_id) {
                Ok(session) if session.user_id == user_id => session,
                Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => return Ok(None),
                Err(error) => return Err(error),
            };
            let credential = match TwoFactorCredential::for_user(tx.conn(), user_id)? {
                Some(credential) => credential,
                None => TwoFactorCredential::create(
                    tx,
                    &creation_encryption,
                    user_id,
                    &totp::generate_secret(),
                )?,
            };
            Ok(Some((session.token, credential)))
        })
        .await?;
    let Some((session_token, credential)) = enrollment else {
        return Ok(Enrollment::SessionGone);
    };
    if credential.enabled() {
        return Ok(Enrollment::Enabled);
    }
    let credential_id = credential.id;
    #[cfg(feature = "test-support")]
    test_hooks::run(&test_hooks::BEFORE_ENROLLMENT_CONFIRM).await;
    let confirmed = db
        .write(move |tx| {
            if !Session::find_by_token(tx.conn(), &session_token)?
                .is_some_and(|session| session.id == session_id && session.user_id == user_id)
            {
                return Ok(Err(Enrollment::SessionGone));
            }
            let Some(setup) = TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())?
            else {
                return Ok(Err(Enrollment::Wrong));
            };
            let Some(mut credential) = TwoFactorCredential::for_user(tx.conn(), user_id)?
                .filter(|current| current.id == credential_id)
            else {
                return Ok(Err(Enrollment::Wrong));
            };
            if credential.enabled() {
                return Ok(Err(Enrollment::Enabled));
            }
            let confirmed = credential.confirm_with_setup_secret(tx, &encryption, &setup, &code)?;
            Ok(confirmed.then_some(()).ok_or(Enrollment::Wrong))
        })
        .await?;
    if let Err(outcome) = confirmed {
        return Ok(outcome);
    }
    let codes = db
        .write(move |tx| TwoFactorBackupCode::regenerate_set(tx, credential_id))
        .await?;
    let session = db
        .write(move |tx| {
            let mut session = Session::find(tx.conn(), session_id)?;
            session.mark_two_factor_verified(tx)?;
            Ok(session)
        })
        .await?;
    let others = db
        .read(move |conn| Session::for_user(conn, user_id))
        .await?
        .into_iter()
        .filter(|s| s.id != session_id)
        .collect::<Vec<_>>();
    let signed_out = others.len();
    for other in others {
        db.write(move |tx| other.destroy(tx)).await?;
    }
    // Authentication#disconnect_remote_connections is best-effort, after the
    // persisted enrollment steps. A realtime failure cannot undo them.
    if let Err(error) = db
        .write(move |tx| {
            user.reset_remote_connections(tx);
            Ok(())
        })
        .await
    {
        tracing::warn!(%error, "Could not disconnect remote connections");
    }
    Ok(Enrollment::Confirmed {
        codes,
        signed_out,
        session: Box::new(session),
    })
}

pub const PASSWORD_SIGN_IN: &str = "password";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordSignInProof {
    digest_proof: String,
    secondary_id: Option<i64>,
}

impl PasswordSignInProof {
    pub(crate) fn new(
        secrets: &rails_compat::Secrets,
        user: &User,
        secondary_id: Option<i64>,
    ) -> Self {
        Self {
            digest_proof: rails_compat::password_reset::sign_in_proof(
                secrets,
                user.password_digest.as_deref().unwrap_or_default(),
            ),
            secondary_id,
        }
    }

    pub(crate) fn still_valid(
        &self,
        conn: &campfire_db::Connection,
        secrets: &rails_compat::Secrets,
        user: &User,
    ) -> Result<bool> {
        if !rails_compat::password_reset::sign_in_proof_matches(
            secrets,
            &self.digest_proof,
            user.password_digest.as_deref().unwrap_or_default(),
        ) {
            return Ok(false);
        }
        if let Some(id) = self.secondary_id {
            return Ok(EmailAddress::find_owned(conn, user.id, id)?.is_some());
        }
        Ok(true)
    }
}

pub enum Challenge {
    Gone,
    Accepted {
        session: Box<Session>,
        remembered: Option<RememberedCookie>,
    },
    Wrong,
    Locked(i64),
}

pub struct RememberedCookie {
    pub token: String,
    pub expires_at: campfire_db::Timestamp,
}

pub struct ChallengeSignIn<'a> {
    pub user_id: i64,
    pub password_proof: Option<&'a PasswordSignInProof>,
    pub code: &'a str,
    pub method: &'a str,
    pub device_id: &'a str,
    pub remember_device: bool,
    pub notify_lockout: bool,
    pub notify_new_sign_in: bool,
}

/// Check the first factor before consuming the second, and commit it with the session and
/// remembered device, so a password reset can only precede or revoke the whole sign-in.
pub fn challenge(
    tx: &mut Tx<'_>,
    secrets: &rails_compat::Secrets,
    encryption: &ArEncryption,
    attempt: ChallengeSignIn<'_>,
    context: &Context,
) -> Result<Challenge> {
    let ChallengeSignIn {
        user_id,
        password_proof,
        code,
        method,
        device_id,
        remember_device,
        notify_lockout,
        notify_new_sign_in,
    } = attempt;
    let Some(user) = User::find_by_id(tx.conn(), user_id)?.filter(User::is_active) else {
        return Ok(Challenge::Gone);
    };
    if method == PASSWORD_SIGN_IN {
        let Some(proof) = password_proof else {
            return Ok(Challenge::Gone);
        };
        if !proof.still_valid(tx.conn(), secrets, &user)? {
            return Ok(Challenge::Gone);
        }
    }
    let Some(mut credential) =
        TwoFactorCredential::for_user(tx.conn(), user_id)?.filter(|v| v.enabled())
    else {
        return Ok(Challenge::Gone);
    };
    let minutes = |credential: &TwoFactorCredential, now: campfire_db::Timestamp| {
        (credential.locked_until.unwrap().as_microsecond() - now.as_microsecond() + 59_999_999)
            / 60_000_000
    };
    if credential.locked_out(tx.now()) {
        return Ok(Challenge::Locked(minutes(&credential, tx.now())));
    }
    let factor = if credential.verify_code(tx, encryption, code)? {
        Some("totp")
    } else if TwoFactorBackupCode::consume(tx, credential.id, code)? {
        Some("backup_code")
    } else {
        None
    };
    if let Some(factor) = factor {
        credential.register_challenge_success(tx)?;
        let attributes = campfire_db::NewSession {
            user_agent: context.user_agent.as_deref(),
            ip_address: context.ip_address.as_deref(),
            device_id: Some(device_id),
            two_factor_verified: true,
        };
        let session = start_session(tx, user_id, attributes, notify_new_sign_in)?;
        let remembered = if remember_device {
            let (device, token) = TwoFactorRememberedDevice::create_for(
                tx,
                user_id,
                context.user_agent.as_deref(),
                context.ip_address.as_deref(),
            )?;
            Some(RememberedCookie {
                token,
                expires_at: device.expires_at,
            })
        } else {
            None
        };
        record_sign_in(tx, &user, method, Some(factor), context)?;
        return Ok(Challenge::Accepted {
            session: Box::new(session),
            remembered,
        });
    }
    let locked = credential.register_challenge_failure(tx)? == ChallengeFailure::Locked;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "sign_in.two_factor.failure".into(),
            changes: Some(json!({"method":method})),
            ..Default::default()
        },
        context,
    )?;
    if locked {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "sign_in.two_factor.lockout".into(),
                changes: Some(json!({"method":method})),
                ..Default::default()
            },
            context,
        )?;
        let item = ActivityItem::refresh_unread(
            tx,
            user_id,
            "TwoFactorCredential",
            credential.id,
            "two_factor_lockout",
        )?;
        // Rails marks unread and touches even an already-unread existing item.
        tx.conn().execute(
            "UPDATE activity_items SET updated_at=? WHERE id=?",
            rusqlite::params![tx.now(), item.id],
        )?;
        if notify_lockout {
            campfire_mail::jobs::lockout_notice_later(tx, user_id);
        }
        Ok(Challenge::Locked(minutes(&credential, tx.now())))
    } else {
        Ok(Challenge::Wrong)
    }
}

pub fn regenerate_backups(tx: &mut Tx<'_>, user: &User, context: &Context) -> Result<Vec<String>> {
    let credential = TwoFactorCredential::for_user(tx.conn(), user.id)?
        .filter(|v| v.enabled())
        .ok_or(campfire_db::Error::RecordNotFound("TwoFactorCredential"))?;
    let codes = TwoFactorBackupCode::regenerate_set(tx, credential.id)?;
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "two_factor.backup_codes.regenerate".into(),
            target: Some(Target::from(user)),
            ..Default::default()
        },
        context,
    )?;
    Ok(codes)
}

pub fn disable(
    tx: &mut Tx<'_>,
    user: &User,
    mut session: Session,
    context: &Context,
) -> Result<Session> {
    user.reset_two_factor(tx)?;
    session.clear_two_factor_verified(tx)?;
    tx.conn().execute(
        "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
        [user.id],
    )?;
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "two_factor.disable".into(),
            target: Some(Target::from(user)),
            ..Default::default()
        },
        context,
    )?;
    Ok(session)
}

pub fn revoke_devices(
    tx: &mut Tx<'_>,
    user: &User,
    id: Option<i64>,
    all: bool,
    context: &Context,
) -> Result<()> {
    if all {
        TwoFactorRememberedDevice::revoke_all(tx, user.id)?;
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "two_factor.devices.revoke_all".into(),
                target: Some(Target::from(user)),
                ..Default::default()
            },
            context,
        )?;
    } else if let Some(id) = id {
        TwoFactorRememberedDevice::revoke(tx, user.id, id)?;
    }
    Ok(())
}

pub fn record_sign_in(
    tx: &mut Tx<'_>,
    user: &User,
    method: &str,
    factor: Option<&str>,
    context: &Context,
) -> Result<()> {
    let changes = match factor {
        Some(factor) => json!({"method": method, "two_factor":factor}),
        None => json!({"method":method}),
    };
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "session.sign_in.success".into(),
            actor: Some(Actor::from(user)),
            changes: Some(changes),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

pub fn setup_secret(tx: &mut Tx<'_>, encryption: &ArEncryption, session_id: i64) -> Result<String> {
    let setup = if let Some(mut setup) =
        TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())?
    {
        setup.extend_expiry(tx)?;
        setup
    } else {
        TwoFactorSetupSecret::issue_for(tx, encryption, session_id)?
    };
    setup.secret(encryption)
}

/// Google identity resolution, audits and the first-factor decision share the caller's write lock.
/// Tokens have already been verified outside the transaction; no request or rendering enters here.
pub fn begin_google_session(
    tx: &mut Tx<'_>,
    claims: &serde_json::Map<String, serde_json::Value>,
    attributes: campfire_db::NewSession<'_>,
    remember_token: Option<&str>,
    notify: bool,
    context: &Context,
) -> Result<(User, Option<Session>)> {
    use campfire_db::models::google_identity::{self, ResolutionKind};
    let resolution = google_identity::GoogleIdentity::resolve(tx, claims)?;
    let user = resolution.user;
    let action = match resolution.kind {
        ResolutionKind::Provisioned => Some(("user.create", json!({"method":"google"}))),
        ResolutionKind::Linked => {
            Some(("google.sign_in.link", json!({"email":claims.get("email")})))
        }
        ResolutionKind::Existing => None,
    };
    if let Some((action, changes)) = action {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: action.into(),
                actor: Some(Actor::from(&user)),
                target: Some(Target::from(&user)),
                changes: Some(changes),
                ..Default::default()
            },
            context,
        )?;
    }
    let enabled = user.two_factor_enabled(tx.conn())?;
    let remembered = enabled
        && campfire_db::TwoFactorRememberedDevice::find_valid(tx, remember_token, Some(user.id))?
            .is_some();
    let session = if enabled && !remembered {
        None
    } else {
        let session = crate::authentication::start_session(
            tx,
            user.id,
            campfire_db::NewSession {
                two_factor_verified: remembered,
                ..attributes
            },
            notify,
        )?;
        crate::authentication::record_sign_in(
            tx,
            &user,
            "google",
            remembered.then_some("remembered_device"),
            context,
        )?;
        Some(session)
    };
    Ok((user, session))
}

//! Transactional auth operations shared by HTML and future JSON controllers.
//! Matches `app/controllers/two_factor/{setups,challenges,backup_codes,remembered_devices}_controller.rb`.
//! No request, cookies or rendering enter this layer.
use campfire_db::models::audit_log::{Actor, AuditLog, Context, NewAuditLog, Target, pair};
use campfire_db::{
    ActivityItem, ChallengeFailure, Result, Session, TwoFactorBackupCode, TwoFactorCredential,
    TwoFactorRememberedDevice, TwoFactorSetupSecret, Tx, User, UserChanges,
};
use rails_compat::{ar_encryption::ArEncryption, totp};
use serde_json::json;

/// `app/controllers/users/profiles_controller.rb`: security audits and the rollback marker
/// commit with the user update.
/// The controller checks the existing password before constructing these changes.
pub fn update_profile(
    tx: &mut Tx<'_>,
    user: &mut User,
    mut changes: UserChanges,
    email_changing: bool,
    password_changing: bool,
    context: &Context,
) -> Result<()> {
    let previous_email = user.email_address.clone();
    if email_changing {
        changes.email_self_changed_at = Some(tx.now());
    }
    user.update(tx, changes)?;
    if email_changing {
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "user.email.change".into(),
                actor: Some(Actor::from(&*user)),
                target: Some(Target::from(&*user)),
                changes: Some(
                    json!({"email_address": pair(json!(previous_email), json!(user.email_address))}),
                ),
                ..Default::default()
            },
            context,
        )?;
    }
    if password_changing {
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
    user.deactivate(tx)?;
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
        user.ban(tx)?;
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

pub enum Enrollment {
    Enabled,
    Wrong,
    Confirmed {
        codes: Vec<String>,
        signed_out: usize,
        session: Box<Session>,
    },
}

pub fn enroll(
    tx: &mut Tx<'_>,
    user: &User,
    session_id: i64,
    encryption: &ArEncryption,
    code: &str,
    context: &Context,
) -> Result<Enrollment> {
    let mut credential = match TwoFactorCredential::for_user(tx.conn(), user.id)? {
        Some(credential) if credential.enabled() => return Ok(Enrollment::Enabled),
        Some(credential) => credential,
        None => TwoFactorCredential::create(tx, encryption, user.id, &totp::generate_secret())?,
    };
    let Some(setup) = TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())? else {
        return Ok(Enrollment::Wrong);
    };
    if !credential.confirm_with_setup_secret(tx, encryption, &setup, code)? {
        return Ok(Enrollment::Wrong);
    }
    let codes = TwoFactorBackupCode::regenerate_set(tx, credential.id)?;
    let mut session = Session::find(tx.conn(), session_id)?;
    session.mark_two_factor_verified(tx)?;
    let others = Session::for_user(tx.conn(), user.id)?
        .into_iter()
        .filter(|s| s.id != session_id)
        .collect::<Vec<_>>();
    for other in &others {
        other.destroy(tx)?;
    }
    user.reset_remote_connections(tx);
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "two_factor.enable".into(),
            target: Some(Target::from(user)),
            changes: (!others.is_empty()).then(|| json!({"signed_out_other_devices":others.len()})),
            ..Default::default()
        },
        context,
    )?;
    Ok(Enrollment::Confirmed {
        codes,
        signed_out: others.len(),
        session: Box::new(session),
    })
}

pub enum Challenge {
    Gone,
    Accepted(&'static str),
    Wrong,
    Locked(i64),
}

pub fn challenge(
    tx: &mut Tx<'_>,
    user_id: i64,
    encryption: &ArEncryption,
    code: &str,
    method: &str,
    context: &Context,
    notify: bool,
) -> Result<Challenge> {
    if campfire_db::User::find_by_id(tx.conn(), user_id)?.is_none_or(|u| !u.is_active()) {
        return Ok(Challenge::Gone);
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
        return Ok(Challenge::Accepted(factor));
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
        if notify {
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
    tx: &mut Tx<'_>, claims: &serde_json::Map<String,serde_json::Value>,
    attributes: campfire_db::NewSession<'_>, remember_token: Option<&str>, notify: bool,
    context: &Context,
) -> Result<(User,Option<Session>)> {
    use campfire_db::models::google_identity::{self,ResolutionKind};
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
                && campfire_db::TwoFactorRememberedDevice::find_valid(
                    tx,
                    remember_token,
                    Some(user.id),
                )?
                .is_some();
            let session = if enabled && !remembered {
                None
            } else {
                let session = crate::authentication::start_session(
                    tx,
                    user.id,
                    campfire_db::NewSession { two_factor_verified: remembered, ..attributes },
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

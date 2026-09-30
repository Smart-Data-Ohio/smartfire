//! Audited mutations in AccountsController and Accounts::{Users,CustomStyles,JoinCodes,Logos}.
use super::audit_log::{AuditLog, Context, NewAuditLog, Target, pair};
use crate::{Account, Result, Role, Tx, User, UserChanges};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub struct SettingsSnapshot {
    name: String,
    restrict: bool,
    logo: bool,
}
impl SettingsSnapshot {
    pub fn take(tx: &Tx<'_>, account: &Account) -> Result<Self> {
        Ok(Self {
            name: account.name.clone(),
            restrict: account
                .settings()
                .restrict_room_creation_to_administrators(),
            logo: logo_attached(tx, account.id)?,
        })
    }
}
fn logo_attached(tx: &Tx<'_>, account: i64) -> Result<bool> {
    Ok(tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='Account' AND record_id=? AND name='logo')",[account],|r|r.get(0))?)
}
pub fn record_settings_changes(
    tx: &Tx<'_>,
    account: &Account,
    before: SettingsSnapshot,
    context: &Context,
) -> Result<()> {
    let mut changes = Map::new();
    if account.name != before.name {
        changes.insert("name".into(), pair(json!(before.name), json!(account.name)));
    }
    let restrict = account
        .settings()
        .restrict_room_creation_to_administrators();
    if restrict != before.restrict {
        changes.insert(
            "restrict_room_creation_to_administrators".into(),
            pair(json!(before.restrict), json!(restrict)),
        );
    }
    let logo = logo_attached(tx, account.id)?;
    if logo != before.logo {
        changes.insert("logo".into(), pair(json!(before.logo), json!(logo)));
    }
    if !changes.is_empty() {
        record(
            tx,
            "account.settings.change",
            Target::from(account),
            Some(Value::Object(changes)),
            context,
        )?;
    }
    Ok(())
}
pub fn change_custom_styles(
    tx: &mut Tx<'_>,
    account: &mut Account,
    styles: Option<Option<&str>>,
    context: &Context,
) -> Result<()> {
    let before = account.custom_styles.clone();
    account.update(tx, None, styles, None)?;
    if account.custom_styles != before {
        record(
            tx,
            "account.custom_styles.change",
            Target::from(&*account),
            Some(
                json!({"custom_styles":pair(style_summary(before.as_deref()),style_summary(account.custom_styles.as_deref()))}),
            ),
            context,
        )?;
    }
    Ok(())
}
fn style_summary(styles: Option<&str>) -> Value {
    let styles = styles.unwrap_or("");
    json!({"size":styles.len(),"digest":&hex::encode(Sha256::digest(styles.as_bytes()))[..12]})
}
pub fn reset_join_code(tx: &mut Tx<'_>, account: &mut Account, context: &Context) -> Result<()> {
    account.reset_join_code(tx)?;
    record(
        tx,
        "account.join_code.reset",
        Target::from(&*account),
        None,
        context,
    )
}
pub fn record_logo_destroy(tx: &Tx<'_>, account: &Account, context: &Context) -> Result<()> {
    record(
        tx,
        "account.settings.change",
        Target::from(account),
        Some(json!({"logo":pair(json!(true),json!(false))})),
        context,
    )
}
pub fn change_role(tx: &mut Tx<'_>, user: &mut User, role: Role, context: &Context) -> Result<()> {
    let before = user.role;
    let saved =
        super::user::profile_settings::update(tx, user.id, Default::default()).and_then(|()| {
            user.update(
                tx,
                UserChanges {
                    role: Some(role),
                    ..Default::default()
                },
            )
        });
    match saved {
        // Rails redirects after an unsuccessful `update`, without recording a role change.
        Err(crate::Error::RecordInvalid(_)) => return Ok(()),
        Err(error) => return Err(error),
        Ok(()) => (),
    }
    if user.role != before {
        record(
            tx,
            "user.role.change",
            Target::from(&*user),
            Some(json!({"role":pair(json!(before.name()),json!(user.role.name()))})),
            context,
        )?;
    }
    Ok(())
}
pub fn deactivate(tx: &mut Tx<'_>, user: &mut User, context: &Context) -> Result<()> {
    let before = user.status.name();
    let target = Target::from(&*user);
    user.deactivate(tx)?;
    record(
        tx,
        "user.deactivate",
        target,
        Some(json!({"status":pair(json!(before),json!("deactivated"))})),
        context,
    )
}
pub fn change_ban(tx: &mut Tx<'_>, user: &mut User, ban: bool, context: &Context) -> Result<()> {
    let before = user.status;
    super::user::profile_settings::update(tx, user.id, Default::default())?;
    if ban {
        user.ban(tx)?;
    } else {
        user.unban(tx)?;
    }
    if user.status != before {
        record(
            tx,
            if ban { "user.ban" } else { "user.unban" },
            Target::from(&*user),
            Some(json!({"status":pair(json!(before.name()),json!(user.status.name()))})),
            context,
        )?;
    }
    Ok(())
}
fn record(
    tx: &Tx<'_>,
    action: &str,
    target: Target,
    changes: Option<Value>,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(target),
            changes,
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

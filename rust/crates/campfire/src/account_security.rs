//! Successful account mutations audited by app/controllers/accounts{,/*}_controller.rb.
use campfire_db::models::audit_log::{AuditLog, Context, NewAuditLog, Target};
use campfire_db::{Account, Result, Tx};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

fn record(
    tx: &Tx<'_>,
    account: &Account,
    action: &str,
    changes: Value,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target::from(account)),
            changes: Some(changes),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

pub fn settings_changed(
    tx: &Tx<'_>,
    before: &Account,
    after: &Account,
    before_logo: bool,
    after_logo: bool,
    context: &Context,
) -> Result<()> {
    let mut changes = Map::new();
    if before.name != after.name {
        changes.insert(
            "name".into(),
            json!({"before":before.name,"after":after.name}),
        );
    }
    let previous = before.settings().restrict_room_creation_to_administrators();
    let current = after.settings().restrict_room_creation_to_administrators();
    if previous != current {
        changes.insert(
            "restrict_room_creation_to_administrators".into(),
            json!({"before":previous,"after":current}),
        );
    }
    if before_logo != after_logo {
        changes.insert(
            "logo".into(),
            json!({"before":before_logo,"after":after_logo}),
        );
    }
    if !changes.is_empty() {
        record(
            tx,
            after,
            "account.settings.change",
            Value::Object(changes),
            context,
        )?;
    }
    Ok(())
}

pub fn reset_join_code(tx: &mut Tx<'_>, account: &mut Account, context: &Context) -> Result<()> {
    account.reset_join_code(tx)?;
    record(tx, account, "account.join_code.reset", json!({}), context)
}

/// Rails AuditLog::DIGEST_PREFIX_LENGTH is 12; sizes count UTF-8 bytes, not characters.
fn style_summary(styles: Option<&str>) -> Value {
    let styles = styles.unwrap_or_default();
    let digest = format!("{:x}", Sha256::digest(styles.as_bytes()));
    json!({"size":styles.len(),"digest":&digest[..12]})
}
pub fn update_styles(
    tx: &mut Tx<'_>,
    account: &mut Account,
    styles: Option<Option<&str>>,
    context: &Context,
) -> Result<()> {
    let before = account.custom_styles.clone();
    account.update(tx, None, styles, None)?;
    if before != account.custom_styles {
        record(
            tx,
            account,
            "account.custom_styles.change",
            json!({"custom_styles":{
                "before":style_summary(before.as_deref()),"after":style_summary(account.custom_styles.as_deref())
            }}),
            context,
        )?;
    }
    Ok(())
}

pub fn logo_removed(tx: &Tx<'_>, account: &Account, context: &Context) -> Result<()> {
    // Accounts::LogosController records this pair even when no attachment existed.
    record(
        tx,
        account,
        "account.settings.change",
        json!({"logo":{"before":true,"after":false}}),
        context,
    )
}

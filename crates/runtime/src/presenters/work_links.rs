//! Credentialed read facts for work links.
use crate::app::AppCtx;
use crate::concerns::require_current_user;
use campfire_kit::Ctx;
use campfire_db::models::google_account::GoogleAccount;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::Value;

/// Best-effort title lookup through the viewer's usable, Drive-enabled Google account.
pub async fn resolve_drive_title(c: &Ctx, file_id: &str) -> Option<String> {
    let api = c.app().google.api();
    if !api.config.configured() {
        return None;
    }
    let user_id = require_current_user(c).ok()?.id;
    let enc = ArEncryption::new(&c.app().secrets);
    let account = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut account) = GoogleAccount::for_user(tx.conn(), user_id)? else {
                return Ok(None);
            };
            Ok((account.usable(tx, &enc)? && account.drive()).then_some(account))
        })
        .await
        .ok()
        .flatten()?;
    let file = api
        .drive_file(
            &c.app().db,
            &c.app().secrets,
            account.user_id,
            file_id,
            campfire_db::Timestamp::from_jiff(c.now()),
        )
        .await
        .ok()?;
    let value = file.get("name")?;
    let title = match value {
        Value::Null | Value::Bool(false) => return None,
        Value::Bool(true) => "t".into(),
        Value::Array(items) if items.is_empty() => return None,
        Value::Object(items) if items.is_empty() => return None,
        value => campfire_richtext::ruby::json_value_to_s(value),
    };
    (!campfire_richtext::ruby::is_blank(&title)).then_some(title)
}

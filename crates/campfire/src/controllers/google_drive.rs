//! Viewer metadata/search and authorized room recipient snapshots, separate from attachments.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::google::api,
};
use campfire_db::{
    Status, Timestamp,
    models::{drive_recipients, google_account::GoogleAccount, google_drive_link::valid_id},
};
use campfire_kit::{Ctx, Error, Result, StatusCode, halt};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
fn now(c: &Ctx) -> Timestamp {
    Timestamp::from_jiff(c.now())
}
async fn account(c: &Ctx) -> Result<Option<GoogleAccount>> {
    let Some(user) = concerns::current_user(c) else {
        return Ok(None);
    };
    let id = user.id;
    let enc = ArEncryption::new(&c.app().secrets);
    c.app()
        .db
        .write(move |tx| {
            let Some(mut a) = GoogleAccount::for_user(tx.conn(), id)? else {
                return Ok(None);
            };
            Ok((a.usable(tx, &enc)? && a.drive()).then_some(a))
        })
        .await
        .map_err(Error::internal)
}
fn file_json(file: &Value) -> Result<Value> {
    if file.is_null() {
        // Rails file_json indexes nil outside the Google error rescue.
        return Err(Error::Status(StatusCode::INTERNAL_SERVER_ERROR));
    }
    let kind = match file["mimeType"].as_str() {
        Some("application/vnd.google-apps.document") => "document",
        Some("application/vnd.google-apps.spreadsheet") => "spreadsheet",
        Some("application/vnd.google-apps.presentation") => "presentation",
        Some("application/vnd.google-apps.form") => "form",
        Some("application/vnd.google-apps.folder") => "folder",
        Some("application/pdf") => "pdf",
        _ => "file",
    };
    Ok(
        json!({"id":file["id"],"name":file["name"],"kind":kind,"modified_at":file["modifiedTime"],"owner":file["owners"].get(0).map(|o|&o["displayName"]),"url":file["webViewLink"]}),
    )
}
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = c.param_str("id").unwrap_or_default().to_owned();
    if !c.app().google.api().config.configured() || !valid_id(&json!(id)) {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let Some(account) = account(c).await? else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    if c.app()
        .google
        .drive()
        .throttled("show", account.user_id, 60, now(c))
    {
        return c.json(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error":"rate_limited"}),
        );
    }
    let cached = c.app().google.drive().cached(account.user_id, &id, now(c));
    let fresh = cached.is_none();
    let result = match cached {
        Some(v) => Ok(v),
        None => {
            c.app()
                .google
                .api()
                .drive_file(&c.app().db, &c.app().secrets, account.user_id, &id, now(c))
                .await
        }
    };
    match result {
        Ok(file) => {
            if fresh {
                c.app()
                    .google
                    .drive()
                    .cache(account.user_id, &id, now(c), file.clone());
            }
            c.json(StatusCode::OK, &file_json(&file)?)
        }
        Err(api::Error::NotFound(_) | api::Error::Unauthorized(_)) => {
            Ok(c.head(StatusCode::NOT_FOUND))
        }
        Err(api::Error::Storage(e)) => Err(Error::internal(e)),
        Err(_) => Ok(c.head(StatusCode::SERVICE_UNAVAILABLE)),
    }
}
pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    concerns::restore_authentication(c).await?;
    if !c.app().google.api().config.configured() {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let Some(account) = account(c).await? else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    if c.app()
        .google
        .drive()
        .throttled("list", account.user_id, 30, now(c))
    {
        return c.json(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error":"rate_limited"}),
        );
    }
    let q = c
        .params
        .get("q")
        .map(|p| campfire_richtext::ruby::json_value_to_s(&p.to_json()))
        .unwrap_or_default();
    let q = campfire_richtext::ruby::strip(&q)
        .chars()
        .take(100)
        .collect::<String>();
    match c
        .app()
        .google
        .api()
        .list_drive_files(&c.app().db, &c.app().secrets, account.user_id, &q, now(c))
        .await
    {
        Ok(result) => {
            if result.is_null() {
                return Err(Error::Status(StatusCode::INTERNAL_SERVER_ERROR));
            }
            let files = result["files"]
                .as_array()
                .map(|a| a.iter().map(file_json).collect::<Result<Vec<_>>>())
                .transpose()?
                .unwrap_or_default();
            c.json(StatusCode::OK, &json!({"files":files}))
        }
        Err(api::Error::NotFound(_) | api::Error::Unauthorized(_)) => {
            Ok(c.head(StatusCode::NOT_FOUND))
        }
        Err(api::Error::Storage(e)) => Err(Error::internal(e)),
        Err(_) => c.json(
            StatusCode::BAD_GATEWAY,
            &json!({"error":"drive_unavailable"}),
        ),
    }
}
async fn recipients_before(c: &mut Ctx) -> Result<i64> {
    // Restore first so the JSON authentication override returns Rails' empty 401.
    concerns::restore_authentication(c).await?;
    if concerns::current_user(c).is_none()
        && !c.params.get("bot_key").is_some_and(|p| p.is_present())
        && concerns::agent_bearer_secret(c.request.header("authorization")).is_none()
        && c.format()?.is_some_and(|f| f.is("json"))
    {
        return halt(c.head(StatusCode::UNAUTHORIZED));
    }
    concerns::before_actions(c, Before::default()).await?;
    let (_, room) = match concerns::set_room(c).await {
        Ok(room) => room,
        Err(Error::NotFound) => return halt(c.head(StatusCode::NOT_FOUND)),
        Err(e) => return Err(e),
    };
    if room.deleted_at.is_some() {
        return halt(c.head(StatusCode::NOT_FOUND));
    }
    if !c.app().google.drive().picker_configured() {
        return halt(concerns::head(StatusCode::NOT_FOUND));
    }
    let user = concerns::require_current_user(c)?;
    let id = user.id;
    let agent = c
        .app()
        .db
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE user_id=?)",
                [id],
                |r| r.get::<_, bool>(0),
            )?)
        })
        .await
        .map_err(Error::internal)?;
    if user.status != Status::Active || user.is_bot() || agent {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    if c.app()
        .google
        .drive()
        .throttled("recipients", id, 60, now(c))
    {
        let response = c.json(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error":"rate_limited"}),
        )?;
        return halt(response);
    }
    Ok(room.id)
}
pub async fn recipients(c: &mut Ctx) -> Result {
    let room = recipients_before(c).await?;
    let id = concerns::require_current_user(c)?.id;
    let members = c
        .app()
        .db
        .read(move |conn| drive_recipients::eligible(conn, room, id))
        .await
        .map_err(Error::internal)?;
    c.set_header("cache-control", "no-store");
    c.json(StatusCode::OK, &json!({"recipients":members}))
}
pub async fn validate_recipients(c: &mut Ctx) -> Result {
    let room = recipients_before(c).await?;
    let raw = c
        .params
        .get("user_ids")
        .map(|p| p.to_json())
        .unwrap_or(Value::Null);
    let Some(ids) = drive_recipients::normalized_ids(&raw) else {
        return c.json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({"error":"invalid_recipients"}),
        );
    };
    if ids.len() > 100 {
        return c.json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({"error":"too_many_recipients","limit":100}),
        );
    }
    let id = concerns::require_current_user(c)?.id;
    let members = c
        .app()
        .db
        .read(move |conn| drive_recipients::eligible(conn, room, id))
        .await
        .map_err(Error::internal)?;
    let invalid = ids
        .iter()
        .filter(|id| {
            !members
                .iter()
                .any(|m| u64::try_from(m.id).ok() == Some(**id))
        })
        .collect::<Vec<_>>();
    if !invalid.is_empty() {
        return c.json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({"error":"invalid_recipients","invalid_ids":invalid}),
        );
    }
    let selected = ids
        .iter()
        .filter_map(|id| {
            members
                .iter()
                .find(|m| u64::try_from(m.id).ok() == Some(*id))
        })
        .collect::<Vec<_>>();
    c.set_header("cache-control", "no-store");
    c.json(StatusCode::OK, &json!({"recipients":selected}))
}

//! Drive search, per-viewer file metadata, and the room share/grant flow
//! (`Google::DriveFilesController`, `Rooms::DriveRecipientsController`).
//!
//! Search and show use the viewer's stored Drive grant and the same cache and minute budgets as
//! classic. Recipients do not need a Drive grant. Sharing grants each chosen member reader access
//! with email notifications off, then the composer attaches the file.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::integrations::google::api::Error as GoogleError;
use campfire_db::models::{
    drive_recipients, google_account::GoogleAccount, google_drive_link::valid_id,
};
use campfire_db::{Status, Timestamp};
use campfire_kit::{Ctx, Error, Result, StatusCode, halt};
use campfire_web::concerns;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};

use crate::endpoints::{before_actions, body, set_room};
use crate::error::{fail, validation};

endpoint!(
    /// `GET /api/v1/drive/picker`
    picker => picker_config
);
endpoint!(
    /// `GET /api/v1/drive/files`
    search => search_files
);
endpoint!(
    /// `GET /api/v1/drive/files/:file_id`
    show => show_file
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/drive/recipients`
    recipients => list_recipients
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/drive/recipients/validate`
    validate_recipients => validate_room_recipients
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/drive/shares`
    share => share_file
);

fn now(c: &Ctx) -> Timestamp {
    Timestamp::from_jiff(c.now())
}

fn empty(c: &mut Ctx, status: StatusCode) -> Result {
    Ok(c.head(status))
}

fn drive_error(c: &mut Ctx, status: StatusCode, error: &str) -> Result {
    c.json(status, &json!({"error": error}))
}

/// A usable Drive grant for the viewer, or `None` when they have none (classic `account`).
async fn drive_account(c: &Ctx) -> Result<Option<GoogleAccount>> {
    let Some(user) = concerns::current_user(c) else {
        return Ok(None);
    };
    let id = user.id;
    let enc = ArEncryption::new(&c.app().secrets);
    c.app()
        .db
        .write(move |tx| {
            let Some(mut account) = GoogleAccount::for_user(tx.conn(), id)? else {
                return Ok(None);
            };
            Ok((account.usable(tx, &enc)? && account.drive()).then_some(account))
        })
        .await
        .map_err(Error::internal)
}

fn kind_of(mime: Option<&str>) -> &'static str {
    match mime {
        Some("application/vnd.google-apps.document") => "document",
        Some("application/vnd.google-apps.spreadsheet") => "spreadsheet",
        Some("application/vnd.google-apps.presentation") => "presentation",
        Some("application/vnd.google-apps.form") => "form",
        Some("application/vnd.google-apps.folder") => "folder",
        Some("application/vnd.google-apps.shortcut") => "shortcut",
        Some("application/pdf") => "pdf",
        _ => "file",
    }
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// Classic `file_json`. A null file is a 500, outside the Google error rescue.
fn file_of(file: &Value) -> Result<api::DriveFile> {
    if file.is_null() {
        return Err(Error::Status(StatusCode::INTERNAL_SERVER_ERROR));
    }
    Ok(api::DriveFile {
        id: text(&file["id"]),
        name: text(&file["name"]),
        kind: kind_of(file["mimeType"].as_str()).into(),
        modified_at: text(&file["modifiedTime"]),
        owner: file["owners"]
            .get(0)
            .and_then(|owner| text(&owner["displayName"])),
        url: text(&file["webViewLink"]),
    })
}

fn query_term(c: &Ctx) -> String {
    let raw = c.param_str("q").unwrap_or_default();
    campfire_richtext::ruby::strip(raw)
        .chars()
        .take(100)
        .collect()
}

async fn picker_config(c: &mut Ctx) -> Result {
    concerns::before_actions(
        c,
        concerns::Before::default().allow_unauthenticated_access(),
    )
    .await?;
    let _ = concerns::restore_authentication(c).await?;
    c.set_header("Cache-Control", "no-store");
    let Some(config) = c.app().config.google_picker.clone() else {
        return empty(c, StatusCode::NOT_FOUND);
    };
    let Some(account) = drive_account(c).await? else {
        return empty(c, StatusCode::NOT_FOUND);
    };
    c.json(
        StatusCode::OK,
        &api::DrivePickerConfig {
            client_id: config.client_id,
            api_key: config.api_key,
            project_number: config.project_number,
            account_email: (!account.email.trim().is_empty()).then_some(account.email),
        },
    )
}

async fn search_files(c: &mut Ctx) -> Result {
    // Classic `index` allows a signed-out caller, then answers the same empty 404 as a viewer
    // with no Drive grant. The other Drive routes stay on the API's JSON 401.
    concerns::before_actions(
        c,
        concerns::Before::default().allow_unauthenticated_access(),
    )
    .await?;
    let _ = concerns::restore_authentication(c).await?;
    if !c.app().google.api().config.configured() {
        return empty(c, StatusCode::NOT_FOUND);
    }
    let Some(account) = drive_account(c).await? else {
        return empty(c, StatusCode::NOT_FOUND);
    };
    if c.app()
        .google
        .drive()
        .throttled("list", account.user_id, 30, now(c))
    {
        return drive_error(c, StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let term = query_term(c);
    match c
        .app()
        .google
        .api()
        .list_drive_files(
            &c.app().db,
            &c.app().secrets,
            account.user_id,
            &term,
            now(c),
        )
        .await
    {
        Ok(result) => {
            if result.is_null() {
                return Err(Error::Status(StatusCode::INTERNAL_SERVER_ERROR));
            }
            let files = result["files"]
                .as_array()
                .map(|files| files.iter().map(file_of).collect::<Result<Vec<_>>>())
                .transpose()?
                .unwrap_or_default();
            c.json(StatusCode::OK, &api::DriveFileList { files })
        }
        Err(GoogleError::NotFound(_) | GoogleError::Unauthorized(_)) => {
            empty(c, StatusCode::NOT_FOUND)
        }
        Err(GoogleError::Storage(error)) => Err(Error::internal(error)),
        Err(_) => drive_error(c, StatusCode::BAD_GATEWAY, "drive_unavailable"),
    }
}

async fn show_file(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let id = c.param_str("file_id").unwrap_or_default().to_owned();
    if !c.app().google.api().config.configured() || !valid_id(&json!(id)) {
        return empty(c, StatusCode::NOT_FOUND);
    }
    let Some(account) = drive_account(c).await? else {
        return empty(c, StatusCode::NOT_FOUND);
    };
    if c.app()
        .google
        .drive()
        .throttled("show", account.user_id, 60, now(c))
    {
        return drive_error(c, StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let cached = c.app().google.drive().cached(account.user_id, &id, now(c));
    let fresh = cached.is_none();
    let result = match cached {
        Some(file) => Ok(file),
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
            c.json(StatusCode::OK, &file_of(&file)?)
        }
        Err(GoogleError::NotFound(_) | GoogleError::Unauthorized(_)) => {
            empty(c, StatusCode::NOT_FOUND)
        }
        Err(GoogleError::Storage(error)) => Err(Error::internal(error)),
        Err(_) => empty(c, StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Classic `recipients_before`: a human member of a live room, and the picker configured.
async fn room_for_share(c: &mut Ctx, budget: &'static str) -> Result<i64> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if !c.app().google.drive().picker_configured() {
        return halt(c.head(StatusCode::NOT_FOUND));
    }
    let user = concerns::require_current_user(c)?;
    let id = user.id;
    if user.status != Status::Active || user.is_bot() {
        return halt(c.head(StatusCode::FORBIDDEN));
    }
    let agent = c
        .app()
        .db
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE user_id=?)",
                [id],
                |row| row.get::<_, bool>(0),
            )?)
        })
        .await
        .map_err(Error::internal)?;
    if agent {
        return halt(c.head(StatusCode::FORBIDDEN));
    }
    if c.app().google.drive().throttled(budget, id, 60, now(c)) {
        let response = c.json(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error": "rate_limited"}),
        )?;
        return halt(response);
    }
    Ok(room.id)
}

fn recipient_of(member: &drive_recipients::Recipient) -> api::DriveRecipient {
    api::DriveRecipient {
        id: member.id,
        name: member.name.clone(),
        email: member.email.clone(),
    }
}

async fn eligible(c: &Ctx, room: i64, viewer: i64) -> Result<Vec<drive_recipients::Recipient>> {
    c.app()
        .db
        .read(move |conn| drive_recipients::eligible(conn, room, viewer))
        .await
        .map_err(Error::internal)
}

async fn list_recipients(c: &mut Ctx) -> Result {
    let room = room_for_share(c, "recipients").await?;
    let viewer = concerns::require_current_user(c)?.id;
    let recipients = eligible(c, room, viewer)
        .await?
        .iter()
        .map(recipient_of)
        .collect();
    c.json(StatusCode::OK, &api::DriveRecipientList { recipients })
}

fn invalid_recipients(c: &mut Ctx, field: &str, invalid: Option<Vec<u64>>) -> Error {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(
        field.into(),
        match &invalid {
            Some(ids) => ids.iter().map(ToString::to_string).collect(),
            None => vec!["invalid_recipients".into()],
        },
    );
    fail(
        c,
        api::ApiError::Validation {
            message: "invalid_recipients".into(),
            fields,
        },
    )
}

/// Classic `normalized_ids` over the SPA's string ids, then the membership check.
fn normalized_user_ids(ids: &[String]) -> Option<Vec<u64>> {
    let raw = Value::Array(ids.iter().cloned().map(Value::String).collect());
    drive_recipients::normalized_ids(&raw)
}

async fn checked_recipients(
    c: &mut Ctx,
    room: i64,
    raw_ids: &[String],
) -> Result<Vec<drive_recipients::Recipient>> {
    let Some(ids) = normalized_user_ids(raw_ids) else {
        return Err(invalid_recipients(c, "userIds", None));
    };
    if ids.len() > 100 {
        return Err(fail(c, validation("userIds", "too_many_recipients")));
    }
    let viewer = concerns::require_current_user(c)?.id;
    let members = eligible(c, room, viewer).await?;
    let invalid = ids
        .iter()
        .copied()
        .filter(|id| {
            !members
                .iter()
                .any(|member| u64::try_from(member.id).ok() == Some(*id))
        })
        .collect::<Vec<_>>();
    if !invalid.is_empty() {
        return Err(invalid_recipients(c, "userIds", Some(invalid)));
    }
    Ok(ids
        .iter()
        .filter_map(|id| {
            members
                .iter()
                .find(|member| u64::try_from(member.id).ok() == Some(*id))
                .cloned()
        })
        .collect())
}

async fn validate_room_recipients(c: &mut Ctx) -> Result {
    let room = room_for_share(c, "recipients").await?;
    let input: api::ValidateDriveRecipients = body(c).await?;
    let selected = checked_recipients(c, room, &input.user_ids).await?;
    c.json(
        StatusCode::OK,
        &api::DriveRecipientList {
            recipients: selected.iter().map(recipient_of).collect(),
        },
    )
}

const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const SHORTCUT_MIME: &str = "application/vnd.google-apps.shortcut";

struct Approved {
    id: u64,
    email: String,
}

/// Ids and the emails the viewer approved, in request order. A non-digit id is malformed.
fn approved_rows(raw: &[api::DriveApprovedRecipient]) -> Option<Vec<Approved>> {
    let mut rows = Vec::new();
    for entry in raw {
        let text = campfire_richtext::ruby::strip(&entry.id);
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let id = text.parse().ok()?;
        if rows.iter().any(|row: &Approved| row.id == id) {
            continue;
        }
        rows.push(Approved {
            id,
            email: entry.email.clone(),
        });
    }
    Some(rows)
}

fn share_result(
    member: &drive_recipients::Recipient,
    status: &str,
    reason: Option<&str>,
) -> api::DriveShareResult {
    api::DriveShareResult {
        recipient: recipient_of(member),
        status: status.into(),
        reason: reason.map(str::to_owned),
    }
}

fn finished(
    c: &mut Ctx,
    file_id: String,
    outcome: &str,
    blocked: Option<&str>,
    changed_ids: Vec<i64>,
    recipients: Vec<api::DriveRecipient>,
    results: Vec<api::DriveShareResult>,
) -> Result {
    c.json(
        StatusCode::OK,
        &api::DriveShare {
            outcome: outcome.into(),
            file_id,
            blocked: blocked.map(str::to_owned),
            changed_ids,
            recipients,
            results,
        },
    )
}

/// Why a share write failed, in the classic dialog's words.
fn grant_reason(error: &GoogleError) -> &'static str {
    match error {
        GoogleError::Forbidden(_) | GoogleError::Rejected(_) | GoogleError::Conflict(_) => "denied",
        GoogleError::RateLimited(_) => "rate_limited",
        GoogleError::NotFound(_) => "not_found",
        GoogleError::Unauthorized(_) => "unauthorized",
        GoogleError::Unavailable(message) if message.contains("failed (5") => "unavailable",
        GoogleError::Unavailable(_) | GoogleError::JsonParser(_) => "network",
        GoogleError::Storage(_) => "unavailable",
    }
}

/// Any direct, non-deleted user permission already covers this address.
fn already_has_access(permissions: &[Value], email: &str) -> bool {
    let wanted = email;
    if wanted.is_empty() {
        return false;
    }
    permissions.iter().any(|permission| {
        permission["type"].as_str() == Some("user")
            && permission["deleted"] != Value::Bool(true)
            && permission["emailAddress"]
                .as_str()
                .is_some_and(|address| address.eq_ignore_ascii_case(wanted))
    })
}

/// Files already pinned, and whether `file_id` is one of them. Blanks and duplicates don't count.
fn attachment_room(file_id: &str, attached: &[String]) -> (bool, usize) {
    let mut unique = Vec::new();
    for raw in attached {
        let id = raw.trim();
        if id.is_empty() || unique.iter().any(|stored: &String| stored == id) {
            continue;
        }
        unique.push(id.to_owned());
    }
    let pinned = unique.iter().any(|id| id == file_id);
    (pinned, unique.len())
}

async fn share_file(c: &mut Ctx) -> Result {
    let room = room_for_share(c, "share").await?;
    let input: api::ShareDriveFile = body(c).await?;
    if !valid_id(&json!(input.file_id)) {
        return Err(fail(c, validation("fileId", "includes an invalid file id")));
    }
    let Some(account) = drive_account(c).await? else {
        return empty(c, StatusCode::NOT_FOUND);
    };
    let Some(approved) = approved_rows(&input.recipients) else {
        return Err(invalid_recipients(c, "recipients", None));
    };
    if approved.is_empty() {
        return Err(invalid_recipients(c, "recipients", None));
    }
    if approved.len() > 100 {
        return Err(fail(c, validation("recipients", "too_many_recipients")));
    }
    let viewer = concerns::require_current_user(c)?.id;
    let members = eligible(c, room, viewer).await?;
    let invalid = approved
        .iter()
        .map(|row| row.id)
        .filter(|id| {
            !members
                .iter()
                .any(|member| u64::try_from(member.id).ok() == Some(*id))
        })
        .collect::<Vec<_>>();
    if !invalid.is_empty() {
        return Err(invalid_recipients(c, "recipients", Some(invalid)));
    }
    let selected = approved
        .iter()
        .filter_map(|row| {
            members
                .iter()
                .find(|member| u64::try_from(member.id).ok() == Some(row.id))
                .cloned()
        })
        .collect::<Vec<_>>();
    // Classic `#confirmIdentities`: a changed email is a fresh review, and nothing is granted.
    let changed_ids = approved
        .iter()
        .zip(&selected)
        .filter(|(row, member)| !member.email.eq_ignore_ascii_case(&row.email))
        .map(|(_, member)| member.id)
        .collect::<Vec<_>>();
    if !changed_ids.is_empty() {
        return finished(
            c,
            input.file_id,
            "confirmation_required",
            None,
            changed_ids,
            members.iter().map(recipient_of).collect(),
            Vec::new(),
        );
    }
    let (pinned, attached) = attachment_room(&input.file_id, &input.attached_file_ids);
    if !pinned && attached >= campfire_db::message::DRIVE_ATTACHMENTS_PER_MESSAGE {
        return finished(
            c,
            input.file_id,
            "full",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
    }
    let file = match c
        .app()
        .google
        .api()
        .drive_share_file(
            &c.app().db,
            &c.app().secrets,
            account.user_id,
            &input.file_id,
            now(c),
        )
        .await
    {
        Ok(file) => file,
        Err(GoogleError::Storage(error)) => return Err(Error::internal(error)),
        Err(_) => {
            return finished(
                c,
                input.file_id,
                "blocked",
                Some("file"),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
        }
    };
    let mime = file["mimeType"].as_str().unwrap_or("");
    let blocked = if mime == FOLDER_MIME {
        Some("folder")
    } else if mime == SHORTCUT_MIME {
        Some("shortcut")
    } else if file["capabilities"]["canShare"] != json!(true) {
        Some("capability")
    } else {
        None
    };
    if let Some(reason) = blocked {
        return finished(
            c,
            input.file_id,
            "blocked",
            Some(reason),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
    }
    let permissions = match c
        .app()
        .google
        .api()
        .list_drive_permissions(
            &c.app().db,
            &c.app().secrets,
            account.user_id,
            &input.file_id,
            now(c),
        )
        .await
    {
        Ok(permissions) => permissions,
        Err(GoogleError::Storage(error)) => return Err(Error::internal(error)),
        Err(error) => {
            let reason = grant_reason(&error);
            return finished(
                c,
                input.file_id,
                "shared",
                None,
                Vec::new(),
                Vec::new(),
                selected
                    .iter()
                    .map(|member| share_result(member, "failed", Some(reason)))
                    .collect(),
            );
        }
    };
    let mut results = Vec::with_capacity(selected.len());
    let mut stop = false;
    for member in &selected {
        if stop {
            results.push(share_result(member, "failed", Some("unauthorized")));
            continue;
        }
        if already_has_access(&permissions, &member.email) {
            results.push(share_result(member, "already", None));
            continue;
        }
        match c
            .app()
            .google
            .api()
            .create_reader_permission(
                &c.app().db,
                &c.app().secrets,
                account.user_id,
                &input.file_id,
                &member.email,
                now(c),
            )
            .await
        {
            Ok(_) => results.push(share_result(member, "granted", None)),
            Err(GoogleError::Storage(error)) => return Err(Error::internal(error)),
            Err(error) => {
                let reason = grant_reason(&error);
                results.push(share_result(member, "failed", Some(reason)));
                stop = reason == "unauthorized";
            }
        }
    }
    finished(
        c,
        input.file_id,
        "shared",
        None,
        Vec::new(),
        Vec::new(),
        results,
    )
}

/// Drive ids the way the classic composer accepts them: ASCII-stripped, blanks skipped, duplicates
/// dropped, invalid ids refused, at most ten.
pub(crate) fn checked_drive_file_ids(
    raw: &[String],
) -> std::result::Result<Vec<String>, &'static str> {
    let mut ids = Vec::new();
    for raw in raw {
        let id = raw.trim_matches([' ', '\t', '\r', '\n', '\u{000b}', '\u{000c}', '\0']);
        if id.chars().all(char::is_whitespace) {
            continue;
        }
        if !campfire_db::message::valid_drive_file_id(id) {
            return Err("includes an invalid file id");
        }
        if !ids.iter().any(|stored: &String| stored == id) {
            ids.push(id.to_owned());
        }
    }
    if ids.len() > campfire_db::message::DRIVE_ATTACHMENTS_PER_MESSAGE {
        return Err("are limited to 10 per message");
    }
    Ok(ids)
}

pub(crate) fn require_drive_file_ids(c: &mut Ctx, raw: &[String]) -> Result<Vec<String>> {
    checked_drive_file_ids(raw).map_err(|message| fail(c, validation("driveFileIds", message)))
}

/// Ids to drop from a message. Same id rules as a create; the ten-file cap is on what remains.
pub(crate) fn require_removed_drive_file_ids(c: &mut Ctx, raw: &[String]) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for raw in raw {
        let id = raw.trim_matches([' ', '\t', '\r', '\n', '\u{000b}', '\u{000c}', '\0']);
        if id.chars().all(char::is_whitespace) {
            continue;
        }
        if !campfire_db::message::valid_drive_file_id(id) {
            return Err(fail(
                c,
                validation("removeDriveFileIds", "includes an invalid file id"),
            ));
        }
        if !ids.iter().any(|stored: &String| stored == id) {
            ids.push(id.to_owned());
        }
    }
    Ok(ids)
}

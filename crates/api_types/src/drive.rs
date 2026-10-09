//! Google Drive search and sharing for the composer (`google/drive_files`,
//! `rooms/drive_recipients`). Metadata stays per viewer. Sharing a file grants the chosen room
//! members reader access, the way the classic review dialog does, and the recipient check uses
//! the same membership rules.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One file from `GET /api/v1/drive/files` or `GET /api/v1/drive/files/:id`. The same fields the
/// classic `file_json` builds (`id`, `name`, `kind`, `modified_at`, `owner`, `url`), in camelCase.
/// `kind` is `document`, `spreadsheet`, `presentation`, `form`, `folder`, `shortcut`, `pdf`,
/// or `file`. A shortcut is its own kind so the composer can keep it attach-only, the way the
/// classic share dialog refuses the shortcut MIME.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveFile {
    pub id: Option<String>,
    pub name: Option<String>,
    pub kind: String,
    pub modified_at: Option<String>,
    pub owner: Option<String>,
    pub url: Option<String>,
}

/// `GET /api/v1/drive/files?q=`: the viewer's Drive files, recent first, filtered by name.
/// A viewer with no usable Drive grant gets 404, the same as classic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveFileList {
    pub files: Vec<DriveFile>,
}

/// A room member the viewer may grant a Drive file to (`drive_recipients`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveRecipient {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// `GET /api/v1/rooms/:id/drive/recipients` and a successful validate or share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveRecipientList {
    pub recipients: Vec<DriveRecipient>,
}

/// `POST /api/v1/rooms/:id/drive/recipients/validate`: confirm these member ids still belong,
/// in this order. An id that isn't a grantable member is 422 `invalid_recipients`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ValidateDriveRecipients {
    pub user_ids: Vec<String>,
}

/// One person the viewer approved in the review dialog, with the email they were shown.
/// Classic compares this address with the member's current email at grant time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveApprovedRecipient {
    pub id: String,
    pub email: String,
}

/// `POST /api/v1/rooms/:id/drive/shares`: grant the approved members reader access to `fileId`.
/// Grants are reader-only and send no email. `attachedFileIds` is the message's current Drive
/// set; a file that isn't already there is refused before any grant when the message is at the
/// classic limit of 10. Folders, shortcuts, and files the viewer cannot share are not granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShareDriveFile {
    pub file_id: String,
    pub recipients: Vec<DriveApprovedRecipient>,
    /// Drive files already on this message. Omitted means none.
    #[serde(default)]
    pub attached_file_ids: Vec<String>,
}

/// One recipient's grant. `status` is `granted`, `already`, or `failed`. `reason`, when the
/// grant failed, is `denied`, `rate_limited`, `not_found`, `unavailable`, `network`, or
/// `unauthorized` — the same distinctions the classic dialog reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveShareResult {
    pub recipient: DriveRecipient,
    pub status: String,
    pub reason: Option<String>,
}

/// The share result. `outcome` is `shared` (see `results`, including partial failure),
/// `confirmation_required` (an approved email no longer matches; nothing was granted;
/// `recipients` is the refreshed list and `changedIds` stay unchecked), `blocked` (`blocked`
/// is `folder`, `shortcut`, `capability`, or `file`), or `full` (the message is already at
/// ten Drive files).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveShare {
    pub outcome: String,
    pub file_id: String,
    pub blocked: Option<String>,
    pub changed_ids: Vec<i64>,
    pub recipients: Vec<DriveRecipient>,
    pub results: Vec<DriveShareResult>,
}

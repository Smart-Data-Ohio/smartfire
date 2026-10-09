//! Google Drive search and sharing for the composer (`google/drive_files`,
//! `rooms/drive_recipients`). Metadata stays per viewer. Sharing a file grants the chosen room
//! members reader access, the way the classic review dialog does, and the recipient check uses
//! the same membership rules.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One file from `GET /api/v1/drive/files` or `GET /api/v1/drive/files/:id`. The same fields the
/// classic `file_json` builds (`id`, `name`, `kind`, `modified_at`, `owner`, `url`), in camelCase.
/// `kind` is `document`, `spreadsheet`, `presentation`, `form`, `folder`, `pdf`, or `file`.
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

/// `POST /api/v1/rooms/:id/drive/shares`: grant the chosen members reader access to `fileId`,
/// then the composer attaches it. Grants are reader-only and send no email, as the classic
/// dialog does. Folders are not shared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShareDriveFile {
    pub file_id: String,
    pub user_ids: Vec<String>,
}

/// The members who were granted reader access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveShare {
    pub file_id: String,
    pub recipients: Vec<DriveRecipient>,
}

//! Files on messages and the direct-upload flow that puts them there.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A message's attached file, from its Active Storage blob (`AttachmentView`). URLs are
/// same-origin paths that need the session cookie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Attachment {
    /// `active_storage_blobs.filename`.
    pub filename: String,
    /// `active_storage_blobs.content_type`; `application/octet-stream` when unknown.
    pub content_type: String,
    /// `active_storage_blobs.byte_size`.
    pub byte_size: i64,
    /// Pixel size from the blob's analyzed metadata (videos' float sizes rounded); `null` until
    /// analysis finishes, and for files that have none.
    pub width: Option<i64>,
    pub height: Option<i64>,
    /// How the timeline shows it.
    pub preview: AttachmentPreview,
    /// The blob itself, inline (`/rails/active_storage/blobs/redirect/:signed_id/:filename`).
    pub url: String,
    /// The same with `?disposition=attachment`: downloads instead of opening.
    pub download_url: String,
    /// A still to show: an image's `resize_to_limit [1200, 800]` representation, or a video's
    /// webp poster of the same size; `null` for other files, and until it's generated.
    pub thumbnail_url: Option<String>,
}

/// `AttachmentView.preview`: an image (thumbnail in a lightbox), a video (inline player) or any
/// other file (icon, name, size and a download link).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AttachmentPreview {
    Image,
    Video,
    File,
}

/// `POST /api/v1/uploads`: start a direct upload (Active Storage's
/// `POST /rails/active_storage/direct_uploads`). Files above the boot account's
/// `uploadLimitBytes` receive a 422 before a blob is created.
///
/// The flow: create the blob here, `PUT` the raw bytes to [`DirectUpload::upload_url`] with
/// `Content-Type: contentType` (204 on success; 422 when the length, type or checksum doesn't
/// match; 404 once the URL's 5 minutes are up), then post the message with
/// `attachmentSignedId`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateUpload {
    pub filename: String,
    /// Exact length of the bytes to be `PUT`.
    pub byte_size: i64,
    /// Base64 of the file's MD5 digest, checked on `PUT`.
    pub checksum: String,
    /// The file's MIME type; the `PUT` must send the same `Content-Type`.
    pub content_type: String,
}

/// The reply to `POST /api/v1/uploads`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DirectUpload {
    /// Attaches the blob: `CreateMessage.attachmentSignedId`. Doesn't expire.
    pub signed_id: String,
    /// Where to `PUT` the bytes (`/rails/active_storage/disk/:encoded_token`), valid for 5
    /// minutes. Same origin: the session cookie goes with it; no CSRF token needed.
    pub upload_url: String,
}

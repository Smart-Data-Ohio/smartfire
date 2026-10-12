use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sticker {
    pub id: i64,
    pub name: String,
    pub description: String,
    /// Search tags or the related Unicode emoji.
    pub tags: String,
    pub image_url: String,
    pub still_url: String,
    pub animated: bool,
    pub uploader_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StickerCatalog {
    pub stickers: Vec<Sticker>,
    pub capacity: i64,
}

/// `POST /api/v1/admin/stickers`, as multipart fields plus an `image` file part.
/// JSON callers may instead pass an existing upload's `signedId`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateSticker {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: String,
    pub signed_id: Option<String>,
}

/// `PATCH /api/v1/admin/stickers/:id` replaces the editable fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateSticker {
    pub name: String,
    pub description: String,
    pub tags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum MessageSticker {
    Available {
        id: i64,
        name: String,
        image_url: String,
        still_url: String,
        animated: bool,
    },
    Deleted {
        name: String,
    },
}

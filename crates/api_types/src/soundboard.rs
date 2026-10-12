//! Voice soundboard catalog. Upload audio through `/api/v1/uploads` before creating a sound.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoundboardSound {
    pub id: i64,
    pub name: String,
    pub emoji: Option<String>,
    /// The default playback volume, between 0.0 and 1.0.
    pub volume: f64,
    pub uploader_id: i64,
    pub audio_url: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `GET /api/v1/soundboard/sounds`, and the response after every administrator catalog write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoundboardCatalog {
    pub sounds: Vec<SoundboardSound>,
    pub capacity: i64,
}

/// `POST /api/v1/admin/soundboard/sounds`. Omitted or null volume defaults to 1.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateSoundboardSound {
    pub name: String,
    pub emoji: Option<String>,
    pub volume: Option<f64>,
    pub signed_id: String,
}

/// `PATCH /api/v1/admin/soundboard/sounds/:id` replaces the editable fields; null emoji clears it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateSoundboardSound {
    pub name: String,
    pub emoji: Option<String>,
    pub volume: f64,
}

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

/// `POST /api/v1/rooms/:room_id/huddle/soundboard` plays a sound to everyone in the room's call
/// (`huddle.soundboard`) and answers 204. `identity` is the calling tab's, from its
/// [`HuddleCredentials`](crate::HuddleCredentials); that tab must be in the call and able to
/// speak. One play per identity per second, else 429.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaySoundboardSound {
    pub identity: String,
    pub sound_id: i64,
}

/// The `huddle.soundboard` event on the `user` topic of everyone in the room's call: each client
/// plays the sound locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleSoundboardPlayed {
    pub room_id: i64,
    pub sound_id: i64,
    /// Who played it, and from which of their tabs.
    pub user_id: i64,
    pub identity: String,
    pub played_at: Timestamp,
}

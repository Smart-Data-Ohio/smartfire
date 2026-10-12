use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `GET /api/v1/gifs/config`: whether the GIF picker is available on this server
/// (a `KLIPY_API_KEY` is configured). When it is not, the other `/api/v1/gifs/*`
/// routes answer `404 NotFound`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifConfig {
    pub enabled: bool,
}

/// One KLIPY GIF as the picker shows it. `slug` is the identifier sent back in
/// `CreateMessage.gifSlug`; `klipyId` is KLIPY's numeric id, kept as a string
/// because it can exceed the JSON safe-integer range in browsers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifItem {
    pub slug: String,
    pub klipy_id: String,
    pub title: String,
    pub attribution: String,
    /// Animated preview, through this server's media proxy (`/api/v1/gifs/media`).
    pub preview_url: String,
    /// A still frame, through the same proxy; shown when motion is reduced.
    pub still_url: Option<String>,
    /// The preview rendition's pixel size; `0` when KLIPY didn't say.
    pub width: i64,
    pub height: i64,
}

/// `GET /api/v1/gifs/search?q=&page=` and `GET /api/v1/gifs/trending?page=`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifList {
    pub items: Vec<GifItem>,
    pub page: i64,
    pub has_next: bool,
}

/// One curated KLIPY category (`GET /api/v1/gifs/categories`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifCategory {
    pub name: String,
    /// The search query this category runs.
    pub query: String,
    /// A representative preview, through the media proxy; `null` when none.
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifCategoryList {
    pub categories: Vec<GifCategory>,
}

/// Search terms from `GET /api/v1/gifs/suggestions?q=` or `/gifs/autocomplete?q=`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifSuggestions {
    pub suggestions: Vec<String>,
}

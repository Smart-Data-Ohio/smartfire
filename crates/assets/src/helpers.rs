use crate::embedded;
pub use campfire_static_assets::MissingAssetError;

/// The digested path (relative to `/assets/`) for a logical path, from the manifest.
pub fn digested_path(logical_path: &str) -> Option<&'static str> {
    embedded::MANIFEST
        .binary_search_by(|(logical, _)| (*logical).cmp(logical_path))
        .ok()
        .map(|index| embedded::MANIFEST[index].1)
}

/// `asset_path(source)`: "/assets/<digested>" for pipeline assets; URLs and absolute paths pass
/// through; a `?query` or `#fragment` tail is kept.
pub fn try_asset_path(source: &str) -> Result<String, MissingAssetError> {
    campfire_static_assets::resolve_asset_path(embedded::MANIFEST, source, None)
}

/// Like [`try_asset_path`], but panics for a missing asset the way Rails raises
/// Propshaft::MissingAssetError while rendering.
pub fn asset_path(source: &str) -> String {
    try_asset_path(source).unwrap_or_else(|e| panic!("{e}"))
}

pub fn image_path(source: &str) -> String {
    asset_path(source)
}

pub fn audio_path(source: &str) -> String {
    asset_path(source)
}

/// `javascript_path`: appends ".js" unless the source already ends in it.
pub fn javascript_path(source: &str) -> String {
    campfire_static_assets::resolve_asset_path(embedded::MANIFEST, source, Some(".js"))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// `stylesheet_path`: appends ".css" unless the source already ends in it.
pub fn stylesheet_path(source: &str) -> String {
    campfire_static_assets::resolve_asset_path(embedded::MANIFEST, source, Some(".css"))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// `asset_url(source)`: the path joined onto the request's base URL (e.g. "https://host:3000"),
/// which is what Rails uses as the host when no asset_host is configured.
pub fn asset_url(base_url: &str, source: &str) -> String {
    campfire_static_assets::resolve_asset_url(embedded::MANIFEST, base_url, source)
}

pub fn image_url(base_url: &str, source: &str) -> String {
    asset_url(base_url, source)
}

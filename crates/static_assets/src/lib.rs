//! Embedded assets shared by retained pages, APIs and persisted rich text.
//! The classic browser bundles remain in `campfire_assets` until their callers are removed.

mod helpers;
mod serve;
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub use helpers::{
    MissingAssetError, asset_path, asset_url, audio_path, digested_path, image_path, image_url,
    javascript_path, resolve_asset_path, resolve_asset_url, stylesheet_path, try_asset_path,
};
pub use serve::{Body, StaticRequest, StaticResponse, serve_embedded};

pub const PREFIX: &str = "/assets";

pub fn manifest() -> &'static [(&'static str, &'static str)] {
    embedded::MANIFEST
}

pub fn manifest_json() -> &'static str {
    embedded::MANIFEST_JSON
}

pub fn serve(request: &StaticRequest) -> Option<StaticResponse> {
    serve_embedded(request, embedded::FILES, embedded::BUILT_AT)
}

/// Full compiled bytes for a logical media/auth path, without constructing an HTTP response.
pub fn asset_bytes(logical: &str) -> Option<&'static [u8]> {
    let path = format!("{PREFIX}/{}", digested_path(logical)?);
    embedded::FILES
        .binary_search_by(|(url, _)| (*url).cmp(&path))
        .ok()
        .map(|index| embedded::FILES[index].1)
}

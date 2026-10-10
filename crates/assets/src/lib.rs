//! The reference's asset pipeline, precomputed at build time and embedded in the binary.
//!
//! `build.rs` digests and compiles `web/app/assets`, `web/app/javascript`,
//! `web/vendor/javascript` and the gem assets in `vendor/` exactly as Propshaft's
//! `assets:precompile` does, renders the import map from `web/config/importmap.rb`, and
//! embeds classic bundles. Retained auth, media and public files live in `campfire_static_assets`.
//!
//! - [`asset_path`] and friends: ActionView's asset URL helpers over the Propshaft manifest.
//! - [`stylesheet_link_tag`] / [`stylesheet_link_tag_all`] and [`javascript_importmap_tags`]:
//!   the layout's head tags, plus the `link` preload header Rails sends alongside them.
//! - [`serve`]: classic bundles with a retained-static fallback.

mod helpers;
mod tags;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub use helpers::{
    MissingAssetError, asset_path, asset_url, audio_path, digested_path, image_path, image_url,
    javascript_path, stylesheet_path, try_asset_path,
};
pub use campfire_static_assets::{Body, StaticRequest, StaticResponse};

/// Classic bundles, followed by the retained static inputs for existing classic callers.
pub fn serve(request: &StaticRequest) -> Option<StaticResponse> {
    serve_classic(request).or_else(|| campfire_static_assets::serve(request))
}

/// Classic bundles and the combined manifest while both pipelines are still served.
pub fn serve_classic(request: &StaticRequest) -> Option<StaticResponse> {
    campfire_static_assets::serve_embedded(request, embedded::FILES, embedded::BUILT_AT)
}
pub use tags::{
    StylesheetTags, WORKER_SELECTION_MODULES, all_stylesheet_paths, append_preload_links,
    javascript_importmap_tags, javascript_importmap_tags_selecting_worker, stylesheet_link_tag,
    stylesheet_link_tag_all,
};

/// The URL prefix digested assets are served under (`config.assets.prefix`).
pub const PREFIX: &str = "/assets";

/// Propshaft's manifest, `{"logical": {"digested_path": ..., "integrity": null}}`, as served
/// from `/assets/.manifest.json`.
pub fn manifest_json() -> &'static str {
    embedded::MANIFEST_JSON
}

/// Every `(logical path, digested path)` pair, sorted by logical path.
pub fn manifest() -> &'static [(&'static str, &'static str)] {
    embedded::MANIFEST
}

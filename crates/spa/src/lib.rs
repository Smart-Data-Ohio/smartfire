//! The React SPA in `frontend/`, built into the binary and served under `/app/`.
//!
//! `build.rs` embeds Vite's output (`frontend/dist`, or `SPA_DIST`): `index.html` as the shell's
//! template, and every other file with its content type, cache policy and precompressed brotli
//! and gzip encodings. Without a dist it embeds a one-page stub, so cargo builds and the Rust CI
//! never need Node ([`built`] says which).
//!
//! - [`file`]: an embedded file by its path under `/app/`, in the encoding the client prefers.
//! - [`render_shell`]: `index.html` with the CSRF meta tags, the CSP nonce and the [`Boot`] JSON
//!   the app starts from.
//! - [`screens`]: which classic pages the SPA has ported, and their URLs on both sides.
//!
//! The HTTP side (routes, authentication, headers) is the app's (`crates/campfire`,
//! `controllers/spa.rs`); this crate knows nothing of requests or sessions.

mod boot;
pub mod pwa;
pub mod screens;
mod serve;
mod shell;

#[cfg(test)]
mod tests;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/spa.rs"));
}

pub use boot::{Boot, BootAccount, BootFlash, BootResponse, BootUser, FlashKind, script_json, text_size, theme};
pub use serve::{File, Served, file};
pub use shell::render_shell;

/// The URL prefix the SPA lives under (Vite's `base`, without its trailing slash).
pub const PREFIX: &str = "/app";

/// The SPA shell's URL. Installed PWAs keep their original root start URL and scope.
pub fn root_path() -> String {
    format!("{PREFIX}/")
}

/// `Cache-Control` for an embedded file whose name isn't content-hashed: the public files'
/// policy (`campfire_static_assets`), so a fix reaches clients within a minute.
pub const REVALIDATE_CACHE_CONTROL: &str = "public, max-age=60, stale-while-revalidate=300";

/// Whether a real dist was embedded, rather than the stub.
pub fn built() -> bool {
    embedded::BUILT
}

/// Every embedded file but `index.html`, sorted by path.
pub fn files() -> &'static [File] {
    embedded::FILES
}

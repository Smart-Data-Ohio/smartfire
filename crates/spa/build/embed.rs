//! What the build embeds for each file of a Vite dist: shared by `build.rs`, which writes it into
//! `$OUT_DIR/spa.rs`, and the tests, which run it over `tests/fixture`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// One servable file of the dist.
pub struct Entry {
    /// Its path under the dist, `/`-separated: `assets/index-B2x8Kq1f.js`.
    pub path: String,
    pub source: PathBuf,
    pub content_type: &'static str,
    pub immutable: bool,
    /// The `.br` and `.gz` Vite wrote beside it, if it did.
    pub br: Option<PathBuf>,
    pub gz: Option<PathBuf>,
}

impl Entry {
    /// The encodings worth sending instead of the file itself.
    pub fn compressible(&self) -> bool {
        compressible(self.content_type)
    }
}

/// The dist's `index.html` (the shell's template) and every other file in it, sorted by path.
/// Hidden files and directories (Vite's `.vite/manifest.json`) are build metadata, not served,
/// and a `.br` or `.gz` beside a file is that file's encoding, not a file of its own.
pub fn read_dist(dist: &Path) -> (PathBuf, Vec<Entry>) {
    let mut files = Vec::new();
    walk(dist, dist, &mut files);
    files.sort();
    let has = |path: &str| files.binary_search_by(|(p, _)| p.as_str().cmp(path)).is_ok();
    let sibling = |path: &str, extension: &str| {
        let encoded = format!("{path}.{extension}");
        has(&encoded).then(|| dist.join(&encoded))
    };

    let index = dist.join("index.html");
    assert!(index.is_file(), "{} has no index.html", dist.display());
    let entries = files
        .iter()
        .filter(|(path, _)| path != "index.html")
        .filter(|(path, _)| {
            let base = path.strip_suffix(".br").or_else(|| path.strip_suffix(".gz"));
            !base.is_some_and(has)
        })
        .map(|(path, source)| Entry {
            path: path.clone(),
            source: source.clone(),
            content_type: content_type(path),
            immutable: immutable(path),
            br: sibling(path, "br"),
            gz: sibling(path, "gz"),
        })
        .collect();
    (index, entries)
}

fn walk(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, files);
        } else {
            let relative = path.strip_prefix(root).unwrap();
            let relative: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            files.push((relative.join("/"), path));
        }
    }
}

/// The `Content-Type` for a dist path, by extension.
pub fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/vnd.microsoft.icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

/// Text and other formats that aren't compressed already.
pub fn compressible(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type.starts_with("application/json")
        || content_type.starts_with("application/manifest+json")
        || content_type.starts_with("application/xml")
        || content_type.starts_with("application/wasm")
        || content_type.starts_with("image/svg+xml")
        || content_type.starts_with("image/vnd.microsoft.icon")
        || content_type.starts_with("font/ttf")
        || content_type.starts_with("font/otf")
}

/// A file Vite named after its content (`assets/[name]-[hash].[ext]`, an eight-character
/// base64url hash), so a URL always means the same bytes. Anything else under `assets/` (or a
/// hash of another length) is served as revalidating rather than guessed at.
pub fn immutable(path: &str) -> bool {
    let Some(name) = path.strip_prefix("assets/") else { return false };
    let file = name.rsplit('/').next().unwrap_or(name).as_bytes();
    let hash_char = |b: &u8| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_');
    // `[name]-[hash]` then only extensions: `index-B2x8Kq1f.js`, `app.config-B2x8Kq1f.js.map`.
    (1..file.len()).any(|dash| {
        let (Some(hash), Some(extensions)) = (file.get(dash + 1..dash + 9), file.get(dash + 9..)) else { return false };
        file[dash] == b'-'
            && hash.iter().all(hash_char)
            && extensions.first() == Some(&b'.')
            && extensions.split(|&b| b == b'.').skip(1).all(|ext| !ext.is_empty() && ext.iter().all(u8::is_ascii_alphanumeric))
    })
}

/// Brotli at its best quality: done once per build, served many times.
pub fn brotli(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let params = brotli::enc::BrotliEncoderParams { quality: 11, lgwin: 22, ..Default::default() };
    brotli::BrotliCompress(&mut &bytes[..], &mut out, &params).expect("compressing into memory can't fail");
    out
}

/// gzip at level 9, with no file name or time in the header so builds are reproducible.
pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::GzBuilder::new().mtime(0).write(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).expect("compressing into memory can't fail");
    encoder.finish().expect("compressing into memory can't fail")
}

/// `encoded` if it's worth sending instead of `original`.
pub fn smaller(encoded: Vec<u8>, original: &[u8]) -> Option<Vec<u8>> {
    (encoded.len() < original.len()).then_some(encoded)
}

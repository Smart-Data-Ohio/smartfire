//! Embeds the built SPA (`frontend/dist`, or `SPA_DIST`) into the crate as `$OUT_DIR/spa.rs`:
//! `index.html` as the shell's template, and every other file with its content type, whether its
//! name is content-hashed, and its brotli and gzip encodings (Vite's own `.br`/`.gz` when it wrote
//! them, made here otherwise).
//!
//! Without a dist, a one-page stub stands in, so cargo builds and the Rust CI never need Node.
//! An explicitly set `SPA_DIST` that doesn't exist fails the build instead: the image build (and
//! the Frontend CI job) mean to embed a real one. A relative `SPA_DIST` is resolved against the
//! workspace root.

#[path = "build/embed.rs"]
mod embed;

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = crate_dir.join("../..").canonicalize().unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build");
    println!("cargo:rerun-if-changed=stub");
    println!("cargo:rerun-if-env-changed=SPA_DIST");
    // The image build's cache mount keeps target/ across builds whose dist files can carry older
    // mtimes than the last build's, so it passes a digest of the dist's contents as well.
    println!("cargo:rerun-if-env-changed=SPA_DIST_DIGEST");

    let explicit = env::var_os("SPA_DIST").filter(|dist| !dist.is_empty()).map(PathBuf::from);
    let dist = match &explicit {
        Some(dist) => workspace.join(dist),
        None => workspace.join("frontend/dist"),
    };

    let code = if dist.join("index.html").is_file() {
        // Only an existing path can be watched: a missing one reruns the script on every build.
        println!("cargo:rerun-if-changed={}", dist.display());
        embed_dist(&dist, &out_dir)
    } else if let Some(explicit) = explicit {
        panic!("SPA_DIST={} has no index.html (run `pnpm build` in frontend/)", explicit.display());
    } else {
        stub(&crate_dir)
    };
    fs::write(out_dir.join("spa.rs"), code).unwrap();
}

/// No dist: the stub page, and no files. Building the SPA later needs `touch crates/spa/build.rs`
/// (or `SPA_DIST`) to be picked up, since a missing directory can't be watched cheaply.
fn stub(crate_dir: &Path) -> String {
    format!(
        "pub(crate) const BUILT: bool = false;\n\
         pub(crate) static INDEX_HTML: &str = include_str!({:?});\n\
         pub(crate) static FILES: &[crate::File] = &[];\n",
        crate_dir.join("stub/index.html").display().to_string()
    )
}

fn embed_dist(dist: &Path, out_dir: &Path) -> String {
    let encoded_dir = out_dir.join("encoded");
    let _ = fs::remove_dir_all(&encoded_dir);
    let (index, entries) = embed::read_dist(dist);

    let mut code = String::new();
    code.push_str("pub(crate) const BUILT: bool = true;\n");
    writeln!(code, "pub(crate) static INDEX_HTML: &str = include_str!({:?});", index.display().to_string()).unwrap();
    code.push_str("pub(crate) static FILES: &[crate::File] = &[\n");
    for entry in &entries {
        let bytes = fs::read(&entry.source).unwrap();
        let encoding = |vite: &Option<PathBuf>, extension: &str, encode: fn(&[u8]) -> Vec<u8>| -> Option<PathBuf> {
            if let Some(vite) = vite {
                return Some(vite.clone());
            }
            if !entry.compressible() {
                return None;
            }
            let encoded = embed::smaller(encode(&bytes), &bytes)?;
            let path = encoded_dir.join(format!("{}.{extension}", entry.path));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, encoded).unwrap();
            Some(path)
        };
        let br = encoding(&entry.br, "br", embed::brotli);
        let gz = encoding(&entry.gz, "gz", embed::gzip);
        let include = |path: &Option<PathBuf>| match path {
            Some(path) => format!("Some(include_bytes!({:?}))", path.display().to_string()),
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    crate::File {{ path: {:?}, content_type: {:?}, immutable: {}, identity: include_bytes!({:?}), br: {}, gz: {} }},",
            entry.path,
            entry.content_type,
            entry.immutable,
            entry.source.display().to_string(),
            include(&br),
            include(&gz),
        )
        .unwrap();
    }
    code.push_str("];\n");
    code
}

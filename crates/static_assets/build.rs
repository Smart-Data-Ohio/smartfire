//! Media and public inputs. No import map or stylesheet compilation.
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

// The former config/initializers/assets.rb version is part of existing persisted URLs.
const VERSION: &[u8] = b"1.0";

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    // Absolute paths: ci/with-release-inputs.sh builds from a temporary copy that shares this
    // target directory. Relative paths would resolve against whichever copy builds next and look
    // fresh, leaving embedded.rs pointing into the deleted copy; absolute ones go missing and rerun.
    for path in ["media", "public", "legacy-digests.json"] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    let mut inputs = BTreeMap::new();
    for directory in [
        root.join("media/emoji"),
        root.join("media/images"),
        root.join("media/sounds"),
    ] {
        for file in files(&directory) {
            let logical = file
                .strip_prefix(&directory)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            // This newly colocated license is an input notice, not an additional media URL.
            if logical != "GEMOJI-LICENSE" {
                inputs.insert(logical, file);
            }
        }
    }
    let contents: BTreeMap<_, _> = inputs
        .iter()
        .map(|(name, path)| (name.clone(), fs::read(path).unwrap()))
        .collect();
    let mut manifest = BTreeMap::new();
    for (logical, content) in &contents {
        let mut hash = Sha1::new();
        hash.update(content);
        hash.update(VERSION);
        let digest = format!("{:x}", hash.finalize());
        let (stem, extension) = logical.rsplit_once('.').unwrap();
        manifest.insert(
            logical.clone(),
            format!("{stem}-{}.{extension}", &digest[..8]),
        );
    }

    let mut code = String::new();
    let mut served = BTreeMap::new();
    let mut identifiers = BTreeMap::new();
    for (index, (logical, source)) in inputs.iter().enumerate() {
        let identifier = format!("ASSET_{index}");
        writeln!(
            code,
            "static {identifier}: &[u8] = {};",
            include(&root, source)
        )
        .unwrap();
        served.insert(format!("/assets/{}", manifest[logical]), identifier.clone());
        identifiers.insert(logical.clone(), identifier);
    }
    let aliases: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(root.join("legacy-digests.json")).unwrap()).unwrap();
    for (digest, logical) in aliases {
        // Changing immutable media requires keeping the old bytes, not aliasing new bytes to an old URL.
        assert_eq!(
            manifest[&logical], digest,
            "preserve the historical bytes for {digest}"
        );
        served.insert(format!("/assets/{digest}"), identifiers[&logical].clone());
    }
    for (index, file) in files(&root.join("public")).iter().enumerate() {
        let relative = file
            .strip_prefix(root.join("public"))
            .unwrap()
            .to_str()
            .unwrap();
        if relative.starts_with("assets/") || relative == "offline.html" {
            continue;
        }
        let identifier = format!("PUBLIC_{index}");
        writeln!(
            code,
            "static {identifier}: &[u8] = {};",
            include(&root, file)
        )
        .unwrap();
        served.insert(format!("/{relative}"), identifier);
    }
    let json: BTreeMap<_, _> = manifest
        .iter()
        .map(|(logical, digest)| {
            (
                logical,
                serde_json::json!({"digested_path": digest, "integrity": null}),
            )
        })
        .collect();
    writeln!(
        code,
        "pub(crate) static MANIFEST_JSON: &str = {:?};",
        serde_json::to_string(&json).unwrap()
    )
    .unwrap();
    served.insert(
        "/assets/.manifest.json".to_owned(),
        "MANIFEST_JSON.as_bytes()".to_owned(),
    );
    code.push_str("pub(crate) static MANIFEST: &[(&str, &str)] = &[\n");
    for (logical, digest) in manifest {
        writeln!(code, "    ({logical:?}, {digest:?}),").unwrap();
    }
    code.push_str("];\npub(crate) static FILES: &[(&str, &[u8])] = &[\n");
    for (url, identifier) in served {
        writeln!(code, "    ({url:?}, {identifier}),").unwrap();
    }
    code.push_str("];\n");
    let time = env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| epoch.parse().ok())
        .map(|epoch| SystemTime::UNIX_EPOCH + Duration::from_secs(epoch))
        .unwrap_or_else(SystemTime::now);
    writeln!(
        code,
        "pub(crate) static BUILT_AT: &str = {:?};",
        httpdate::fmt_http_date(time)
    )
    .unwrap();
    fs::write(out.join("embedded.rs"), code).unwrap();
}

/// `include_bytes!` relative to the compiling crate's manifest directory, so the generated code
/// never names the directory the build script happened to run in.
fn include(root: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(root).unwrap().to_str().unwrap();
    format!(
        "include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {:?}))",
        format!("/{relative}")
    )
}

fn files(directory: &Path) -> Vec<PathBuf> {
    let mut output = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_str().unwrap().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            output.extend(files(&path));
        } else {
            output.push(path);
        }
    }
    output.sort();
    output
}

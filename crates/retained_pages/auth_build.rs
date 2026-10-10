//! Retained auth CSS is bundled without Node by static_assets/build.rs.
//! Tokens and fonts stay in frontend/src; scripts live in static_assets/auth.
use fancy_regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn prepare(crate_dir: &Path, out_dir: &Path) -> PathBuf {
    let frontend = crate_dir.join("../../frontend/src");
    for path in [
        "auth",
        "styles",
        "motion",
        "ui/button.css",
        "ui/text-field.css",
        "ui/checkbox.css",
    ] {
        println!("cargo:rerun-if-changed={}", frontend.join(path).display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("auth").display()
    );
    let destination = out_dir.join("auth-inputs");
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(destination.join("fonts")).unwrap();
    let mut bundle = Bundle {
        destination: &destination,
        fonts: frontend
            .join("styles/fonts")
            .canonicalize()
            .unwrap_or_else(|e| missing(&frontend.join("styles/fonts"), e)),
        visited: HashSet::new(),
        // Comments and strings are opaque; only actual import/url tokens are processed.
        tokens: Regex::new(r#"(?s)/\*.*?\*/|@import\s+(?:url\(\s*)?["'](?P<import>[^"']+)["']\s*\)?\s*;|url\(\s*(?:["'](?P<quoted>[^"']+)["']|(?P<bare>[^\s)]+))\s*\)|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'"#).unwrap(),
    };
    let css = bundle.inline(&frontend.join("auth/auth.css"));
    fs::write(destination.join("auth.css"), css).unwrap();
    fs::copy(crate_dir.join("auth/auth.js"), destination.join("auth.js")).unwrap();
    fs::copy(
        crate_dir.join("auth/unsupported.js"),
        destination.join("unsupported.js"),
    )
    .unwrap();
    destination
}

/// The auth stylesheet is built from frontend/src; a build without those sources (an image or CI
/// step that didn't copy them) stops here, naming what's missing.
fn missing(path: &Path, error: std::io::Error) -> ! {
    panic!(
        "auth assets: {} is unavailable ({error}). The build reads frontend/src/{{auth,styles,motion}} \
         and frontend/src/ui/{{button,text-field,checkbox}}.css: copy them as the Dockerfile and \
         ci/with-release-inputs.sh do.",
        path.display()
    )
}

struct Bundle<'a> {
    destination: &'a Path,
    fonts: PathBuf,
    visited: HashSet<PathBuf>,
    tokens: Regex,
}

impl Bundle<'_> {
    fn inline(&mut self, path: &Path) -> String {
        let path = path.canonicalize().unwrap_or_else(|e| missing(path, e));
        if !self.visited.insert(path.clone()) {
            return String::new();
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let source = fs::read_to_string(&path).unwrap();
        // Capture first so recursion can borrow the bundler mutably.
        let tokens = self
            .tokens
            .captures_iter(&source)
            .map(|c| {
                let c = c.unwrap();
                let span = c.get(0).unwrap();
                (
                    span.start(),
                    span.end(),
                    c.name("import").map(|m| m.as_str().to_owned()),
                    c.name("quoted")
                        .or_else(|| c.name("bare"))
                        .map(|m| m.as_str().to_owned()),
                )
            })
            .collect::<Vec<_>>();
        let mut output = String::new();
        let mut last = 0;
        for (start, end, import, url) in tokens {
            output.push_str(&source[last..start]);
            if let Some(import) = import {
                assert!(
                    !import.starts_with('/') && !import.contains(':'),
                    "auth CSS imports must be relative: {import}"
                );
                output.push_str(&self.inline(&path.parent().unwrap().join(import)));
            } else if let Some(url) = url {
                if !url.starts_with('/') && !url.starts_with('#') && !url.contains(':') {
                    let (file, tail) = url
                        .find(['?', '#'])
                        .map_or((url.as_str(), ""), |i| (&url[..i], &url[i..]));
                    let file = path.parent().unwrap().join(file);
                    let file = file.canonicalize().unwrap_or_else(|e| missing(&file, e));
                    assert!(
                        file.starts_with(&self.fonts),
                        "auth CSS local URLs must name bundled fonts: {}",
                        file.display()
                    );
                    let relative = file.strip_prefix(&self.fonts).unwrap();
                    let target = self.destination.join("fonts").join(relative);
                    fs::create_dir_all(target.parent().unwrap()).unwrap();
                    fs::copy(&file, target).unwrap();
                    println!("cargo:rerun-if-changed={}", file.display());
                    output.push_str(&format!("url(\"fonts/{}{tail}\")", relative.display()));
                } else {
                    output.push_str(&source[start..end]);
                }
            } else {
                output.push_str(&source[start..end]);
            }
            last = end;
        }
        output.push_str(&source[last..]);
        output
    }
}

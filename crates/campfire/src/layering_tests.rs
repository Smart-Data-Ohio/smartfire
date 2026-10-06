//! The layers this crate splits into (`plans/crate-split-plan.md`): every module belongs to one,
//! and its code may name only modules in its own layer or below. Test modules (`#[cfg(test)] mod`)
//! are exempt, since the split moves the ones that reach up into the crates above; `#[cfg(test)]`
//! items in ordinary modules are not, since they stay where they are.
//!
//! This reads the sources rather than the compiler's view: it follows `mod` declarations from
//! `main.rs`, and resolves `crate::`, `super::` and `self::` paths (in `use` trees and in code)
//! to the longest module they name. Paths through other names (imported modules, re-exports)
//! are covered by the `use` that brought the name in.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use regex::Regex;

/// The layers, lowest first, by module path. A path ending in `::*` covers that module's
/// children but not the module itself; otherwise it covers the module and everything inside it.
/// The longest match wins; anything unmatched is the server's.
const LAYERS: &[(&str, &[&str])] = &[
    // `campfire_app`'s modules that still hold tests here (the rest are `use`s of campfire_app).
    ("app", &["app", "huddle", "integrations"]),
    // `campfire_web`'s mirrors here, which hold tests of theirs.
    ("web", &["concerns", "controllers::presenters", "mail"]),
    // `campfire_channels`'s mirrors here, which hold tests of theirs.
    ("channels", &["channels", "jobs"]),
    // `campfire_messages`'s mirrors here, which hold tests of theirs.
    ("message_features", &["controllers::message_features"]),
    ("messages", &["controllers::messages"]),
    // `campfire_rooms`'s mirrors here, which hold tests of theirs (and the test-only `shell`).
    ("rooms", &["controllers::rooms", "controllers::room_categories"]),
    // `campfire_people`'s mirrors here, which hold tests of theirs.
    ("people", &[
        "controllers::accounts", "controllers::qr_code", "controllers::sessions",
        "controllers::sudos", "controllers::two_factor", "controllers::users",
    ]),
    ("controllers", &["controllers::*"]),
    ("server", &[
        "", "admin", "controllers", "controllers::health", "controllers::mailbox",
        "controllers::turbo_native", "server",
    ]),
    // The test harness, which boots the whole app: above everything, so only tests use it.
    ("harness", &["app::google_test_support", "controllers::presenters::test_support"]),
];

#[test]
fn modules_name_only_their_own_layer_and_below() {
    let tree = ModuleTree::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    // A walk that stopped early would pass vacuously.
    assert!(tree.modules.len() > 550, "found only {} modules", tree.modules.len());
    let violations = tree.violations();
    assert!(
        violations.is_empty(),
        "{} references to a higher layer (move the code down, or see plans/crate-split-plan.md):\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

#[test]
fn an_upward_reference_is_a_violation() {
    let tree = ModuleTree::from_sources(&[
        ("main.rs", "mod huddle;\nmod controllers;\n#[cfg(test)]\nmod tests;"),
        ("huddle.rs", "use crate::controllers::rooms::pins::list;\nfn f() { super::controllers::rooms::show(); }"),
        ("controllers.rs", "pub mod rooms;"),
        ("controllers/rooms.rs", "pub mod pins;\nuse crate::huddle;\nmod inline { use super::super::super::huddle::Config; }"),
        ("controllers/rooms/pins.rs", "pub fn list() {}"),
        ("tests.rs", "use crate::controllers::rooms;"),
    ]);
    assert_eq!(tree.violations(), [
        "huddle.rs:1 (app) -> crate::controllers::rooms::pins (rooms)",
        "huddle.rs:2 (app) -> crate::controllers::rooms (rooms)",
    ]);
}

struct Module {
    path: String,
    test: bool,
}

/// The modules, and for each file its blanked source and the module each span belongs to.
#[derive(Default)]
struct ModuleTree {
    modules: Vec<Module>,
    module_paths: BTreeSet<String>,
    files: Vec<SourceFile>,
}

struct SourceFile {
    name: String,
    text: String,
    /// `(start, end, module index)`: inline modules come after the span that contains them.
    spans: Vec<(usize, usize, usize)>,
}

trait Sources {
    fn read(&self, file: &str) -> Option<String>;
}

struct Disk(PathBuf);

impl Sources for Disk {
    fn read(&self, file: &str) -> Option<String> {
        std::fs::read_to_string(self.0.join(file)).ok()
    }
}

impl Sources for [(&str, &str)] {
    fn read(&self, file: &str) -> Option<String> {
        self.iter().find(|(name, _)| *name == file).map(|(_, text)| (*text).to_owned())
    }
}

impl ModuleTree {
    fn read(src: &Path) -> Self {
        Self::walk(&Disk(src.to_owned()))
    }

    fn from_sources(sources: &[(&str, &str)]) -> Self {
        Self::walk(sources)
    }

    fn walk(sources: &(impl Sources + ?Sized)) -> Self {
        let mut tree = Self::default();
        tree.add_file(sources, "main.rs", String::new(), false, "");
        tree
    }

    /// Adds `file` as module `path`, then the modules it declares. `dir` is where its
    /// out-of-line children live.
    fn add_file(&mut self, sources: &(impl Sources + ?Sized), file: &str, path: String, test: bool, dir: &str) {
        let Some(raw) = sources.read(file) else {
            panic!("module file {file} is missing");
        };
        let text = strip(&raw);
        let index = self.files.len();
        self.files.push(SourceFile { name: file.to_owned(), text: text.clone(), spans: Vec::new() });
        let file_dir = file.rfind('/').map_or("", |slash| &file[..=slash]).to_owned();
        let block = Block { file: index, raw: &raw, text: &text, start: 0, end: text.len(), inline: false };
        self.add_block(sources, &block, path, test, dir, &file_dir);
    }

    /// Adds the module `path` spanning `block`, then the modules declared in it. `file_dir` is
    /// the directory of the file it's in, which `#[path]` attributes outside inline modules
    /// are relative to.
    fn add_block(
        &mut self,
        sources: &(impl Sources + ?Sized),
        block: &Block<'_>,
        path: String,
        test: bool,
        dir: &str,
        file_dir: &str,
    ) {
        let module = self.modules.len();
        self.module_paths.insert(path.clone());
        self.modules.push(Module { path: path.clone(), test });
        self.files[block.file].spans.push((block.start, block.end, module));
        let declaration = declaration();
        let mut at = block.start;
        while let Some(found) = declaration.captures_at(&block.text[..block.end], at) {
            let whole = found.get(0).expect("match");
            let name = &found["name"];
            let attrs = &found["attrs"];
            let child_test = test || attrs.contains("cfg(test)") || attrs.contains("cfg(all(test");
            let child = if path.is_empty() { name.to_owned() } else { format!("{path}::{name}") };
            if &found["end"] == "{" {
                let close = matching_brace(block.text, whole.end() - 1);
                let inner = Block { start: whole.end(), end: close, inline: true, ..*block };
                self.add_block(sources, &inner, child, child_test, &format!("{dir}{name}/"), file_dir);
                at = close;
                continue;
            }
            at = whole.end();
            // `#[path]` strings were blanked, so read them from the same span of the source.
            let attrs_span = found.name("attrs").expect("attrs");
            let (child_file, child_dir) = match path_attribute().captures(&block.raw[attrs_span.range()]) {
                // A file named by `#[path]` declares its children beside it, like a `mod.rs`.
                Some(attr) => {
                    let base = if block.inline { dir } else { file_dir };
                    let child_file = normalize(&format!("{base}{}", &attr[1]));
                    let child_dir = child_file.rfind('/').map_or("", |slash| &child_file[..=slash]).to_owned();
                    (child_file, child_dir)
                }
                None if sources.read(&format!("{dir}{name}.rs")).is_some() => {
                    (format!("{dir}{name}.rs"), format!("{dir}{name}/"))
                }
                None => (format!("{dir}{name}/mod.rs"), format!("{dir}{name}/")),
            };
            // Only files under `src/` belong to the crate's layers.
            if !child_file.starts_with("../") {
                self.add_file(sources, &child_file, child, child_test, &child_dir);
            }
        }
    }

    /// Every reference from an ordinary module to a module in a higher layer.
    fn violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        for file in &self.files {
            for (at, path) in references(&file.text) {
                let module = &self.modules[file.module_at(at)];
                if module.test {
                    continue;
                }
                let Some(target) = self.resolve(&module.path, &path) else { continue };
                // A path that names no module under the root names an item of `main.rs` or a
                // module `main.rs` brings in from a crate below (`use campfire_app::config`),
                // whose boundary the compiler checks.
                if target.is_empty() {
                    continue;
                }
                let (from, to) = (layer(&module.path), layer(&target));
                if to.0 > from.0 {
                    let line = file.text[..at].matches('\n').count() + 1;
                    violations.push(format!("{}:{line} ({}) -> crate::{target} ({})", file.name, from.1, to.1));
                }
            }
        }
        violations.sort();
        violations.dedup();
        violations
    }

    /// The longest module a `crate::`, `super::` or `self::` path names from `module`.
    fn resolve(&self, module: &str, path: &str) -> Option<String> {
        let mut segments = path.split("::");
        let mut base: Vec<&str> = module.split("::").filter(|s| !s.is_empty()).collect();
        let mut next = segments.next();
        match next {
            Some("crate") => base.clear(),
            Some("self") => {}
            Some("super") => {
                while next == Some("super") {
                    base.pop()?;
                    next = segments.next();
                }
                // `next` is the first segment after the `super`s; put it back.
                return self.longest_module(base, next.into_iter().chain(segments));
            }
            _ => return None,
        }
        self.longest_module(base, segments)
    }

    fn longest_module<'a>(&self, base: Vec<&str>, segments: impl Iterator<Item = &'a str>) -> Option<String> {
        let mut resolved = base.join("::");
        for segment in segments {
            let candidate = if resolved.is_empty() { segment.to_owned() } else { format!("{resolved}::{segment}") };
            if !self.module_paths.contains(&candidate) {
                break;
            }
            resolved = candidate;
        }
        Some(resolved)
    }
}

/// A module's span of a file's source (`raw`) and of its blanked copy (`text`).
#[derive(Clone, Copy)]
struct Block<'a> {
    file: usize,
    raw: &'a str,
    text: &'a str,
    start: usize,
    end: usize,
    inline: bool,
}

impl SourceFile {
    /// The innermost module whose span contains `at`.
    fn module_at(&self, at: usize) -> usize {
        self.spans
            .iter()
            .rev()
            .find(|(start, end, _)| (*start..*end).contains(&at))
            .map_or(self.spans[0].2, |span| span.2)
    }
}

/// `(index, name)` of the layer `module` is in.
fn layer(module: &str) -> (usize, &'static str) {
    let mut best: Option<(usize, usize, &'static str)> = None;
    for (index, (name, prefixes)) in LAYERS.iter().enumerate() {
        for prefix in *prefixes {
            let (base, children_only) = match prefix.strip_suffix("::*") {
                Some(base) => (base, true),
                None => (*prefix, false),
            };
            let matches = if base.is_empty() {
                true
            } else if children_only {
                module.starts_with(&format!("{base}::"))
            } else {
                module == base || module.starts_with(&format!("{base}::"))
            };
            let weight = base.len() * 2 + usize::from(children_only);
            if matches && best.is_none_or(|(_, w, _)| weight > w) {
                best = Some((index, weight, name));
            }
        }
    }
    best.map(|(index, _, name)| (index, name)).expect("the server layer covers every module")
}

/// `mod name;` or `mod name {`, with its attributes.
fn declaration() -> Regex {
    Regex::new(r"(?m)^[ \t]*(?P<attrs>(?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<end>[;{])")
        .expect("declaration pattern")
}

fn path_attribute() -> Regex {
    Regex::new(r#"#\[\s*path\s*=\s*"([^"]+)"\s*\]"#).expect("path pattern")
}

/// Each `crate::`, `super::` and `self::` path in `text` with its offset; a `use` group's
/// branches (`crate::{a::b, c::{d, self}}`) are read as separate paths at the group's offset.
fn references(text: &str) -> Vec<(usize, String)> {
    let reference = Regex::new(
        r"(?:^|[^A-Za-z0-9_$:])(?P<path>(?:crate|super|self)(?:\s*::\s*[A-Za-z_][A-Za-z0-9_]*)*)(?P<group>\s*::\s*\{)?",
    )
    .expect("reference pattern");
    let mut references = Vec::new();
    let mut at = 0;
    while let Some(found) = reference.captures_at(text, at) {
        let path = found.name("path").expect("path");
        let prefix: String = path.as_str().split_whitespace().collect();
        match found.name("group") {
            Some(group) => {
                let close = matching_brace(text, group.end() - 1);
                let mut branches = Vec::new();
                use_branches(&format!("{prefix}::"), &text[group.end()..close.min(text.len())], &mut branches);
                references.extend(branches.into_iter().map(|branch| (path.start(), branch)));
                at = close.min(text.len());
            }
            None => {
                if prefix.contains("::") {
                    references.push((path.start(), prefix));
                }
                at = path.end();
            }
        }
    }
    references
}

fn matching_brace(text: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (offset, byte) in text.bytes().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return offset;
                }
            }
            _ => {}
        }
    }
    text.len()
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

/// The source with comments, string and character literals blanked, keeping every offset and
/// newline.
fn strip(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let blank = |out: &mut [u8], from: usize, to: usize| {
        for byte in &mut out[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let ident = |at: usize| at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"//") {
            let end = rest.iter().position(|b| *b == b'\n').map_or(bytes.len(), |p| i + p);
            blank(&mut out, i, end);
            i = end;
        } else if rest.starts_with(b"/*") {
            let (mut depth, mut j) = (1, i + 2);
            while j < bytes.len() && depth > 0 {
                if bytes[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if bytes[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if !ident(i) && raw_string_start(rest).is_some() {
            let (prefix, hashes) = raw_string_start(rest).expect("raw string");
            let mut close = vec![b'"'];
            close.extend(std::iter::repeat_n(b'#', hashes));
            let body = i + prefix;
            let end = bytes[body..]
                .windows(close.len())
                .position(|window| window == close.as_slice())
                .map_or(bytes.len(), |p| body + p + close.len());
            blank(&mut out, i, end);
            i = end;
        } else if rest.starts_with(b"\"") || (!ident(i) && rest.starts_with(b"b\"")) {
            let mut j = i + if rest[0] == b'b' { 2 } else { 1 };
            while j < bytes.len() && bytes[j] != b'"' {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(bytes.len());
            blank(&mut out, i, end);
            i = end;
        } else if rest.starts_with(b"'") {
            // A character literal ('x', '\n', '\u{7b}'), not a lifetime ('a).
            let end = if rest.get(1) == Some(&b'\\') {
                rest.iter().skip(2).position(|b| *b == b'\'').map(|p| i + 2 + p + 1)
            } else {
                let width = source[i + 1..].chars().next().map_or(1, char::len_utf8);
                (rest.get(1 + width) == Some(&b'\'')).then_some(i + 2 + width)
            };
            match end {
                Some(end) => {
                    blank(&mut out, i, end);
                    i = end;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(out).expect("blanking keeps UTF-8 boundaries")
}

/// `r"`, `r#"`, `br##"` and so on: the prefix's length and its number of hashes.
fn raw_string_start(rest: &[u8]) -> Option<(usize, usize)> {
    let after_b = usize::from(rest.first() == Some(&b'b'));
    if rest.get(after_b) != Some(&b'r') {
        return None;
    }
    let hashes = rest[after_b + 1..].iter().take_while(|b| **b == b'#').count();
    (rest.get(after_b + 1 + hashes) == Some(&b'"')).then_some((after_b + 2 + hashes, hashes))
}

/// The full paths of a `use` group's branches under `prefix` (which ends in `::`).
fn use_branches(prefix: &str, body: &str, branches: &mut Vec<String>) {
    let mut depth = 0;
    let mut start = 0;
    let mut items = Vec::new();
    for (offset, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&body[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    items.push(&body[start..]);
    for item in items {
        let item = item.trim();
        if let Some(open) = item.find('{') {
            let head: String = item[..open].split_whitespace().collect();
            let inner = item[open + 1..].strip_suffix('}').unwrap_or(&item[open + 1..]);
            use_branches(&format!("{prefix}{head}"), inner, branches);
            continue;
        }
        let words: Vec<&str> = item.split_whitespace().collect();
        let leaf: String = words.split(|word| *word == "as").next().unwrap_or_default().concat();
        match leaf.as_str() {
            "" | "*" => {}
            "self" => branches.push(prefix.trim_end_matches("::").to_owned()),
            leaf => branches.push(format!("{prefix}{}", leaf.trim_end_matches("::*"))),
        }
    }
}


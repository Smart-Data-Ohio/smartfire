//! Embedded files, found by path and content-negotiated.

use crate::embedded;

/// A file of the dist, as `build.rs` embedded it.
#[derive(Debug, PartialEq, Eq)]
pub struct File {
    /// Its path under `/app/`: `assets/index-B2x8Kq1f.js`.
    pub path: &'static str,
    pub content_type: &'static str,
    /// Named after its content, so cacheable forever.
    pub immutable: bool,
    pub identity: &'static [u8],
    pub br: Option<&'static [u8]>,
    pub gz: Option<&'static [u8]>,
}

/// A file in the encoding chosen for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Served {
    pub file: &'static File,
    /// `Content-Encoding`: `br` or `gzip`, `None` for the file itself.
    pub content_encoding: Option<&'static str>,
    pub body: &'static [u8],
}

impl Served {
    /// Whether the response depends on `Accept-Encoding` (`Vary`): the file has encodings.
    pub fn varies(&self) -> bool {
        self.file.br.is_some() || self.file.gz.is_some()
    }
}

/// The embedded file at `path` (under `/app/`, no leading slash), in the best encoding
/// `accept_encoding` allows. `index.html` isn't one: it's the shell's template.
pub fn file(path: &str, accept_encoding: Option<&str>) -> Option<Served> {
    find(embedded::FILES, path).map(|file| negotiate(file, accept_encoding))
}

pub(crate) fn find(files: &'static [File], path: &str) -> Option<&'static File> {
    files.binary_search_by(|file| file.path.cmp(path)).ok().map(|index| &files[index])
}

/// Brotli, then gzip, then the file itself; a client's higher q-value wins over that order.
pub(crate) fn negotiate(file: &'static File, accept_encoding: Option<&str>) -> Served {
    let accepted = parse(accept_encoding.unwrap_or(""));
    let mut best: Option<(f32, &'static str, &'static [u8])> = None;
    for (coding, body) in [("br", file.br), ("gzip", file.gz)] {
        let (Some(body), q) = (body, quality(&accepted, coding)) else { continue };
        if q > 0.0 && best.is_none_or(|(best_q, _, _)| q > best_q) {
            best = Some((q, coding, body));
        }
    }
    match best {
        Some((_, coding, body)) => Served { file, content_encoding: Some(coding), body },
        None => Served { file, content_encoding: None, body: file.identity },
    }
}

/// `Accept-Encoding` as (coding, q) pairs, codings lowercased; `x-gzip` is gzip (RFC 9110 8.4.1.3).
fn parse(header: &str) -> Vec<(String, f32)> {
    header
        .split(',')
        .filter_map(|item| {
            let mut parts = item.split(';');
            let coding = parts.next()?.trim().to_ascii_lowercase();
            if coding.is_empty() {
                return None;
            }
            let q = parts
                .filter_map(|param| {
                    let (name, value) = param.split_once('=')?;
                    name.trim().eq_ignore_ascii_case("q").then(|| value.trim().parse::<f32>().ok()).flatten()
                })
                .next()
                .unwrap_or(1.0);
            let coding = if coding == "x-gzip" { "gzip".to_string() } else { coding };
            Some((coding, q))
        })
        .collect()
}

/// A coding's q-value: its own entry, else `*`'s, else 0.
fn quality(accepted: &[(String, f32)], coding: &str) -> f32 {
    let entry = |name: &str| accepted.iter().find(|(c, _)| c == name).map(|(_, q)| *q);
    entry(coding).or_else(|| entry("*")).unwrap_or(0.0)
}

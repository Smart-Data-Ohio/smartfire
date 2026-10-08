//! The shell: the dist's `index.html` with what a classic layout's head carries for the request.

use crate::boot::{Boot, script_json};
use crate::embedded;

/// Where Vite's `index.html` (and the stub) want the request's tags.
const PLACEHOLDER: &str = "<!--boot-->";

/// `index.html` with, in place of `<!--boot-->`: `csrf_meta_tags` (the masked global token), the
/// `csp_meta_tag`, Turbo's `turbo-visit-control` (reload), and the boot JSON in
/// `<script type="application/json" id="boot">`. With a nonce, every script and `modulepreload`
/// link carries it, as the classic layout's import-map tags do.
pub fn render_shell(boot: &Boot, csrf_token: &str, csp_nonce: Option<&str>) -> String {
    render(embedded::INDEX_HTML, boot, csrf_token, csp_nonce)
}

pub(crate) fn render(template: &str, boot: &Boot, csrf_token: &str, csp_nonce: Option<&str>) -> String {
    let nonce = csp_nonce.map(|nonce| format!(" nonce=\"{}\"", escape(nonce))).unwrap_or_default();
    let mut tags = format!(
        "<meta name=\"csrf-param\" content=\"authenticity_token\" />\n<meta name=\"csrf-token\" content=\"{}\" />\n",
        escape(csrf_token)
    );
    if let Some(csp_nonce) = csp_nonce {
        tags.push_str(&format!("<meta name=\"csp-nonce\" content=\"{}\" />\n", escape(csp_nonce)));
    }
    // A classic page's Turbo Drive visit that ends here (a ported screen's redirect) loads the
    // page in full rather than swapping its body in.
    tags.push_str("<meta name=\"turbo-visit-control\" content=\"reload\" />\n");
    tags.push_str(&format!("<link rel=\"manifest\" href=\"{}manifest.webmanifest\" />\n", crate::root_path()));
    tags.push_str(&format!("<script type=\"application/json\" id=\"boot\"{nonce}>{}</script>", script_json(boot)));

    let page = with_nonce(template, &nonce);
    match page.split_once(PLACEHOLDER) {
        Some((before, after)) => format!("{before}{tags}{after}"),
        None => match page.split_once("</head>") {
            Some((before, after)) => format!("{before}{tags}\n</head>{after}"),
            None => format!("{tags}\n{page}"),
        },
    }
}

/// `nonce` (an attribute, or empty) added to each `<script>` and `<link rel="modulepreload">`.
fn with_nonce(template: &str, nonce: &str) -> String {
    if nonce.is_empty() {
        return template.to_string();
    }
    let mut out = String::with_capacity(template.len() + 8 * nonce.len());
    let mut rest = template;
    while let Some((start, tag)) = ["<script", "<link rel=\"modulepreload\""]
        .iter()
        .filter_map(|tag| next_tag(rest, tag).map(|start| (start, *tag)))
        .min_by_key(|(start, _)| *start)
    {
        let name_end = start + tag.find(' ').unwrap_or(tag.len());
        out.push_str(&rest[..name_end]);
        out.push_str(nonce);
        out.push_str(&rest[name_end..start + tag.len()]);
        rest = &rest[start + tag.len()..];
    }
    out.push_str(rest);
    out
}

/// Where `tag` next starts a tag (`<script>` or `<script `, not `<scripts`).
fn next_tag(text: &str, tag: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find(tag) {
        let start = from + found;
        if text[start + tag.len()..].starts_with([' ', '>', '\n', '\t', '/']) {
            return Some(start);
        }
        from = start + tag.len();
    }
    None
}

/// `ERB::Util.html_escape`
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

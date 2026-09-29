//! Times message_presentation on adversarial bodies. Copied into crates/richtext/examples/ to run:
//! `cargo run --release -p campfire_richtext --example timing -- <kb>...`

use std::time::Instant;

use campfire_richtext::{AttachableResolver, GidLookup, RenderContext, SignedLookup, message_presentation, to_plain_text};

struct NoRecords;

impl AttachableResolver for NoRecords {
    fn locate_signed(&self, _: &str) -> SignedLookup {
        SignedLookup::Invalid
    }
    fn find_gid(&self, _: &str) -> GidLookup {
        GidLookup::NotFound
    }
}

fn main() {
    let ctx = RenderContext { resolver: &NoRecords, request_host: Some("once.campfire.test".into()) };
    let shape = std::env::var("SHAPE").unwrap_or_else(|_| "domains".into());
    for kb in std::env::args().skip(1).map(|a| a.parse::<usize>().unwrap()) {
        let body = match shape.as_str() {
            "domains" => "<p>www.a.com</p>".repeat(kb * 1024 / 16),
            "nested" => nested_content_attachments(kb * 1024),
            _ => panic!("unknown SHAPE"),
        };
        let started = Instant::now();
        let html = message_presentation(&body, &ctx).map(|h| h.len());
        let presented = started.elapsed();
        let started = Instant::now();
        let text = to_plain_text(&body, &ctx).map(|t| t.len());
        println!("{shape} {} bytes: presentation {presented:?} ({html:?}), plain text {:?} ({text:?})", body.len(), started.elapsed());
    }
}

/// Content attachments each holding the next, HTML-escaped into the `content` attribute.
fn nested_content_attachments(bytes: usize) -> String {
    let mut body = String::from("<p>x</p>");
    while body.len() < bytes {
        let escaped = body.replace('&', "&amp;").replace('"', "&quot;");
        let next = format!("<action-text-attachment content-type=\"text/html\" content=\"{escaped}\"></action-text-attachment>");
        if next.len() > bytes {
            break;
        }
        body = next;
    }
    body
}

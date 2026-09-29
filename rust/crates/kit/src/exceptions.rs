//! `ActionDispatch::PublicExceptions`: what an error looks like on the wire in production.
//!
//! JSON, XML and YAML requests get the `{ status:, error: }` hash in that format (the formats the
//! hash responds to `to_<format>` for); everything else gets `public/<status>.html`, or an empty
//! body with the status when that file doesn't exist. HEAD gets an empty body in the request's
//! format, `*/*` included.
//!
//! Unlike rendered responses (`charset=utf-8`), these say `charset=UTF-8`: PublicExceptions and
//! `ShowExceptions#pass_response` interpolate `ActionDispatch::Response.default_charset`, which
//! the railtie sets from `config.encoding` (verified against the reference).

use std::collections::HashMap;

use axum::body::Bytes;
use axum::http::{StatusCode, header};

use crate::format::Format;
use crate::response::Response;

const CHARSET: &str = "charset=UTF-8";

/// The `public/<status>.html` pages, held in memory.
#[derive(Debug, Clone, Default)]
pub struct ErrorPages(HashMap<u16, Bytes>);

impl ErrorPages {
    pub fn new(pages: impl IntoIterator<Item = (u16, Bytes)>) -> Self {
        Self(pages.into_iter().collect())
    }

    fn get(&self, status: StatusCode) -> Option<Bytes> {
        self.0.get(&status.as_u16()).cloned()
    }
}

pub fn render(pages: &ErrorPages, status: StatusCode, format: Option<Format>, head: bool) -> Response {
    // `request.formats.first`, `*/*` included (only `formats` failing falls back to HTML).
    let content_type = format.map(|f| f.string).unwrap_or("text/html");
    if head {
        return with_length(status, content_type, Bytes::new());
    }
    let (status_code, error) = (status.as_u16(), reason(status));
    // `body.public_send("to_#{format}")` when the `{ status:, error: }` hash responds to it.
    let body = match format.map(|f| f.symbol) {
        Some("json") => Some(format!(r#"{{"status":{status_code},"error":"{error}"}}"#)),
        Some("xml") => Some(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<hash>\n  <status type=\"integer\">{status_code}</status>\n  <error>{}</error>\n</hash>\n",
            xml_escape(error)
        )),
        Some("yaml") => Some(format!("---\n:status: {status_code}\n:error: {error}\n")),
        _ => None,
    };
    if let Some(body) = body {
        return with_length(status, content_type, Bytes::from(body));
    }
    match pages.get(status) {
        Some(html) => with_length(status, "text/html", html),
        // PublicExceptions answers X-Cascade: pass; ShowExceptions#pass_response turns that into
        // an empty page with the error's status.
        None => with_length(status, "text/html", Bytes::new()),
    }
}

/// `render_format` and `pass_response` set `Content-Length` themselves, so `Rack::Deflater`
/// leaves the empty ones alone (`should_deflate?` skips `Content-Length: 0`).
fn with_length(status: StatusCode, content_type: &str, body: Bytes) -> Response {
    let length = body.len().to_string();
    Response::with_body(status, &format!("{content_type}; {CHARSET}"), body).header(header::CONTENT_LENGTH, &length)
}

/// Builder's text escaping (`Hash#to_xml`).
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `Rack::Utils::HTTP_STATUS_CODES`.
fn reason(status: StatusCode) -> &'static str {
    match status.as_u16() {
        422 => "Unprocessable Content",
        413 => "Content Too Large",
        _ => status.canonical_reason().unwrap_or("Internal Server Error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format;

    #[test]
    fn json_errors() {
        let response = render(&ErrorPages::default(), StatusCode::UNPROCESSABLE_ENTITY, Some(&format::JSON), false);
        assert_eq!(response.body_bytes().unwrap().as_ref(), br#"{"status":422,"error":"Unprocessable Content"}"#);
    }

    #[test]
    fn html_pages() {
        let pages = ErrorPages::new([(404, Bytes::from("<h1>404</h1>"))]);
        let response = render(&pages, StatusCode::NOT_FOUND, Some(&format::HTML), false);
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert_eq!(response.body_bytes().unwrap().as_ref(), b"<h1>404</h1>");
        assert_eq!(response.headers[axum::http::header::CONTENT_TYPE], "text/html; charset=UTF-8");
        let missing = render(&pages, StatusCode::BAD_REQUEST, None, false);
        assert_eq!(missing.status, StatusCode::BAD_REQUEST);
        assert!(missing.body_bytes().unwrap().is_empty());
    }

    #[test]
    fn xml_and_yaml_errors() {
        let xml = render(&ErrorPages::default(), StatusCode::NOT_ACCEPTABLE, Some(&format::XML), false);
        assert_eq!(
            xml.body_bytes().unwrap().as_ref(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<hash>\n  <status type=\"integer\">406</status>\n  <error>Not Acceptable</error>\n</hash>\n"
        );
        assert_eq!(xml.headers[header::CONTENT_TYPE], "application/xml; charset=UTF-8");
        assert_eq!(xml.headers[header::CONTENT_LENGTH], "124");
        let yaml = render(&ErrorPages::default(), StatusCode::NOT_ACCEPTABLE, Some(&format::YAML), false);
        assert_eq!(yaml.body_bytes().unwrap().as_ref(), b"---\n:status: 406\n:error: Not Acceptable\n");
    }

    #[test]
    fn head_errors_are_empty_in_the_request_format() {
        let head = render(&ErrorPages::default(), StatusCode::NOT_FOUND, Some(&format::ALL), true);
        assert_eq!(head.headers[header::CONTENT_TYPE], "*/*; charset=UTF-8");
        assert_eq!(head.headers[header::CONTENT_LENGTH], "0");
        let head = render(&ErrorPages::default(), StatusCode::NOT_FOUND, None, true);
        assert_eq!(head.headers[header::CONTENT_TYPE], "text/html; charset=UTF-8");
    }
}

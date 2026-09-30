//! RoomMailbox's deliberately narrow Authentication-Results and Nokogiri HTML5 contracts.
use crate::ruby::{blank, regex_space, strip};
use campfire_richtext::dom::{Dom, NodeData, NodeId};
use mailparse::{MailAddr, MailHeaderMap, ParsedMail};
use regex::Regex;
use std::sync::LazyLock;

/// Last successful depth in pinned production Rails (Mail 2.9.1, 1 MiB Ruby VM stack).
/// Depth 1,752 raises SystemStackError; the WS10 decision makes that a terminal bounce.
/// Root multipart counts as one. See vectors/mail/mime-depth.json and its reference tool.
pub const MAX_MIME_DEPTH: usize = 1_751;

pub fn authenticated_sender(headers: &[String], authserv_id: Option<&str>, address: &str) -> bool {
    let Some(authserv_id) = authserv_id.filter(|s| !blank(s)) else {
        return false;
    };
    let domain = address.rsplit('@').next().unwrap_or("").to_lowercase();
    if blank(&domain) {
        return false;
    }
    let Some(header) = headers
        .iter()
        .find(|h| strip(h.split(';').next().unwrap_or("")).eq_ignore_ascii_case(authserv_id))
    else {
        return false;
    };
    static PROPERTIES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(header\.d|header\.from|smtp\.mailfrom)=([^\x09-\x0d ;()]+)").unwrap()
    });
    header.split(';').skip(1).any(|clause| {
        let Some((method, rest)) = strip(clause).split_once('=') else {
            return false;
        };
        if !strip(rest)
            .split(|c: char| regex_space(c) || c == '(')
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("pass")
        {
            return false;
        }
        let property = match strip(method).to_lowercase().as_str() {
            "dkim" => "header.d",
            "dmarc" => "header.from",
            "spf" => "smtp.mailfrom",
            _ => return false,
        };
        PROPERTIES.captures_iter(clause).any(|cap| {
            cap[1].eq_ignore_ascii_case(property)
                && cap[2]
                    .trim_start_matches('@')
                    .rsplit('@')
                    .next()
                    .is_some_and(|d| d.eq_ignore_ascii_case(&domain))
        })
    })
}

pub fn html_to_text(html: &str) -> anyhow::Result<String> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html)?;
    fn text(dom: &Dom, id: NodeId, out: &mut String) {
        let tag = dom.element(id).map(|e| e.name.local.as_ref());
        if tag.is_some_and(|t| matches!(t, "script" | "style" | "head" | "template")) {
            return;
        }
        if tag == Some("br") {
            out.push('\n');
            return;
        }
        if let NodeData::Text(s) = &dom.node(id).data {
            out.push_str(s);
        }
        for &child in dom.children(id) {
            text(dom, child, out);
        }
        if tag.is_some_and(|t| {
            matches!(
                t,
                "p" | "div"
                    | "li"
                    | "tr"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "blockquote"
                    | "pre"
            )
        }) {
            out.push('\n');
        }
    }
    let mut out = String::new();
    text(&dom, root, &mut out);
    static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new("[ \\t]+\\n").unwrap());
    static LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new("\\n{3,}").unwrap());
    Ok(strip(&LINES.replace_all(&SPACE.replace_all(&out, "\n"), "\n\n")).to_owned())
}

pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct File {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub verdict: Verdict,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    TooBig,
    Disallowed,
}
pub fn attachment_verdict(content_type: &str, decoded: usize, encoded: usize) -> Verdict {
    if encoded.saturating_mul(3) / 4 > MAX_ATTACHMENT_BYTES * 105 / 100
        || decoded > MAX_ATTACHMENT_BYTES
    {
        return Verdict::TooBig;
    }
    let t = content_type.to_lowercase();
    if [
        "image/",
        "text/",
        "application/vnd.openxmlformats-officedocument",
        "application/vnd.oasis.opendocument",
    ]
    .iter()
    .any(|p| t.starts_with(p))
        || [
            "application/pdf",
            "application/rtf",
            "application/msword",
            "application/vnd.ms-excel",
            "application/vnd.ms-powerpoint",
        ]
        .contains(&t.as_str())
    {
        Verdict::Ok
    } else {
        Verdict::Disallowed
    }
}
#[derive(Debug, Clone)]
pub struct Email {
    pub recipients: Vec<String>,
    pub from: Option<String>,
    pub sender_name: Option<String>,
    pub subject: String,
    pub body: String,
    pub auth_headers: Vec<String>,
    pub message_id: Option<String>,
    pub files: Vec<File>,
    /// Multipart containers along the deepest inspected path; routing may stop at its cutoff.
    pub mime_depth: usize,
}
fn addresses(mail: &ParsedMail<'_>, key: &str) -> Vec<mailparse::SingleInfo> {
    mail.headers
        .get_all_values(key)
        .iter()
        .filter_map(|h| mailparse::addrparse(h).ok())
        .flat_map(|list| list.into_inner())
        .flat_map(|a| match a {
            MailAddr::Single(s) => vec![s],
            MailAddr::Group(g) => g.addrs,
        })
        .collect()
}

fn authentication_values(raw: &[u8]) -> Vec<String> {
    // Mail::Header applies Utilities.to_crlf before splitting fields. Mailparse strips
    // leading bare CR while parsing a field, so rebuild the physical fields first.
    let mut normalized = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'\r' {
            normalized.push(b'\n');
            if raw.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
        } else {
            normalized.push(raw[index]);
        }
        index += 1;
    }
    let mut fields: Vec<Vec<u8>> = Vec::new();
    for line in normalized.split(|b| *b == b'\n') {
        if matches!(line.first(), Some(b' ' | b'\t'))
            && let Some(field) = fields.last_mut()
        {
            field.extend_from_slice(b"\r\n");
            field.extend_from_slice(line);
        } else if !line.is_empty() {
            fields.push(line.to_vec());
        }
    }
    fields
        .iter()
        .filter_map(|field| mailparse::parse_header(field).ok())
        .filter(|(header, _)| {
            header
                .get_key_ref()
                .eq_ignore_ascii_case("Authentication-Results")
        })
        // RoomMailbox reads field.value, not field.decoded. Keep encoded words and
        // Unicode whitespace intact; Mail::Field.split applies Ruby String#strip.
        .map(|(header, _)| strip(&String::from_utf8_lossy(header.get_value_raw())).to_owned())
        .collect()
}

#[cfg(test)]
thread_local! {
    static BOUNDARY_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn boundary_work(_units: usize) {
    #[cfg(test)]
    BOUNDARY_WORK.with(|count| count.set(count.get() + _units));
}

#[derive(Clone, Copy)]
struct DelimiterLine {
    start: usize,
    end: usize,
    next_line: Option<usize>,
}

#[derive(Default)]
struct BoundaryNode {
    lines: Vec<DelimiterLine>,
    cursor: usize,
    children: Option<std::collections::HashMap<u8, usize>>,
}

/// Index line prefixes, retaining mailparse's prefix matching (not whole-line matching).
/// Expand a prefix only when a MIME header requests it. Each delimiter byte is indexed
/// at most once, including when boundary names share prefixes or are reused by siblings.
struct BoundaryIndex<'a> {
    raw: &'a [u8],
    nodes: Vec<BoundaryNode>,
}

impl<'a> BoundaryIndex<'a> {
    fn new(raw: &'a [u8], body: usize) -> Self {
        let mut root = BoundaryNode::default();
        let mut start = body;
        boundary_work(raw.len() - body);
        for line in raw[body..].split_inclusive(|byte| *byte == b'\n') {
            boundary_work(line.len().min(2));
            if line.starts_with(b"--") {
                let next_line = line.ends_with(b"\n").then_some(start + line.len());
                root.lines.push(DelimiterLine {
                    start,
                    end: next_line.map_or(raw.len(), |next| next - 1),
                    next_line,
                });
            }
            start += line.len();
        }
        Self {
            raw,
            nodes: vec![root],
        }
    }

    fn boundary_node(&mut self, boundary: &[u8]) -> Option<usize> {
        if self.nodes[0].lines.is_empty() {
            return None;
        }
        let mut node = 0;
        for (offset, byte) in boundary.iter().enumerate() {
            boundary_work(1);
            if self.nodes[node].children.is_none() {
                let mut children = std::collections::HashMap::new();
                for index in 0..self.nodes[node].lines.len() {
                    let line = self.nodes[node].lines[index];
                    let position = line.start + 2 + offset;
                    boundary_work(1);
                    if position < line.end {
                        let key = self.raw[position];
                        let child = *children.entry(key).or_insert_with(|| {
                            self.nodes.push(BoundaryNode::default());
                            self.nodes.len() - 1
                        });
                        self.nodes[child].lines.push(line);
                    }
                }
                self.nodes[node].children = Some(children);
            }
            node = *self.nodes[node].children.as_ref()?.get(byte)?;
        }
        Some(node)
    }

    fn parts(&mut self, body: usize, end: usize, boundary: &str) -> Vec<std::ops::Range<usize>> {
        let Some(node) = self.boundary_node(boundary.as_bytes()) else {
            return Vec::new();
        };
        let node = &mut self.nodes[node];
        // The explicit stack visits headers in source order. A cursor per prefix avoids
        // even repeated binary searches when many sibling multiparts reuse a boundary.
        while node
            .lines
            .get(node.cursor)
            .is_some_and(|line| line.start < body)
        {
            boundary_work(1);
            node.cursor += 1;
        }
        let delimiter_len = 2 + boundary.len();
        let in_part = |line: &&DelimiterLine| line.start + delimiter_len <= end;
        let Some(mut current) = node.lines.get(node.cursor).filter(in_part).copied() else {
            return Vec::new();
        };
        node.cursor += 1;
        boundary_work(1);
        let mut parts = Vec::new();
        while let Some(start) = current.next_line.filter(|start| *start <= end) {
            let next = node.lines.get(node.cursor).filter(in_part).copied();
            let mut part_end = next.map_or(end, |line| line.start);
            if next.is_some() && part_end > start && self.raw[part_end - 1] == b'\n' {
                part_end -= 1;
                if part_end > start && self.raw[part_end - 1] == b'\r' {
                    part_end -= 1;
                }
            }
            parts.push(start..part_end);
            let Some(next) = next else { break };
            node.cursor += 1;
            boundary_work(3);
            let cursor = next.start + delimiter_len;
            if cursor + 2 > end || self.raw[cursor..end].starts_with(b"--") {
                break;
            }
            current = next;
        }
        parts
    }
}

/// Mail 2.9.1 splits multipart bodies lazily and has no numeric depth limit. Mailparse's
/// eager recursive tree can exhaust the process stack before Action Mailbox routes a bounce.
/// Traverse indexed part offsets on an explicit stack; only leaves enter parse_mail.
fn mime_leaves(
    raw: &[u8],
    depth_limit: Option<usize>,
) -> anyhow::Result<(ParsedMail<'_>, Vec<ParsedMail<'_>>, bool, usize)> {
    fn shallow(raw: &[u8]) -> anyhow::Result<(ParsedMail<'_>, usize)> {
        let (_, body) = mailparse::parse_headers(raw)?;
        Ok((mailparse::parse_mail(&raw[..body])?, body))
    }
    let (root, body) = shallow(raw)?;
    let mut boundaries = BoundaryIndex::new(raw, body);
    let mut stack = vec![(0..raw.len(), false, 0)];
    let mut leaves = Vec::new();
    let mut multipart = false;
    let mut mime_depth = 0;
    while let Some((range, digest, depth)) = stack.pop() {
        let part = &raw[range.clone()];
        let (head, body) = shallow(part)?;
        let children = if head.ctype.mimetype.starts_with("multipart/") {
            head.ctype
                .params
                .get("boundary")
                .map(|b| boundaries.parts(range.start + body, range.end, b))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if range.start == 0 && range.end == raw.len() {
            multipart = !children.is_empty();
        }
        if children.is_empty() {
            // No children means parse_mail cannot recurse, including a malformed multipart
            // without a delimiter. Preserve multipart/digest's default MIME type.
            let mut leaf = mailparse::parse_mail(part)?;
            if digest && leaf.headers.get_first_value("Content-Type").is_none() {
                leaf.ctype.mimetype = "message/rfc822".into();
            }
            leaves.push(leaf);
        } else {
            let depth = depth + 1;
            mime_depth = mime_depth.max(depth);
            if depth_limit.is_some_and(|limit| depth > limit) {
                // Preserve the headers and excessive depth for the terminal routing bounce.
                // Nothing from an over-depth body is decoded or staged as an attachment.
                return Ok((root, Vec::new(), true, mime_depth));
            }
            let digest = head.ctype.mimetype == "multipart/digest";
            stack.extend(children.into_iter().rev().map(|part| (part, digest, depth)));
        }
    }
    Ok((root, leaves, multipart, mime_depth))
}
impl Email {
    /// Action Mailbox routes recipients before splitting the body. Non-room mail goes
    /// straight to BounceMailbox, even when its body contains deeply nested MIME.
    pub fn parse_for_routing(raw: &[u8]) -> anyhow::Result<Self> {
        let (_, body) = mailparse::parse_headers(raw)?;
        let headers = Self::parse(&raw[..body])?;
        if headers.room_token().is_none() {
            return Ok(headers);
        }
        Self::parse_with_depth_limit(raw, Some(MAX_MIME_DEPTH))
    }
    pub fn parse(raw: &[u8]) -> anyhow::Result<Self> {
        Self::parse_with_depth_limit(raw, None)
    }
    fn parse_with_depth_limit(raw: &[u8], depth_limit: Option<usize>) -> anyhow::Result<Self> {
        let (mail, leaves, multipart, mime_depth) = mime_leaves(raw, depth_limit)?;
        let recipients = ["To", "Cc", "Bcc"]
            .iter()
            .flat_map(|h| addresses(&mail, h))
            .map(|s| s.addr)
            .collect();
        let from = addresses(&mail, "From").into_iter().next();
        let mut files = Vec::new();
        let mut plain = None;
        let mut html = None;
        for part in &leaves {
            let d = part.get_content_disposition();
            let name = d
                .params
                .get("filename")
                .or_else(|| part.ctype.params.get("name"));
            if let Some(name) = name {
                // Reject the encoded size before allocating the decoded attachment, as Ruby does.
                let encoded = match part.get_body_encoded() {
                    mailparse::body::Body::Base64(b)
                    | mailparse::body::Body::QuotedPrintable(b) => b.get_raw().len(),
                    mailparse::body::Body::SevenBit(b) | mailparse::body::Body::EightBit(b) => {
                        b.get_raw().len()
                    }
                    mailparse::body::Body::Binary(b) => b.get_raw().len(),
                };
                let bytes = if encoded.saturating_mul(3) / 4 > MAX_ATTACHMENT_BYTES * 105 / 100 {
                    Vec::new()
                } else {
                    part.get_body_raw()?
                };
                let verdict = attachment_verdict(&part.ctype.mimetype, bytes.len(), encoded);
                files.push(File {
                    filename: name.clone(),
                    content_type: part.ctype.mimetype.clone(),
                    bytes,
                    verdict,
                });
            } else if part.ctype.mimetype.eq_ignore_ascii_case("text/plain") && plain.is_none() {
                plain = Some(part.get_body()?);
            } else if part.ctype.mimetype.eq_ignore_ascii_case("text/html") && html.is_none() {
                html = Some(part.get_body()?);
            }
        }
        let body = if let Some(plain) = plain {
            plain
        } else if let Some(html) = html {
            html_to_text(&html)?
        } else if !multipart {
            leaves
                .first()
                .map(|p| p.get_body())
                .transpose()?
                .unwrap_or_default()
        } else {
            String::new()
        };
        Ok(Self {
            recipients,
            from: from
                .as_ref()
                .map(|s| strip(&s.addr).to_owned())
                .filter(|s| !blank(s)),
            sender_name: from
                .and_then(|s| s.display_name)
                .map(|s| strip(&s).to_owned())
                .filter(|s| !blank(s)),
            subject: mail.headers.get_first_value("Subject").unwrap_or_default(),
            body: strip(&body.replace("\r\n", "\n").replace('\r', "\n")).to_owned(),
            auth_headers: authentication_values(mail.raw_bytes),
            message_id: mail.headers.get_first_value("Message-ID").map(|s| {
                strip(&s)
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_owned()
            }),
            files,
            mime_depth,
        })
    }
    pub fn room_token(&self) -> Option<String> {
        static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)room-(.+)@").unwrap());
        self.recipients
            .iter()
            .find_map(|a| TOKEN.captures(a))
            .map(|c| strip(&c[1]).to_owned())
            .filter(|s| !blank(s))
    }
    pub fn sender_display(&self) -> String {
        match (&self.sender_name, &self.from) {
            (Some(n), Some(a)) if n != a => format!("{n} <{a}>"),
            (_, Some(a)) => a.clone(),
            _ => "unknown sender".into(),
        }
    }
    pub fn attachment_note(&self) -> Option<String> {
        let names = self
            .files
            .iter()
            .filter(|f| !blank(strip(&f.filename)))
            .map(|f| {
                let reason = match f.verdict {
                    Verdict::Ok => return strip(&f.filename).to_owned(),
                    Verdict::TooBig => "over the 10 MB limit",
                    Verdict::Disallowed => "file type not allowed",
                };
                format!("{} (not attached: {reason})", strip(&f.filename))
            })
            .collect::<Vec<_>>();
        (!names.is_empty()).then(|| format!("Attached files: {}", names.join(", ")))
    }
    pub fn source(&self, member: bool) -> Option<String> {
        let note = self.attachment_note();
        if blank(&self.subject) && blank(&self.body) && note.is_none() {
            return None;
        }
        let mut parts = Vec::new();
        if !member {
            parts.push(format!("From {}", self.sender_display()));
        }
        if !blank(&self.subject) {
            parts.push(format!("**{}**", self.subject));
        }
        if !blank(&self.body) {
            parts.push(self.body.clone());
        }
        if let Some(note) = note {
            parts.push(note);
        }
        let source = parts.join("\n\n");
        Some(if source.chars().count() > 50_000 {
            source.chars().take(49_997).collect::<String>() + "..."
        } else {
            source
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_wide_room_has_linear_boundary_work() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
        let fixture = &corpus["review"]["wide"];
        let mut raw = fixture["prefix"].as_str().unwrap().to_owned();
        raw.push_str(
            &fixture["part"]
                .as_str()
                .unwrap()
                .repeat(fixture["parts"].as_u64().unwrap() as usize),
        );
        raw.push_str(fixture["suffix"].as_str().unwrap());
        BOUNDARY_WORK.with(|count| count.set(0));
        let email = Email::parse_for_routing(raw.as_bytes()).unwrap();
        let scanned = BOUNDARY_WORK.with(std::cell::Cell::get);
        println!(
            "wide fixture bytes={} boundary work units={scanned}",
            raw.len()
        );
        assert!(scanned <= 4 * raw.len());
        assert_eq!(email.body, "x");
        assert_eq!(email.room_token().as_deref(), Some("token"));
    }

    #[test]
    fn review_deep_room_has_linear_boundary_work() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
        let raw = corpus["review"]["deep_fixed_width_raw"]
            .as_str()
            .unwrap()
            .replace("nobody@mail.test", "room-token@mail.test");
        assert_eq!(raw.len(), 324_088);
        for bounded in [false, true] {
            BOUNDARY_WORK.with(|count| count.set(0));
            let email = if bounded {
                Email::parse_for_routing(raw.as_bytes())
            } else {
                Email::parse(raw.as_bytes())
            }
            .unwrap();
            let work = BOUNDARY_WORK.with(std::cell::Cell::get);
            println!(
                "room fixture bytes={} bounded={bounded} boundary work units={work}",
                raw.len()
            );
            assert!(
                work <= 4 * raw.len(),
                "quadratic boundary traversal: {work} work units for {} input bytes",
                raw.len()
            );
            assert_eq!(email.room_token().as_deref(), Some("token"));
            assert_eq!(email.body, if bounded { "" } else { "Hello" });
            assert_eq!(
                email.mime_depth,
                if bounded { MAX_MIME_DEPTH + 1 } else { 4_000 }
            );
            assert!(email.files.is_empty());
        }
    }

    #[test]
    fn review_deep_bounce_has_linear_boundary_work() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
        let raw = corpus["review"]["deep_fixed_width_raw"]
            .as_str()
            .unwrap()
            .as_bytes();
        assert_eq!(raw.len(), 324_084);
        BOUNDARY_WORK.with(|count| count.set(0));
        let email = Email::parse_for_routing(raw).unwrap();
        let scanned = BOUNDARY_WORK.with(std::cell::Cell::get);
        println!("fixture bytes={} boundary work units={scanned}", raw.len());
        assert!(
            scanned <= raw.len(),
            "quadratic boundary traversal: scanned {scanned} bytes for {} input bytes",
            raw.len()
        );
        assert_eq!(scanned, 0, "BounceMailbox must not split the body");
        assert!(email.room_token().is_none());
        assert!(email.body.is_empty());
        assert!(email.files.is_empty());
    }
}

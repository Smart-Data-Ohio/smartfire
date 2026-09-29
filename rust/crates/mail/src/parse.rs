//! RoomMailbox's deliberately narrow Authentication-Results and Nokogiri HTML5 contracts.
use campfire_richtext::dom::{Dom, NodeData, NodeId};
use mailparse::{MailAddr, MailHeaderMap, ParsedMail};
use regex::Regex;
use std::sync::LazyLock;

pub fn authenticated_sender(headers: &[String], authserv_id: Option<&str>, address: &str) -> bool {
    let Some(authserv_id) = authserv_id.filter(|s| !s.trim().is_empty()) else {
        return false;
    };
    let domain = address.rsplit('@').next().unwrap_or("").to_lowercase();
    if domain.trim().is_empty() {
        return false;
    }
    let Some(header) = headers.iter().find(|h| {
        h.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case(authserv_id)
    }) else {
        return false;
    };
    static PROPERTIES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(header\.d|header\.from|smtp\.mailfrom)=([^\s;()]+)").unwrap()
    });
    header.split(';').skip(1).any(|clause| {
        let Some((method, rest)) = clause.trim().split_once('=') else {
            return false;
        };
        if !rest
            .trim()
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("pass")
        {
            return false;
        }
        let property = match method.trim().to_lowercase().as_str() {
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
    Ok(LINES
        .replace_all(&SPACE.replace_all(&out, "\n"), "\n\n")
        .trim()
        .to_owned())
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
impl Email {
    pub fn parse(raw: &[u8]) -> anyhow::Result<Self> {
        let mail = mailparse::parse_mail(raw)?;
        let recipients = ["To", "Cc", "Bcc"]
            .iter()
            .flat_map(|h| addresses(&mail, h))
            .map(|s| s.addr)
            .collect();
        let from = addresses(&mail, "From").into_iter().next();
        let mut files = Vec::new();
        fn parts<'a>(mail: &'a ParsedMail<'a>, leaves: &mut Vec<&'a ParsedMail<'a>>) {
            if mail.subparts.is_empty() {
                leaves.push(mail);
            } else {
                for part in &mail.subparts {
                    parts(part, leaves);
                }
            }
        }
        let mut leaves = Vec::new();
        parts(&mail, &mut leaves);
        let mut plain = None;
        let mut html = None;
        for part in leaves {
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
        } else if mail.subparts.is_empty() {
            mail.get_body()?
        } else {
            String::new()
        };
        Ok(Self {
            recipients,
            from: from
                .as_ref()
                .map(|s| s.addr.trim().to_owned())
                .filter(|s| !s.is_empty()),
            sender_name: from
                .and_then(|s| s.display_name)
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
            subject: mail.headers.get_first_value("Subject").unwrap_or_default(),
            body: body
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .trim()
                .to_owned(),
            auth_headers: mail.headers.get_all_values("Authentication-Results"),
            message_id: mail.headers.get_first_value("Message-ID").map(|s| {
                s.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_owned()
            }),
            files,
        })
    }
    pub fn room_token(&self) -> Option<String> {
        static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)room-(.+)@").unwrap());
        self.recipients
            .iter()
            .find_map(|a| TOKEN.captures(a))
            .map(|c| c[1].trim().to_owned())
            .filter(|s| !s.is_empty())
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
            .filter(|f| !f.filename.trim().is_empty())
            .map(|f| {
                let reason = match f.verdict {
                    Verdict::Ok => return f.filename.trim().to_owned(),
                    Verdict::TooBig => "over the 10 MB limit",
                    Verdict::Disallowed => "file type not allowed",
                };
                format!("{} (not attached: {reason})", f.filename.trim())
            })
            .collect::<Vec<_>>();
        (!names.is_empty()).then(|| format!("Attached files: {}", names.join(", ")))
    }
    pub fn source(&self, member: bool) -> Option<String> {
        let note = self.attachment_note();
        if self.subject.trim().is_empty() && self.body.trim().is_empty() && note.is_none() {
            return None;
        }
        let mut parts = Vec::new();
        if !member {
            parts.push(format!("From {}", self.sender_display()));
        }
        if !self.subject.trim().is_empty() {
            parts.push(format!("**{}**", self.subject));
        }
        if !self.body.trim().is_empty() {
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

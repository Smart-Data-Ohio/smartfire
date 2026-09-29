//! SecurityMailer, TwoFactorMailer, and the Mail gem's multipart/alternative wire format.
use crate::config::{Config, Smtp};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    address::Envelope,
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        client::{AsyncSmtpConnection, Tls, TlsParameters},
        extension::ClientId,
    },
};

pub const SIGN_IN_SUBJECT: &str = "New sign-in to your Smartfire account";
pub const LOCKOUT_SUBJECT: &str = "Several wrong sign-in codes were entered";
const LOCKOUT: &str = "Several wrong sign-in codes were entered for your account. If that was you, wait a little and try again; if it was not you, your password may be compromised, so change it and review your remembered devices on your profile.";
#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
    pub email: String,
}
#[derive(Debug, Clone)]
pub struct SignIn {
    pub user: User,
    pub device: String,
    pub created_at: jiff::Timestamp,
}
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}
fn h(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn layout(body: &str) -> String {
    include_str!("../templates/layout.html").replace("<%= yield %>", body)
}
fn address_with_name(user: &User) -> String {
    if user.name.trim().is_empty() || user.name == user.email {
        return user.email.clone();
    }
    let name = if !user.name.is_ascii() {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(&user.name))
    } else if user.name.chars().any(|c| {
        matches!(
            c,
            '(' | ')' | '<' | '>' | '@' | ',' | ';' | ':' | '\\' | '"' | '.' | '[' | ']'
        )
    }) {
        format!(
            "\"{}\"",
            user.name.replace('\\', "\\\\").replace('"', "\\\"")
        )
    } else {
        user.name.clone()
    };
    format!("{name} <{}>", user.email)
}
pub fn new_sign_in_alert(config: &Config, item: Option<&SignIn>) -> Option<Message> {
    let item = item?;
    let date = item
        .created_at
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%B %-d, %Y at %-I:%M %p %Z")
        .to_string();
    let url = format!(
        "{}/users/me/sessions",
        config.app_url.as_deref().unwrap_or("http://example.com")
    );
    let text = format!(
        "Hi {},\n\nNew sign-in to your account from {}, {date}. Wasn't you? Review your sessions:\n\n{url}\n\n",
        item.user.name, item.device
    );
    let body = format!(
        "<p>Hi {},</p>\n\n<p>\n  New sign-in to your account from {},\n  {}.\n  Wasn't you? <a href=\"{}\">Review your sessions</a>.\n</p>\n",
        h(&item.user.name),
        h(&item.device),
        h(&date),
        h(&url)
    );
    Some(Message {
        from: config
            .mailer_from
            .clone()
            .unwrap_or_else(|| "Smartfire <noreply@example.com>".into()),
        to: address_with_name(&item.user),
        subject: SIGN_IN_SUBJECT.into(),
        text,
        html: layout(&body),
    })
}
pub fn lockout_notice(user: &User) -> Message {
    Message {
        from: "Smartfire <noreply@smartdata.net>".into(),
        to: user.email.clone(),
        subject: LOCKOUT_SUBJECT.into(),
        text: format!("Hi {},\n\n{LOCKOUT}\n\n", user.name),
        html: layout(&format!(
            "<p>Hi {},</p>\n\n<p>{LOCKOUT}</p>\n",
            h(&user.name)
        )),
    }
}
fn crlf(s: &str) -> String {
    s.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}
/// Ruby String#pack('M'): soft breaks after 73 bytes, never inside an =XX escape.
fn quoted_printable(s: &str) -> String {
    let mut out = String::new();
    let mut column = 0;
    for line in s
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split_inclusive('\n')
    {
        let terminated = line.ends_with('\n');
        let data = line.trim_end_matches('\n').as_bytes();
        for (i, &b) in data.iter().enumerate() {
            let atom = if ((33..=60).contains(&b) || (62..=126).contains(&b))
                || ((b == b' ' || b == b'\t') && i + 1 != data.len())
            {
                String::from(b as char)
            } else {
                format!("={b:02X}")
            };
            if column + atom.len() > 73 {
                out.push_str("=\r\n");
                column = 0;
            }
            out.push_str(&atom);
            column += atom.len();
        }
        if terminated {
            out.push_str("\r\n");
            column = 0;
        }
    }
    if column > 0 {
        out.push_str("=\r\n");
    }
    out
}
impl Message {
    /// Date/Message-ID/boundary are injectable for reference goldens; production mints fresh ids.
    pub fn encoded(
        &self,
        now: jiff::Timestamp,
        message_id: &str,
        boundary: &str,
    ) -> anyhow::Result<Vec<u8>> {
        for value in [&self.from, &self.to, &self.subject, message_id, boundary] {
            anyhow::ensure!(
                !value.contains(['\r', '\n']),
                "mail header contains a newline"
            );
        }
        let date = now
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime("%a, %d %b %Y %H:%M:%S %z");
        let mut raw = format!(
            "Date: {date}\r\nFrom: {}\r\nTo: {}\r\nMessage-ID: <{message_id}>\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative;\r\n boundary=\"{boundary}\";\r\n charset=UTF-8\r\nContent-Transfer-Encoding: 7bit\r\n\r\n",
            self.from, self.to, self.subject
        );
        for (content_type, body) in [("text/plain", &self.text), ("text/html", &self.html)] {
            let (encoding, body) = if body.is_ascii() && body.lines().all(|l| l.len() <= 998) {
                ("7bit", crlf(body))
            } else {
                ("quoted-printable", quoted_printable(body))
            };
            raw.push_str(&format!("\r\n--{boundary}\r\nContent-Type: {content_type};\r\n charset=UTF-8\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}"));
        }
        raw.push_str(&format!("\r\n--{boundary}--\r\n"));
        Ok(raw.into_bytes())
    }
}
pub fn transport(settings: &Smtp) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.address)
        .port(settings.port)
        .timeout(Some(std::time::Duration::from_secs(5)));
    builder = builder.hello_name(ClientId::Domain(
        settings
            .domain
            .clone()
            .unwrap_or_else(|| "localhost.localdomain".into()),
    ));
    if settings.enable_starttls {
        builder = builder.tls(Tls::Required(TlsParameters::new(settings.address.clone())?));
    }
    anyhow::ensure!(
        settings.user_name.is_none() || settings.password.is_some(),
        "SMTP_PASSWORD is required when SMTP_USER_NAME is configured"
    );
    if let (Some(user), Some(password)) = (&settings.user_name, &settings.password) {
        let mechanism = match settings.authentication.as_deref().unwrap_or("plain") {
            "plain" => Mechanism::Plain,
            "login" => Mechanism::Login,
            other => anyhow::bail!("unsupported SMTP_AUTHENTICATION {other}"),
        };
        builder = builder
            .credentials(Credentials::new(user.clone(), password.clone()))
            .authentication(vec![mechanism]);
    }
    Ok(builder.build())
}
pub async fn deliver(
    settings: &Smtp,
    message: &Message,
    now: jiff::Timestamp,
) -> anyhow::Result<()> {
    let token = hex::encode(rand::random::<[u8; 12]>());
    let domain = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".into());
    let raw = message.encoded(
        now,
        &format!("{:x}_{token}@{domain}.mail", now.as_second()),
        &format!("--==_mimepart_{token}"),
    )?;
    let first_address = |header: &str| -> anyhow::Result<lettre::Address> {
        let list = mailparse::addrparse(header)?.into_inner();
        match list.into_iter().next() {
            Some(mailparse::MailAddr::Single(info)) => Ok(info.addr.parse()?),
            _ => anyhow::bail!("missing mail address"),
        }
    };
    let envelope = Envelope::new(
        Some(first_address(&message.from)?),
        vec![first_address(&message.to)?],
    )?;
    if settings.user_name.is_some() && settings.authentication.as_deref() == Some("cram_md5") {
        deliver_cram_md5(settings, &envelope, &raw).await?;
    } else {
        transport(settings)?.send_raw(&envelope, &raw).await?;
    }
    Ok(())
}

/// Mail/Net::SMTP also supports CRAM-MD5, which Lettre's high-level SASL enum omits.
/// Its low-level connection still owns SMTP framing, STARTTLS and certificate verification.
async fn deliver_cram_md5(settings: &Smtp, envelope: &Envelope, raw: &[u8]) -> anyhow::Result<()> {
    let work = async {
        let hello = ClientId::Domain(
            settings
                .domain
                .clone()
                .unwrap_or_else(|| "localhost.localdomain".into()),
        );
        let mut connection = AsyncSmtpConnection::connect_tokio1(
            (settings.address.as_str(), settings.port),
            Some(std::time::Duration::from_secs(5)),
            &hello,
            None,
            None,
        )
        .await?;
        if settings.enable_starttls {
            connection
                .starttls(TlsParameters::new(settings.address.clone())?, &hello)
                .await?;
        }
        let response = connection.command("AUTH CRAM-MD5\r\n").await?;
        anyhow::ensure!(
            response.has_code(334),
            "SMTP did not issue a CRAM-MD5 challenge"
        );
        let challenge = STANDARD.decode(response.first_word().unwrap_or(""))?;
        let username = settings
            .user_name
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing SMTP_USER_NAME"))?;
        let password = settings
            .password
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing SMTP_PASSWORD"))?;
        let answer = cram_md5_response(username, password, &challenge);
        let response = connection.command(format!("{answer}\r\n")).await?;
        anyhow::ensure!(
            response.has_code(235),
            "SMTP did not accept CRAM-MD5 authentication"
        );
        connection.send(envelope, raw).await?;
        let _ = connection.quit().await;
        Ok::<_, anyhow::Error>(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), work).await?
}
pub fn cram_md5_response(username: &str, password: &str, challenge: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    let mut mac =
        Hmac::<md5::Md5>::new_from_slice(password.as_bytes()).expect("HMAC accepts any key length");
    mac.update(challenge);
    STANDARD.encode(format!(
        "{username} {}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

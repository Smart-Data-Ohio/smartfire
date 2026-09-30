//! config/initializers/{outbound_mail,default_url_options}.rb and Room's inbound ENV.
use crate::ruby::{blank, regex_space};
use base64::{
    Engine as _, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use subtle::ConstantTimeEq;

#[derive(Clone, Default)]
pub struct Config {
    pub domain: Option<String>,
    pub authserv_id: Option<String>,
    pub ingress_password: Option<String>,
    pub smtp: Option<Smtp>,
    pub mailer_from: Option<String>,
    pub app_url: Option<String>,
}
#[derive(Clone, Default)]
pub struct Smtp {
    pub address: String,
    pub port: u16,
    pub domain: Option<String>,
    pub user_name: Option<String>,
    pub password: Option<String>,
    pub authentication: Option<String>,
    pub enable_starttls: bool,
}
impl std::fmt::Debug for Smtp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Smtp")
            .field("address", &self.address)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_map(&std::env::vars().collect())
    }
    pub fn from_map(values: &HashMap<String, String>) -> anyhow::Result<Self> {
        let get = |key| values.get(key).filter(|s| !blank(s)).cloned();
        let app_url = get("APP_URL").map(|origin| {
            let uri = url::Url::parse(&origin)?;
            anyhow::ensure!(matches!(uri.scheme(), "http" | "https") && uri.host_str().is_some() && uri.username().is_empty() && uri.password().is_none() && uri.query().is_none() && uri.fragment().is_none() && uri.path() == "/", "APP_URL must be an http or https origin without credentials, a path, query, or fragment");
            Ok::<_, anyhow::Error>(uri.as_str().trim_end_matches('/').to_owned())
        }).transpose()?;
        let smtp = get("SMTP_ADDRESS").map(|address| Smtp {
            address,
            port: values
                .get("SMTP_PORT")
                .map_or(25, |port| ruby_to_i(port) as u16),
            domain: get("SMTP_DOMAIN"),
            user_name: get("SMTP_USER_NAME"),
            password: get("SMTP_PASSWORD"),
            authentication: get("SMTP_AUTHENTICATION"),
            enable_starttls: values
                .get("SMTP_ENABLE_STARTTLS")
                .is_none_or(|s| s != "false"),
        });
        Ok(Self {
            domain: get("INBOUND_EMAIL_DOMAIN"),
            authserv_id: get("INBOUND_EMAIL_AUTHSERV_ID"),
            ingress_password: get("RAILS_INBOUND_EMAIL_PASSWORD"),
            smtp,
            mailer_from: get("MAILER_FROM"),
            app_url,
        })
    }
    pub fn security_configured(&self) -> bool {
        self.smtp.is_some() && self.mailer_from.is_some() && self.app_url.is_some()
    }
    pub fn two_factor_configured(&self) -> bool {
        self.smtp.is_some()
    }
}
fn ruby_to_i(s: &str) -> i64 {
    let s = s.trim_start_matches(regex_space);
    let n = s
        .char_indices()
        .skip(usize::from(s.starts_with(['+', '-'])))
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(s.len(), |(i, _)| i);
    s[..n].parse().unwrap_or(0)
}
#[derive(Debug, PartialEq, Eq)]
pub enum RelayAuth {
    Accepted,
    Disabled,
    MissingPassword,
    Unauthorized,
}
pub fn relay_auth(config: &Config, authorization: Option<&str>) -> RelayAuth {
    if config.domain.is_none() {
        return RelayAuth::Disabled;
    }
    let Some(password) = config.ingress_password.as_deref().filter(|s| !blank(s)) else {
        return RelayAuth::MissingPassword;
    };
    // Rails uses String#split(" ", 2) and permissive Base64.decode64, not strict_decode64.
    let decoded = authorization.and_then(|a| {
        let mut parts = a.trim_start_matches(regex_space).splitn(2, regex_space);
        if !parts.next()?.eq_ignore_ascii_case("Basic") {
            return None;
        }
        let mut token = parts
            .next()
            .unwrap_or("")
            .bytes()
            .take_while(|b| *b != b'=')
            .filter(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/')
            .collect::<Vec<_>>();
        if token.len() % 4 == 1 {
            token.pop();
        }
        GeneralPurpose::new(
            &alphabet::STANDARD,
            GeneralPurposeConfig::new()
                .with_decode_padding_mode(DecodePaddingMode::Indifferent)
                .with_decode_allow_trailing_bits(true),
        )
        .decode(token)
        .ok()
    });
    let Some(decoded) = decoded else {
        return RelayAuth::Unauthorized;
    };
    let expected = format!("actionmailbox:{password}");
    if bool::from(Sha256::digest(&decoded).ct_eq(&Sha256::digest(expected.as_bytes()))) {
        RelayAuth::Accepted
    } else {
        RelayAuth::Unauthorized
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("domain", &self.domain)
            .field("authserv_id", &self.authserv_id)
            .field("smtp", &self.smtp)
            .field("app_url", &self.app_url)
            .finish_non_exhaustive()
    }
}

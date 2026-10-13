//! RFC 7523 client assertions. Only CIMD-owned keys can establish client identity.
use super::client::{self, AuthMethod};
use crate::{app::App, json_cache::CacheSize};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use campfire_db::{Timestamp, Tx};
use campfire_kit::{Error, Result};
use ring::signature;
use rusqlite::params;
use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
};
use serde_json::{Map, Value};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

pub const TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";
const SKEW: i64 = 60;
const LIFETIME: i64 = 300;

// Reject repeated security fields rather than letting the JSON parser pick a winner.
struct Object(Map<String, Value>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: de::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Unique;
        impl<'de> Visitor<'de> for Unique {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object with unique fields")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Object, M::Error> {
                let mut fields = Map::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if fields.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate field"));
                    }
                }
                Ok(Object(fields))
            }
        }
        deserializer.deserialize_map(Unique)
    }
}

pub struct Assertion {
    client_id: String,
    algorithm: String,
    kid: Option<String>,
    signed: String,
    signature: Vec<u8>,
    jti_digest: String,
    expires_at: Timestamp,
}
impl Assertion {
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

fn decode(value: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = URL_SAFE_NO_PAD.decode(value)?;
    anyhow::ensure!(
        URL_SAFE_NO_PAD.encode(&bytes) == value,
        "Noncanonical base64url"
    );
    Ok(bytes)
}

pub fn parse(
    raw: &str,
    client_id: Option<&str>,
    issuer: &str,
    now: Timestamp,
) -> anyhow::Result<Assertion> {
    let parts: Vec<_> = raw.split('.').collect();
    anyhow::ensure!(parts.len() == 3, "Expected one compact signed JWT");
    let Object(header) = serde_json::from_slice(&decode(parts[0])?)?;
    anyhow::ensure!(
        !["jwk", "jku", "x5u", "x5c", "crit"]
            .iter()
            .any(|field| header.contains_key(*field)),
        "Untrusted or critical header"
    );
    let algorithm = header.get("alg").and_then(Value::as_str).unwrap_or("");
    anyhow::ensure!(
        matches!(algorithm, "RS256" | "ES256"),
        "Unsupported algorithm"
    );
    let kid = header
        .get("kid")
        .map(|value| value.as_str().ok_or_else(|| anyhow::anyhow!("Invalid kid")))
        .transpose()?;
    let Object(claims) = serde_json::from_slice(&decode(parts[1])?)?;
    let iss = claims
        .get("iss")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing issuer"))?;
    anyhow::ensure!(
        claims.get("sub").and_then(Value::as_str) == Some(iss)
            && client_id.is_none_or(|id| id == iss),
        "Client identity mismatch"
    );
    client::metadata_url(iss)?;
    let audience = match claims.get("aud") {
        Some(Value::String(value)) => Some(value.as_str()),
        Some(Value::Array(values)) if values.len() == 1 => values[0].as_str(),
        _ => None,
    };
    let endpoint = format!("{issuer}/oauth/token");
    anyhow::ensure!(
        audience.is_some_and(|aud| aud == issuer || aud == endpoint),
        "Wrong audience"
    );
    let exp = claims
        .get("exp")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("Missing expiry"))?;
    let now = now.as_second();
    anyhow::ensure!(exp > now - SKEW && exp <= now + LIFETIME, "Invalid expiry");
    if let Some(iat) = claims.get("iat") {
        let iat = iat
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("Invalid issued-at"))?;
        anyhow::ensure!(
            iat <= now && (0..=LIFETIME).contains(&exp.saturating_sub(iat)),
            "Invalid issued-at"
        );
    }
    if let Some(nbf) = claims.get("nbf") {
        anyhow::ensure!(
            nbf.as_i64().is_some_and(|nbf| nbf <= now + SKEW),
            "Not yet valid"
        );
    }
    let jti = claims
        .get("jti")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing jti"))?;
    anyhow::ensure!(!jti.is_empty() && jti.chars().count() <= 256, "Invalid jti");
    Ok(Assertion {
        client_id: iss.into(),
        algorithm: algorithm.into(),
        kid: kid.map(str::to_owned),
        signed: format!("{}.{}", parts[0], parts[1]),
        signature: decode(parts[2])?,
        jti_digest: campfire_db::user::digest_bot_token(jti),
        expires_at: Timestamp::from_jiff(jiff::Timestamp::from_second(exp + SKEW)?),
    })
}

#[derive(Clone)]
struct Keys {
    values: Vec<Value>,
    refetch_after: Timestamp,
}
#[derive(Clone, Default)]
struct CacheEntry(Arc<Mutex<Option<Keys>>>);
impl CacheSize for CacheEntry {
    fn cache_size(&self) -> usize {
        // Account for the bounded JSON document and its parsed representation.
        128 * 1024
    }
}

pub struct Verified {
    oauth_client_id: i64,
    jti_digest: String,
    expires_at: Timestamp,
}
impl Verified {
    /// The single SQLite writer makes concurrent assertions and restarts obey one-use semantics.
    pub fn consume(self, tx: &Tx<'_>) -> campfire_db::Result<bool> {
        if self.expires_at <= tx.now() {
            return Ok(false);
        }
        Ok(tx.conn().execute(
            "INSERT INTO oauth_assertion_jtis(oauth_client_id,jti_digest,expires_at) VALUES(?,?,?) ON CONFLICT(oauth_client_id,jti_digest) DO NOTHING",
            params![self.oauth_client_id, self.jti_digest, self.expires_at],
        )? == 1)
    }
}

pub async fn verify(
    app: &App,
    assertion: Assertion,
    mut may_fetch: impl FnMut() -> Result<()>,
) -> Result<Verified> {
    let id = assertion.client_id.clone();
    let digest = assertion.jti_digest.clone();
    let replay = app.db.read(move |conn| {
        Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM oauth_assertion_jtis j JOIN oauth_clients c ON c.id=j.oauth_client_id WHERE c.client_id=? AND j.jti_digest=?)", params![id, digest], |r| r.get::<_, bool>(0))?)
    }).await.map_err(Error::internal)?;
    if replay {
        return Err(Error::NotFound);
    }
    let client = client::identify(app, &assertion.client_id, &mut may_fetch).await?;
    if client.registration != "cimd" || !client::allows(&client, AuthMethod::PrivateKeyJwt) {
        return Err(Error::NotFound);
    }
    let uri = client.jwks_uri.as_deref().ok_or(Error::NotFound)?;
    let url = url::Url::parse(uri).map_err(|_| Error::NotFound)?;
    let origin = client::metadata_url(&client.client_id)
        .map_err(|_| Error::NotFound)?
        .origin();
    if uri.len() > 2048
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.origin() != origin
    {
        return Err(Error::NotFound);
    }
    let entry = app
        .json_cache
        .fetch_value(&format!("oauth:jwks:{uri}"), CacheEntry::default);
    let mut cache = entry.0.lock().await;
    let now = Timestamp::from_jiff(app.clock.now());
    if cache.as_ref().is_none_or(|keys| keys.refetch_after <= now) {
        *cache = Some(fetch_keys(app, &url, &mut may_fetch).await?);
    }
    let mut key = select_key(&cache.as_ref().expect("fetched keys").values, &assertion);
    // Only an unknown explicit kid allows one rotation fetch. Bad signatures never do.
    if matches!(key, Selection::UnknownKid) {
        *cache = Some(fetch_keys(app, &url, &mut may_fetch).await?);
        key = select_key(&cache.as_ref().expect("refetched keys").values, &assertion);
    }
    let Selection::Key(key) = key else {
        return Err(Error::NotFound);
    };
    verify_signature(key, &assertion).map_err(|_| Error::NotFound)?;
    Ok(Verified {
        oauth_client_id: client.id,
        jti_digest: assertion.jti_digest,
        expires_at: assertion.expires_at,
    })
}

async fn fetch_keys(
    app: &App,
    url: &url::Url,
    may_fetch: &mut impl FnMut() -> Result<()>,
) -> Result<Keys> {
    may_fetch()?;
    let (value, seconds) = client::fetch_json(
        &app.subscription_network,
        url,
        &["application/jwk-set+json", "application/json"],
    )
    .await
    .map_err(|_| Error::NotFound)?;
    let values = value
        .get("keys")
        .and_then(Value::as_array)
        .filter(|values| values.iter().all(Value::is_object))
        .ok_or(Error::NotFound)?
        .clone();
    Ok(Keys {
        values,
        refetch_after: Timestamp::from_jiff(app.clock.now())
            .since(jiff::SignedDuration::from_secs(seconds)),
    })
}

enum Selection<'a> {
    Key(&'a Value),
    UnknownKid,
    Invalid,
}
fn select_key<'a>(keys: &'a [Value], assertion: &Assertion) -> Selection<'a> {
    let mut selected = keys.iter().filter(|key| match &assertion.kid {
        Some(kid) => key["kid"].as_str() == Some(kid.as_str()),
        None => matches!(
            (assertion.algorithm.as_str(), key["kty"].as_str()),
            ("RS256", Some("RSA")) | ("ES256", Some("EC"))
        ),
    });
    match (selected.next(), selected.next()) {
        (Some(key), None) => Selection::Key(key),
        (None, _) if assertion.kid.is_some() => Selection::UnknownKid,
        _ => Selection::Invalid,
    }
}

fn verify_signature(key: &Value, assertion: &Assertion) -> anyhow::Result<()> {
    anyhow::ensure!(
        key.get("alg")
            .is_none_or(|alg| alg.as_str() == Some(&assertion.algorithm))
            && key
                .get("use")
                .is_none_or(|usage| usage.as_str() == Some("sig"))
            && key.get("key_ops").is_none_or(|ops| ops
                .as_array()
                .is_some_and(|ops| ops.iter().any(|op| op.as_str() == Some("verify")))),
        "Key is not for this signature"
    );
    let component = |name| {
        decode(
            key[name]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing key component"))?,
        )
    };
    let message = assertion.signed.as_bytes();
    match (assertion.algorithm.as_str(), key["kty"].as_str()) {
        ("RS256", Some("RSA")) => {
            let n = component("n")?;
            let e = component("e")?;
            anyhow::ensure!(
                n.first().is_some_and(|first| *first != 0)
                    && n.len() * 8 - n[0].leading_zeros() as usize >= 2048,
                "RSA key too small"
            );
            signature::RsaPublicKeyComponents { n, e }
                .verify(
                    &signature::RSA_PKCS1_2048_8192_SHA256,
                    message,
                    &assertion.signature,
                )
                .map_err(|_| anyhow::anyhow!("Invalid RSA signature"))?;
        }
        ("ES256", Some("EC")) => {
            anyhow::ensure!(key["crv"].as_str() == Some("P-256"), "Wrong EC curve");
            let x = component("x")?;
            let y = component("y")?;
            anyhow::ensure!(x.len() == 32 && y.len() == 32, "Invalid EC point");
            let point = [vec![4], x, y].concat();
            signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, point)
                .verify(message, &assertion.signature)
                .map_err(|_| anyhow::anyhow!("Invalid EC signature"))?;
        }
        _ => anyhow::bail!("Algorithm/key mismatch"),
    }
    Ok(())
}

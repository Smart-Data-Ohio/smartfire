//! Client ID Metadata Documents, fetched without redirects through the pinned public network.
use crate::{
    app::App,
    net::{
        self, Network,
        http::{self, Body, Endpoint, Request, Timeouts},
    },
};
use campfire_db::{Timestamp, models::oauth::OAuthClient};
use serde_json::Value;
use std::{io, net::IpAddr, time::Duration};
use url::Url;

const DEADLINE: Duration = Duration::from_secs(5);
const LIMIT: usize = 16 * 1024;

pub fn metadata_url(value: &str) -> anyhow::Result<Url> {
    let url = Url::parse(value)?;
    anyhow::ensure!(
        value.len() <= 2048
            && url.scheme() == "https"
            && url.host_str().is_some()
            && url.path() != "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none(),
        "Invalid client metadata URL"
    );
    Ok(url)
}

pub fn redirect_url(value: &str) -> Option<Url> {
    let url = Url::parse(value).ok()?;
    (value.len() <= 2048
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "https" || loopback(&url)))
    .then_some(url)
}

pub fn loopback(url: &Url) -> bool {
    url.scheme() == "http"
        && match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        }
}

pub fn redirect_matches(registered: &str, requested: &str) -> bool {
    let (Some(mut registered_url), Some(mut requested_url)) =
        (redirect_url(registered), redirect_url(requested))
    else {
        return false;
    };
    if registered == requested {
        return true;
    }
    if !loopback(&registered_url) || !loopback(&requested_url) {
        return false;
    }
    let _ = registered_url.set_port(None);
    let _ = requested_url.set_port(None);
    registered_url == requested_url
}

pub fn allows_none(client: &OAuthClient) -> bool {
    serde_json::from_str::<Vec<String>>(&client.auth_methods)
        .is_ok_and(|methods| methods.iter().any(|method| method == "none"))
}

struct PublicResolver<'a>(&'a dyn net::Resolver);
impl net::Resolver for PublicResolver<'_> {
    fn lookup<'a>(&'a self, host: &'a str) -> net::BoxFuture<'a, io::Result<Vec<IpAddr>>> {
        Box::pin(async move {
            let ips = self.0.lookup(host).await?;
            if ips.iter().any(|ip| net::guard::blocked_address(*ip)) {
                return Err(io::Error::other(
                    "Client metadata resolves to an internal address",
                ));
            }
            Ok(ips)
        })
    }
}

pub struct Document {
    pub value: Value,
    pub redirect_uris: Vec<String>,
    pub methods: Vec<String>,
    pub cache_seconds: i64,
}

/// Includes DNS, TLS, headers and the bounded body in one total deadline.
pub async fn fetch(network: &Network, client_id: &str) -> anyhow::Result<Document> {
    let url = metadata_url(client_id)?;
    tokio::time::timeout(DEADLINE, async {
        let host = url.host_str().expect("validated host");
        let pinned_ip =
            net::guard::resolve(&PublicResolver(network.resolver.as_ref()), host).await?;
        let endpoint = Endpoint {
            https: true,
            host: host.into(),
            port: url.port_or_known_default().expect("https port"),
            pinned_ip: Some(pinned_ip),
        };
        let mut target = url.path().to_string();
        if let Some(query) = url.query() {
            target.push('?');
            target.push_str(query);
        }
        let request = Request::net_http(
            hyper::Method::GET,
            target,
            Some(endpoint.host_header()),
            vec![
                ("Accept".into(), "application/json".into()),
                ("Accept-Encoding".into(), "identity".into()),
            ],
        );
        let response = http::exchange(
            network,
            &endpoint,
            request,
            &Timeouts {
                open: DEADLINE,
                read: DEADLINE,
                write: DEADLINE,
            },
        )
        .await?;
        anyhow::ensure!(
            response.status == 200
                && response
                    .content_type()
                    .is_some_and(|t| t.eq_ignore_ascii_case("application/json")),
            "Client metadata must be a JSON 200 response"
        );
        let cache_seconds = cache_seconds(response.header("cache-control").as_deref());
        anyhow::ensure!(
            response
                .content_length()?
                .is_none_or(|length| length <= LIMIT as u64),
            "Client metadata exceeds 16 KB"
        );
        let Body::Complete(bytes) = response.read_body(LIMIT).await? else {
            anyhow::bail!("Client metadata exceeds 16 KB")
        };
        let value: Value = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(
            value["client_id"].as_str() == Some(client_id),
            "Client ID mismatch"
        );
        anyhow::ensure!(
            value["client_name"]
                .as_str()
                .is_some_and(|name| !name.trim().is_empty() && name.len() <= 100),
            "Client name required"
        );
        let redirect_uris: Vec<String> = serde_json::from_value(value["redirect_uris"].clone())?;
        anyhow::ensure!(
            !redirect_uris.is_empty()
                && redirect_uris.iter().all(|uri| redirect_url(uri).is_some()),
            "Invalid redirect URIs"
        );
        let methods = if let Some(array) = value.get("token_endpoint_auth_methods_supported") {
            serde_json::from_value::<Vec<String>>(array.clone())?
        } else {
            vec![
                value
                    .get("token_endpoint_auth_method")
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| anyhow::anyhow!("Invalid auth method"))
                    })
                    .transpose()?
                    .unwrap_or("client_secret_basic")
                    .into(),
            ]
        };
        Ok(Document {
            value,
            redirect_uris,
            methods,
            cache_seconds,
        })
    })
    .await?
}

fn cache_seconds(header: Option<&str>) -> i64 {
    header
        .and_then(|header| {
            header.split(',').find_map(|part| {
                let (key, value) = part.trim().split_once('=')?;
                key.eq_ignore_ascii_case("max-age")
                    .then(|| value.trim_matches('"').parse::<i64>().ok())
                    .flatten()
            })
        })
        .unwrap_or(300)
        .clamp(300, 86400)
}

pub async fn identify(
    app: &App,
    client_id: &str,
    may_fetch: impl FnOnce() -> campfire_kit::Result<()>,
) -> campfire_kit::Result<OAuthClient> {
    use campfire_kit::Error;
    use rusqlite::params;
    let id = client_id.to_string();
    let cached = app
        .db
        .read(move |conn| OAuthClient::find_by_client_id(conn, &id))
        .await
        .map_err(Error::internal)?;
    if let Some(client) = cached {
        if client.blocked_at.is_some() {
            return Err(Error::NotFound);
        }
        if client.registration == "dcr"
            || client
                .refetch_after
                .is_some_and(|at| at > Timestamp::from_jiff(app.clock.now()))
        {
            return Ok(client);
        }
    }
    metadata_url(client_id).map_err(|_| Error::NotFound)?;
    may_fetch()?;
    let document = fetch(&app.subscription_network, client_id)
        .await
        .map_err(|_| Error::NotFound)?;
    let id = client_id.to_string();
    app.db.write(move |tx| {
        let sql = "INSERT INTO oauth_clients(client_id,registration,client_name,client_uri,redirect_uris,auth_methods,jwks_uri,metadata,fetched_at,refetch_after,created_at,updated_at) VALUES(?,'cimd',?,?,?,?,?,?,?,?,?,?) ON CONFLICT(client_id) DO UPDATE SET client_name=excluded.client_name,client_uri=excluded.client_uri,redirect_uris=excluded.redirect_uris,auth_methods=excluded.auth_methods,jwks_uri=excluded.jwks_uri,metadata=excluded.metadata,fetched_at=excluded.fetched_at,refetch_after=excluded.refetch_after,updated_at=excluded.updated_at";
        tx.conn().execute(sql, params![id,document.value["client_name"].as_str(),document.value["client_uri"].as_str(),serde_json::json!(document.redirect_uris).to_string(),serde_json::json!(document.methods).to_string(),document.value["jwks_uri"].as_str(),document.value.to_string(),tx.now(),tx.now().since(jiff::SignedDuration::from_secs(document.cache_seconds)),tx.now(),tx.now()])?;
        OAuthClient::find_by_client_id(tx.conn(),&id)?.filter(|client|client.blocked_at.is_none()).ok_or(campfire_db::Error::RecordNotFound("OAuthClient"))
    }).await.map_err(Error::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;
    use std::sync::Arc;

    struct StalledResolver;
    impl net::Resolver for StalledResolver {
        fn lookup<'a>(&'a self, _host: &'a str) -> net::BoxFuture<'a, io::Result<Vec<IpAddr>>> {
            Box::pin(std::future::pending())
        }
    }
    #[tokio::test(start_paused = true)]
    async fn oauth_cimd_total_deadline_includes_stalled_dns() {
        let network = Network {
            resolver: Arc::new(StalledResolver),
            ..Network::system()
        };
        let request = fetch(&network, "https://client.example/metadata.json");
        tokio::pin!(request);
        assert!(request.as_mut().now_or_never().is_none());
        tokio::time::advance(Duration::from_millis(4999)).await;
        assert!(request.as_mut().now_or_never().is_none());
        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(request.now_or_never().expect("5-second deadline").is_err());
    }
    #[test]
    fn oauth_cimd_cache_lifetime_is_clamped() {
        assert_eq!(cache_seconds(None), 300);
        assert_eq!(cache_seconds(Some("public, max-age=1")), 300);
        assert_eq!(cache_seconds(Some("max-age=600")), 600);
        assert_eq!(cache_seconds(Some("max-age=999999999")), 86400);
    }
}

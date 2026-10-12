//! KLIPY's native API and bounded per-app request state.
//! Contract: https://docs.klipy.com/gifs-api and /search-suggestions-and-autocomplete.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

use hmac::{Hmac, Mac};
use hyper::Method;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use tokio::sync::Semaphore;

use crate::net::{
    Network,
    http::{self, Body, Endpoint, Request, Timeouts},
};

pub const DEFAULT_BASE: &str = "https://api.klipy.com";
/// Hosts the media proxy will fetch from (KLIPY's media CDN).
pub const MEDIA_HOSTS: &[&str] = &["static.klipy.com", "static1.klipy.com", "static2.klipy.com"];
const TIMEOUTS: Timeouts = Timeouts {
    open: Duration::from_secs(5),
    read: Duration::from_secs(10),
    write: Duration::from_secs(5),
};
const CACHE_TTL: Duration = Duration::from_secs(60);
const CACHE_ENTRIES: usize = 512;
const RATE_ENTRIES: usize = 10_000;
const SAFETY: &str = "content_filter=high&rating=pg";
const JSON_LIMIT: usize = 2 * 1024 * 1024;

/// The opaque per-user value sent as KLIPY's `customer_id`: an HMAC-SHA256 of
/// the user id with the server's secret, hex encoded. Stable per user, and not
/// reversible to the user id without the secret.
pub fn customer_id(secret: &str, user_id: i64) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(format!("klipy-customer:{user_id}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A KLIPY failure the caller can only report as "unavailable": a transport
/// error, a non-2xx status, `result != true`, or an unparsable body.
#[derive(Debug)]
pub struct Unavailable;

/// One size/format rendition of a GIF, as KLIPY returned it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Rendition {
    pub url: String,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    #[serde(default)]
    pub size: i64,
}

/// A GIF item with its renditions: `file[size][format]`, sizes `hd|md|sm|xs`,
/// formats `gif|webp|jpg|mp4|webm`.
#[derive(Debug, Clone)]
pub struct Item {
    /// KLIPY's numeric id as a string (it can exceed 2^53).
    pub klipy_id: String,
    pub slug: String,
    pub title: String,
    pub files: BTreeMap<String, BTreeMap<String, Rendition>>,
}

impl Item {
    fn rendition(&self, size: &str, format: &str) -> Option<&Rendition> {
        self.files.get(size)?.get(format)
    }

    /// The small animated preview for the picker grid.
    pub fn preview(&self) -> Option<&Rendition> {
        self.rendition("sm", "webp")
            .or_else(|| self.rendition("sm", "gif"))
            .or_else(|| self.rendition("xs", "webp"))
            .or_else(|| self.rendition("xs", "gif"))
            .or_else(|| self.rendition("md", "webp"))
    }

    /// A still frame for reduced-motion display.
    pub fn still(&self) -> Option<&Rendition> {
        self.rendition("sm", "jpg")
            .or_else(|| self.rendition("xs", "jpg"))
            .or_else(|| self.rendition("md", "jpg"))
    }

    /// The rendition to download when the GIF is sent: the smallest reasonable
    /// animated one, preferring webp, within `cap` bytes (renditions with an
    /// unknown size count as within the cap and are re-checked after download).
    pub fn send_rendition(&self, cap: i64) -> Option<(&Rendition, &'static str, &'static str)> {
        const ORDER: &[(&str, &str, &str)] = &[
            ("md", "webp", "webp"),
            ("sm", "webp", "webp"),
            ("md", "mp4", "mp4"),
            ("sm", "mp4", "mp4"),
            ("sm", "gif", "gif"),
            ("md", "gif", "gif"),
            ("hd", "webp", "webp"),
        ];
        ORDER.iter().find_map(|(size, format, ext)| {
            let rendition = self.rendition(size, format)?;
            (rendition.size <= 0 || rendition.size <= cap).then_some((rendition, *format, *ext))
        })
    }
}

#[derive(Debug, Clone)]
pub struct ListPage {
    pub items: Vec<Item>,
    pub page: i64,
    pub has_next: bool,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub query: String,
    pub preview_url: Option<String>,
}

#[derive(Clone)]
enum Cached {
    List(ListPage),
    Categories(Vec<Category>),
    Suggestions(Vec<String>),
}

struct Inner {
    cache: Mutex<HashMap<String, (Instant, Cached)>>,
    hits: Mutex<HashMap<(i64, &'static str), VecDeque<Instant>>>,
    media_slots: Semaphore,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            cache: Mutex::default(),
            hits: Mutex::default(),
            media_slots: Semaphore::new(8),
        }
    }
}

#[derive(Clone, Copy)]
pub enum SuggestionKind {
    Suggestions,
    Autocomplete,
}

impl SuggestionKind {
    fn endpoint(self) -> &'static str {
        match self {
            Self::Suggestions => "search-suggestions",
            Self::Autocomplete => "autocomplete",
        }
    }
}

/// Per-app KLIPY transport, cache and rate limiter. The API key itself lives in
/// `Config` (`KLIPY_API_KEY`).
#[derive(Clone)]
pub struct State {
    pub network: Network,
    pub base: String,
    inner: Arc<Inner>,
}

impl State {
    pub fn new(network: Network, base: String) -> Self {
        Self {
            network,
            base: base.trim_end_matches('/').into(),
            inner: Arc::default(),
        }
    }

    pub fn system() -> Self {
        Self::new(Network::system(), DEFAULT_BASE.into())
    }

    /// Records a call for `user_id` in `bucket` and says whether it is allowed:
    /// at most `limit` calls within the trailing minute.
    pub fn allow(&self, user_id: i64, bucket: &'static str, limit: usize) -> bool {
        let now = Instant::now();
        let mut hits = self.inner.hits.lock().unwrap();
        hits.retain(|_, calls| {
            calls
                .back()
                .is_some_and(|at| now.duration_since(*at) < CACHE_TTL)
        });
        if hits.len() >= RATE_ENTRIES && !hits.contains_key(&(user_id, bucket)) {
            return false;
        }
        let entry = hits.entry((user_id, bucket)).or_default();
        while entry
            .front()
            .is_some_and(|at| now.duration_since(*at) >= CACHE_TTL)
        {
            entry.pop_front();
        }
        if entry.len() >= limit {
            return false;
        }
        entry.push_back(now);
        true
    }

    fn cached(&self, key: &str) -> Option<Cached> {
        let cache = self.inner.cache.lock().unwrap();
        let (at, value) = cache.get(key)?;
        (at.elapsed() < CACHE_TTL).then(|| value.clone())
    }

    fn store(&self, key: String, value: Cached) {
        let mut cache = self.inner.cache.lock().unwrap();
        cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
        if cache.len() >= CACHE_ENTRIES
            && let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest);
        }
        cache.insert(key, (Instant::now(), value));
    }

    async fn get_json(&self, target: String) -> Result<Value, Unavailable> {
        let parsed = campfire_richtext::uri::parse(&self.base).map_err(|_| Unavailable)?;
        let host = parsed
            .host
            .filter(|host| !host.is_empty())
            .ok_or(Unavailable)?;
        let https = parsed.scheme.as_deref() == Some("https");
        let port = parsed
            .port
            .unwrap_or(if https { 443 } else { 80 })
            .try_into()
            .map_err(|_| Unavailable)?;
        let endpoint = Endpoint {
            https,
            host,
            port,
            pinned_ip: None,
        };
        let request = Request::net_http(
            Method::GET,
            target,
            None,
            vec![
                ("Accept".into(), "application/json".into()),
                ("User-Agent".into(), "Smartfire-GIF".into()),
            ],
        )
        .transport(false, &endpoint);
        let response = http::exchange(&self.network, &endpoint, request, &TIMEOUTS)
            .await
            .map_err(|_| Unavailable)?;
        if !(200..300).contains(&response.status) {
            return Err(Unavailable);
        }
        let Body::Complete(body) = response
            .read_body(JSON_LIMIT)
            .await
            .map_err(|_| Unavailable)?
        else {
            return Err(Unavailable);
        };
        serde_json::from_slice(&body).map_err(|_| Unavailable)
    }

    async fn list(&self, target: String, cache_key: String) -> Result<ListPage, Unavailable> {
        if let Some(Cached::List(page)) = self.cached(&cache_key) {
            return Ok(page);
        }
        let value = self.get_json(target).await?;
        let parsed: ListResponse = serde_json::from_value(value).map_err(|_| Unavailable)?;
        let page = parsed.page()?;
        self.store(cache_key, Cached::List(page.clone()));
        Ok(page)
    }

    pub async fn search(
        &self,
        key: &str,
        customer: &str,
        query: &str,
        page: u32,
    ) -> Result<ListPage, Unavailable> {
        let target = format!(
            "/api/v1/{}/gifs/search?q={}&page={page}&per_page=24&customer_id={customer}&{SAFETY}",
            escape_query(key),
            escape_query(query)
        );
        self.list(target.clone(), target).await
    }

    pub async fn trending(
        &self,
        key: &str,
        customer: &str,
        page: u32,
    ) -> Result<ListPage, Unavailable> {
        let target = format!(
            "/api/v1/{}/gifs/trending?page={page}&per_page=24&customer_id={customer}&{SAFETY}",
            escape_query(key)
        );
        self.list(target.clone(), target).await
    }

    pub async fn categories(
        &self,
        key: &str,
        customer: &str,
    ) -> Result<Vec<Category>, Unavailable> {
        let target = format!(
            "/api/v1/{}/gifs/categories?customer_id={customer}&{SAFETY}",
            escape_query(key)
        );
        if let Some(Cached::Categories(categories)) = self.cached(&target) {
            return Ok(categories);
        }
        let value = self.get_json(target.clone()).await?;
        let parsed: CategoriesResponse = serde_json::from_value(value).map_err(|_| Unavailable)?;
        if !parsed.result {
            return Err(Unavailable);
        }
        let categories = parsed
            .data
            .map(|data| {
                data.categories
                    .into_iter()
                    .map(|raw| Category {
                        name: raw.category,
                        query: raw.query,
                        preview_url: raw.preview_url.filter(|url| !url.is_empty()),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.store(target, Cached::Categories(categories.clone()));
        Ok(categories)
    }

    pub async fn suggestions(
        &self,
        key: &str,
        customer: &str,
        query: &str,
        kind: SuggestionKind,
    ) -> Result<Vec<String>, Unavailable> {
        let target = format!(
            "/api/v1/{}/{}/{}?limit=10&customer_id={customer}&{SAFETY}",
            escape_query(key),
            kind.endpoint(),
            escape_query(query)
        );
        if let Some(Cached::Suggestions(terms)) = self.cached(&target) {
            return Ok(terms);
        }
        let parsed: SuggestionsResponse =
            serde_json::from_value(self.get_json(target.clone()).await?)
                .map_err(|_| Unavailable)?;
        if !parsed.result {
            return Err(Unavailable);
        }
        let terms: Vec<String> = parsed
            .data
            .into_iter()
            .take(10)
            .map(|term| term.chars().take(200).collect())
            .collect();
        self.store(target, Cached::Suggestions(terms.clone()));
        Ok(terms)
    }

    /// The one item behind `slug`, through the Items endpoint.
    pub async fn item(&self, key: &str, customer: &str, slug: &str) -> Result<Item, Unavailable> {
        let value = self
            .get_json(format!(
                "/api/v1/{}/gifs/items?slugs={}&customer_id={customer}&{SAFETY}",
                escape_query(key),
                escape_query(slug)
            ))
            .await?;
        let parsed: ListResponse = serde_json::from_value(value).map_err(|_| Unavailable)?;
        parsed
            .page()?
            .items
            .into_iter()
            .find(|item| item.slug == slug)
            .ok_or(Unavailable)
    }

    /// Fetches a KLIPY media URL's bytes, refusing anything past `limit`.
    pub async fn fetch_media(
        &self,
        url: &str,
        limit: usize,
    ) -> Result<(Vec<u8>, String), MediaError> {
        let mut parsed = allowed_media_url(url).ok_or(MediaError::NotAllowed)?;
        let _permit = self
            .inner
            .media_slots
            .try_acquire()
            .map_err(|_| MediaError::Unavailable)?;
        tokio::time::timeout(Duration::from_secs(15), async {
            let ip = crate::net::guard::resolve(&*self.network.resolver, &parsed.endpoint.host)
                .await
                .map_err(|_| MediaError::NotAllowed)?;
            parsed.endpoint.pinned_ip = Some(ip);
            let response =
                http::exchange(&self.network, &parsed.endpoint, parsed.request, &TIMEOUTS)
                    .await
                    .map_err(|_| MediaError::Unavailable)?;
            if !(200..300).contains(&response.status) {
                return Err(MediaError::Unavailable);
            }
            let content_type = response
                .content_type()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !ALLOWED_MEDIA_TYPES.contains(&content_type.as_str()) {
                return Err(MediaError::BadType);
            }
            match response
                .read_body(limit)
                .await
                .map_err(|_| MediaError::Unavailable)?
            {
                Body::Complete(bytes) => Ok((bytes, content_type)),
                Body::TooLarge => Err(MediaError::TooLarge),
            }
        })
        .await
        .map_err(|_| MediaError::Unavailable)?
    }
}

const ALLOWED_MEDIA_TYPES: &[&str] = &[
    "image/gif",
    "image/webp",
    "image/jpeg",
    "image/png",
    "video/mp4",
    "video/webm",
];

#[derive(Debug)]
pub enum MediaError {
    /// Not an https URL on one of KLIPY's media hosts.
    NotAllowed,
    /// The response was not an allowed image/video type.
    BadType,
    /// The body ran past the caller's size limit.
    TooLarge,
    Unavailable,
}

struct MediaTarget {
    endpoint: Endpoint,
    request: Request,
}

/// Parses `url` and keeps it only when it is https on a KLIPY media host.
fn allowed_media_url(url: &str) -> Option<MediaTarget> {
    if url.len() > 2048 {
        return None;
    }
    let parsed = campfire_richtext::uri::parse(url).ok()?;
    if parsed.scheme.as_deref() != Some("https") || parsed.userinfo.is_some() {
        return None;
    }
    let host = parsed.host?.to_ascii_lowercase();
    if !MEDIA_HOSTS.contains(&host.as_str()) {
        return None;
    }
    if !matches!(parsed.port, None | Some(443)) {
        return None;
    }
    let port = 443;
    let mut target = parsed
        .path
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "/".into());
    if let Some(query) = parsed.query {
        target.push('?');
        target.push_str(&query);
    }
    let endpoint = Endpoint {
        https: true,
        host,
        port,
        pinned_ip: None,
    };
    let request = Request::net_http(
        Method::GET,
        target,
        None,
        vec![("User-Agent".into(), "Smartfire-GIF".into())],
    )
    .transport(false, &endpoint);
    Some(MediaTarget { endpoint, request })
}

/// Our proxy path for a KLIPY media URL (`/api/v1/gifs/media?u=…`).
pub fn media_proxy_path(url: &str) -> Option<String> {
    allowed_media_url(url)?;
    Some(format!("/api/v1/gifs/media?u={}", escape_query(url)))
}

fn escape_query(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[derive(Deserialize)]
struct SuggestionsResponse {
    result: bool,
    data: Vec<String>,
}

#[derive(Deserialize)]
struct ListResponse {
    result: bool,
    data: Option<ListData>,
}

#[derive(Deserialize)]
struct ListData {
    #[serde(default)]
    data: Vec<RawItem>,
    #[serde(default)]
    current_page: i64,
    #[serde(default)]
    has_next: bool,
}

#[derive(Deserialize)]
struct RawItem {
    id: Value,
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default, rename = "file")]
    files: BTreeMap<String, BTreeMap<String, Rendition>>,
}

impl ListResponse {
    fn page(self) -> Result<ListPage, Unavailable> {
        if !self.result {
            return Err(Unavailable);
        }
        let data = self.data.ok_or(Unavailable)?;
        Ok(ListPage {
            items: data
                .data
                .into_iter()
                .map(|raw| Item {
                    klipy_id: match &raw.id {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    },
                    slug: raw.slug,
                    title: raw.title,
                    files: raw.files,
                })
                .collect(),
            page: data.current_page,
            has_next: data.has_next,
        })
    }
}

#[derive(Deserialize)]
struct CategoriesResponse {
    result: bool,
    data: Option<CategoriesData>,
}

#[derive(Deserialize)]
struct CategoriesData {
    #[serde(default)]
    categories: Vec<RawCategory>,
}

#[derive(Deserialize)]
struct RawCategory {
    category: String,
    query: String,
    preview_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn customer_id_is_stable_opaque_and_secret_dependent() {
        let first = customer_id("secret-one", 42);
        assert_eq!(first, customer_id("secret-one", 42));
        assert_ne!(first, customer_id("secret-one", 43));
        assert_ne!(first, customer_id("secret-two", 42));
        assert_eq!(
            first,
            "52c34cd05f76dbc52b91898df4d2784a7a545872122caacea52b8239a59ccd41"
        );
    }

    #[test]
    fn media_urls_are_limited_to_klipy_hosts_over_https() {
        assert!(allowed_media_url("https://static.klipy.com/ii/x/y.webp").is_some());
        assert!(allowed_media_url("https://static1.klipy.com/ii/x/y.webp").is_some());
        assert!(allowed_media_url("https://static2.klipy.com/ii/x/y.webp").is_some());
        assert!(allowed_media_url("https://static.klipy.co/ii/x/y.webp").is_none());
        assert!(allowed_media_url("https://static.klipy.com:8443/ii/x/y.webp").is_none());
        assert!(allowed_media_url("http://static.klipy.com/ii/x/y.webp").is_none());
        assert!(allowed_media_url("https://static.klipy.com.evil.example/x.webp").is_none());
        assert!(allowed_media_url("https://evil.example/x.webp").is_none());
        assert!(allowed_media_url("https://user@static.klipy.com/x.webp").is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn cache_and_rate_windows_expire_and_cache_stays_bounded() {
        let state = State::system();
        for i in 0..(CACHE_ENTRIES + 10) {
            state.store(i.to_string(), Cached::Suggestions(vec!["hello".into()]));
            tokio::time::advance(Duration::from_millis(1)).await;
        }
        assert_eq!(state.inner.cache.lock().unwrap().len(), CACHE_ENTRIES);
        assert!(state.cached("0").is_none());
        assert!(state.cached(&(CACHE_ENTRIES + 9).to_string()).is_some());
        assert!(state.allow(1, "api", 1));
        assert!(!state.allow(1, "api", 1));
        assert!(state.allow(2, "api", 1));
        tokio::time::advance(CACHE_TTL).await;
        assert!(state.cached(&(CACHE_ENTRIES + 9).to_string()).is_none());
        assert!(state.allow(1, "api", 1));
        state.store("fresh".into(), Cached::Suggestions(vec![]));
        assert_eq!(state.inner.cache.lock().unwrap().len(), 1);
        assert_eq!(state.inner.hits.lock().unwrap().len(), 1);
    }

    #[test]
    fn send_rendition_prefers_small_webp_within_the_cap() {
        let item = Item {
            klipy_id: "1".into(),
            slug: "x".into(),
            title: "X".into(),
            files: BTreeMap::from([
                (
                    "md".to_string(),
                    BTreeMap::from([
                        (
                            "webp".to_string(),
                            Rendition {
                                url: "md.webp".into(),
                                width: 1,
                                height: 1,
                                size: 9000,
                            },
                        ),
                        (
                            "gif".to_string(),
                            Rendition {
                                url: "md.gif".into(),
                                width: 1,
                                height: 1,
                                size: 500,
                            },
                        ),
                    ]),
                ),
                (
                    "sm".to_string(),
                    BTreeMap::from([(
                        "webp".to_string(),
                        Rendition {
                            url: "sm.webp".into(),
                            width: 1,
                            height: 1,
                            size: 400,
                        },
                    )]),
                ),
            ]),
        };
        let (rendition, format, _) = item.send_rendition(1000).unwrap();
        assert_eq!(rendition.url, "sm.webp");
        assert_eq!(format, "webp");
        let (rendition, _, _) = item.send_rendition(20_000).unwrap();
        assert_eq!(rendition.url, "md.webp");
    }
}

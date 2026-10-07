//! Views for `reference/app/views/pwa`.

use askama::Template;

use crate::ViewContext;
use crate::helpers as h;

/// `pwa/service_worker.js`, served verbatim.
pub const SERVICE_WORKER_JS: &str = include_str!("../templates/pwa/service_worker.js");

/// The deliberate differences from Rails' worker, as `(rails, smartfire)` replacements: on
/// activation it keeps the SPA worker's build caches, and it serves `/app/assets/` from them, so
/// an SPA tab still open on an older build keeps loading its chunks after another tab switches
/// to the classic UI. Pass-through requests use a static network route when supported, so
/// they do not wake the worker during a script swap. Parity tests apply exactly these to the
/// frozen Rails body.
pub const SERVICE_WORKER_SPA_PATCH: [(&str, &str); 5] = [
    (
        "const OFFLINE_URL = \"/offline.html\"\n",
        "const OFFLINE_URL = \"/offline.html\"\n\
         // The SPA worker's build caches outlive a switch to this worker: SPA tabs still open on an\n\
         // older build keep loading its chunks from them. They are read here, never written.\n\
         const SPA_CACHE_PREFIX = \"smartfire-spa-\"\n\
         const SPA_ASSETS = \"/app/assets/\"\n",
    ),
    (
        "keys.filter((key) => key !== STATIC_CACHE)",
        "keys.filter((key) => key !== STATIC_CACHE && !key.startsWith(SPA_CACHE_PREFIX))",
    ),
    (
        "    event.respondWith(cacheFirst(request))\n    return\n  }\n",
        "    event.respondWith(cacheFirst(request))\n    return\n  }\n\n\
         \x20 if (url.pathname.startsWith(SPA_ASSETS)) {\n\
         \x20   event.respondWith(spaAsset(request))\n\
         \x20   return\n\
         \x20 }\n",
    ),
    (
        "self.addEventListener(\"install\", (event) => {\n\
         \x20 event.waitUntil(\n\
         \x20   caches.open(STATIC_CACHE)\n\
         \x20     .then((cache) => cache.add(OFFLINE_URL))\n\
         \x20     .then(() => self.skipWaiting())\n\
         \x20 )\n\
         })\n",
        "// Pass-through requests reach the network without waking the worker during a script swap.\n\
         async function installNetworkRoute(event) {\n\
         \x20 if (!(\"addRoutes\" in event) || typeof event.addRoutes !== \"function\") return\n\n\
         \x20 try {\n\
         \x20   await event.addRoutes({\n\
         \x20     condition: { not: { or: [\n\
         \x20       { requestMethod: \"GET\", requestMode: \"navigate\" },\n\
         \x20       { requestMethod: \"GET\", urlPattern: new URL(\"/assets/*\", self.location.origin).href },\n\
         \x20       { requestMethod: \"GET\", urlPattern: new URL(`${SPA_ASSETS}*`, self.location.origin).href },\n\
         \x20       { requestMethod: \"GET\", urlPattern: new URL(OFFLINE_URL, self.location.origin).href }\n\
         \x20     ] } },\n\
         \x20     source: \"network\"\n\
         \x20   })\n\
         \x20 } catch {\n\
         \x20   // Older implementations expose addRoutes but reject not/or; keep the fetch handler.\n\
         \x20 }\n\
         }\n\n\
         self.addEventListener(\"install\", (event) => {\n\
         \x20 event.waitUntil(\n\
         \x20   Promise.all([\n\
         \x20     installNetworkRoute(event),\n\
         \x20     caches.open(STATIC_CACHE).then((cache) => cache.add(OFFLINE_URL))\n\
         \x20   ]).then(() => self.skipWaiting())\n\
         \x20 )\n\
         })\n",
    ),
    (
        "async function networkThenOffline(request) {",
        "async function spaAsset(request) {\n\
         \x20 for (const key of await caches.keys()) {\n\
         \x20   if (!key.startsWith(SPA_CACHE_PREFIX)) continue\n\
         \x20   const cached = await caches.match(request, { cacheName: key, ignoreVary: true })\n\
         \x20   if (cached) return cached\n\
         \x20 }\n\
         \x20 return fetch(request)\n\
         }\n\n\
         async function networkThenOffline(request) {",
    ),
];

/// Rails' worker body with [`SERVICE_WORKER_SPA_PATCH`] applied; each replaced span must occur
/// exactly once, so a drifting Rails body or patch fails loudly.
pub fn rails_service_worker_with_spa_patch(rails: &str) -> String {
    SERVICE_WORKER_SPA_PATCH
        .iter()
        .fold(rails.to_owned(), |body, (from, to)| {
            assert_eq!(body.matches(from).count(), 1, "worker patch span: {from:?}");
            body.replacen(from, to, 1)
        })
}

/// `pwa/manifest.json.erb`. Preserve ERB's HTML escaping inside JSON, including `&amp;` in
/// the small-logo URL. Byte parity includes that behavior.
#[derive(Template)]
#[template(path = "pwa/manifest.json")]
pub struct Manifest<'a> {
    /// `Current.account&.name`.
    pub account_name: Option<String>,
    /// `fresh_account_logo_path(size: :small)`.
    pub logo_path_small: String,
    /// `fresh_account_logo_path`.
    pub logo_path: String,
    /// `request.base_url`, for `image_url`.
    pub base_url: String,
    /// The start URL and scope share one setting; classic keeps `/`.
    pub root: &'a str,
    pub new_room_url: String,
    pub profile_url: String,
    pub asset_path: &'a dyn Fn(&str) -> String,
}

impl Manifest<'_> {
    /// `image_url(source)`.
    fn image_url(&self, source: &str) -> String {
        format!("{}{}", self.base_url, (self.asset_path)(source))
    }
}

/// `pwa/_install_instructions.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_install_instructions.html")]
pub struct InstallInstructions<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `pwa/_browser_settings.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_browser_settings.html")]
pub struct BrowserSettings<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `pwa/_system_settings.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_system_settings.html")]
pub struct SystemSettings<'a> {
    pub ctx: &'a ViewContext<'a>,
}

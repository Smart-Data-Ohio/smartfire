//! Installed PWA assets, independent of the classic UI.

use askama::Template;

use crate::{File, Served};

/// `pwa/service_worker.js`, served verbatim.
pub const SERVICE_WORKER_JS: &str = include_str!("../pwa/service_worker.js");

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

/// Manifest illustrations are embedded here so removing `web/` does not break installation.
pub fn asset_path(path: &str) -> String {
    format!("/pwa/assets/{path}")
}

/// ERB::Util.html_escape, retained for the manifest's recorded JSON rendering contract.
#[derive(Clone, Copy)]
pub struct ManifestEscaper;

impl askama::filters::Escaper for ManifestEscaper {
    fn write_escaped_str<W: std::fmt::Write>(&self, mut dest: W, string: &str) -> std::fmt::Result {
        let mut last = 0;
        for (index, byte) in string.bytes().enumerate() {
            let replacement = match byte {
                b'&' => "&amp;",
                b'<' => "&lt;",
                b'>' => "&gt;",
                b'"' => "&quot;",
                b'\'' => "&#39;",
                _ => continue,
            };
            dest.write_str(&string[last..index])?;
            dest.write_str(replacement)?;
            last = index + 1;
        }
        dest.write_str(&string[last..])
    }
}

macro_rules! pwa_file {
    ($path:literal, $identity:expr, $content_type:literal) => {
        File {
            path: $path,
            content_type: $content_type,
            immutable: false,
            identity: $identity,
            br: None,
            gz: None,
        }
    };
}

static WORKER: File = pwa_file!(
    "service-worker.js",
    include_bytes!("../pwa/service_worker.js"),
    "text/javascript; charset=utf-8"
);
static OFFLINE: File = pwa_file!(
    "offline.html",
    include_bytes!("../pwa/offline.html"),
    "text/html"
);
static ASSETS: &[File] = &[
    pwa_file!(
        "assets/add.svg",
        include_bytes!("../pwa/assets/add.svg"),
        "image/svg+xml"
    ),
    pwa_file!(
        "assets/person.svg",
        include_bytes!("../pwa/assets/person.svg"),
        "image/svg+xml"
    ),
    pwa_file!(
        "assets/screenshots/android-chat.png",
        include_bytes!("../pwa/assets/screenshots/android-chat.png"),
        "image/png"
    ),
    pwa_file!(
        "assets/screenshots/android-dark-mode.png",
        include_bytes!("../pwa/assets/screenshots/android-dark-mode.png"),
        "image/png"
    ),
    pwa_file!(
        "assets/screenshots/android-sidebar.png",
        include_bytes!("../pwa/assets/screenshots/android-sidebar.png"),
        "image/png"
    ),
];

/// One public worker and offline URL for both UIs. A stub build still supports installed PWAs.
pub fn file(path: &str, spa_enabled: bool, accept_encoding: Option<&str>) -> Option<Served> {
    let fallback = match path {
        "service-worker.js" => &WORKER,
        "offline.html" => &OFFLINE,
        _ => {
            return crate::serve::find(ASSETS, path)
                .map(|file| crate::serve::negotiate(file, accept_encoding));
        }
    };
    if spa_enabled && let Some(served) = crate::file(path, accept_encoding) {
        return Some(served);
    }
    Some(crate::serve::negotiate(fallback, accept_encoding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwa_endpoints_match_recorded_rails_before_and_after_first_run() {
        for vectors in [
            include_str!("../../../vectors/users_pwa_default.json"),
            include_str!("../../../vectors/users_pwa_first_run.json"),
        ] {
            let vectors: serde_json::Value = serde_json::from_str(vectors).unwrap();
            let body = vectors["responses"][0]["body"].as_str().unwrap();
            let manifest: serde_json::Value = serde_json::from_str(body).unwrap();
            // Preserve the frozen renderer receipt without depending on classic asset digesting.
            let assets = |path: &str| {
                let index = match path {
                    "add.svg" => &manifest["shortcuts"][0]["icons"][0]["src"],
                    "person.svg" => &manifest["shortcuts"][1]["icons"][0]["src"],
                    "screenshots/android-chat.png" => &manifest["screenshots"][0]["src"],
                    "screenshots/android-sidebar.png" => &manifest["screenshots"][1]["src"],
                    "screenshots/android-dark-mode.png" => &manifest["screenshots"][2]["src"],
                    _ => panic!("unexpected manifest asset {path}"),
                };
                index
                    .as_str()
                    .unwrap()
                    .trim_start_matches("http://campfire.test")
                    .to_owned()
            };
            let rendered = Manifest {
                account_name: Some(manifest["name"].as_str().unwrap().to_owned()),
                logo_path_small: manifest["icons"][0]["src"]
                    .as_str()
                    .unwrap()
                    .replace("&amp;", "&"),
                logo_path: manifest["icons"][1]["src"].as_str().unwrap().to_owned(),
                base_url: "http://campfire.test".into(),
                root: "/",
                new_room_url: "rooms/opens/new".into(),
                profile_url: "/users/me/profile".into(),
                asset_path: &assets,
            }
            .render()
            .unwrap();
            assert_eq!(rendered, body);
            assert_eq!(
                SERVICE_WORKER_JS,
                rails_service_worker_with_spa_patch(
                    vectors["responses"][1]["body"].as_str().unwrap()
                )
            );
            assert_eq!(
                std::str::from_utf8(OFFLINE.identity).unwrap(),
                vectors["responses"][2]["body"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn pwa_fallback_and_manifest_illustrations_are_embedded_without_classic_assets() {
        for asset in ASSETS {
            assert_eq!(file(asset.path, false, None).unwrap().body, asset.identity);
            assert!(!asset.identity.is_empty());
        }
        assert_eq!(
            file("service-worker.js", false, None).unwrap().body,
            SERVICE_WORKER_JS.as_bytes()
        );
        assert_eq!(
            file("offline.html", false, None).unwrap().body,
            OFFLINE.identity
        );
        assert_eq!(
            file("offline.html", true, None),
            crate::file("offline.html", None).or_else(|| file("offline.html", false, None))
        );
        assert!(file("assets/no-such-image.png", true, None).is_none());
    }
}

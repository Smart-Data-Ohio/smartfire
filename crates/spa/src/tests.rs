//! The embedded build (the stub, or a real dist when `SPA_DIST` names one), and the embedding,
//! negotiation and shell over `tests/fixture`, a dist shaped like Vite's.

#[path = "../build/embed.rs"]
mod embed;

use std::io::Read as _;
use std::path::Path;

use crate::serve::{find, negotiate};
use crate::shell::render;
use crate::*;

fn boot() -> Boot {
    Boot {
        user: BootUser { id: 7, name: "David".into(), avatar_url: "/users/abc/avatar?v=1".into() },
        account: BootAccount { name: Some("Smart Data".into()), logo_url: None, logo_still_url: None, banner_url: None, banner_still_url: None },
        theme: theme(Some("dark")),
        text_size: text_size(None),
        cable_url: "/cable".into(),
        service_worker_url: Some("/service-worker.js".into()),
        version: "1.2.3".into(),
        flash: None,
        revision: Some("0123abc".into()),
    }
}

/// The boot JSON a page carries, decoded.
fn boot_json(html: &str) -> serde_json::Value {
    let start = html.find("id=\"boot\"").expect("the page has the boot script");
    let json = &html[html[start..].find('>').unwrap() + start + 1..];
    serde_json::from_str(&json[..json.find("</script>").unwrap()]).unwrap()
}

/// `build.rs`'s embedding of `tests/fixture`, made here with the same code (`build/embed.rs`).
fn fixture() -> &'static [File] {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture");
    let (_, entries) = embed::read_dist(&dist);
    let leak = |bytes: Vec<u8>| -> &'static [u8] { Box::leak(bytes.into_boxed_slice()) };
    let files: Vec<File> = entries
        .into_iter()
        .map(|entry| {
            let bytes = std::fs::read(&entry.source).unwrap();
            let encoded = |vite: &Option<std::path::PathBuf>, encode: fn(&[u8]) -> Vec<u8>| match vite {
                Some(path) => Some(leak(std::fs::read(path).unwrap())),
                None if entry.compressible() => embed::smaller(encode(&bytes), &bytes).map(leak),
                None => None,
            };
            File {
                path: Box::leak(entry.path.clone().into_boxed_str()),
                content_type: entry.content_type,
                immutable: entry.immutable,
                br: encoded(&entry.br, embed::brotli),
                gz: encoded(&entry.gz, embed::gzip),
                identity: leak(bytes),
            }
        })
        .collect();
    Box::leak(files.into_boxed_slice())
}

fn unbrotli(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    brotli::BrotliDecompress(&mut &bytes[..], &mut out).unwrap();
    out
}

fn gunzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut out).unwrap();
    out
}

// --- The embedded build ----------------------------------------------------------------------------

#[test]
fn the_stub_stands_in_without_a_dist() {
    if built() {
        return; // built_dist_is_embedded_whole covers a real dist.
    }
    assert!(files().is_empty());
    assert_eq!(file("assets/index.js", Some("br, gzip")), None);
    let page = render_shell(&boot(), "token", Some("nonce"));
    assert!(page.contains("isn't built into this server"), "{page}");
    assert!(page.contains("<meta name=\"csrf-token\" content=\"token\" />"));
    assert_eq!(boot_json(&page)["user"]["id"], 7);
}

/// With `SPA_DIST` set (the Frontend CI job, after `pnpm build`), the real dist is embedded, and
/// every file in it can be served the way the fixture's are.
#[test]
fn built_dist_is_embedded_whole() {
    if option_env!("SPA_DIST").is_some_and(|dist| !dist.is_empty()) {
        assert!(built(), "SPA_DIST is set, so the stub must not be embedded");
    }
    if !built() {
        return;
    }
    assert!(!files().is_empty());
    assert!(files().windows(2).all(|pair| pair[0].path < pair[1].path), "sorted for lookup");
    let page = render_shell(&boot(), "token", Some("nonce"));
    assert!(page.contains("<script nonce=\"nonce\" type=\"module\""), "Vite's entry script carries the nonce: {page}");
    assert_eq!(boot_json(&page)["account"]["name"], "Smart Data");
    for file in files() {
        assert!(!file.path.starts_with('.') && !file.path.contains("/."), "{} is build metadata", file.path);
        assert_ne!(file.path, "index.html");
        assert_ne!(file.content_type, "application/octet-stream", "{} has no known type", file.path);
        let extension = file.path.rsplit_once('.').map_or("", |(_, ext)| ext);
        let expected = match extension {
            "js" => Some("text/javascript; charset=utf-8"),
            "css" => Some("text/css; charset=utf-8"),
            "woff2" => Some("font/woff2"),
            "svg" => Some("image/svg+xml"),
            _ => None,
        };
        if let Some(expected) = expected {
            assert_eq!(file.content_type, expected, "{}", file.path);
        }
        if extension == "woff2" {
            assert_eq!((file.br, file.gz), (None, None), "{}: woff2 is compressed already", file.path);
        }
        // The fonts' licences keep their names, so they're found beside the fonts (and revalidated).
        if file.path.starts_with("assets/LICENSE-") {
            assert!(!file.immutable && extension == "txt", "{}", file.path);
        } else if file.path.starts_with("assets/") && !file.path.ends_with(".map") {
            assert!(file.immutable, "Vite hashes every asset's name: {}", file.path);
        }
        if let Some(br) = file.br {
            assert_eq!(unbrotli(br), file.identity, "{}", file.path);
        }
        if let Some(gz) = file.gz {
            assert_eq!(gunzip(gz), file.identity, "{}", file.path);
        }
        if embed::compressible(file.content_type) && file.identity.len() > 1024 {
            assert!(file.br.is_some() && file.gz.is_some(), "{} is served compressed", file.path);
        }
    }
    let entry = page.split("src=\"/app/").nth(1).and_then(|rest| rest.split('"').next()).expect("an entry script");
    assert!(file(entry, None).is_some(), "the shell's entry script {entry} is embedded");
}

// --- Embedding -----------------------------------------------------------------------------------

#[test]
fn embedding_keeps_servable_files_with_their_types_and_cache_policy() {
    let files = fixture();
    let summary: Vec<(&str, &str, bool)> = files.iter().map(|f| (f.path, f.content_type, f.immutable)).collect();
    assert_eq!(
        summary,
        [
            ("assets/index-B2x8Kq1f.js", "text/javascript; charset=utf-8", true),
            ("assets/index-Dk_9-xYz.css", "text/css; charset=utf-8", true),
            ("assets/logo-AbCdEf12.png", "image/png", true),
            ("assets/unhashed.js", "text/javascript; charset=utf-8", false),
            ("assets/vendor-Q1w2E3r4.js", "text/javascript; charset=utf-8", true),
            ("favicon.svg", "image/svg+xml", false),
            ("offline.html", "text/html; charset=utf-8", false),
            ("service-worker.js", "text/javascript; charset=utf-8", false),
        ],
        "index.html is the template, .vite/ and dotfiles are build metadata, and Vite's .br/.gz \
         are encodings of the file beside them"
    );
}

#[test]
fn embedding_compresses_text_and_keeps_vites_own_encodings() {
    let files = fixture();
    let js = find(files, "assets/index-B2x8Kq1f.js").unwrap();
    assert_eq!(unbrotli(js.br.unwrap()), js.identity);
    assert_eq!(gunzip(js.gz.unwrap()), js.identity);
    assert!(js.br.unwrap().len() < js.gz.unwrap().len() && js.gz.unwrap().len() < js.identity.len());

    let vendor = find(files, "assets/vendor-Q1w2E3r4.js").unwrap();
    assert_eq!(vendor.br, Some(&b"vite's own brotli bytes"[..]), "Vite's .br is used as it is");
    assert_eq!(gunzip(vendor.gz.unwrap()), vendor.identity);

    let png = find(files, "assets/logo-AbCdEf12.png").unwrap();
    assert_eq!((png.br, png.gz), (None, None), "a PNG is compressed already");
    let tiny = find(files, "assets/unhashed.js").unwrap();
    assert_eq!((tiny.br, tiny.gz), (None, None), "an encoding no smaller than the file isn't kept");
}

#[test]
fn only_vites_hashed_asset_names_are_immutable() {
    for (path, immutable) in [
        ("assets/index-B2x8Kq1f.js", true),
        ("assets/index-Dk_9-xYz.css", true),
        ("assets/app.config-AbCd1234.js", true),
        ("assets/worker-AbCd1234.js.map", true),
        ("assets/index-AbCd1234x.js", false), // the hash must end the name
        ("assets/index-AbCd1234.", false),
        ("assets/nested/font-ZZZZ____.woff2", true),
        ("assets/index-short.js", false),
        ("assets/index.js", false),
        ("assets/-AbCd1234.js", false),
        ("favicon-AbCd1234.svg", false), // public files keep their names
        ("index.html", false),
    ] {
        assert_eq!(embed::immutable(path), immutable, "{path}");
    }
}

// --- Negotiation ---------------------------------------------------------------------------------

#[test]
fn negotiation_prefers_brotli_then_gzip_then_the_file() {
    let js = find(fixture(), "assets/index-B2x8Kq1f.js").unwrap();
    let encoding = |accept: Option<&str>| negotiate(js, accept).content_encoding;
    assert_eq!(encoding(Some("gzip, deflate, br, zstd")), Some("br"));
    assert_eq!(encoding(Some("gzip, deflate")), Some("gzip"));
    assert_eq!(encoding(Some("x-gzip")), Some("gzip"));
    assert_eq!(encoding(Some("BR;q=0.5, GZIP;q=0.9")), Some("gzip"), "the client's q-values win");
    assert_eq!(encoding(Some("br;q=0, gzip")), Some("gzip"));
    assert_eq!(encoding(Some("*")), Some("br"));
    assert_eq!(encoding(Some("*;q=0")), None);
    assert_eq!(encoding(Some("br;q=0, *;q=0.1")), Some("gzip"));
    assert_eq!(encoding(Some("identity")), None);
    assert_eq!(encoding(Some("")), None);
    assert_eq!(encoding(None), None);

    let served = negotiate(js, Some("br"));
    assert_eq!(served.body, js.br.unwrap());
    assert!(served.varies());
    assert_eq!(negotiate(js, None).body, js.identity);

    let png = find(fixture(), "assets/logo-AbCdEf12.png").unwrap();
    let served = negotiate(png, Some("br, gzip"));
    assert_eq!((served.content_encoding, served.body, served.varies()), (None, png.identity, false));
}

#[test]
fn lookups_are_exact() {
    let files = fixture();
    assert!(find(files, "assets/index-B2x8Kq1f.js").is_some());
    for path in ["index.html", "assets/../index.html", "/assets/index-B2x8Kq1f.js", "assets/index-B2x8Kq1f.js.br", ".vite/manifest.json", "assets"] {
        assert_eq!(find(files, path), None, "{path}");
    }
}

/// The SIL Open Font License travels with each font the build ships: its licence sits beside the
/// fonts in `assets/` (vite.config.ts copies them from `frontend/src/styles/fonts`).
#[test]
fn built_dist_ships_each_fonts_licence() {
    if !built() {
        return;
    }
    let licences = [
        ("inter-", "LICENSE-Inter.txt"),
        ("jetbrains-mono-", "LICENSE-JetBrainsMono.txt"),
        ("atkinson-hyperlegible-next-", "LICENSE-AtkinsonHyperlegibleNext.txt"),
        ("SourceSerif4-", "LICENSE-SourceSerif4.txt"),
    ];
    let fonts: Vec<&File> = files().iter().filter(|file| file.path.ends_with(".woff2")).collect();
    assert!(!fonts.is_empty(), "the build ships fonts");
    for font in fonts {
        let name = font.path.rsplit('/').next().unwrap_or(font.path);
        let (_, licence) = licences
            .iter()
            .find(|(prefix, _)| name.starts_with(prefix))
            .unwrap_or_else(|| panic!("{name} has no licence listed here"));
        let path = format!("assets/{licence}");
        let file = files().iter().find(|file| file.path == path).unwrap_or_else(|| panic!("{path} isn't in the build, beside {name}"));
        let text = std::str::from_utf8(file.identity).expect("the licence is text");
        assert!(text.contains("SIL Open Font License"), "{path}");
    }
}

// --- The shell -----------------------------------------------------------------------------------

#[test]
fn the_shell_carries_the_csrf_meta_tags_the_nonce_and_the_boot_json() {
    let template = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/index.html")).unwrap();
    let page = render(&template, &boot(), "masked+token/=", Some("n0nce+/="));
    assert!(!page.contains("<!--boot-->"));
    assert!(page.contains(
        "<meta name=\"csrf-param\" content=\"authenticity_token\" />\n\
         <meta name=\"csrf-token\" content=\"masked+token/=\" />\n\
         <meta name=\"csp-nonce\" content=\"n0nce+/=\" />\n\
         <meta name=\"turbo-visit-control\" content=\"reload\" />\n\
         <link rel=\"manifest\" href=\"/webmanifest.json\" crossorigin=\"use-credentials\" />\n\
         <script type=\"application/json\" id=\"boot\" nonce=\"n0nce+/=\">"
    ), "{page}");
    assert!(page.contains("<script nonce=\"n0nce+/=\" type=\"module\" crossorigin src=\"/app/assets/index-B2x8Kq1f.js\"></script>"), "{page}");
    assert!(page.contains("<link nonce=\"n0nce+/=\" rel=\"modulepreload\" crossorigin href=\"/app/assets/vendor-Q1w2E3r4.js\">"), "{page}");
    assert!(page.contains("<link rel=\"stylesheet\" crossorigin href=\"/app/assets/index-Dk_9-xYz.css\">"), "styles need no nonce");
    assert_eq!(page.matches("nonce=").count(), 3);
    assert_eq!(
        boot_json(&page),
        serde_json::json!({
            "user": {"id": 7, "name": "David", "avatarUrl": "/users/abc/avatar?v=1"},
            "account": {"name": "Smart Data", "logoUrl": null, "logoStillUrl": null, "bannerUrl": null, "bannerStillUrl": null},
            "theme": "dark",
            "textSize": "default",
            "cableUrl": "/cable",
            "serviceWorkerUrl": "/service-worker.js",
            "version": "1.2.3",
            "revision": "0123abc",
        })
    );

    let without_nonce = render(&template, &boot(), "token", None);
    assert!(!without_nonce.contains("nonce"));
    assert!(without_nonce.contains("<script type=\"application/json\" id=\"boot\">"));

    let no_placeholder = render("<html><head><title>x</title></head></html>", &boot(), "token", None);
    assert!(no_placeholder.contains("id=\"boot\">{") && no_placeholder.contains("</script>\n</head>"), "{no_placeholder}");
}

/// Every `<script>` start tag in `page`, up to its `>`.
fn script_tags(page: &str) -> Vec<&str> {
    page.match_indices("<script").map(|(start, _)| &page[start..start + page[start..].find('>').unwrap() + 1]).collect()
}

/// `frontend/index.html`'s own scripts (the blocking appearance initializer before the stylesheet
/// paints, and Vite's entry) run under the enforced policy only with the nonce; so does the built
/// dist's, when one is embedded.
#[test]
fn every_script_in_the_spa_page_carries_the_nonce() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/index.html")).unwrap();
    let mut pages = vec![("frontend/index.html", render(&source, &boot(), "token", Some("n0nce")))];
    if built() {
        pages.push(("the embedded dist", render_shell(&boot(), "token", Some("n0nce"))));
    }
    for (name, page) in pages {
        let tags = script_tags(&page);
        assert!(page.contains("smartfire.appearance"), "{name} keeps the appearance initializer: {page}");
        assert!(tags.len() >= 3, "{name}: the boot JSON, the initializer and the entry: {tags:?}");
        for tag in tags {
            assert!(tag.contains(" nonce=\"n0nce\""), "{name}: {tag} runs only with the nonce");
        }
    }
}

#[test]
fn boot_json_cannot_end_its_script_element() {
    let mut boot = boot();
    boot.user.name = "</script><script>alert(1)</script><!-- & \u{2028}\u{2029}".into();
    let json = script_json(&boot);
    for raw in ["<", ">", "&", "\u{2028}", "\u{2029}"] {
        assert!(!json.contains(raw), "{raw:?} in {json}");
    }
    assert!(json.contains("\"name\":\"\\u003c/script\\u003e\\u003cscript\\u003ealert(1)\\u003c/script\\u003e\\u003c!-- \\u0026 \\u2028\\u2029\""), "{json}");

    let page = render("<head><!--boot--></head>", &boot, "token", None);
    assert_eq!(page.matches("</script>").count(), 1);
    assert_eq!(boot_json(&page)["user"]["name"], boot.user.name, "the escapes decode to the name");
}

#[test]
fn boot_response_adds_the_csrf_token_to_the_boot_json() {
    let boot = boot();
    let json = serde_json::to_value(BootResponse { boot: &boot, csrf_token: "token" }).unwrap();
    let mut expected = serde_json::to_value(&boot).unwrap();
    expected["csrfToken"] = "token".into();
    assert_eq!(json, expected);
}

#[test]
fn theme_and_text_size_fall_back_like_the_layout() {
    assert_eq!([theme(Some("light")), theme(Some("dark")), theme(Some("system")), theme(Some("neon")), theme(None)], ["light", "dark", "system", "system", "system"]);
    assert_eq!(
        ["smaller", "small", "default", "large", "larger", "huge"].map(|size| text_size(Some(size))),
        ["smaller", "small", "default", "large", "larger", "default"]
    );
    assert_eq!(text_size(None), "default");
}

/// The image's cargo build keeps target/ in a cache mount, so `build.rs` reruns on a digest of the
/// dist's contents rather than trusting mtimes, and the dist it embeds is the one built in-image.
#[test]
fn the_image_build_embeds_its_own_dist_and_tracks_it_by_content() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build = std::fs::read_to_string(root.join("build.rs")).unwrap();
    assert!(build.contains("cargo:rerun-if-env-changed=SPA_DIST_DIGEST"));
    let dockerfile = std::fs::read_to_string(root.join("../../Dockerfile")).unwrap();
    assert!(dockerfile.contains("COPY --from=spa /src/frontend/dist frontend/dist\nENV SPA_DIST=/src/frontend/dist\n"));
    let build_step = dockerfile.split("RUN --mount=type=cache").find(|step| step.contains("cargo build")).unwrap();
    let digest = build_step.find("export SPA_DIST_DIGEST=").expect("the cargo build step exports SPA_DIST_DIGEST");
    assert!(Some(digest) < build_step.find("cargo build"));
}

#[test]
fn pwa_worker_and_offline_page_are_files_while_only_index_is_the_shell() {
    let files = fixture();
    let offline = find(files, "offline.html").unwrap();
    assert_eq!(offline.content_type, "text/html; charset=utf-8");
    assert!(!offline.immutable);
    assert!(std::str::from_utf8(offline.identity).unwrap().contains("/app/assets/index-B2x8Kq1f.js"));
    let worker = find(files, "service-worker.js").unwrap();
    assert_eq!(worker.content_type, "text/javascript; charset=utf-8");
    assert!(!worker.immutable);
    assert!(find(files, "index.html").is_none());
    if built() {
        for path in ["offline.html", "service-worker.js"] {
            assert!(file(path, None).is_some(), "the built PWA emits {path}");
        }
    }
}

#[test]
fn optional_boot_flash_uses_camel_case_and_is_safe_in_the_shell() {
    let mut boot = boot();
    let value = serde_json::to_value(&boot).unwrap();
    assert!(value.get("flash").is_none());
    for (kind, wire) in [(FlashKind::Notice, "notice"), (FlashKind::Alert, "alert")] {
        boot.flash = Some(BootFlash { kind, message: "Connected <&> </script>.".into() });
        let html = render_shell(&boot, "TOKEN", Some("NONCE"));
        assert_eq!(boot_json(&html)["flash"], serde_json::json!({"kind":wire,"message":"Connected <&> </script>."}));
        assert!(!html.contains("Connected <&> </script>."));
    }
}

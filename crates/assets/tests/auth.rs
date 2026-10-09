//! Auth assets work through the classic pipeline independently of the SPA build.
use campfire_assets::{StaticRequest, asset_path, serve};

#[test]
fn standalone_auth_assets_and_fonts_are_served_without_changing_classic_tags() {
    let css_path = asset_path("auth.css");
    let css = serve(&StaticRequest {
        method: "GET",
        path: &css_path,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(css.status, 200);
    assert_eq!(css.header("content-type"), Some("text/css"));
    let css = std::str::from_utf8(&css.body).unwrap();
    assert!(
        !css.lines()
            .any(|line| line.trim_start().starts_with("@import"))
    );
    assert!(css.contains("@layer tokens"));
    assert!(css.contains("@layer base"));
    assert!(
        css.contains(".browser-list") && css.contains(".language-list-menu"),
        "the unsupported-browser panel and translation popup travel in the auth bundle"
    );
    for logical in [
        "fonts/inter-latin-var.woff2",
        "fonts/inter-latin-var-italic.woff2",
        "fonts/jetbrains-mono-latin-var.woff2",
    ] {
        let path = asset_path(logical);
        assert!(css.contains(&format!("url(\"{path}\")")));
        let font = serve(&StaticRequest {
            method: "GET",
            path: &path,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(font.header("content-type"), Some("font/woff2"));
        assert_eq!(&font.body[..4], b"wOF2");
        assert_eq!(
            font.body.as_ref(),
            std::fs::read(format!(
                "{}/../../frontend/src/styles/{logical}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap()
        );
    }
    let script_path = asset_path("auth.js");
    let script = serve(&StaticRequest {
        method: "GET",
        path: &script_path,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(script.body.as_ref(), include_bytes!("../auth/auth.js"));
    let script_text = std::str::from_utf8(&script.body).unwrap();
    assert!(
        script_text.contains("data-controller~='popup'"),
        "auth.js dismisses the classic translation popup"
    );
    assert!(!campfire_assets::all_stylesheet_paths().contains(&"auth.css"));
    assert!(
        !campfire_assets::stylesheet_link_tag_all(&[])
            .html
            .contains(&css_path)
    );
    assert!(!campfire_assets::javascript_importmap_tags().contains(&script_path));
}

#[path = "../../retained_pages/auth_build.rs"]
mod bundle;

#[test]
fn auth_bundle_recurses_in_order_deduplicates_and_resolves_font_urls_at_their_source() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let crate_dir = root.join("crates/assets");
    let frontend = root.join("frontend/src");
    for path in [
        crate_dir.join("auth"),
        frontend.join("auth"),
        frontend.join("styles/fonts"),
    ] {
        std::fs::create_dir_all(path).unwrap();
    }
    std::fs::write(crate_dir.join("auth/auth.js"), "window.fixture = true;").unwrap();
    std::fs::write(
        frontend.join("auth/auth.css"),
        "@import '../styles/index.css';\n@import '../styles/shared.css';\n.auth { color: blue; }",
    )
    .unwrap();
    std::fs::write(frontend.join("styles/index.css"), "/* @import './absent.css'; url('./absent.woff2') */\n@import 'shared.css';\n.index { content: \"url('./absent.woff2')\"; }").unwrap();
    std::fs::write(frontend.join("styles/shared.css"), "@import './index.css';\n@font-face { src: url('./fonts/example.woff2?#test') }\n.shared { color: red; }").unwrap();
    std::fs::write(frontend.join("styles/fonts/example.woff2"), b"wOF2fixture").unwrap();
    let output = bundle::prepare(&crate_dir, root);
    let css = std::fs::read_to_string(output.join("auth.css")).unwrap();
    assert_eq!(css.matches(".shared").count(), 1);
    assert!(css.find(".shared").unwrap() < css.find(".index").unwrap());
    assert!(css.find(".index").unwrap() < css.find(".auth").unwrap());
    assert!(css.contains("url(\"fonts/example.woff2?#test\")"));
    assert!(css.contains("/* @import './absent.css'; url('./absent.woff2') */"));
    assert!(css.contains("content: \"url('./absent.woff2')\""));
    assert_eq!(
        std::fs::read(output.join("fonts/example.woff2")).unwrap(),
        b"wOF2fixture"
    );
    assert_eq!(
        std::fs::read(output.join("auth.js")).unwrap(),
        b"window.fixture = true;"
    );
}

//! Deterministic standalone pages for frontend screenshots, rendered by the Rust views.
use crate::controllers::presenters::test_support::{TestApp, seed_clock};
use askama::Template;
use campfire_retained::{first_runs, helpers as h, layouts, sessions, sudos, two_factor, users};
use std::collections::BTreeMap;
use std::path::Path;

const IMAGE: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA2NCA2NCI+PHJlY3Qgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0IiByeD0iMTIiIGZpbGw9IiM1YzZhODQiLz48dGV4dCB4PSIzMiIgeT0iNDEiIGZpbGw9IndoaXRlIiBmb250LXNpemU9IjI4IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIj5TPC90ZXh0Pjwvc3ZnPg==";

fn shell(
    ctx: &campfire_retained::Context,
    title: Option<String>,
    head: impl Template,
    content: impl Template,
) -> String {
    layouts::Auth {
        ctx,
        page_title: title,
        head: h::raw(head.render().unwrap()),
        content: h::raw(content.render().unwrap()),
    }
    .render()
    .unwrap()
}

fn asset(logical: &str) -> Vec<u8> {
    let path = campfire_assets::asset_path(logical);
    campfire_assets::serve(&campfire_assets::StaticRequest {
        method: "GET",
        path: &path,
        ..Default::default()
    })
    .unwrap()
    .body
    .to_vec()
}

fn local_html(mut html: String, files: &mut BTreeMap<String, Vec<u8>>) -> String {
    // Use the exact fingerprinted URLs produced by the asset owner, with local copies.
    for (logical, _) in campfire_assets::manifest() {
        let url = campfire_assets::asset_path(logical);
        if html.contains(&url) {
            let relative = match *logical {
                "auth.css" => "auth.css".to_string(),
                "auth.js" => "auth.js".to_string(),
                _ => format!("images/{logical}"),
            };
            html = html.replace(&url, &format!("./{relative}"));
            files.entry(relative).or_insert_with(|| asset(logical));
        }
    }
    // Account logos/avatars are signed storage URLs, never available to a static server.
    let images =
        regex::Regex::new(r#"(?:src|href)="(/(?:rails/active_storage|users/[^/]+/avatar)[^"]*)""#)
            .unwrap();
    let urls = images
        .captures_iter(&html)
        .map(|c| c[1].to_owned())
        .collect::<Vec<_>>();
    for url in urls {
        html = html.replace(&url, IMAGE);
    }
    html
}

#[tokio::test]
async fn auth_pages_screenshot_fixtures_match_rust_rendering() {
    use layouts::Page;
    let app = TestApp::boot_seed_with_env(
        "default",
        seed_clock(),
        &[
            ("LEGAL_OPERATOR_NAME", "Example <&> Labs"),
            ("LEGAL_CONTACT_EMAIL", "team+auth@example.test"),
            ("LEGAL_EFFECTIVE_DATE", "2026-10-07"),
        ],
    )
    .await
    .expect("restore the frozen seeds before rendering auth fixtures");
    let first = TestApp::boot_seed_with_env("first_run", seed_clock(), &[])
        .await
        .expect("first-run seed required");
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/two_factor_views.json")).unwrap();
    let qr = crate::controllers::qr_code::two_factor_svg(vectors["uri"].as_str().unwrap()).unwrap();
    let codes = vectors["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let mut files = BTreeMap::new();
    let mut css = String::from_utf8(asset("auth.css")).unwrap();
    for (logical, _) in campfire_assets::manifest()
        .iter()
        .filter(|(logical, _)| logical.starts_with("fonts/"))
    {
        css = css.replace(
            &campfire_assets::asset_path(logical),
            &format!("./{logical}"),
        );
        files.insert(logical.to_string(), asset(logical));
    }
    files.insert("auth.css".into(), css.into_bytes());
    files.insert("auth.js".into(), asset("auth.js"));
    for name in [
        "sign-in",
        "sign-in-google-alert",
        "join",
        "first-run",
        "two-factor-setup",
        "two-factor-challenge-alert",
        "backup-codes",
        "sudo-all",
        "sudo-totp",
        "sudo-continue",
        "transfer",
    ] {
        let html = crate::controllers::users::people_tests::render_with(
            if name == "first-run" { &first } else { &app },
            |ctx| {
                ctx.account.name = "Signal".into();
                ctx.account.logo_url = IMAGE.into();
                ctx.account.has_logo = true;
                if matches!(
                    name,
                    "sign-in"
                        | "sign-in-google-alert"
                        | "join"
                        | "first-run"
                        | "two-factor-challenge-alert"
                        | "transfer"
                ) {
                    ctx.current_user = None;
                }
                if let Some(user) = ctx.current_user.as_mut() {
                    user.avatar_url = IMAGE.into();
                }
                ctx.flash_alert = match name {
                    "sign-in-google-alert" => Some("Too many requests or unauthorized.".into()),
                    "two-factor-challenge-alert" => Some(
                        "That code didn't work. Check your authenticator app or try a backup code."
                            .into(),
                    ),
                    _ => None,
                };
            },
            |ctx| {
                let ctx = crate::controllers::users::people_tests::retained(ctx);
                match name {
                    "sign-in" | "sign-in-google-alert" => {
                        let page = sessions::New {
                            ctx: &ctx,
                            email_address: None,
                            help_contact: None,
                            google_sign_in_domains: if name == "sign-in" {
                                vec![]
                            } else {
                                vec!["smartdata.net".into(), "cnbssoftware.com".into()]
                            },
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "join" => {
                        let page = users::New {
                            ctx: &ctx,
                            join_code: "fixture-join-code".into(),
                            help_contact: None,
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "first-run" => {
                        let page = first_runs::Show { ctx: &ctx };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "two-factor-setup" => {
                        let page = two_factor::Setup {
                            ctx: &ctx,
                            key: vectors["key"].as_str().unwrap().into(),
                            qr: qr.clone(),
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "two-factor-challenge-alert" => {
                        let page = two_factor::Challenge { ctx: &ctx };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "backup-codes" => {
                        let page = two_factor::BackupCodes {
                            ctx: &ctx,
                            codes: codes.clone(),
                            signed_out: 2,
                            continue_url: "/app/settings/security".into(),
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "sudo-all" | "sudo-totp" => {
                        let page = sudos::New {
                            ctx: &ctx,
                            password: name == "sudo-all",
                            totp: true,
                            google: name == "sudo-all",
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "sudo-continue" => {
                        let page = sudos::Continue {
                            ctx: &ctx,
                            method: "patch".into(),
                            path: "/account/users/127326141".into(),
                            params: serde_json::json!({"user":{"name":"David <&>"}}),
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    "transfer" => {
                        let page = sessions::TransferShow {
                            ctx: &ctx,
                            action: "/session/transfers/fixture-transfer-token".into(),
                        };
                        shell(&ctx, page.page_title(), page.as_head(), page.as_content())
                    }
                    _ => unreachable!(),
                }
            },
        );
        let html = local_html(html, &mut files);
        files.insert(format!("{name}.html"), html.into_bytes());
    }
    for name in ["about", "privacy", "terms"] {
        let reply = app.anonymous().get(&format!("/{name}")).await;
        assert_eq!(reply.status, axum::http::StatusCode::OK);
        let html = local_html(reply.text(), &mut files);
        files.insert(format!("{name}.html"), html.into_bytes());
    }
    assert!(
        !std::str::from_utf8(&files["auth.css"])
            .unwrap()
            .contains("url(\"/assets/"),
        "fixture CSS must retain its local font URLs"
    );
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/e2e/fixtures/auth-pages");
    let update = std::env::var("UPDATE_AUTH_PAGE_FIXTURES").as_deref() == Ok("1");
    let mut differing = Vec::new();
    for (name, bytes) in files {
        let bytes = tidy(&name, bytes);
        let path = directory.join(&name);
        if update {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        } else if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
            differing.push(name);
        }
    }
    assert!(
        differing.is_empty(),
        "auth fixtures differ: {differing:?}. Rerun with UPDATE_AUTH_PAGE_FIXTURES=1 cargo test -j 4 -p campfire --bin campfire -- auth_pages_screenshot_fixtures"
    );
}

/// Text fixtures without trailing whitespace on any line, so they pass `git diff --check`; the
/// pages have no preformatted text for that to change.
fn tidy(name: &str, bytes: Vec<u8>) -> Vec<u8> {
    if ![".html", ".css", ".js"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        return bytes;
    }
    let text = String::from_utf8(bytes).unwrap();
    let mut tidied = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    tidied.truncate(tidied.trim_end().len());
    tidied.push('\n');
    tidied.into_bytes()
}

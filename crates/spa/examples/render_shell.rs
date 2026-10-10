//! Prints the shell `render_shell` makes from the embedded dist, for the first-paint e2e
//! (`frontend/e2e/first-paint.spec.ts`), which serves it under the production CSP:
//!
//! `SPA_DIST=frontend/dist cargo run -p campfire_spa --example render_shell -- <theme> <text size> <nonce>`

use campfire_spa::{Boot, BootAccount, BootUser, built, render_shell, text_size, theme};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [stored_theme, stored_size, nonce] = args.as_slice() else {
        eprintln!("usage: render_shell <theme> <text size> <nonce>");
        std::process::exit(2);
    };
    if !built() {
        eprintln!("no dist is embedded: build the SPA and set SPA_DIST=frontend/dist");
        std::process::exit(1);
    }
    let boot = Boot {
        user: BootUser { id: 1, name: "Riel St. Amand".into(), avatar_url: "/avatar.svg".into() },
        custom_styles: None,
        account: BootAccount {
            name: Some("Smart Data".into()),
            logo_url: None,
            logo_still_url: None,
            banner_url: None,
            banner_still_url: None,
        },
        theme: theme(Some(stored_theme)),
        text_size: text_size(Some(stored_size)),
        cable_url: "/cable".into(),
        service_worker_url: None,
        version: "test".into(),
        revision: None,
        flash: None,
    };
    print!("{}", render_shell(&boot, "test-csrf-token", Some(nonce)));
}

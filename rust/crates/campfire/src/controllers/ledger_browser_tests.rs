//! Current pinned original declarations; the correctness runner owns prerequisites.
fn replay(mode: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("reference-tools/users/run_ledger_browser_assertions.py"))
        .arg(mode)
        .env("WS11UI_LEDGER_TEST_HOST", std::env::current_exe().unwrap())
        .output()
        .expect("paired original browser runner must start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{mode}: {stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains(&format!("Original browser {mode}:")), "missing paired assertion receipt");
    print!("{stdout}");
}

#[test]
#[ignore = "paired Rails/Rust Chromium gate: rust/parity/system/ws12"]
fn original_ledger_navigation_assertions() { replay("navigation"); }

#[test]
#[ignore = "paired Rails/Rust Chromium gate: rust/parity/system/ws12"]
fn original_ledger_member_assertions() { replay("members"); }

#[test]
#[ignore = "paired Rails/Rust Chromium gate: rust/parity/system/ws12"]
fn original_ledger_surface_assertions() { replay("surfaces"); }

#[test]
#[ignore = "paired Rails/Rust Chromium gate: rust/parity/system/ws12"]
fn original_ledger_lifecycle_assertions() { replay("lifecycle"); }

use crate::test_support::FORGERY_DISABLED;
mod lifecycle_host;

/// Exact pinned TestSessionController#create, available only in the private
/// test renderer. This verifies the original password and creates a real new
/// two-factor-satisfied session rather than importing a fixture session cookie.
pub(crate) async fn original_test_session(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
    use crate::app::AppCtx;
    use crate::concerns::{self, Before};
    use campfire_kit::{Response, StatusCode};

    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    if c.app().config.environment != "test" || std::env::var("WS11UI_LEDGER_HOST").as_deref() != Ok("1") {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let credentials = c.param_str("email_address").zip(c.param_str("password"))
        .map(|(email, password)| (email.to_owned(), password.to_owned()));
    let user = match credentials {
        Some((email, password)) => concerns::authenticate_by(c, email, password).await?,
        None => None,
    };
    if let Some(user) = user {
        concerns::start_new_verified_session_for(c, user).await?;
        let location = concerns::post_authenticating_url(c);
        c.redirect_to(&location)
    } else {
        Ok(Response::with_body(StatusCode::UNAUTHORIZED, "text/plain; charset=utf-8", "Unauthorized"))
    }
}

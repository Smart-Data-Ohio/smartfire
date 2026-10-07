//! Current pinned original declarations; the correctness runner owns prerequisites.
fn replay(mode: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("reference-tools/users/run_ledger_browser_assertions.py"))
        .arg(mode)
        .env("WS11UI_LEDGER_TEST_HOST", std::env::current_exe().unwrap())
        .output()
        .expect("original browser runner must start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{mode}: {stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains(&format!("Original browser {mode}:")), "missing original assertion receipt");
    print!("{stdout}");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_ledger_navigation_assertions() { replay("navigation"); }

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_ledger_member_assertions() { replay("members"); }

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_ledger_surface_assertions() { replay("surfaces"); }

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_ledger_lifecycle_assertions() { replay("lifecycle"); }

use crate::test_support::FORGERY_DISABLED;
mod lifecycle_host;

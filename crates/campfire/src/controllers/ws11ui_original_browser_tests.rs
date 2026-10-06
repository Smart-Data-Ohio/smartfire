//! Enabled by the full browser gate; each receipt has individual original assertions
//! run against the Rust server alone.
//! The ordinary toolchain has no browser. Missing image/binary/seed is a failure here.
fn replay(mode: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("reference-tools/users/run_original_browser_assertions.py"))
        .arg(mode)
        .output()
        .expect("original browser runner must start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{mode}: {stdout}\n{stderr}");
    assert!(
        stdout.contains(&format!("Original browser {mode}:")),
        "missing original receipt: {stdout}"
    );
    print!("{stdout}");
}
#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_people_assertions() {
    replay("people");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_picker_assertions() {
    replay("pickers");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_mobile_member_assertions() {
    replay("members");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_group_lifecycle_assertions() {
    replay("group");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_tour_assertions() {
    replay("tours");
}

#[test]
#[ignore = "Rust Chromium gate: parity/system/ws12"]
fn original_starred_people_assertions() {
    replay("stars");
}

// Runs in ordinary CI without Chromium, Rails or a server binary. The ignored
// gate separately exercises these leases through the real server.
#[test]
fn original_browser_port_leases_are_host_coordinated_and_ephemeral_safe() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .args(["-B", "-m", "unittest", "discover", "-s"])
        .arg(root.join("reference-tools/users"))
        .args(["-p", "test_browser_port_leases.py", "-v"])
        .output()
        .expect("kernel lease regressions must start");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

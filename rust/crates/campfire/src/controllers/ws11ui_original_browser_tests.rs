//! Enabled by the full browser gate; each receipt has individual original assertions.
//! The ordinary toolchain has no browser. Missing image/binary/seed is a failure here.
fn replay(mode: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("reference-tools/users/run_original_browser_assertions.py"))
        .arg(mode)
        .output()
        .expect("paired browser runner must start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{mode}: {stdout}\n{stderr}");
    assert!(
        stdout.contains(&format!("Original browser {mode}:")),
        "missing paired receipt: {stdout}"
    );
    print!("{stdout}");
}
macro_rules! cases {($($test:ident=>$mode:literal),+$(,)?)=>{$(
 #[test]
 #[ignore="paired Rails/Rust Chromium gate: rust/parity/system/ws12"]
 fn $test(){replay($mode);}
)+};}
cases!(original_people_assertions=>"people",original_picker_assertions=>"pickers",
 original_mobile_member_assertions=>"members",original_group_lifecycle_assertions=>"group",
 original_tour_assertions=>"tours",original_starred_people_assertions=>"stars",
 original_node_event_harness_assertions=>"worker");

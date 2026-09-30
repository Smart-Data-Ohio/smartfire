use std::{env, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=CAMPFIRE_REFERENCE");
    let root = env::var_os("CAMPFIRE_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../..")
        });
    let icons = root
        .join("config/icons.yml")
        .canonicalize()
        .expect("reference icon catalog");
    println!("cargo:rerun-if-changed={}", icons.display());
    println!("cargo:rustc-env=CAMPFIRE_ICON_CONFIG={}", icons.display());
}

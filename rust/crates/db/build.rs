//! Compiles `migrations/<VERSION>_<name>.sql` into the binary (`migrations::catalog`), so a build
//! carries exactly the schema changes its code was written against and `campfire db-migrate`
//! needs nothing beside the binary.
use std::path::Path;

fn main() {
    let dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("migrations");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("crates/db/migrations") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file = entry.file_name().into_string().expect("UTF-8 migration file name");
        if file.starts_with('.') {
            continue;
        }
        println!("cargo::rerun-if-changed={}", path.display());
        let stem = file
            .strip_suffix(".sql")
            .unwrap_or_else(|| panic!("crates/db/migrations/{file}: only <VERSION>_<name>.sql files belong here"));
        let (version, name) = stem
            .split_once('_')
            .unwrap_or_else(|| panic!("crates/db/migrations/{file}: name it <VERSION>_<name>.sql"));
        assert!(
            version.len() == 14 && version.bytes().all(|b| b.is_ascii_digit()) && !version.starts_with('0'),
            "crates/db/migrations/{file}: the version must be a 14-digit UTC timestamp (YYYYMMDDHHMMSS), as Rails names migrations"
        );
        assert!(
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "crates/db/migrations/{file}: the name must be snake_case"
        );
        assert!(entry.file_type().unwrap().is_file(), "crates/db/migrations/{file}: not a regular file");
        entries.push((version.to_owned(), name.to_owned(), path));
    }
    entries.sort();
    for pair in entries.windows(2) {
        assert!(pair[0].0 != pair[1].0, "crates/db/migrations: duplicate version {}", pair[0].0);
    }
    let mut out = String::from("&[\n");
    for (version, name, path) in &entries {
        out.push_str(&format!(
            "    ({version:?}, {name:?}, include_str!({:?})),\n",
            path.to_str().expect("UTF-8 path")
        ));
    }
    out.push_str("]\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("migrations.rs");
    std::fs::write(dest, out).unwrap();
}

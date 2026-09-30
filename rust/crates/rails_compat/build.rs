fn main() {
    let vendor = "vendor/onigmo";
    println!("cargo:rerun-if-changed={vendor}");
    println!("cargo:rerun-if-changed=native/keyword_regex.c");
    let mut build = cc::Build::new();
    build
        .include(vendor)
        .define("NOT_RUBY", None)
        .flag("-include")
        .flag(format!("{vendor}/config.h"))
        .warnings(false)
        .file("native/keyword_regex.c");
    for source in [
        "regcomp",
        "regenc",
        "regerror",
        "regexec",
        "regparse",
        "regsyntax",
        "st",
        "ascii",
        "utf_8",
        "unicode",
    ] {
        build.file(format!("{vendor}/{source}.c"));
    }
    build.compile("campfire_keyword_regex");
}

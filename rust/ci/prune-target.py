#!/usr/bin/env python3
"""Keep only registry dependencies' artifacts in a Cargo target directory before caching it.

A fresh checkout gives every workspace file a new mtime, so Cargo rebuilds workspace crates
(and their test and build-script binaries) on every run whatever the cache holds. Caching
them only costs upload, download and the eviction of other caches. Registry crates'
fingerprints don't depend on checkout mtimes, so their artifacts are what the cache is for.

Usage: prune-target.py TARGET_DIR [PROFILE_DIR...]   (default profile directory: debug)
"""
import re
import shutil
import sys
import tomllib
from pathlib import Path

LOCK = Path(__file__).resolve().parents[1] / "Cargo.lock"
KEPT = {"deps", "build", ".fingerprint"}
ARTIFACT = re.compile(r"^(.+?)-[0-9a-f]{16}(?:[.-].*)?$")


def registry_crates(lock=LOCK):
    packages = tomllib.loads(lock.read_text())["package"]
    return {p["name"].replace("-", "_") for p in packages if "source" in p}


def is_registry(name, registry):
    """deps/, build/ and .fingerprint/ entries are CRATE-HASH..., libraries libCRATE-HASH...;
    a crate may itself start with "lib" (libc: libc-HASH.d, liblibc-HASH.rlib)."""
    match = ARTIFACT.match(name)
    if not match:
        return False
    crate = match.group(1).replace("-", "_")
    return crate in registry or (crate.startswith("lib") and crate[3:] in registry)


def remove(path):
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink()


def prune(profile, registry):
    removed = 0
    if not profile.is_dir():
        return removed
    for entry in profile.iterdir():
        if entry.name not in KEPT:
            remove(entry)
            removed += 1
            continue
        for artifact in entry.iterdir():
            if not is_registry(artifact.name, registry):
                remove(artifact)
                removed += 1
    return removed


def main(argv):
    if not argv:
        sys.exit(__doc__)
    target = Path(argv[0])
    registry = registry_crates()
    for name in argv[1:] or ["debug"]:
        print(f"prune-target: removed {prune(target / name, registry)} entries from {target / name}")
    for entry in target.iterdir():
        if entry.name not in (argv[1:] or ["debug"]):
            remove(entry)


if __name__ == "__main__":
    main(sys.argv[1:])

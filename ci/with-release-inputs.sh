#!/usr/bin/env bash
set -euo pipefail

# Run a Cargo command with only the source inputs copied by Dockerfile's builder.
# In CI this wraps the existing binary build, so the guard needs no extra build or Docker image.
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$root"
mkdir -p .scratch
inputs=$(mktemp -d .scratch/release-inputs.XXXXXX)
trap 'rm -rf -- "$inputs"' EXIT

cp -a Cargo.toml Cargo.lock crates "$inputs/"

# The asset build script reads the frontend inputs the Dockerfile copies from web/.
# No vectors, fixtures, parity files, reference tools or Rails files are available.
for path in app/assets app/javascript vendor/javascript public config/importmap.rb config/initializers/assets.rb; do
    mkdir -p "$inputs/web/$(dirname -- "$path")"
    cp -a "web/$path" "$inputs/web/$path"
done

"$@" --manifest-path "$inputs/Cargo.toml" --target-dir "${CARGO_TARGET_DIR:-target}"

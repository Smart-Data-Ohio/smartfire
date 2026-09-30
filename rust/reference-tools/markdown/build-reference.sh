#!/usr/bin/env bash
# Build the accepted oracle without changing another worker's shared Docker tags.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
pin=79b453836f9e7a40f96aabb1fae7613a089d939d
image=ws5-reference-79b45383
scratch="$root/../.scratch/reference-$pin"
mkdir -p "$scratch"
git -C "$root/.." archive "$pin" | tar -x -C "$scratch"
docker build -t "$image:app" -f "$scratch/Dockerfile" \
  --build-arg APP_VERSION=parity --build-arg GIT_REVISION="$pin" "$scratch"
docker build -t "$image" -f "$scratch/rust/parity/docker/Dockerfile" \
  --build-arg BASE_IMAGE="$image:app" \
  --build-context fixtures="$scratch/test/fixtures" "$scratch/rust/parity/docker"

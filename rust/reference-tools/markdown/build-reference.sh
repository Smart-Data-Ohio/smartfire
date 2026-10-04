#!/usr/bin/env bash
# Build the accepted oracle without changing another worker's shared Docker tags.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
pin=$(cat "$root/parity/reference.sha")
image=${WS5_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}}
scratch="$root/../.scratch/reference-$pin"
mkdir -p "$scratch"
git -C "$root/.." archive "$pin" | tar -x -C "$scratch"
docker build -t "$image:app" -f "$scratch/Dockerfile" \
  --build-arg APP_VERSION=parity --build-arg GIT_REVISION="$pin" "$scratch"
docker build -t "$image" -f "$scratch/rust/parity/docker/Dockerfile" \
  --build-arg BASE_IMAGE="$image:app" \
  --build-context fixtures="$scratch/test/fixtures" "$scratch/rust/parity/docker"

#!/usr/bin/env bash
# Render actual default-seed image messages using the pinned Rails image and an owned seed copy.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
SEED=${SEED:-$ROOT/parity/.seed/default}
STORE=${STORE:-/home/riels/.cache/rust-port/ws6/image-reference}
OUT=${OUT:-crates/views/tests/golden/core/images.json}
rm -rf "$STORE"; mkdir -p "$STORE/db" "$STORE/storage"
cp -a --reflink=auto "$SEED/db/." "$STORE/db/"
cp -a --reflink=auto "$SEED/storage/." "$STORE/storage/"
docker run --rm --name "ws6-image-goldens-$$" --cpus 2 \
  --user "$(id -u):$(id -g)" --env-file parity/.env.reference \
  -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -e 'FAKETIME=2026-03-02 16:00:00' \
  -v "$(realpath "$STORE/db"):/rails/storage/db" -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$(realpath "$SEED"):/seed:ro" -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws6-reference-d7c7de92}" \
  bin/rails runner /work/reference-tools/views/core/images.rb
cp "$STORE/db/image-goldens.json" "$OUT"

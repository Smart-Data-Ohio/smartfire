#!/usr/bin/env bash
# Regenerates crates/views/tests/golden/core from our Rails (goldens.rb): a fresh database with the
# test fixtures (loaded by goldens.rb), the clock frozen at FROZEN_AT, rendered in the reference image. The HTML goldens
# land as one file each; the helper and cache-key vectors as JSON.
#
#   reference-tools/views/core/run.sh
#
# PARITY_IMAGE / PARITY_OWNER pick the reference image and label the containers as for
# parity/bin/reference.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
FROZEN_AT=${FROZEN_AT:-2026-02-10T12:00:00Z}
STORE=${STORE:-target/views-core-reference}
OUT=${OUT:-crates/views/tests/golden/core}

rm -rf "$STORE"; mkdir -p "$STORE/db" "$STORE/storage"
parity/bin/reference exec --storage "$STORE" --time "$FROZEN_AT" --freeze -- bin/rails db:prepare >/dev/null
parity/bin/reference runner --storage "$STORE" --time "$FROZEN_AT" --freeze reference-tools/views/core/goldens.rb
python3 reference-tools/views/core/split.py "$STORE/db/goldens.json" "$OUT"

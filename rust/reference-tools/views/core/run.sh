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
# Give the throwaway container an explicit worker-owned name. The shared parity runner's
# anonymous exec containers otherwise get random names, outside the worker's namespace.
docker run --rm --name "${PARITY_OWNER:-ws6}-views-goldens-$$" --cpus 2 \
  --user "$(id -u):$(id -g)" --env-file parity/.env.reference \
  -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -e "FAKETIME=$(date -u -d "$FROZEN_AT" '+%Y-%m-%d %H:%M:%S')" \
  -v "$(realpath "$STORE/db"):/rails/storage/db" -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-campfire-reference}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/views/core/goldens.rb'
mkdir -p crates/views/data
cp "$STORE/db/message-icons.json" crates/views/data/message-icons.json
python3 reference-tools/views/core/split.py "$STORE/db/goldens.json" "$OUT"
python3 - "$OUT" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
print(f"Rails core goldens: {len(list((root / 'layouts').iterdir()))} layouts, {len(list((root / 'partials').iterdir()))} partials, {len(list((root / 'pages').iterdir()))} pages")
PY

#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
STORE=${STORE:-../.scratch/member-panel-reference}
mkdir -p "$STORE"
WORK=$(mktemp -d -p "$STORE" run.XXXXXX)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/db" "$WORK/storage"
docker run --rm --name "${PARITY_OWNER:-ws11ui}-member-panel-goldens" --cpus 2 \
  --user "$(id -u):$(id -g)" --env-file parity/.env.reference \
  -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -e 'FAKETIME=2026-02-10 12:00:00' \
  -v "$(realpath "$WORK/db"):/rails/storage/db" -v "$(realpath "$WORK/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws11ui-reference:d7c7de92}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/views/member_panel/goldens.rb'
mkdir -p crates/views/tests/golden/member_panel
cp "$WORK/db/member-panel.json" crates/views/tests/golden/member_panel/partials.json

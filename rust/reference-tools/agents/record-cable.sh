#!/usr/bin/env bash
# WS11 re-records the existing WS7 goldens from the current pin after fixing bot seeds.
set -euo pipefail
RUST_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$RUST_ROOT"
export PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92
export TMPDIR="$RUST_ROOT/../.scratch" CARGO_TARGET_DIR="$RUST_ROOT/target" CABLE_TEST_PORT_RANGE=52200-52239
mkdir -p "$TMPDIR/cable"
if docker container inspect ws11-reference-52240 >/dev/null 2>&1; then
  echo 'ws11-reference-52240 already exists; refusing to replace it' >&2
  exit 1
fi
trap 'parity/bin/reference down --port 52240 >/dev/null 2>&1 || true' EXIT
parity/bin/reference up --seed first_run --port 52240
parity/bin/reference runner --port 52240 crates/campfire/src/channels/tests/golden/fixtures.rb > "$TMPDIR/cable/channels-fixtures.json"
parity/bin/reference runner --port 52240 crates/cable/tests/golden/fixtures.rb > "$TMPDIR/cable/cable-fixtures.json"
CHANNELS_REFERENCE_PORT=52240 CHANNELS_REFERENCE_CONTAINER=ws11-reference-52240 CHANNELS_REFERENCE_FIXTURES="$TMPDIR/cable/channels-fixtures.json" \
  mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire channels::tests::golden::record_reference -- --ignored --nocapture
CABLE_REFERENCE_URL=ws://127.0.0.1:52240/cable CABLE_REFERENCE_FIXTURES="$TMPDIR/cable/cable-fixtures.json" \
  mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_cable --test golden record_reference -- --ignored --nocapture
printf '%s\n' 'WS11 cable recording: both Rails goldens regenerated (d7c7de92)'

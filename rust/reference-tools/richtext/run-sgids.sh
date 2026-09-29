#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
image="${WS5_REFERENCE_IMAGE:-ws5-reference-79b45383}"
"$root/reference-tools/markdown/verify-reference.sh"
docker run --rm --name ws5-sgid-json-goldens --entrypoint '' \
  --env-file "$root/parity/.env.reference" -e RAILS_LOG_LEVEL=error \
  -e DATABASE_URL=sqlite3:/tmp/ws5-sgid-json.sqlite3 \
  -e SGID_RANDOM_CASES="${SGID_RANDOM_CASES:-10000}" \
  -v "$root/reference-tools/richtext:/tools:ro" \
  -v "$root/crates/richtext/tests/corpus:/corpus" \
  --user "$(id -u):$(id -g)" "$image" \
  bash -c 'bin/rails db:schema:load >/dev/null && bin/rails runner /tools/generate-sgids.rb'

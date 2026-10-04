#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
image="${WS5_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}}"
"$root/reference-tools/markdown/verify-reference.sh"
docker run --rm --name ws5-markdown-goldens --entrypoint '' \
  --env-file "$root/parity/.env.reference" -e RAILS_LOG_LEVEL=error \
  -e DATABASE_URL=sqlite3:/tmp/ws5-markdown.sqlite3 \
  -v "$root/reference-tools/markdown:/tools:ro" \
  -v "$root/crates/richtext:/output" \
  "$image" bash -c 'bin/rails db:schema:load >/dev/null && bin/rails runner /tools/generate.rb'

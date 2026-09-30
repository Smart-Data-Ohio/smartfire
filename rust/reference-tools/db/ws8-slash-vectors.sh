#!/usr/bin/env bash
# Generate slash expectations from our immutable Rails pin, with no Rails source edits.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
scratch=/home/riels/.cache/rust-port/ws8slash
pin=fec615be407f2350de9c364f78a322c4ad48a2cf
mkdir -p "$scratch/pin" "$scratch/tmp"
git -C "$root" archive "$pin" app db lib config test Gemfile Gemfile.lock | tar -x -C "$scratch/pin"
docker run --rm --cpus 2 --name ws8slash-oracle --entrypoint '' \
  --env-file "$root/rust/parity/.env.reference" -e RAILS_ENV=test -e TMPDIR=/rails/tmp \
  -e CAMPFIRE_FIXTURES_NOW='2026-09-23 12:00:00' \
  -v "$scratch/pin/app:/rails/app:ro" -v "$scratch/pin/db:/rails/db:ro" \
  -v "$scratch/pin/test:/rails/test:ro" -v "$scratch/pin/config:/rails/config:ro" \
  -v "$scratch/pin/lib:/rails/lib:ro" -v "$scratch/pin/Gemfile.lock:/oracle-Gemfile.lock:ro" \
  -v "$root/rust/crates/db/ruby:/tools:ro" -v "$root/rust/crates/db/src/tests:/out" \
  -v "$scratch/tmp:/rails/tmp" "${PARITY_IMAGE:-ws8-reference-models}" \
  sh -ec 'cmp /rails/Gemfile.lock /oracle-Gemfile.lock; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_slash_vectors.rb /out/ws8_slash_vectors.json /out/ws8_slash_zones.json'

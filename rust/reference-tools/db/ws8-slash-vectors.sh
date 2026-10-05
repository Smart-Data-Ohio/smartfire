#!/usr/bin/env bash
# Generate slash expectations from our immutable Rails pin, with no Rails source edits.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
pin=$(cat "$root/rust/parity/reference.sha")
[[ $pin =~ ^[0-9a-f]{40}$ ]] || { echo 'ws8-slash-vectors: invalid reference.sha' >&2; exit 1; }
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ws8-slash.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/pin" "$scratch/tmp"
git -C "$root" archive "$pin" | tar -x -C "$scratch/pin"
image=${PARITY_IMAGE:-ws8-slash-reference:$pin}
# The default image is keyed by the Rails pin, including its Gemfile.lock. An
# explicit PARITY_IMAGE still works, but must pass the lockfile check below.
if [ -z "${PARITY_IMAGE:-}" ] && ! docker image inspect "$image" >/dev/null 2>&1; then
  CAMPFIRE_REFERENCE="$scratch/pin" PARITY_RUNTIME=docker \
    PARITY_IMAGE="$image" PARITY_REFERENCE_APP_IMAGE="ws8-slash-reference-app:$pin" \
    PARITY_REFERENCE_REVISION="$pin" "$root/rust/parity/bin/reference" build
fi
docker run --rm --cpus 2 --name ws8-slash-oracle --entrypoint '' \
  --env-file "$root/rust/parity/.env.reference" -e RAILS_ENV=test -e TMPDIR=/rails/tmp \
  -e CAMPFIRE_FIXTURES_NOW='2026-09-23 12:00:00' -e PARITY_REFERENCE_SHA="$pin" \
  -v "$scratch/pin/app:/rails/app:ro" -v "$scratch/pin/db:/rails/db:ro" \
  -v "$scratch/pin/test:/rails/test:ro" -v "$scratch/pin/config:/rails/config:ro" \
  -v "$scratch/pin/lib:/rails/lib:ro" -v "$scratch/pin/Gemfile.lock:/oracle-Gemfile.lock:ro" \
  -v "$root/rust/crates/db/ruby:/tools:ro" -v "$root/rust/crates/db/src/tests:/out" \
  -v "$scratch/tmp:/rails/tmp" "$image" \
  sh -ec 'cmp /rails/Gemfile.lock /oracle-Gemfile.lock; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_slash_vectors.rb /out/ws8_slash_vectors.json /out/ws8_slash_zones.json; bin/rails runner /tools/ws8_slash_review_vectors.rb /out/ws8_slash_review_vectors.json /out/ws8_slash_review_zones.json /out/ws8_slash_offsets.json'

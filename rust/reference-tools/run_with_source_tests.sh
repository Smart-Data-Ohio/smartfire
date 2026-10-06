#!/usr/bin/env bash
# Named declaration producers need the pinned tests omitted from the production image.
# Mount them as read-only inputs; application behavior remains the plain reference image.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/reference-tools/reference_image_env.sh"
REFERENCE=${CAMPFIRE_REFERENCE:-$ROOT/..}
source "$ROOT/reference-tools/rails_link_mounts.sh"
mapfile -t LINK_MOUNTS < <(rails_link_mounts "$REFERENCE")
# test/fixtures may be a symlink to rust/fixtures: compare what it resolves to with the pin's.
git -C "$REFERENCE" diff --quiet "$PARITY_REFERENCE_SHA" -- test ':(exclude)test/fixtures'
pinned_fixtures=$(mktemp -d)
git -C "$REFERENCE" archive "$PARITY_REFERENCE_SHA" test/fixtures | tar -x -C "$pinned_fixtures"
diff -rq "$pinned_fixtures/test/fixtures" "$REFERENCE/test/fixtures/" >/dev/null
rm -rf "$pinned_fixtures"
WORK=$(mktemp -d -p "${TMPDIR:-$ROOT/../.scratch}" source-tests.XXXXXX)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/db" "$WORK/storage"
cp -a --reflink=auto "$ROOT/parity/.seed/default/db/." "$WORK/db/"
cp -a --reflink=auto "$ROOT/parity/.seed/default/storage/." "$WORK/storage/"
SCRIPT=$1
shift
docker run --rm --network none --name "${PARITY_OWNER:-parity}-source-tests-$$" --cpus "${PARITY_CPUS:-2}" \
  --user "$(id -u):$(id -g)" --env-file "$ROOT/parity/.env.reference" \
  -e PARITY_REFERENCE_SHA -e PARITY_WORK=/work -e PARITY_REDIS=1 -e RAILS_LOG_LEVEL=warn \
  -e 'FAKETIME=2026-03-02 16:00:00' \
  -v "$WORK/db:/rails/storage/db" -v "$WORK/storage:/rails/storage/files" \
  -v "$ROOT:/work:ro" -v "$ROOT/vectors:/work/vectors" -v "$(realpath "$REFERENCE/test"):/rails/test:ro" "${LINK_MOUNTS[@]}" \
  "$PARITY_IMAGE" bin/rails runner --skip-executor "/work/$SCRIPT" "$@"

#!/usr/bin/env bash
# #213's reviewed Rails quote-URL fix is newer than the shared reference pin.
# Build an unmodified checkout of that exact Rails commit; never patch the pin image.
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
revision=955af4c3781bef07b97b7aefce12c376110a812c
scratch=${WS8BM2_REFERENCE_SCRATCH:-$root/.scratch/twitter-reference-955af4c37}
mkdir -p "$scratch"
context=$(mktemp -d "$scratch/source.XXXXXXXX")
trap 'rm -rf -- "$context"' EXIT
git -C "$root" archive "$revision" | tar -x -C "$context"
CAMPFIRE_REFERENCE="$context" PARITY_REFERENCE_REVISION="$revision" \
 PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_RUNTIME=docker \
 PARITY_IMAGE=ws8bm2-reference:twitter-955af4c37 \
 PARITY_REFERENCE_APP_IMAGE=ws8bm2-reference:twitter-955af4c37-app \
 "$root/rust/parity/bin/reference" build

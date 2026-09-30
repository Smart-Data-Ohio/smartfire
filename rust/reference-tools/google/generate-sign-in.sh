#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_NAMESPACE=ws14g PARITY_OWNER=ws14g PARITY_IMAGE="${WS14G_REFERENCE_IMAGE:-ws9-reference:d7c7de92}" \
  "$ROOT/parity/bin/reference" runner "$ROOT/reference-tools/google/sign_in.rb" > "$ROOT/vectors/google_sign_in.json"
echo 'Google sign-in vectors: generated from pinned Rails'

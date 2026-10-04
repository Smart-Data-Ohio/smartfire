#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$ROOT/../.scratch/auth"
export PARITY_IMAGE=${WS9_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}}
source "$ROOT/reference-tools/reference_image_env.sh"
docker run --rm --name ws9-two-factor-vectors --network none --entrypoint "" \
  --user "$(id -u):$(id -g)" --env-file "$ROOT/parity/.env.reference" \
  -e PARITY_REFERENCE_SHA -e RAILS_LOG_LEVEL=fatal -e DATABASE_URL=sqlite3:/work/scratch/vectors.sqlite3 -e VECTORS_DIR=/work/vectors \
  -v "$ROOT/reference-tools:/work/reference-tools:ro" -v "$ROOT/vectors:/work/vectors" \
  -v "$ROOT/../.scratch/auth:/work/scratch" "$PARITY_IMAGE" \
  bin/rails runner /work/reference-tools/auth/two_factor_vectors.rb

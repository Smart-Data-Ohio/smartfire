#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_NAMESPACE=ws9 PARITY_OWNER=ws9 PARITY_IMAGE=${WS9_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}} \
  "$ROOT/parity/bin/reference" runner --seed default \
  -e WS9_TIME_ZONE_VECTORS=/work/crates/db/src/rails_time_zones.json \
  -e WS9_ROUND_FOUR_VECTORS=/work/vectors/round_four_security.json \
  --time 2026-03-02T16:00:00Z --freeze "$ROOT/reference-tools/auth/round_four.rb"

#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_NAMESPACE=ws9 PARITY_OWNER=ws9 PARITY_IMAGE=${WS9_REFERENCE_IMAGE:-ws9-reference:d7c7de92} \
  "$ROOT/parity/bin/reference" runner --seed default -e WS9_ROUND_THREE_VECTORS=/work/vectors/round_three_security.json \
  --time 2026-03-02T16:00:00Z --freeze "$ROOT/reference-tools/auth/round_three.rb"

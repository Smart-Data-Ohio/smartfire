#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_NAMESPACE=ws9 PARITY_OWNER=ws9 PARITY_IMAGE=${WS9_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}} \
  "$ROOT/parity/bin/reference" runner --seed default -e WS9_FULL_PAGE_GOLDENS=/work/vectors/auth_full_pages.json \
  --time 2026-03-02T16:00:00Z --freeze "$ROOT/reference-tools/auth/full_pages.rb"

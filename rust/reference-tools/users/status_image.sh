#!/usr/bin/env bash
# Validate the plain pinned status oracle image; #163 is now part of the pin.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_IMAGE="${PARITY_IMAGE:-campfire-reference}" python3 "$ROOT/reference-tools/users/verify_status_image.py"

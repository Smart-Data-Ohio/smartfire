#!/usr/bin/env bash
# Build the canonical plain Rails reference through the shared parity builder.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
exec "$ROOT/parity/bin/reference" build

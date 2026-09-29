#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
image="${WS5_REFERENCE_IMAGE:-campfire-reference}"
reference="$(cd "${CAMPFIRE_REFERENCE:-$root/..}" && pwd)"
mapfile -t files < "$root/reference-tools/markdown/reference-files.txt"
# Hash the actual fork, including helpers/partials and gem versions, before recording goldens.
diff -u <(cd "$reference" && sha256sum "${files[@]}") \
  <(docker run --rm --name ws5-oracle-check --entrypoint '' -w /rails "$image" sha256sum "${files[@]}")

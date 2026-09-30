#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
image="${WS5_REFERENCE_IMAGE:-ws5-reference-79b45383}"
reference="$(cd "${CAMPFIRE_REFERENCE:-$root/..}" && pwd)"
mapfile -t files < "$root/reference-tools/markdown/reference-files.txt"
# Hash the actual fork, including helpers/partials and gem versions, before recording goldens.
hashes="$root/reference-tools/markdown/reference-hashes.sha256"
diff -u "$hashes" <(cd "$reference" && sha256sum "${files[@]}")
diff -u "$hashes" <(docker run --rm --name ws5-oracle-check --entrypoint '' -w /rails "$image" sha256sum "${files[@]}")

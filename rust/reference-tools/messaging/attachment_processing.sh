#!/usr/bin/env bash
# Fresh #226 output from the plain pinned reference image and media/harness.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
script="${1:-$root/reference-tools/messaging/attachment_processing.rb}"
if [ "$#" -gt 0 ]; then shift; fi
image="${PARITY_PINNED_IMAGE:-campfire-reference}"
revision=$(docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$image" | sed -n 's/^GIT_REVISION=//p')
[ "$revision" = "$(cat "$root/parity/reference.sha")" ] || { echo 'attachment-processing oracle requires the pinned reference image' >&2; exit 1; }
PARITY_IMAGE="$image" "$root/parity/bin/reference" runner --seed default --time 2026-03-02T16:00:00Z --freeze "$script" "$@"

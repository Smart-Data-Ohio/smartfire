#!/usr/bin/env bash
# Fresh #226 output on the pinned media/harness, with the approved post-pin behavior overlaid.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$root/.scratch"
scratch=$(mktemp -d "$root/.scratch/attachment-processing-oracle.XXXXXX")
image="attachment-processing-oracle-$$"
cleanup() { docker image rm "$image" >/dev/null 2>&1 || true; rm -rf "$scratch"; }
trap cleanup EXIT
script="${1:-$root/reference-tools/messaging/attachment_processing.rb}"
if [ "$#" -gt 0 ]; then shift; fi
reference="${CAMPFIRE_REFERENCE:-$root/..}"
base="${PARITY_PINNED_IMAGE:-campfire-reference}"
revision=$(docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$base" | sed -n 's/^GIT_REVISION=//p')
[ "$revision" = "$(cat "$root/parity/reference.sha")" ] || { echo 'attachment-processing oracle requires the pinned reference image' >&2; exit 1; }
for file in app/models/message/attachment.rb app/jobs/message/attachment_processing_job.rb app/helpers/messages/attachment_presentation.rb app/helpers/github/pull_requests_helper.rb; do
  mkdir -p "$scratch/$(dirname "$file")"
  cp "$reference/$file" "$scratch/$file"
done
printf 'FROM %s\nCOPY app /rails/app\n' "$base" > "$scratch/Dockerfile"
docker build -q -t "$image" "$scratch" >&2
PARITY_IMAGE="$image" "$root/parity/bin/reference" runner --seed default --time 2026-03-02T16:00:00Z --freeze "$script" "$@"

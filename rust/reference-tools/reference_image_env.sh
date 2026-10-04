#!/usr/bin/env bash
# Source before direct Docker oracle runs; preserve the harness's stable X-Rev header.
REFERENCE_TOOLS_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
export PARITY_IMAGE=${PARITY_IMAGE:-campfire-reference}
PARITY_REFERENCE_SHA=$(docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$PARITY_IMAGE" | sed -n 's/^GIT_REVISION=//p')
[ "$PARITY_REFERENCE_SHA" = "$(cat "$REFERENCE_TOOLS_ROOT/parity/reference.sha")" ] || {
  echo 'reference image revision does not match parity/reference.sha' >&2
  return 1
}
export PARITY_REFERENCE_SHA

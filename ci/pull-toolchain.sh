#!/usr/bin/env bash
set -euo pipefail

reference="$1"
error="$(mktemp)"
trap 'rm -f "$error"' EXIT

# A missing package can also look like an auth error on GHCR, especially for fork tokens.
# Resolution failures use a local build; a resolved digest must pull successfully.
if ! digest="$(docker buildx imagetools inspect "$reference" --format '{{json .Manifest}}' 2>"$error" | jq -er '.digest')"; then
  cat "$error" >&2
  echo '::warning::GHCR toolchain manifest unavailable; building the toolchain locally.'
  echo 'pulled=false' >> "$GITHUB_OUTPUT"
  exit 0
fi
[[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || { echo '::error::Invalid toolchain digest'; exit 1; }
repository="${reference%:*}"
docker pull --platform linux/amd64 "${repository}@${digest}"
docker tag "${repository}@${digest}" campfire-toolchain
echo 'pulled=true' >> "$GITHUB_OUTPUT"
echo "Using ${repository}@${digest}"

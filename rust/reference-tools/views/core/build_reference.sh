#!/usr/bin/env bash
# Reuse the canonical Rails runtime/gems with the accepted Rails source pin.
# The base must be a parity reference image built from our Rails Dockerfile and Gemfile.lock.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
REPO=$(git -C "$ROOT" rev-parse --show-toplevel)
SOURCE=${REFERENCE_REF:-d7c7de92}
BASE=${REFERENCE_BASE_IMAGE:-ws6-reference}
IMAGE=${PARITY_IMAGE:-ws6-reference-d7c7de92}
SCRATCH=${WS6_SCRATCH:-/home/riels/.cache/rust-port/ws6}
mkdir -p "$SCRATCH"
WORK=$(mktemp -d -p "$SCRATCH" ws6-reference.XXXXXX)
trap 'rm -rf "$WORK"' EXIT
SHA=$(git -C "$ROOT" rev-parse "$SOURCE")
git -C "$REPO" show "$SOURCE:Gemfile.lock" > "$WORK/reference.lock"
docker run --rm --name "ws6-reference-lock-$$" --entrypoint cat "$BASE" /rails/Gemfile.lock > "$WORK/base.lock"
cmp "$WORK/reference.lock" "$WORK/base.lock"
git -C "$REPO" archive "$SOURCE" app config lib db test vendor public | tar -xf - -C "$WORK"
cat > "$WORK/Dockerfile" <<'DOCKERFILE'
ARG BASE
FROM ${BASE}
COPY --chown=1000:1000 app /rails/app
COPY --chown=1000:1000 config /rails/config
COPY --chown=1000:1000 lib /rails/lib
COPY --chown=1000:1000 db /rails/db
COPY --chown=1000:1000 test /rails/test
COPY --chown=1000:1000 vendor /rails/vendor
COPY --chown=1000:1000 public /rails/public
DOCKERFILE
docker build --build-arg "BASE=$BASE" -t "$IMAGE" "$WORK"
printf 'WS6 reference image: %s at %s -> %s\n' "$SOURCE" "$SHA" "$IMAGE"

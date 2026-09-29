#!/usr/bin/env bash
# Reuse the canonical Rails runtime/gems/fixtures, with our actual PR #148 source trees.
# The base must be a parity reference image built from our Rails Dockerfile and Gemfile.lock.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
REPO=$(git -C "$ROOT" rev-parse --show-toplevel)
SOURCE=${REFERENCE_REF:-origin/fix/cached-fragment-csrf}
BASE=${REFERENCE_BASE_IMAGE:-ws6-reference}
IMAGE=${PARITY_IMAGE:-ws6-reference-pr148}
mkdir -p "$ROOT/target"
WORK=$(mktemp -d -p "$ROOT/target" ws6-reference.XXXXXX)
trap 'rm -rf "$WORK"' EXIT
SHA=$(git -C "$ROOT" rev-parse "$SOURCE")
git -C "$REPO" show "$SOURCE:Gemfile.lock" > "$WORK/reference.lock"
docker run --rm --name "ws6-reference-lock-$$" --entrypoint cat "$BASE" /rails/Gemfile.lock > "$WORK/base.lock"
cmp "$WORK/reference.lock" "$WORK/base.lock"
git -C "$REPO" archive "$SOURCE" app config lib | tar -xf - -C "$WORK"
cat > "$WORK/Dockerfile" <<'DOCKERFILE'
ARG BASE
FROM ${BASE}
COPY --chown=1000:1000 app /rails/app
COPY --chown=1000:1000 config /rails/config
COPY --chown=1000:1000 lib /rails/lib
DOCKERFILE
docker build --build-arg "BASE=$BASE" -t "$IMAGE" "$WORK"
printf 'WS6 reference image: %s at %s -> %s\n' "$SOURCE" "$SHA" "$IMAGE"

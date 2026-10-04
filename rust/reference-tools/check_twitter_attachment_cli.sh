#!/usr/bin/env bash
# Check a built production binary in the plain pinned Rails media/clock runtime.
set -euo pipefail
TWITTER_ATTACHMENT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TWITTER_ATTACHMENT_WORKTREE=$(cd "$TWITTER_ATTACHMENT_ROOT/.." && pwd)

if [ "${1:-}" = twitter-backfill-references ]; then
  exec docker run --rm --network none --cpus 2 --user "$(id -u):$(id -g)" \
    -v "$TWITTER_ATTACHMENT_WORKTREE:$TWITTER_ATTACHMENT_WORKTREE" -w "$TWITTER_ATTACHMENT_WORKTREE" \
    -e LD_PRELOAD=/usr/local/lib/faketime/libfaketime.so.1 \
    -e 'FAKETIME=2026-03-02 16:00:00' -e FAKETIME_DONT_FAKE_MONOTONIC=1 \
    --entrypoint "$TWITTER_ATTACHMENT_BINARY" "$PARITY_IMAGE" "$@"
fi

source "$TWITTER_ATTACHMENT_ROOT/reference-tools/reference_image_env.sh"
export TWITTER_ATTACHMENT_BINARY=${1:-$TWITTER_ATTACHMENT_ROOT/target/debug/campfire}
test -x "$TWITTER_ATTACHMENT_BINARY"
export TMPDIR
TMPDIR=$(mktemp -d "$TWITTER_ATTACHMENT_ROOT/target/twitter-attachment-cli.XXXXXX")
trap 'rm -rf "$TMPDIR"' EXIT
python3 "$TWITTER_ATTACHMENT_ROOT/reference-tools/check_twitter_attachment_cli.py" "${BASH_SOURCE[0]}"

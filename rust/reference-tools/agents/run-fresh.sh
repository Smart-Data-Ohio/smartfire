#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
FRESH="$ROOT/.scratch/fresh-ws11-github-reconciled"
test ! -e "$FRESH"
SOURCE_SHA=$(git -C "$ROOT" rev-parse HEAD)
REMOTE_SHA=$(git -C "$ROOT" ls-remote origin refs/heads/rust/ws11-agents | cut -f1)
test "$SOURCE_SHA" = "$REMOTE_SHA"
git clone --single-branch --branch rust/ws11-agents "$(git -C "$ROOT" remote get-url origin)" "$FRESH" > "$ROOT/.scratch/fresh-clone.log" 2>&1
test "$(git -C "$FRESH" rev-parse HEAD)" = "$SOURCE_SHA"
mkdir -p "$FRESH/.scratch"
cp -a "$ROOT/rust/parity/.seed" "$FRESH/rust/parity/.seed"
cd "$FRESH"
export CI=1 TMPDIR="$FRESH/.scratch" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only
export CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249
export INTEGRATION_TEST_PORT_RANGE=52250-52298 WS15E_TEST_PORT_RANGE=52250-52298 GITHUB_TEST_PORT_RANGE=52250-52298
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$FRESH/rust/reference-tools/agents/pinned-media-runner.py"
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > "$ROOT/.scratch/fresh-metadata.json"
python3 rust/reference-tools/agents/check-seeds.py > "$ROOT/.scratch/fresh-seeds.log"
printf 'WS11 fresh source: %s\n' "$SOURCE_SHA" > "$ROOT/.scratch/fresh-source.log"
set +e
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > "$ROOT/.scratch/fresh-workspace.log" 2>&1
TEST_STATUS=$?
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > "$ROOT/.scratch/fresh-clippy.log" 2>&1
CLIPPY_STATUS=$?
set -e
printf 'WS11 fresh exits: test=%s clippy=%s\n' "$TEST_STATUS" "$CLIPPY_STATUS" > "$ROOT/.scratch/fresh-exits.log"
test "$TEST_STATUS" = 0 && test "$CLIPPY_STATUS" = 0

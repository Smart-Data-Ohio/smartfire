#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
export CI=1 TMPDIR="$ROOT/.scratch" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only
export CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249
export INTEGRATION_TEST_PORT_RANGE=52250-52298 WS15E_TEST_PORT_RANGE=52250-52298 GITHUB_TEST_PORT_RANGE=52250-52298
cargo test --locked --manifest-path rust/Cargo.toml "$@" -- --test-threads=4

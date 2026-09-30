#!/usr/bin/env bash
set -euo pipefail

# CI-only linker/profile settings: ordinary developer builds never require mold or Docker.
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
: "${RUNNER_TEMP:?Set RUNNER_TEMP to a disk-backed scratch directory}"
docker run --rm --user "$(id -u):$(id -g)" \
  --volume "$repo:/src" --volume "$RUNNER_TEMP:/ci-tmp" --workdir /src/rust \
  --env HOME=/ci-tmp --env TMPDIR=/ci-tmp \
  --env CARGO_HOME="${CARGO_HOME:-/src/rust/.cargo-home}" \
  --env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/src/rust/target}" \
  --env CARGO_TERM_COLOR=always --env CARGO_INCREMENTAL=0 \
  --env CARGO_PROFILE_DEV_DEBUG=line-tables-only \
  --env CARGO_PROFILE_TEST_DEBUG=line-tables-only \
  --env RUSTFLAGS='-C link-arg=-fuse-ld=mold' \
  "${RUST_CI_IMAGE:-campfire-toolchain}" cargo "$@"

#!/usr/bin/env bash
set -euo pipefail

# CI-only linker/profile settings: ordinary developer builds never require mold or Docker.
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
: "${RUNNER_TEMP:?Set RUNNER_TEMP to a disk-backed scratch directory}"
# Runner file-command channels (including GITHUB_ENV) live under RUNNER_TEMP. Build scripts
# must only see their own scratch directory, never those channels or the parent directory.
scratch="$RUNNER_TEMP/rust-scratch"
mkdir -p -- "$scratch"
# The repository is mounted at its host path for consistent Cargo fingerprints.
# /src stays mounted for local overrides that use it.
cargo_home="${CARGO_HOME:-$repo/.cargo-home}"
if [[ -z "${CARGO_HOME:-}" ]]; then
  mkdir -p -- "$cargo_home"
  install -m 644 -- "$repo/ci/cargo-config.toml" "$cargo_home/config.toml"
fi
docker run --rm --name "${RUST_CI_CONTAINER_PREFIX:-campfire-ci}-cargo-$$" --user "$(id -u):$(id -g)" \
  --volume "$repo:$repo" --volume "$repo:/src" --volume "$scratch:/ci-tmp" --workdir "${RUST_CI_WORKDIR:-$repo}" \
  --env HOME=/ci-tmp --env TMPDIR=/ci-tmp ${RUSTUP_TOOLCHAIN:+--env RUSTUP_TOOLCHAIN} \
  --env CI \
  --env CARGO_HOME="$cargo_home" \
  --env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}" \
  --env CARGO_TERM_COLOR=always --env CARGO_INCREMENTAL=0 \
  --env CI --env RUST_TEST_THREADS=4 --env CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" \
  --env CARGO_PROFILE_DEV_DEBUG=line-tables-only \
  --env CARGO_PROFILE_TEST_DEBUG=line-tables-only \
  --env RUSTFLAGS='-C link-arg=-fuse-ld=mold' \
  "${RUST_CI_IMAGE:-campfire-toolchain}" cargo "$@"

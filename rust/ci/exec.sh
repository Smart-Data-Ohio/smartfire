#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
: "${RUNNER_TEMP:?Set RUNNER_TEMP to a disk-backed scratch directory}"
scratch="$RUNNER_TEMP/rust-correctness"
mkdir -p "$scratch"
# The same CI profile and Cargo home as cargo.sh (see cargo-config.toml).
mkdir -p "$repo/rust/.cargo-home"
install -m 644 "$repo/rust/ci/cargo-config.toml" "$repo/rust/.cargo-home/config.toml"
git_mount=()
git_common=$(git -C "$repo" rev-parse --path-format=absolute --git-common-dir)
if [[ "$git_common" != "$repo/"* ]]; then
  # Linked developer worktrees keep their objects/refs outside the source bind.
  git_mount=(--volume "$git_common:$git_common:ro")
fi
# Child Docker mounts must use the same absolute paths on the host and in this container.
# Expose only our scratch directory, never the runner's file-command channels.
# Native Selenium isolates its driver with unprivileged user/network/PID
# namespaces. Docker's default syscall/profile restrictions forbid that setup.
# Give the container a short, disk-backed home: the original managed Chrome
# profile under ~/.cache must leave room for its 108-byte SingletonSocket path.
docker run --rm --init --network host --user "$(id -u):$(id -g)" \
  --security-opt seccomp=unconfined --security-opt "apparmor=${RUST_CI_APPARMOR_PROFILE:-unconfined}" \
  --group-add "$(stat -c %g /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  "${git_mount[@]}" \
  --volume "$repo:$repo" --volume "$scratch:$scratch" --volume "$scratch:/ci-home" --workdir "$repo" \
  --env HOME=/ci-home --env TMPDIR="$scratch" --env CI=true \
  --env CAMPFIRE_CARGO=cargo --env RUST_TEST_THREADS=4 --env CARGO_BUILD_JOBS=4 \
  --env CARGO_HOME="$repo/rust/.cargo-home" --env CARGO_TARGET_DIR="$repo/rust/target" \
  --env CARGO_INCREMENTAL=0 --env CARGO_PROFILE_DEV_DEBUG=line-tables-only \
  --env CARGO_PROFILE_TEST_DEBUG=line-tables-only --env RUSTFLAGS='-C link-arg=-fuse-ld=mold' \
  --env WS12_BROWSER_SCRATCH="$scratch/ws12" --env WS13_BROWSER_SCRATCH="$scratch/ws13" \
  --env WS11UI_BROWSER_SCRATCH="$scratch/ws11ui" \
  --env WS8BM_BROWSER_SCRATCH="$scratch/ws8bm" \
  --env WS8BM_PINNED_BROWSER=1 \
  --env CORRECTNESS_SHARD --env WS8BM_PREBUILT_APP --env WS8BM_PREBUILT_TEST_HOST \
  --env WS8BM_HOST_BUILD_JOBS \
  "${RUST_CORRECTNESS_IMAGE:-campfire-correctness}" "$@"

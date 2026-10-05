#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
: "${RUNNER_TEMP:?Set RUNNER_TEMP to a disk-backed scratch directory}"
scratch="$RUNNER_TEMP/rust-correctness"
mkdir -p "$scratch"
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
docker run --rm --init --network host --user "$(id -u):$(id -g)" \
  --security-opt seccomp=unconfined --security-opt "apparmor=${RUST_CI_APPARMOR_PROFILE:-unconfined}" \
  --group-add "$(stat -c %g /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  "${git_mount[@]}" \
  --volume "$repo:$repo" --volume "$scratch:$scratch" --workdir "$repo" \
  --env HOME="$scratch" --env TMPDIR="$scratch" --env CI=true \
  --env CAMPFIRE_CARGO=cargo --env RUST_TEST_THREADS=4 --env CARGO_BUILD_JOBS=4 \
  --env CARGO_HOME="$repo/rust/.cargo-home" --env CARGO_TARGET_DIR="$repo/rust/target" \
  --env CARGO_INCREMENTAL=0 --env CARGO_PROFILE_DEV_DEBUG=line-tables-only \
  --env CARGO_PROFILE_TEST_DEBUG=line-tables-only --env RUSTFLAGS='-C link-arg=-fuse-ld=mold' \
  --env WS12_BROWSER_SCRATCH="$scratch/ws12" --env WS13_BROWSER_SCRATCH="$scratch/ws13" \
  --env WS11UI_BROWSER_SCRATCH="$scratch/ws11ui" \
  --env WS8BM_BROWSER_SCRATCH="$scratch/ws8bm" \
  --env WS14_BROWSER_SCRATCH="$scratch/ws14" \
  --env PARITY_IMAGE="${PARITY_IMAGE:-ws19b-ci-reference}" --env PARITY_CAPTURE_RUNTIME=docker \
  --env WS8BM_PINNED_BROWSER=1 \
  "${RUST_CORRECTNESS_IMAGE:-campfire-correctness}" "$@"

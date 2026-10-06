#!/usr/bin/env bash
# Runs system_cases.mjs in the pinned Playwright image (parity/Dockerfile.playwright). The
# browser container has no network; it reaches the app on the host's loopback through
# parity/capture/forward.ts, which runs on the host network and listens on a Unix socket.
set -euo pipefail
# Deliberately broken implementation used only by check_system_network.py.
if [[ "${WS11UI_BROKEN_HOST_NETWORK:-}" == 1 ]]; then
  WS11UI_HOST_NETWORK="$(readlink /proc/self/ns/net)" CHROMIUM_PATH=/usr/bin/chromium node "$(dirname "$0")/system_cases.mjs" "$@"
  exit "$?"
fi
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# system_cases.mjs resolves rust/parity/package.json from its working directory.
REPO=$(cd "$ROOT/.." && pwd)
PARITY=$ROOT/parity
die() { echo "system_browser: $*" >&2; exit 1; }
wait_for_socket() {
  for _ in $(seq 1 100); do [ -S "$1" ] && return 0; sleep 0.1; done
  die "the upstream forwarder did not start ($1)"
}
export PARITY_NAMESPACE=${PARITY_NAMESPACE:-ws11ui-system}
hash=$(cat "$PARITY/Dockerfile.playwright" "$PARITY/package-lock.json" | sha256sum | cut -c1-12)
image=$PARITY_NAMESPACE-parity-playwright:$hash
if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "system_browser: building $image" >&2
  docker build -q -f "$PARITY/Dockerfile.playwright" -t "$image" "$PARITY" >&2
fi
# Docker cannot create a missing tmpfs mountpoint below the forwarder's read-only bind.
# This is an empty mountpoint, not a source of browser dependencies.
modules_mount_created=0
if [[ ! -d "$PARITY/node_modules" ]]; then
  mkdir -p "$PARITY/node_modules"
  modules_mount_created=1
fi
net_root=${WS11UI_BROWSER_SCRATCH:-${TMPDIR:-/tmp}/ws11ui-browser}
mkdir -p "$net_root"
net_dir=$(mktemp -d "$net_root/net.XXXXXX")
socket=$net_dir/upstream.sock
name=${PARITY_NAMESPACE}-behavior-browser-$$
cleanup() {
  docker kill "$name" "$name-forward" >/dev/null 2>&1 || true
  rm -rf "$net_dir"
  if [[ "$modules_mount_created" == 1 ]]; then
    rmdir -- "$PARITY/node_modules" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM
docker run -d --rm --init --name "$name-forward" --network host --cpus 1 \
  -u "$(id -u):$(id -g)" -v "$ROOT:$ROOT:ro" -v "$net_dir:$net_dir" \
  --tmpfs "$PARITY/node_modules" -w "$PARITY" "$image" node capture/forward.ts "$socket" >/dev/null
wait_for_socket "$socket"
docker run --rm --init --name "$name" --network none --cpus 2 --shm-size 256m \
  -u "$(id -u):$(id -g)" -e TMPDIR="$net_dir" -e TZ=UTC \
  -e WS11UI_ACTIVITY_CONTROL="${WS11UI_ACTIVITY_CONTROL:-}" \
  -e WS11UI_HOST_NETWORK="$(readlink /proc/self/ns/net)" -e PARITY_UPSTREAM_SOCKET="$socket" \
  -v "$REPO:$REPO" -v "$net_dir:$net_dir" --tmpfs "$PARITY/node_modules" \
  -w "$REPO" "$image" node "$ROOT/reference-tools/views/agents_ui/system_cases.mjs" "$@"

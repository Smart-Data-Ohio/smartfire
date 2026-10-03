#!/usr/bin/env bash
# Behavior browser uses the capture harness's pinned image and network forwarder.
set -euo pipefail
# Deliberately broken implementation used only by check_system_network.py.
if [[ "${WS11UI_BROKEN_HOST_NETWORK:-}" == 1 ]]; then
  WS11UI_HOST_NETWORK="$(readlink /proc/self/ns/net)" CHROMIUM_PATH=/usr/bin/chromium node "$(dirname "$0")/system_cases.mjs" "$@"
  exit "$?"
fi
source "$(dirname "$0")/../../../parity/capture/sandbox/run.sh"
trap parity_cleanup EXIT INT TERM
export PARITY_NAMESPACE=${PARITY_NAMESPACE:-ws11ui-system}
image=$(docker_image)
net_root=/home/riels/.cache/rust-port/ws11ui/browser
mkdir -p "$net_root"
net_dir=$(mktemp -d "$net_root/net.XXXXXX")
NET_DIRS+=("$net_dir")
socket=$net_dir/upstream.sock
name=${PARITY_NAMESPACE}-behavior-browser-$$
CAPTURE_CONTAINERS+=("$name" "$name-forward")
docker run -d --rm --init --name "$name-forward" --network host --cpus 1 \
  -u "$(id -u):$(id -g)" -v "$ROOT:$ROOT:ro" -v "$net_dir:$net_dir" \
  --tmpfs "$PARITY/node_modules" -w "$PARITY" "$image" node capture/forward.ts "$socket" >/dev/null
wait_for_socket "$socket"
docker run --rm --init --name "$name" --network none --cpus 2 --shm-size 256m \
  -u "$(id -u):$(id -g)" -e TMPDIR="$net_dir" -e TZ=UTC \
  -e WS11UI_HOST_NETWORK="$(readlink /proc/self/ns/net)" -e PARITY_UPSTREAM_SOCKET="$socket" \
  -v "$REFERENCE_ROOT:$REFERENCE_ROOT" -v "$net_dir:$net_dir" --tmpfs "$PARITY/node_modules" \
  -w "$REFERENCE_ROOT" "$image" node "$ROOT/reference-tools/views/agents_ui/system_cases.mjs" "$@"

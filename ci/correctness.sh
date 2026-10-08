#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
suite=${1:?Expected acme, browsers, drive, livekit, messaging, agents-ui, or pwa}
# CI splits the longest suites across parallel jobs. CORRECTNESS_SHARD=K/N runs one
# deterministic slice; ci/correctness_gate.py then requires the slices' receipts to
# cover every selected test exactly once.
shard=${CORRECTNESS_SHARD:-}
if [[ -n "$shard" && ! "$shard" =~ ^[1-9][0-9]*/[1-9][0-9]*$ ]]; then
  echo "CORRECTNESS_SHARD must be K/N, got $shard" >&2
  exit 1
fi
tag=$suite
[[ -z "$shard" ]] || tag+="-${shard/\//of}"
first_shard() { [[ -z "$shard" || "$shard" == 1/* ]]; }
receipts="$repo/target/ci-receipts"
mkdir -p "$receipts"
started=$(date +%s)

ignored() {
  local filter package selection=()
  [[ -z "$shard" ]] || selection=(--shard "$shard")
  filter=$(python3 ci/ignored_tests.py --filter "$suite" "${selection[@]}")
  package=$(python3 ci/ignored_tests.py --filter "$suite" --package)
  cargo nextest run --locked -p "$package" \
    --profile ci --build-jobs 4 -j 4 --no-fail-fast --success-output final --run-ignored only --no-tests fail -E "$filter"
  cp target/nextest/ci/junit.xml "$receipts/$tag-junit.xml"
  python3 ci/ignored_tests.py --filter "$suite" "${selection[@]}" --junit "$receipts/$tag-junit.xml"
}

browser_images() {
  local ws12_hash ws13_hash
  ws12_hash=$(cat parity/Dockerfile.playwright parity/package.json parity/package-lock.json | sha256sum | cut -c1-12)
  ws13_hash=$(cat parity/Dockerfile.playwright parity/package-lock.json | sha256sum | cut -c1-12)
  docker build -f parity/Dockerfile.playwright -t "ws12-playwright:$ws12_hash" \
    -t "ws13-parity-playwright:$ws13_hash" -t "ws11ui-system-parity-playwright:$ws13_hash" parity
  export WS13_PLAYWRIGHT_IMAGE="ws13-parity-playwright:$ws13_hash"
}

run_suite() {
  python3 ci/ignored_tests.py
  case "$suite" in
    acme)
      local pebble="campfire-ci-pebble-$$"
      trap "docker rm -f '$pebble' >/dev/null 2>&1 || true" EXIT
      docker run -d --name "$pebble" --network host --add-host campfire.test:127.0.0.1 \
        --env PEBBLE_VA_NOSLEEP=1 --env PEBBLE_WFE_NONCEREJECT=0 \
        ghcr.io/letsencrypt/pebble:2.8.0@sha256:d9080f68f6cb6af8d82134ab26de0aaaf312ac9cba42aecc6d3aede6cb63007b
      export PEBBLE_MINICA="$TMPDIR/pebble.minica.pem"
      docker cp "$pebble:/test/certs/pebble.minica.pem" "$PEBBLE_MINICA"
      local ready=0
      for _ in $(seq 1 100); do
        if curl -fsS --cacert "$PEBBLE_MINICA" https://localhost:14000/dir >/dev/null 2>&1; then ready=1; break; fi
        sleep .1
      done
      [[ "$ready" == 1 ]] || { docker logs "$pebble"; return 1; }
      ignored
      ;;
    browsers)
      browser_images
      if first_shard; then
        docker run --rm --init --network none --ipc host --cpus 2 \
          --volume "$repo:/work:ro" "ws12-playwright:$(cat parity/Dockerfile.playwright parity/package.json parity/package-lock.json | sha256sum | cut -c1-12)" \
          node --test /work/reference-tools/users/browser_navigation.test.mjs
      fi
      # The paired original-assertion wrappers launch the normal server, which
      # nextest's cfg(test) harness does not build.
      cargo build --locked -p campfire --bin campfire
      export WS11UI_BROWSER_BINARY="$repo/target/debug/campfire"
      export CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499
      ignored
      ;;
    drive)
      # Each declaration boots its own app in the test process and drives it from the pinned
      # Playwright image (WS13_PLAYWRIGHT_IMAGE).
      browser_images
      ignored
      ;;
    livekit)
      browser_images
      # Start the private media server after the cold compile so its lifetime
      # and logs cover the browser run rather than several minutes of rustc.
      cargo test --locked -p campfire --no-run -j 4
      web/bin/livekit-local setup
      # Only this job's private signaling server is needed; the test owns its gateway.
      web/bin/livekit-local start >"$receipts/livekit-server.log" 2>&1 &
      local livekit_pid=$!
      trap "kill $livekit_pid 2>/dev/null || true; wait $livekit_pid 2>/dev/null || true" EXIT
      source web/.bundle/livekit/env
      local ready=0
      for _ in $(seq 1 100); do
        if curl -fsS http://127.0.0.1:7880/ >/dev/null 2>&1; then ready=1; break; fi
        sleep .1
      done
      [[ "$ready" == 1 ]] || { cat "$receipts/livekit-server.log"; return 1; }
      docker run --rm --init --network none --ipc host --cpus 2 --user "$(id -u):$(id -g)" \
        --volume "$repo/parity:$repo/parity:ro" --volume "$TMPDIR:$TMPDIR" \
        --env TMPDIR="$TMPDIR" --workdir "$repo/parity" \
        "$WS13_PLAYWRIGHT_IMAGE" node --test --test-concurrency=4 \
        system/ws13-browser-poll.test.mjs system/ws13-media-network.test.mjs system/ws13-gateway-readiness.test.mjs
      ignored
      ;;
    messaging)
      # behavior-check.py runs each case on the Rust app, from the frozen default seed and the
      # case's recorded Rails fixture. These listeners stay outside Linux's ephemeral
      # outbound-client range.
      export WS8BM_BROWSER_PORT_BASE=22020 WS8BM_CHROMEDRIVER_PORT=22023
      node ci/native-network-smoke.mjs
      if first_shard; then
        python3 -m unittest discover -s reference-tools/messaging -p '*_test.py'
        npm ci --prefix parity
        node --test --test-concurrency=4 reference-tools/messaging/*.test.mjs
      fi
      local selection=()
      [[ -z "$shard" ]] || selection=(--shard "$shard")
      python3 reference-tools/messaging/behavior-check.py --keep-going "${selection[@]}" \
        --receipt "$receipts/$tag-cases.json"
      ;;
    agents-ui)
      browser_images
      cargo build --locked -j 4 -p campfire --bin campfire
      python3 reference-tools/views/agents_ui/system_behavior.py \
        --binary "$repo/target/debug/campfire" --scenario all
      ;;
    pwa)
      # Reuse the production frontend stage so application registration is tested against the
      # embedded real SPA, alongside the real Rust layout and classic asset overrides.
      local pwa_image="campfire-pwa-spa:$$" pwa_container
      docker build --target spa -t "$pwa_image" .
      pwa_container=$(docker create "$pwa_image")
      trap "docker rm -f '$pwa_container' >/dev/null 2>&1 || true" EXIT
      docker cp "$pwa_container:/src/frontend/dist" frontend/
      docker rm "$pwa_container"
      trap - EXIT
      export SPA_DIST="$repo/frontend/dist"
      browser_images
      export PWA_PLAYWRIGHT_IMAGE="$WS13_PLAYWRIGHT_IMAGE"
      ignored
      ;;
    *) echo "Unknown correctness suite: $suite" >&2; return 1 ;;
  esac
}

# A separate shell preserves errexit inside the suite (a function on the left of an
# OR-list would silently disable it for prerequisite commands).
if [[ "${2:-}" == --execute ]]; then
  run_suite
  exit 0
fi
status=0
bash "$0" "$suite" --execute 2>&1 | tee "$receipts/$tag.log" || status=$?
python3 - "$suite" "$started" "$status" "$receipts/$tag.json" "$shard" <<'PY'
import json, subprocess, sys, time
from pathlib import Path
suite, started, status, output, shard = sys.argv[1:]
Path(output).write_text(json.dumps(dict(suite=suite, shard=shard or None, head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                                      duration_seconds=int(time.time())-int(started), exit_code=int(status)), indent=2)+'\n')
PY
exit "$status"

#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo"
suite=${1:?Expected acme, browsers, livekit, messaging, or agents-ui}
# CI splits the longest suites across parallel jobs. CORRECTNESS_SHARD=K/N runs one
# deterministic slice; CORRECTNESS_PART selects messaging's behaviour cases or its
# original WS14/WS15 declarations (default: both). rust/ci/correctness_gate.py then
# requires the slices' receipts to cover every selected test exactly once.
shard=${CORRECTNESS_SHARD:-}
part=${CORRECTNESS_PART:-all}
case "$part" in all | behavior | originals) ;; *) echo "Unknown correctness part: $part" >&2; exit 1 ;; esac
if [[ -n "$shard" && ! "$shard" =~ ^[1-9][0-9]*/[1-9][0-9]*$ ]]; then
  echo "CORRECTNESS_SHARD must be K/N, got $shard" >&2
  exit 1
fi
tag=$suite
[[ "$part" == all ]] || tag+="-$part"
[[ -z "$shard" ]] || tag+="-${shard/\//of}"
first_shard() { [[ -z "$shard" || "$shard" == 1/* ]]; }
receipts="$repo/rust/target/ci-receipts"
mkdir -p "$receipts"
started=$(date +%s)

ignored() {
  local filter package selection=()
  [[ -z "$shard" ]] || selection=(--shard "$shard")
  filter=$(python3 rust/ci/ignored_tests.py --filter "$suite" "${selection[@]}")
  package=$(python3 rust/ci/ignored_tests.py --filter "$suite" --package)
  cargo nextest run --manifest-path rust/Cargo.toml --locked -p "$package" \
    --profile ci --build-jobs 4 -j 4 --no-fail-fast --success-output final --run-ignored only --no-tests fail -E "$filter"
  cp rust/target/nextest/ci/junit.xml "$receipts/$tag-junit.xml"
  python3 rust/ci/ignored_tests.py --filter "$suite" "${selection[@]}" --junit "$receipts/$tag-junit.xml"
}

browser_images() {
  local ws12_hash ws13_hash
  ws12_hash=$(cat rust/parity/Dockerfile.playwright rust/parity/package.json rust/parity/package-lock.json | sha256sum | cut -c1-12)
  ws13_hash=$(cat rust/parity/Dockerfile.playwright rust/parity/package-lock.json | sha256sum | cut -c1-12)
  docker build -f rust/parity/Dockerfile.playwright -t "ws12-playwright:$ws12_hash" \
    -t "ws13-parity-playwright:$ws13_hash" -t "ws11ui-system-parity-playwright:$ws13_hash" rust/parity
  export WS13_PLAYWRIGHT_IMAGE="ws13-parity-playwright:$ws13_hash"
}

run_suite() {
  python3 rust/ci/ignored_tests.py
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
          --volume "$repo:/work:ro" "ws12-playwright:$(cat rust/parity/Dockerfile.playwright rust/parity/package.json rust/parity/package-lock.json | sha256sum | cut -c1-12)" \
          node --test /work/rust/reference-tools/users/browser_navigation.test.mjs
      fi
      # The paired original-assertion wrappers launch the normal server, which
      # nextest's cfg(test) harness does not build.
      cargo build --manifest-path rust/Cargo.toml --locked -p campfire --bin campfire
      export WS11UI_BROWSER_BINARY="$repo/rust/target/debug/campfire"
      export CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499
      ignored
      ;;
    livekit)
      browser_images
      # Start the private media server after the cold compile so its lifetime
      # and logs cover the browser run rather than several minutes of rustc.
      cargo test --manifest-path rust/Cargo.toml --locked -p campfire --no-run -j 4
      rust/web/bin/livekit-local setup
      # Only this job's private signaling server is needed; the test owns its gateway.
      rust/web/bin/livekit-local start >"$receipts/livekit-server.log" 2>&1 &
      local livekit_pid=$!
      trap "kill $livekit_pid 2>/dev/null || true; wait $livekit_pid 2>/dev/null || true" EXIT
      source rust/web/.bundle/livekit/env
      local ready=0
      for _ in $(seq 1 100); do
        if curl -fsS http://127.0.0.1:7880/ >/dev/null 2>&1; then ready=1; break; fi
        sleep .1
      done
      [[ "$ready" == 1 ]] || { cat "$receipts/livekit-server.log"; return 1; }
      docker run --rm --init --network none --ipc host --cpus 2 --user "$(id -u):$(id -g)" \
        --volume "$repo/rust/parity:$repo/rust/parity:ro" --volume "$TMPDIR:$TMPDIR" \
        --env TMPDIR="$TMPDIR" --workdir "$repo/rust/parity" \
        "$WS13_PLAYWRIGHT_IMAGE" node --test --test-concurrency=4 \
        system/ws13-browser-poll.test.mjs system/ws13-media-network.test.mjs system/ws13-gateway-readiness.test.mjs
      ignored
      ;;
    messaging)
      # The scripted paired-browser runner owns its fixture/paused-job seeds and checks rows.
      # These listeners stay outside Linux's ephemeral outbound-client range.
      export WS8BM_BROWSER_PORT_BASE=22020 WS8BM_CHROMEDRIVER_PORT=22023
      node rust/ci/native-network-smoke.mjs
      if [[ "$part" != originals ]] && first_shard; then
        python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
        npm ci --prefix rust/parity
        node --test --test-concurrency=4 rust/reference-tools/messaging/*.test.mjs
      fi
      case "$part" in
        all) python3 rust/reference-tools/messaging/behavior-check.py --keep-going ;;
        behavior)
          local selection=()
          [[ -z "$shard" ]] || selection=(--shard "$shard")
          python3 rust/reference-tools/messaging/behavior-check.py --keep-going "${selection[@]}" \
            --receipt "$receipts/$tag-cases.json"
          ;;
        # The behaviour run's preparation (browser image, seeds, npm, pinned Selenium atom).
        originals) python3 rust/reference-tools/messaging/behavior-check.py --prepare-only ;;
      esac
      if [[ "$part" != behavior ]]; then
        # Pinned original WS14/WS15 declarations share the isolated native browser.
        export WS14_BROWSER_RUBY_IMAGE="ws8bm-browser-reference-$(cut -c1-9 rust/parity/reference.sha)"
        ignored
      fi
      ;;
    agents-ui)
      browser_images
      cargo build --manifest-path rust/Cargo.toml --locked -j 4 -p campfire --bin campfire
      python3 rust/reference-tools/views/agents_ui/system_behavior.py \
        --binary "$repo/rust/target/debug/campfire" --scenario all
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
python3 - "$suite" "$started" "$status" "$receipts/$tag.json" "$part" "$shard" <<'PY'
import json, subprocess, sys, time
from pathlib import Path
suite, started, status, output, part, shard = sys.argv[1:]
Path(output).write_text(json.dumps(dict(suite=suite, part=part, shard=shard or None, head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                                      duration_seconds=int(time.time())-int(started), exit_code=int(status)), indent=2)+'\n')
PY
exit "$status"

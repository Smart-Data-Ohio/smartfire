#!/bin/bash
# The 100,000-client runs in report.md.
#
#   bench/results/pi-100k-20260928/run100k.sh BIN LABEL [CLIENTS] [loadgen cable options...]
#
# Starts BIN (a campfire release build) on CPUs 8-11, or with PI=1 under a cgroup quota of 1.2 cores
# and 8 GB (a Raspberry Pi 5's budget, see report.md), then runs `loadgen cable` on CPUs 12-23:
# CLIENTS clients (default 100,000), a 30 s idle hold, paced posts and 10 s of throughput. Pass
# `--deflate 1` for browser-like compressed clients. It prints CPU and memory per phase and writes
# $WORK/cable-LABEL.json. The first run sets up $WORK (default bench/.work/pi-100k): a database
# filled by ../splice-20260927/fill.py, a signed-in cookie and the room's stream names.
#
# Many clients need many local ports: they're spread over ten loopback addresses, and a run leaves
# its ports in TIME_WAIT for a minute, so wait between runs (or set SOURCES to other addresses).
set -euo pipefail
[ $# -ge 2 ] || { sed -n '4,12p' "$0"; exit 1; }
BIN=$(realpath "$1"); LABEL=$2; shift 2
N=100000
if [ $# -gt 0 ] && [[ $1 =~ ^[0-9]+$ ]]; then N=$1; shift; fi

REPO=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
WORK=${WORK:-$REPO/bench/.work/pi-100k}
PORT=${PORT:-4555}
SOURCES=${SOURCES:-$(seq -s, -f '127.0.0.%g' 10 19)}
LOADGEN=${LOADGEN:-$REPO/target/bench/release/loadgen}
[ -x "$LOADGEN" ] || CARGO_TARGET_DIR=$REPO/target/bench cargo build --release -q --manifest-path "$REPO/bench/loadgen/Cargo.toml"
mkdir -p "$WORK/storage"

start_app() {
  [ -f "$WORK/pid" ] && kill "$(cat "$WORK/pid")" 2>/dev/null && sleep 1
  local wrap=()
  [ "${PI:-0}" = 1 ] && wrap=(systemd-run --user --scope -q -p CPUQuota=120% -p MemoryMax=8G)
  (
    cd "$WORK"
    echo $BASHPID > "$WORK/pid"
    SECRET_KEY_BASE=$(printf 'a%.0s' {1..128}) DISABLE_SSL=1 HTTP_PORT=$PORT TARGET_PORT=$((PORT + 1)) \
      CAMPFIRE_STORAGE_PATH=$WORK/storage exec "${wrap[@]}" taskset -c 8-11 "$BIN" server > "$WORK/log" 2>&1
  ) &
  for _ in $(seq 50); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/up" && return; sleep 0.2; done
  echo "the app didn't start; see $WORK/log" >&2; exit 1
}

start_app
if [ ! -f "$WORK/scrape.json" ]; then
  python3 "$REPO/bench/results/splice-20260927/fill.py" "http://127.0.0.1:$PORT" > /dev/null
  "$LOADGEN" login --base "http://127.0.0.1:$PORT" --email david@example.com --password secret123456 |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["cookie"])' > "$WORK/cookie"
  "$LOADGEN" scrape --base "http://127.0.0.1:$PORT" --cookie "$(cat "$WORK/cookie")" --room 1 > "$WORK/scrape.json"
fi
PID=$(cat "$WORK/pid"); COOKIE=$(cat "$WORK/cookie")
STREAMS=$(python3 -c 'import json,sys; print(",".join(json.load(open(sys.argv[1]))["streams"]))' "$WORK/scrape.json")

# time, app Pss (kB), app CPU ticks, load generator CPU ticks
( while sleep 1; do
  a=$(awk '{print $14+$15}' /proc/$PID/stat 2>/dev/null) || break
  p=$(awk '/^Pss:/{print $2}' /proc/$PID/smaps_rollup 2>/dev/null)
  l=$(pgrep -x loadgen | head -1 || true); lc=$( [ -n "$l" ] && awk '{print $14+$15}' /proc/$l/stat || echo 0)
  echo "$(date +%s.%N) $p $a $lc"
done ) > "$WORK/samp-$LABEL.txt" 2>/dev/null & SAMPLER=$!
taskset -c 12-23 "$LOADGEN" cable --base "http://127.0.0.1:$PORT" --cookie "$COOKIE" --room 1 --streams "$STREAMS" \
  --clients "$N" --sources "$SOURCES" --hold-secs 30 --latency-msgs 10 --tput-secs 10 --posters 2 "$@" \
  > "$WORK/cable-$LABEL.json" 2> "$WORK/cable-$LABEL.err"
kill $SAMPLER

python3 - "$WORK" "$LABEL" <<'PY'
import json, os, sys
work, label = sys.argv[1:]
ticks = os.sysconf("SC_CLK_TCK")
r = json.load(open(f"{work}/cable-{label}.json"))
t = r["throughput"]
print(label, "ready", r["ready"], "failed", r["failed"], "connect_s", r["connect_secs"],
      "post->all p50/p99", r["latency"]["all_clients"].get("p50_ms"), r["latency"]["all_clients"].get("p99_ms"),
      "deliveries/s", round(t["delivered_msgs_per_sec"] * r["ready"]), "POST p50", t["post"].get("p50_ms"),
      "wire MB/s", t.get("wire_mb_per_sec"))
rows = [tuple(map(float, line.split())) for line in open(f"{work}/samp-{label}.txt") if len(line.split()) == 4]
phases = {}
for line in open(f"{work}/cable-{label}.err"):
    if line.startswith("PHASE"):
        _, name, ms = line.split()
        phases[name] = int(ms) / 1000
names = sorted(phases, key=phases.get)
for start, end in zip(names, names[1:] + [None]):
    until = phases[end] if end else rows[-1][0]
    sel = [row for row in rows if phases[start] <= row[0] <= until]
    if len(sel) < 2:
        continue
    secs = sel[-1][0] - sel[0][0]
    print(f"  {start:18s} {secs:5.1f}s app {(sel[-1][2] - sel[0][2]) / ticks / secs:.2f} cores"
          f"  loadgen {(sel[-1][3] - sel[0][3]) / ticks / secs:.2f} cores  pss {max(row[1] for row in sel) / 1024:.0f} MB")
PY

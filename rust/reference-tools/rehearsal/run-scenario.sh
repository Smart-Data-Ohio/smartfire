#!/usr/bin/env bash
#
# Drives scenario.mjs through the two container swaps: Rails (already running) -> Rust -> Rails.
# Between swaps it records the schema and the durable job queue, so the caller can check that
# Rust migrated nothing and processed its jobs. Outputs land in $REHEARSAL_DIR/work.
#
# Usage: REHEARSAL_DIR=... RAILS_IMAGE=... RUST_IMAGE=... run-scenario.sh

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${REHEARSAL_DIR:?}"
W="$REHEARSAL_DIR/work"
R="$HERE/rehearsal.sh"

wait_for() {
  local file="$1" pid="$2"
  for _ in $(seq 1 1800); do
    [ -e "$file" ] && return 0
    kill -0 "$pid" 2>/dev/null || { echo "driver exited before $(basename "$file")" >&2; return 1; }
    sleep 0.5
  done
  echo "timed out waiting for $file" >&2; return 1
}

rm -rf "$W/signals"; mkdir -p "$W/signals"; chmod 777 "$W/signals"
"$R" schema "$W/schema-before.txt"
"$R" driver scenario.mjs > "$W/scenario.log" 2>&1 &
driver=$!

wait_for "$W/signals/ready-for-rust" "$driver"
"$R" swap rust
touch "$W/signals/rust-up"

wait_for "$W/signals/ready-for-rails" "$driver"
"$R" schema "$W/schema-after-rust.txt"
sleep 5   # let the job runner finish what the last writes enqueued
"$R" sql "select job_class || ' ' || status, count(*) from background_jobs group by 1 order by 1" > "$W/jobs-after-rust.txt"
"$R" swap rails
touch "$W/signals/rails-up"

wait "$driver" || { echo "scenario driver failed" >&2; tail -20 "$W/scenario.log" >&2; exit 1; }
"$R" schema "$W/schema-after-rollback.txt"
tail -n 1 "$W/scenario.log"

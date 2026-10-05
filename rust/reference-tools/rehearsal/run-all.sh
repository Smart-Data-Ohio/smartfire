#!/usr/bin/env bash
#
# The whole production-copy cutover rehearsal, end to end, against an already prepared
# $REHEARSAL_DIR (rehearsal.sh prepare). Prints counts and pass/fail only; details stay in
# $REHEARSAL_DIR/work. Delete $REHEARSAL_DIR afterwards: it holds production data.
#
# Usage: REHEARSAL_DIR=... RAILS_IMAGE=... RUST_IMAGE=... run-all.sh A_ID B_ID C_ID
#   A_ID, B_ID: users given a password and TOTP (A is the smoke viewer; A and B share a room);
#   C_ID: a user given a password and no TOTP credential (enrolls on Rust).

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${REHEARSAL_DIR:?}" "${RAILS_IMAGE:?}" "${RUST_IMAGE:?}"
A="${1:?A id}" B="${2:?B id}" C="${3:?C id}"
W="$REHEARSAL_DIR/work"
R="$HERE/rehearsal.sh"
NET="${REHEARSAL_NET:-sfreh-net}"
ALIAS="${REHEARSAL_ALIAS:-smartfire}"
S="$W/summary.txt"

note() { printf '%s\n' "$*" | tee -a "$S"; }
probe() { docker run --rm --network "$NET" busybox:latest wget -q -O /dev/null -T 5 "$1" >/dev/null 2>&1 && echo reachable || echo unreachable; }

rm -rf "$W/logs" "$W/signals"; rm -f "$W"/smoke-*.json "$W"/scenario* "$W"/rust-writes.json "$W"/jobs-* "$W"/schema-* "$S"
"$R" reset
"$R" schema "$W/schema-pristine.txt"
"$R" start rails
"$HERE/setup-users.sh" "$A" "$B" "$C"
python3 "$HERE/targets.py" "$REHEARSAL_DIR/storage/db/production.sqlite3" "$A" "$B" > "$W/targets.json"
note "images: rails=$(docker image inspect -f '{{index .RepoDigests 0}}' "$RAILS_IMAGE" 2>/dev/null || echo "$RAILS_IMAGE") rust=$(docker image inspect -f '{{index .RepoDigests 0}}' "$RUST_IMAGE" 2>/dev/null || echo "$RUST_IMAGE")"

# Egress: the internal network has no route out.
note "egress from the rehearsal network: http://1.1.1.1 $(probe http://1.1.1.1) / https://www.google.com $(probe https://www.google.com)"
note "egress from the Rails container: $(docker exec "${REHEARSAL_APP:-sfreh-app}" curl -s -o /dev/null -m 5 https://oauth2.googleapis.com && echo reachable || echo unreachable)"

sleep 10; note "rss rails idle: $("$R" rss)"
note "rails bare app port 3000 from the network: $(probe "http://$ALIAS:3000/up")"
"$R" driver smoke.mjs rails-warm 1 | tee -a "$S"
"$R" driver smoke.mjs rails 10 | tee -a "$S"
note "rss rails after smoke: $("$R" rss)"

"$R" swap rust
sleep 5; note "rss rust idle: $("$R" rss)"
note "rust bare app port 3000 from the network: $(probe "http://$ALIAS:3000/up")"
"$R" driver smoke.mjs rust-warm 1 | tee -a "$S"
"$R" driver smoke.mjs rust 10 | tee -a "$S"
note "rss rust after smoke: $("$R" rss)"
rm -f "$W/smoke-state.json"
"$R" driver smoke.mjs rust-same 1 --reuse-session | tee -a "$S"
"$R" swap rails
"$R" driver smoke.mjs rails-same 1 --reuse-session | tee -a "$S"
python3 "$HERE/compare.py" "$W" rails rust | tee -a "$S"
python3 "$HERE/compare.py" "$W" rails-same rust-same | head -1 | sed 's/^/identical-data rerun: /' | tee -a "$S"

"$HERE/run-scenario.sh" | tee -a "$S"
grep -E '\] (PASS|FAIL) ' "$W/scenario.log" >> "$S" || true

"$R" driver smoke.mjs rails-after-rollback 1 --reuse-session | tee -a "$S"
docker logs "${REHEARSAL_APP:-sfreh-app}" > "$W/logs/rails-final.log" 2>&1

for f in schema-before schema-after-rust schema-after-rollback; do
  if cmp -s "$W/schema-pristine.txt" "$W/$f.txt"; then note "schema $f: identical to pristine copy"; else note "schema $f: DIFFERS"; fi
done
note "jobs left in background_jobs after Rust: $(wc -l < "$W/jobs-after-rust.txt")"
for log in "$W"/logs/rust-*.log; do
  note "$(basename "$log"): $(sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -c 'campfire_jobs::runner: performed') jobs performed, $(sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -E -c ' (ERROR|WARN) |panicked') ERROR/WARN/panic lines"
done
for log in "$W"/logs/rails-*.log; do
  note "$(basename "$log"): $(grep -c 'Completed 5[0-9][0-9]' "$log") 5xx completions, $(grep -c -E 'FATAL|Error \(' "$log") FATAL/Error lines"
done

#!/usr/bin/env bash
#
# Runs deploy/gcp/campfire-release.sh against real containers on this machine:
# real images, a real local registry (so IMAGE_REF is a repository@digest the
# script pulls), a real storage volume and real `campfire` commands. Only ONCE,
# the host's root, systemd and the TLS front door are faked. test_release.py
# covers the decisions exhaustively with fakes; this proves the same flow with
# the real binaries and databases.
#
# Usage:
#   ops/tests/simulate_release.sh PREVIOUS CANDIDATE MIGRATING
#
#   PREVIOUS   a deployed Rust image (local tag), e.g. the current production one
#   CANDIDATE  an image built from this checkout with no pending migrations
#   MIGRATING  the same, built with one extra example migration and its blessed
#              schema files (never committed). For example:
#                git worktree add ../mig && cd ../mig
#                $EDITOR crates/db/migrations/<VERSION>_<name>.sql
#                CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files
#                docker build -t smartfire:migrating .
#
# Needs docker, the registry:2 image, sqlite3, curl and jq, and must run as uid
# 1000 (the image's user owns the bind-mounted volume). Uses 127.0.0.1:5000 and
# 127.0.0.1:18443, a scratch directory, and Docker objects named campfire-sim-*.
# Exits non-zero at the first scenario that does not behave.
set -euo pipefail

[ "$#" -eq 3 ] || { sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
PREVIOUS_TAG="$1" CANDIDATE_TAG="$2" MIGRATING_TAG="$3"
[ "$(id -u)" = 1000 ] || { echo "run as uid 1000: the containers write the volume as 1000" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT="$ROOT/deploy/gcp/campfire-release.sh"
REAL_DOCKER="$(command -v docker)"
REAL_CURL="$(command -v curl)"
SIM="$(mktemp -d "${TMPDIR:-/tmp}/campfire-sim.XXXXXX")"
VOLUME=campfire-sim-storage
REGISTRY=campfire-sim-registry
HOST=chat.sim.test
export SIM SIM_VOLUME="$VOLUME" REAL_DOCKER REAL_CURL

say() { printf '\n### %s\n' "$*"; }
fail() { printf 'SIMULATION FAILED: %s\n' "$*" >&2; exit 1; }

cleanup() {
  "$REAL_DOCKER" ps -aq --filter label=once-sim | xargs -r "$REAL_DOCKER" rm -f >/dev/null
  "$REAL_DOCKER" rm -f "$REGISTRY" >/dev/null 2>&1 || true
  "$REAL_DOCKER" volume rm -f "$VOLUME" >/dev/null 2>&1 || true
  for label in a0 a b c d e f; do
    "$REAL_DOCKER" rmi "campfire-rollback:before-$label" >/dev/null 2>&1 || true
  done
  echo "simulation state kept in $SIM"
}
trap cleanup EXIT

# ---------------------------------------------------------------- fakes --
mkdir -p "$SIM/bin" "$SIM/volume" "$SIM/state"
cat > "$SIM/bin/docker" <<'EOF'
#!/usr/bin/env bash
# Real Docker, except registry auth (the local registry has none) and the
# volume's Mountpoint, which is the bind-backed scratch directory.
case "$1" in
  login) cat >/dev/null; echo "Login Succeeded"; exit 0 ;;
  logout) exit 0 ;;
  volume) if [ "$2" = inspect ] && [ "$3" = "$SIM_VOLUME" ]; then echo "$SIM/volume"; exit 0; fi ;;
esac
exec "$REAL_DOCKER" "$@"
EOF
cat > "$SIM/bin/once" <<'EOF'
#!/usr/bin/env bash
# ONCE 0.3.2 as the release script uses it: one container labelled `once`
# with the JSON settings, the volume at /storage and /rails/storage.
set -euo pipefail
state="$SIM/once.json"
current() { "$REAL_DOCKER" ps -a --filter label=once --format '{{.Names}}' | grep '^once-app-' | head -n1 || true; }
case "$1" in
  version) echo 0.3.2 ;;
  stop) c="$(current)"; [ -z "$c" ] || "$REAL_DOCKER" stop -t 10 "$c" >/dev/null ;;
  start) c="$(current)"; [ -n "$c" ] || exit 1; "$REAL_DOCKER" start "$c" >/dev/null ;;
  backup) tar -czf "$3" -C "$SIM" volume once.json ;;
  update)
    image=""; shift 2
    while [ "$#" -gt 0 ]; do case "$1" in --image) image="$2"; shift 2 ;; *) shift ;; esac; done
    "$REAL_DOCKER" pull -q "$image" >/dev/null
    label="$(jq -c --arg image "$image" '.image = $image' "$state")"; printf '%s\n' "$label" > "$state"
    old="$(current)"; [ -z "$old" ] || "$REAL_DOCKER" rm -f "$old" >/dev/null
    n=$(( $(cat "$SIM/serial" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$SIM/serial"
    "$REAL_DOCKER" run -d --name "once-app-sim-$n" --label "once=$label" --label once-sim=1 \
      -v "$SIM_VOLUME:/storage" -v "$SIM_VOLUME:/rails/storage" -p 127.0.0.1:18443:80 \
      --memory 768m --env-file "$SIM/app.env" "$image" >/dev/null ;;
  *) echo "fake once: unsupported $*" >&2; exit 2 ;;
esac
EOF
cat > "$SIM/bin/curl" <<'EOF'
#!/usr/bin/env bash
# The release probes https://<host>/up through 127.0.0.1:443; the app's plain
# HTTP port is published on 127.0.0.1:18443. SIM_UNHEALTHY=1 reports 503 (a
# front-door fault) while the candidate really runs, jobs and all.
if [ "${SIM_UNHEALTHY:-0}" = 1 ]; then printf 503; exit 0; fi
exec "$REAL_CURL" -s -o /dev/null -w '%{http_code}' --max-time 10 http://127.0.0.1:18443/up
EOF
cat > "$SIM/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
case "$1" in is-enabled) echo disabled; exit 1 ;; is-active) exit 3 ;; *) exit 0 ;; esac
EOF
cat > "$SIM/bin/id" <<'EOF'
#!/usr/bin/env bash
if [ "${1:-}" = -u ]; then echo 0; else exec /usr/bin/id "$@"; fi
EOF
chmod +x "$SIM/bin/"*
grep -v '^#' "$ROOT/parity/.env.reference" | grep . > "$SIM/app.env"

export PATH="$SIM/bin:$PATH" REGISTRY_HOST=127.0.0.1:5000 STATE_ROOT="$SIM/state" \
  LOCK_FILE="$SIM/release.lock" ALLOW_BACKUP_WINDOW=1 FEED_DRAIN_TIMEOUT=5 \
  OPEN_ROLES_PATHS="$SIM/no-feed" TIMER_UNIT=campfire-sim-feed.timer \
  MIN_FREE_DISK_MB=1024 EXPECTED_APP_HOST="$HOST"

up() { "$REAL_CURL" -s -o /dev/null -w '%{http_code}' --max-time 5 http://127.0.0.1:18443/up || true; }
wait_up() { for _ in $(seq 1 60); do [ "$(up)" = 200 ] && return 0; sleep 1; done; fail "the app never served /up"; }
db() { echo "$SIM/volume/db/production.sqlite3"; }
running_image() { jq -r .image "$SIM/once.json"; }
result() { jq -r "$2" "$SIM/state/campfire-$1/$3"; }
serving() { if [ "$(running_image)" = "$1" ] && [ "$(up)" = 200 ]; then return 0; fi; fail "$2"; }
publish() {
  local repo="127.0.0.1:5000/campfire/app:$2"
  "$REAL_DOCKER" tag "$1" "$repo" && "$REAL_DOCKER" push -q "$repo" >/dev/null
  "$REAL_DOCKER" image inspect "$repo" --format '{{range .RepoDigests}}{{println .}}{{end}}' \
    | grep '^127.0.0.1:5000/campfire/app@' | head -n1
}
revision_of() { "$REAL_DOCKER" image inspect "$1" --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^GIT_REVISION=//p'; }
phases() { # label image phase... -> exit status of the first failing phase
  local label="$1" image="$2" phase revision; shift 2
  # As deploy-gcp.yml does: the commit the image claims, unless a scenario sets one.
  revision="${EXPECTED_GIT_REVISION-$(revision_of "$image")}"
  for phase in "$@"; do
    RELEASE_LABEL="$label" IMAGE_REF="$image" EXPECTED_GIT_REVISION="$revision" bash "$SCRIPT" "$phase" <<<"token" \
      >>"$SIM/$label.log" 2>&1 || return $?
  done
}
expect() { # label status image phase...
  local label="$1" want="$2" status=0; shift 2
  phases "$label" "$@" || status=$?
  [ "$status" = "$want" ] || fail "$label: expected exit $want, got $status (see $SIM/$label.log)"
}
reset_to() { # frozen copy, image: put a stopped database back and serve it
  once stop "$HOST"
  rm -f "$(db)-wal" "$(db)-shm"; cp "$1"/db/* "$SIM/volume/db/"
  once update "$HOST" --image "$2"; wait_up
}

# ---------------------------------------------------------------- setup --
say "registry, images and a seeded production on PREVIOUS"
"$REAL_DOCKER" run -d --name "$REGISTRY" -p 127.0.0.1:5000:5000 registry:2 >/dev/null
sleep 2
PREVIOUS="$(publish "$PREVIOUS_TAG" previous)"
CANDIDATE="$(publish "$CANDIDATE_TAG" candidate)"
MIGRATING="$(publish "$MIGRATING_TAG" migrating)"
"$REAL_DOCKER" volume create --driver local -o type=none -o o=bind -o device="$SIM/volume" "$VOLUME" >/dev/null
jq -n --arg image "$PREVIOUS" --arg host "$HOST" '{name:"sim", host:$host, image:$image,
  autoUpdate:false, disableTLS:true, backup:{}, resources:{},
  env:{SECRET_KEY_BASE:"x", VAPID_PUBLIC_KEY:"x", VAPID_PRIVATE_KEY:"x", DISABLE_SSL:"true"}}' > "$SIM/once.json"
once update "$HOST" --image "$PREVIOUS"; wait_up
# First run through the app itself, then a few messages.
jar="$SIM/cookies"
token="$("$REAL_CURL" -s -c "$jar" -b "$jar" -L http://127.0.0.1:18443/ \
  | grep -o 'name="authenticity_token" value="[^"]*"' | head -n1 | sed 's/.*value="//; s/"$//')"
"$REAL_CURL" -s -o /dev/null -c "$jar" -b "$jar" -F "authenticity_token=$token" -F "user[name]=Sim Admin" \
  -F "user[email_address]=admin@sim.test" -F "user[password]=simulation-password-1" http://127.0.0.1:18443/first_run
sqlite3 "$(db)" "PRAGMA busy_timeout=5000;
  INSERT INTO messages (client_message_id, created_at, updated_at, creator_id, room_id)
  VALUES ('sim-1','2026-10-06 01:00:00','2026-10-06 01:00:00',1,1), ('sim-2','2026-10-06 01:01:00','2026-10-06 01:01:00',1,1);" >/dev/null
echo kept > "$SIM/volume/files/sim-upload"
[ "$(sqlite3 "$(db)" 'select count(*) from users')" = 1 ] || fail "first run did not create the administrator"

# ------------------------------------------------------------ scenarios --
say "A: release CANDIDATE over PREVIOUS (no migrations); a wrong GIT_REVISION is refused first"
EXPECTED_GIT_REVISION=0000000000000000000000000000000000000000 expect a0 1 "$CANDIDATE" preflight
grep -q 'not the requested 0000000000000000000000000000000000000000' "$SIM/a0.log" || fail "A0 refused for another reason"
EXPECTED_GIT_REVISION="" expect a0 1 "$CANDIDATE" preflight
grep -q 'EXPECTED_GIT_REVISION is empty' "$SIM/a0.log" || fail "A0 accepted an empty expected revision"
expect a 0 "$CANDIDATE" preflight freeze cutover finish
[ "$(result a .revision_verified preflight-result.json)" = true ] || fail "A did not verify the revision"
[ "$(result a '.applied | length' live-migration-result.json)" = 0 ] || fail "A applied migrations"
serving "$CANDIDATE" "A is not serving the candidate"

say "B: release MIGRATING over CANDIDATE (one migration, strict boot afterwards)"
expect b 0 "$MIGRATING" preflight freeze cutover finish
[ "$(result b '.migrations | length' rehearsal-result.json)" = 1 ] || fail "B rehearsal did not migrate"
[ "$(result b '.matches_rehearsal' live-migration-result.json)" = true ] || fail "B live != rehearsal"
serving "$MIGRATING" "B is not serving the migrated database"
migrated_versions="$(sqlite3 -readonly "$(db)" 'select count(*) from schema_migrations')"

say "E: releasing CANDIDATE back over the migrated database is refused at the rehearsal"
expect e 1 "$CANDIDATE" preflight freeze
grep -q 'unknown migrations' "$SIM/state/campfire-e/migration-verification.txt" || fail "E refused for another reason"
serving "$MIGRATING" "E did not bring MIGRATING back"
[ "$(sqlite3 -readonly "$(db)" 'select count(*) from schema_migrations')" = "$migrated_versions" ] || fail "E touched the schema"

say "C: MIGRATING never becomes healthy over PREVIOUS; its boot fails a due job -> migration-reverted"
# PREVIOUS is the deployed image, so its own verifier decides the rollback. A
# job of a class nobody registered falls due after the freeze stopped PREVIOUS:
# the candidate's runner claims it and marks it failed, changing only
# background_jobs. Its other boot writes are whatever the real image does.
reset_to "$SIM/state/campfire-b/frozen-live" "$PREVIOUS"
due_at=$(( $(date +%s) + 45 ))
sqlite3 "$(db)" "PRAGMA busy_timeout=5000;
  INSERT INTO background_jobs (queue_name, job_class, arguments, created_at, updated_at, run_at)
  VALUES ('default', 'ReleaseSimulation::ProbeJob', '[]', strftime('%Y-%m-%d %H:%M:%f', 'now'), strftime('%Y-%m-%d %H:%M:%f', 'now'),
          strftime('%Y-%m-%d %H:%M:%f', $due_at, 'unixepoch'));" >/dev/null
expect c 0 "$MIGRATING" preflight freeze
# Read a copy: opening the kept frozen copy itself would add a -shm/-wal beside
# it, and rollback rightly refuses a frozen copy that no longer matches.
cp "$SIM/state/campfire-c/frozen-live/db/"* "$SIM/" && mv "$SIM/production.sqlite3" "$SIM/c-frozen.sqlite3"
if [ -f "$SIM/production.sqlite3-wal" ]; then mv "$SIM/production.sqlite3-wal" "$SIM/c-frozen.sqlite3-wal"; fi
[ "$(sqlite3 "$SIM/c-frozen.sqlite3" \
     "select status from background_jobs where job_class = 'ReleaseSimulation::ProbeJob'")" = ready ] \
  || fail "C: PREVIOUS ran the probe job before the freeze; rerun on a less loaded machine"
SIM_UNHEALTHY=1 HEALTH_TIMEOUT=10 expect c 10 "$MIGRATING" cutover
while [ "$(date +%s)" -lt $(( due_at + 15 )) ]; do sleep 2; done
[ "$(sqlite3 -readonly "$(db)" "PRAGMA busy_timeout=5000; select status from background_jobs where job_class = 'ReleaseSimulation::ProbeJob'" | tail -n1)" = failed ] \
  || fail "C: the candidate did not run the due probe job"
expect c 0 "$MIGRATING" rollback
[ "$(result c .action rollback-result.json)" = migration-reverted ] || fail "C did not revert the migration"
[ "$(result c .job_queue_changes_discarded rollback-result.json)" = true ] || fail "C did not record the discarded job-queue changes"
grep -qx 'background_jobs: preexisting row data changed MISMATCH' "$SIM/state/campfire-c/rollback-compare.txt" \
  || fail "C: the previous image's verifier did not see the job-queue change"
serving "$PREVIOUS" "C is not serving PREVIOUS"
cmp -s "$(db)" "$SIM/state/campfire-c/frozen-live/db/production.sqlite3" || fail "C database is not the frozen one"
echo "C: $(result c .reason rollback-result.json)"

say "D: MIGRATING never becomes healthy but a message got written -> refused-database-incompatible"
expect d 0 "$MIGRATING" preflight freeze
SIM_UNHEALTHY=1 HEALTH_TIMEOUT=10 expect d 10 "$MIGRATING" cutover
sqlite3 "$(db)" "PRAGMA busy_timeout=5000; INSERT INTO messages (client_message_id, created_at, updated_at, creator_id, room_id)
  VALUES ('sim-after-cutover','2026-10-06 02:05:00','2026-10-06 02:05:00',1,1);" >/dev/null
expect d 30 "$MIGRATING" rollback
[ "$(result d .action rollback-result.json)" = refused-database-incompatible ] || fail "D wrong action"
serving "$MIGRATING" "D did not keep MIGRATING serving"
[ "$(sqlite3 -readonly "$(db)" "select count(*) from messages where client_message_id = 'sim-after-cutover'")" = 1 ] \
  || fail "D lost the message written after the cutover"

say "F: CANDIDATE never becomes healthy over PREVIOUS -> image-rolled-back, database untouched"
reset_to "$SIM/state/campfire-d/frozen-live" "$PREVIOUS"
expect f 0 "$CANDIDATE" preflight freeze
SIM_UNHEALTHY=1 HEALTH_TIMEOUT=10 expect f 10 "$CANDIDATE" cutover
expect f 0 "$CANDIDATE" rollback
[ "$(result f .action rollback-result.json)" = image-rolled-back ] || fail "F wrong action"
serving "$PREVIOUS" "F is not serving PREVIOUS"
echo "F: database_restored=$(result f .database_restored rollback-result.json) job_queue_changes_discarded=$(result f .job_queue_changes_discarded rollback-result.json)"

say "G: /hooks/post-restore migrates an older backup and refuses a newer one"
restore="$SIM/restore"
mkdir -p "$restore/db" "$restore/backups" "$restore/files"
cp "$SIM/state/campfire-b/frozen-live/db/production.sqlite3" "$restore/backups/production.sqlite3"
"$REAL_DOCKER" run --rm --network none -v "$restore:/rails/storage" "$MIGRATING" /hooks/post-restore \
  > "$SIM/g-older.log" 2>&1 || fail "G: MIGRATING's post-restore failed on an older backup (see $SIM/g-older.log)"
"$REAL_DOCKER" run --rm --network none -v "$restore:/rails/storage" "$MIGRATING" \
  campfire db-check /rails/storage/db/production.sqlite3 >/dev/null || fail "G: MIGRATING refuses the restored database"
cp "$restore/db/production.sqlite3" "$restore/backups/production.sqlite3"
rm -f "$restore/db/production.sqlite3"
if "$REAL_DOCKER" run --rm --network none -v "$restore:/rails/storage" "$CANDIDATE" /hooks/post-restore \
     > "$SIM/g-newer.log" 2>&1; then
  fail "G: CANDIDATE's post-restore accepted a backup with a migration it doesn't know"
fi
grep -q 'post-restore: campfire db-migrate refused' "$SIM/g-newer.log" || fail "G: refused for another reason (see $SIM/g-newer.log)"

say "all scenarios behaved"

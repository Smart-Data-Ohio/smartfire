#!/usr/bin/env bash
#
# Production-copy cutover rehearsal: runs the Rails image and then the Rust image against the same
# restored /rails/storage, swapping containers the way `once update --image` does (same volume,
# same environment, same network name), entirely on this machine.
#
# Nothing here talks to production. The containers sit on an internal Docker network (no route
# out), so mail, Web Push, webhooks, unfurls and Google/Slack/GitHub calls cannot leave the host;
# none of their credentials are set either. Secrets are fresh, rehearsal-only values generated into
# $REHEARSAL_DIR/secrets.env (0600). No path or secret is baked into this script.
#
# Usage: REHEARSAL_DIR=/some/private/dir rehearsal.sh <command> [args]
#
#   prepare BACKUP_FILE AGE_IDENTITY   decrypt and verify the backup (deploy/backups/restore-check.sh),
#                                      lay out $REHEARSAL_DIR/base/{db,files}, a working copy in
#                                      $REHEARSAL_DIR/storage, secrets, a TLS cert and the network.
#   reset                              replace the working copy with base (drops all rehearsal writes).
#   start rails|rust                   start the app container on the working copy; waits for /up.
#   swap rails|rust                    stop the running app, start the other runtime on the same
#                                      volume and environment (the release's container swap).
#   stop                               stop and remove the app container.
#   runner RUBY                        bin/rails runner inside the running Rails container.
#   sql QUERY                          read-only sqlite3 query against the working copy.
#   schema FILE                        write the schema + migration list of the working copy to FILE.
#   rss                                current memory usage of the app container (docker stats).
#   driver SCRIPT [ARGS]               run a Node script from this directory in the Playwright image
#                                      on the internal network ($REHEARSAL_DIR/work mounted at /work).
#   teardown                           remove containers and the network (the data dir is the
#                                      caller's to delete).
#
# Network: REHEARSAL_NET (default sfreh-net, created --internal), REHEARSAL_SUBNET (default
# 10.231.77.0/24), REHEARSAL_ALIAS (default smartfire); REHEARSAL_APP names the container and
# REHEARSAL_MEMORY its memory limit (default 2g).
#
# Images: RAILS_IMAGE and RUST_IMAGE (required for start/swap), PLAYWRIGHT_IMAGE for `driver`
# (default campfire-parity-playwright:<tag> built from rust/parity/Dockerfile.playwright).

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
: "${REHEARSAL_DIR:?set REHEARSAL_DIR to a private (0700) directory}"
NET="${REHEARSAL_NET:-sfreh-net}"
APP="${REHEARSAL_APP:-sfreh-app}"
ALIAS="${REHEARSAL_ALIAS:-smartfire}"
PLAYWRIGHT_IMAGE="${PLAYWRIGHT_IMAGE:-campfire-parity-playwright:1c7609be336e}"
MEMORY="${REHEARSAL_MEMORY:-2g}"

log() { printf '[rehearsal] %s\n' "$*" >&2; }
die() { printf '[rehearsal] ERROR: %s\n' "$*" >&2; exit 1; }

storage() { echo "$REHEARSAL_DIR/storage"; }
db() { echo "$(storage)/db/production.sqlite3"; }

ensure_network() {
  docker network inspect "$NET" >/dev/null 2>&1 || docker network create --internal --subnet "${REHEARSAL_SUBNET:-10.231.77.0/24}" "$NET" >/dev/null
  [ "$(docker network inspect "$NET" --format '{{.Internal}}')" = true ] || die "$NET exists but is not --internal"
}

cmd_prepare() {
  local backup="${1:?backup file}" identity="${2:?age identity}"
  mkdir -p "$REHEARSAL_DIR"; chmod 700 "$REHEARSAL_DIR"
  bash "$REPO/deploy/backups/restore-check.sh" --backup "$backup" --work-dir "$REHEARSAL_DIR/checked" \
    --age-identity "$identity" | grep -v ': OK$'
  local stage
  stage="$(find "$REHEARSAL_DIR/checked/extracted" -mindepth 1 -maxdepth 1 -name 'smartfire-backup-*')"
  rm -rf "$REHEARSAL_DIR/base"; mkdir -p "$REHEARSAL_DIR/base/db"
  cp "$stage/production.sqlite3" "$REHEARSAL_DIR/base/db/production.sqlite3"
  cp -a "$stage/files" "$REHEARSAL_DIR/base/files"
  rm -f "$REHEARSAL_DIR/checked/backup.tar.gz"
  cmd_reset
  if [ ! -f "$REHEARSAL_DIR/secrets.env" ]; then
    [ -n "${RAILS_IMAGE:-}" ] || die "RAILS_IMAGE is needed to generate VAPID keys"
    local vapid
    vapid="$(docker run --rm --network none -e SECRET_KEY_BASE_DUMMY=1 "$RAILS_IMAGE" script/admin/create-vapid-key)"
    ( umask 077
      {
        echo "SECRET_KEY_BASE=$(openssl rand -hex 64)"
        echo "VAPID_PRIVATE_KEY=$(sed -n 's/^PRIVATE KEY : //p' <<<"$vapid")"
        echo "VAPID_PUBLIC_KEY=$(sed -n 's/^PUBLIC KEY  : //p' <<<"$vapid")"
      } > "$REHEARSAL_DIR/secrets.env" )
  fi
  mkdir -p "$REHEARSAL_DIR/work"; chmod 777 "$REHEARSAL_DIR/work"
  if [ ! -f "$REHEARSAL_DIR/work/tls.key" ]; then
    openssl req -x509 -newkey rsa:2048 -nodes -days 2 -subj "/CN=chat.rehearsal.test" \
      -keyout "$REHEARSAL_DIR/work/tls.key" -out "$REHEARSAL_DIR/work/tls.crt" 2>/dev/null
    chmod 644 "$REHEARSAL_DIR/work/tls.key"
  fi
  ensure_network
  log "prepared: $(sqlite3 "$(db)" 'select count(*) from users') users, $(find "$(storage)/files" -type f | wc -l) files"
}

cmd_reset() {
  cmd_stop
  rm -rf "$(storage)"; cp -a "$REHEARSAL_DIR/base" "$(storage)"
  mkdir -p "$(storage)/backups"
  chmod -R u+rwX,go-rwx "$(storage)"
  # The containers run as uid 1000 (rails); this keeps working when the caller is another uid.
  [ "$(id -u)" = 1000 ] || chmod -R a+rwX "$(storage)"
}

image_for() {
  case "$1" in
    rails) echo "${RAILS_IMAGE:?set RAILS_IMAGE}" ;;
    rust) echo "${RUST_IMAGE:?set RUST_IMAGE}" ;;
    *) die "runtime must be rails or rust" ;;
  esac
}

# The environment both runtimes get, as ONCE passes one environment map to whichever image runs.
# No SMTP, LiveKit, Google, GitHub, Slack or Sentry settings: those integrations stay unconfigured,
# and the internal network stops anything that tries anyway.
app_env_args() {
  echo --env-file "$REHEARSAL_DIR/secrets.env" \
    -e SKIP_TELEMETRY=1 -e APP_URL=https://chat.rehearsal.test:8443 \
    -e WEB_CONCURRENCY=0 -e RAILS_MAX_THREADS=3 -e JOB_CONCURRENCY=1
}

cmd_start() {
  local runtime="${1:?rails|rust}" image
  image="$(image_for "$runtime")"
  ensure_network
  docker rm -f "$APP" >/dev/null 2>&1 || true
  # shellcheck disable=SC2046
  docker run -d --name "$APP" --network "$NET" --network-alias "$ALIAS" --memory "$MEMORY" \
    --label net.smartdata.rehearsal.runtime="$runtime" \
    -v "$(storage):/rails/storage" $(app_env_args) "$image" >/dev/null
  local started; started=$(date +%s%N)
  for _ in $(seq 1 180); do
    if [ "$(docker inspect -f '{{.State.Running}}' "$APP" 2>/dev/null)" != true ]; then
      docker logs --tail 20 "$APP" >&2 || true; die "$runtime container exited"
    fi
    if docker run --rm --network "$NET" busybox:latest wget -q -O /dev/null -T 2 "http://$ALIAS/up" 2>/dev/null; then
      log "$runtime up in $(( ($(date +%s%N) - started) / 1000000 )) ms ($image)"
      return 0
    fi
    sleep 1
  done
  docker logs --tail 30 "$APP" >&2; die "$runtime never answered /up"
}

cmd_stop() {
  docker stop -t 20 "$APP" >/dev/null 2>&1 || true
  docker rm -f "$APP" >/dev/null 2>&1 || true
}

cmd_swap() {
  local runtime="${1:?rails|rust}"
  local logs="$REHEARSAL_DIR/work/logs"; mkdir -p "$logs"
  local previous; previous="$(docker inspect -f '{{index .Config.Labels "net.smartdata.rehearsal.runtime"}}' "$APP" 2>/dev/null || echo none)"
  docker logs "$APP" > "$logs/$previous-$(date +%s).log" 2>&1 || true
  cmd_stop
  cmd_start "$runtime"
}

cmd_runner() {
  docker exec -i "$APP" bin/rails runner "${1:?ruby}"
}

cmd_sql() {
  sqlite3 -readonly "$(db)" "${1:?query}"
}

cmd_schema() {
  local out="${1:?file}"
  { sqlite3 -readonly "$(db)" .schema; sqlite3 -readonly "$(db)" 'select version from schema_migrations order by version'; } > "$out"
}

cmd_rss() {
  docker stats --no-stream --format '{{.MemUsage}}' "$APP"
}

cmd_driver() {
  local script="${1:?script}"; shift
  ensure_network
  docker run --rm --network "$NET" --shm-size 1g --init --add-host chat.rehearsal.test:127.0.0.1 \
    -v "$HERE:/scripts:ro" -v "$REHEARSAL_DIR/work:/work" -e NODE_PATH=/node_modules \
    -e APP_HOST="$ALIAS" "$PLAYWRIGHT_IMAGE" node "/scripts/$script" "$@"
}

cmd_teardown() {
  cmd_stop
  docker ps -aq --filter "network=$NET" | xargs -r docker rm -f >/dev/null
  docker network rm "$NET" >/dev/null 2>&1 || true
}

command="${1:-}"; shift || true
case "$command" in
  prepare|reset|start|swap|stop|runner|sql|schema|rss|driver|teardown) "cmd_$command" "$@" ;;
  *) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 1 ;;
esac

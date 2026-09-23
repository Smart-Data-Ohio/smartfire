#!/usr/bin/env bash
#
# Smart Data Campfire - nightly off-machine backup.
#
# Packages a consistent SQLite snapshot (taken through the online backup API,
# never a raw copy of the live database), the Active Storage uploads tree, and
# a non-secret config manifest into one tarball, compresses and encrypts it
# client-side, and leaves the encrypted archive in an output directory.
#
# This runs on the VM host, NOT inside the app container, and never stops the
# app or freezes writes: the database snapshot uses SQLite's online backup
# API, which takes only brief page locks.
#
# The nightly GitHub Actions workflow (.github/workflows/nightly-backup.yml)
# drives this script over IAP SSH, downloads the encrypted archive, verifies
# its checksum, and uploads it to gs://<backup-bucket>/daily/ (plus weekly/
# on Sundays and monthly/ on the 1st). Encryption happens HERE, on the VM, to
# the age public recipient, so plaintext never leaves the VM. The VM itself
# holds no cloud credentials and never uploads anything: it cannot reach
# Cloud Storage at all.
#
# --- usage ---------------------------------------------------------------
# campfire-backup.sh --output-dir DIR
#
# Prints two machine-readable lines on success, which the workflow parses:
#   BACKUP_FILE=/path/to/smartfire-backup-<stamp>.tar.gz.age
#   BACKUP_SHA256=<hex>
#
# --- inputs ---------------------------------------------------------------
# --output-dir DIR / BACKUP_OUTPUT_DIR
#                   where the encrypted archive is left (default
#                   BACKUP_STATE_ROOT). Created when missing. Must not sit
#                   inside this run's work directory.
# BACKUP_ENCRYPTION   age (the default) or gpg.
# BACKUP_AGE_RECIPIENT  required for age: the age1... public recipient. The
#                     matching private key must NEVER be on this VM.
# BACKUP_GPG_RECIPIENT  required for gpg: key id, fingerprint or email.
# BACKUP_GPG_HOME     optional GnuPG home holding the recipient's public key.
# BACKUP_VOLUME_DIR   override for the ONCE storage volume mountpoint. When
#                     set, the database is snapshotted with the sqlite3 CLI
#                     and docker is not touched (this is also the test mode).
#                     When unset, the volume is discovered through docker and
#                     the snapshot is taken inside the running app container
#                     via script/admin/prepare-backup (the VM has no sqlite3).
# BACKUP_DB_REL       database path inside the volume (default db/production.sqlite3).
# BACKUP_FILES_REL    uploads path inside the volume (default files).
# BACKUP_APP_HOST     app hostname for the manifest. Discovered from the ONCE
#                     container label unless set.
# BACKUP_STATE_ROOT   parent of the work directories (default /var/backups).
# BACKUP_WORK_DIR     override for this run's work directory.
# BACKUP_DATETIME     override for the stamp, YYYYMMDD-HHMMSS in UTC (tests).
# BACKUP_LOCK_FILE    backup-vs-backup lock (default
#                     /var/lock/campfire-backup.lock).
# BACKUP_RELEASE_LOCK_FILE  the RELEASE lock (default
#                     /var/lock/campfire-release.lock, the same lock
#                     campfire-release.sh uses). Taken non-blocking around the
#                     prepare-backup snapshot: when a release holds it the
#                     backup exits 75 so the workflow retries or alerts, and
#                     storage/backups/production.sqlite3 is never written
#                     concurrently with a release.
# BACKUP_FREE_SPACE_FACTOR  free-space multiple of database+uploads required
#                     up front (default 3).
# BACKUP_KEEP_WORKDIR 1 keeps the unencrypted work directory for debugging.
#
# SECURITY: the ONCE container label and its Config.Env contain secret_key_base,
# VAPID keys and LiveKit secrets. Nothing here prints or archives them: only
# the non-secret subset (host, image) goes into the manifest.

set -euo pipefail

BACKUP_ENCRYPTION="${BACKUP_ENCRYPTION:-age}"
BACKUP_AGE_RECIPIENT="${BACKUP_AGE_RECIPIENT:-}"
BACKUP_GPG_RECIPIENT="${BACKUP_GPG_RECIPIENT:-}"
BACKUP_GPG_HOME="${BACKUP_GPG_HOME:-}"
BACKUP_VOLUME_DIR="${BACKUP_VOLUME_DIR:-}"
BACKUP_DB_REL="${BACKUP_DB_REL:-db/production.sqlite3}"
BACKUP_FILES_REL="${BACKUP_FILES_REL:-files}"
BACKUP_APP_HOST="${BACKUP_APP_HOST:-}"
BACKUP_STATE_ROOT="${BACKUP_STATE_ROOT:-/var/backups}"
BACKUP_WORK_DIR="${BACKUP_WORK_DIR:-}"
BACKUP_OUTPUT_DIR="${BACKUP_OUTPUT_DIR:-}"
BACKUP_DATETIME="${BACKUP_DATETIME:-}"
BACKUP_LOCK_FILE="${BACKUP_LOCK_FILE:-/var/lock/campfire-backup.lock}"
BACKUP_RELEASE_LOCK_FILE="${BACKUP_RELEASE_LOCK_FILE:-/var/lock/campfire-release.lock}"
BACKUP_FREE_SPACE_FACTOR="${BACKUP_FREE_SPACE_FACTOR:-3}"
BACKUP_KEEP_WORKDIR="${BACKUP_KEEP_WORKDIR:-0}"

# EX_TEMPFAIL: a release holds the lock, so retrying later may succeed.
EXIT_RELEASE_BUSY=75

WORK_DIR=""
OUTPUT_DIR=""
APP_IMAGE="unknown"
APP_HOST="unknown"
VOLUME_DIR=""

log()  { printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }
warn() { printf '[%s] WARNING: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
die()  { printf '[%s] ERROR: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; exit 1; }

usage() {
  sed -n '2,/^# SECURITY/p' "$0" | sed 's/^# \{0,1\}//'
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --output-dir)
        [ "$#" -ge 2 ] || die "--output-dir needs a directory"
        OUTPUT_DIR="$2"
        shift 2
        ;;
      --output-dir=*)
        OUTPUT_DIR="${1#--output-dir=}"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1 (see --help)"
        ;;
    esac
  done
  [ -n "$OUTPUT_DIR" ] || OUTPUT_DIR="$BACKUP_OUTPUT_DIR"
  [ -n "$OUTPUT_DIR" ] || OUTPUT_DIR="$BACKUP_STATE_ROOT"
  [ -n "$OUTPUT_DIR" ] || die "no output directory (pass --output-dir)"
}

cleanup() {
  if [ -n "$WORK_DIR" ] && [ -d "$WORK_DIR" ] && [ "$BACKUP_KEEP_WORKDIR" != "1" ]; then
    rm -rf "$WORK_DIR"
  fi
}

require_tools() {
  for tool in tar gzip sha256sum flock stat du df realpath; do
    command -v "$tool" >/dev/null || die "$tool is not installed"
  done
  case "$BACKUP_ENCRYPTION" in
    age) command -v age >/dev/null || die "age is not installed (BACKUP_ENCRYPTION=age)" ;;
    gpg) command -v gpg >/dev/null || die "gpg is not installed (BACKUP_ENCRYPTION=gpg)" ;;
    *) die "BACKUP_ENCRYPTION must be age or gpg, not '$BACKUP_ENCRYPTION'" ;;
  esac
}

# Refuse to run on top of an ONCE backup, restore or update: the volume may be
# mid-rewrite and the app may be stopped. Same guard as the release preflight.
refuse_once_busy() {
  command -v pgrep >/dev/null || { warn "pgrep is missing; skipping the ONCE-busy guard"; return 0; }
  local busy
  busy="$(pgrep -a -f '/usr/local/bin/once[[:space:]]+(backup|restore|update|deploy)' || true)"
  [ -z "$busy" ] || die "an ONCE operation is in flight; refusing to back up now: $busy"
}

# Work directories older than a day are leftovers of a killed run (a clean
# run removes its own through the EXIT trap): prune them so a dead run's
# plaintext staging does not pile up. Only our own naming pattern, only under
# the state root, never anything fresh enough to belong to a live run.
prune_stale_workdirs() {
  [ -d "$BACKUP_STATE_ROOT" ] || return 0
  local stale
  while IFS= read -r stale; do
    [ -n "$stale" ] || continue
    log "pruning stale work directory: $stale"
    rm -rf "$stale"
  done < <(find "$BACKUP_STATE_ROOT" -maxdepth 1 -mindepth 1 -type d \
    -name 'campfire-backup.*' -mmin +1440 -print)
}

# The single running ONCE application container. A stopped app means a release
# freeze or an outage is in progress; backing up that half-state would only
# confuse a restore, so fail loudly instead.
discover_container() {
  command -v docker >/dev/null || die "docker is not available and BACKUP_VOLUME_DIR is not set"
  local -a running=()
  local name
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    running+=("$name")
  done < <(docker ps --filter 'label=once' --format '{{.Names}}' | grep '^once-app-' || true)
  case "${#running[@]}" in
    1) printf '%s' "${running[0]}" ;;
    0) die "no running ONCE application container found; refusing to back up a stopped app" ;;
    *) die "more than one ONCE application container is running: ${running[*]}" ;;
  esac
}

# Best-effort non-secret facts for the manifest. Never prints or records values.
container_facts() {
  local container="$1"
  command -v jq >/dev/null || return 0
  local settings
  settings="$(docker inspect --format '{{index .Config.Labels "once"}}' "$container" 2>/dev/null || true)"
  [ -n "$settings" ] || return 0
  APP_HOST="$(printf '%s' "$settings" | jq -r '.host // "unknown"' 2>/dev/null || echo unknown)"
  APP_IMAGE="$(printf '%s' "$settings" | jq -r '.image // "unknown"' 2>/dev/null || echo unknown)"
}

resolve_volume() {
  if [ -n "$BACKUP_VOLUME_DIR" ]; then
    VOLUME_DIR="$BACKUP_VOLUME_DIR"
    [ -d "$VOLUME_DIR" ] || die "BACKUP_VOLUME_DIR does not exist: $VOLUME_DIR"
    APP_HOST="${BACKUP_APP_HOST:-unknown}"
    return 0
  fi
  local container
  container="$(discover_container)"
  log "app container: $container"
  container_facts "$container"
  [ -n "$BACKUP_APP_HOST" ] && APP_HOST="$BACKUP_APP_HOST"
  VOLUME_DIR="$(docker volume inspect "$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/rails/storage"}}{{.Name}}{{end}}{{end}}' "$container")" --format '{{.Mountpoint}}')"
  [ -n "$VOLUME_DIR" ] || die "could not find the /rails/storage volume for container $container"
  log "app volume: $VOLUME_DIR"
}

# Live bytes that will be staged: the database plus the uploads tree.
live_data_bytes() {
  local db="$VOLUME_DIR/$BACKUP_DB_REL" total=0 size
  [ -f "$db" ] || die "database file not found: $db"
  size="$(stat -c%s "$db")"
  total=$((total + size))
  if [ -d "$VOLUME_DIR/$BACKUP_FILES_REL" ]; then
    size="$(du -sb "$VOLUME_DIR/$BACKUP_FILES_REL" | awk '{print $1}')"
    total=$((total + size))
  fi
  printf '%s' "$total"
}

# Peak staging is about 2.5x the live data (snapshot copy + tarball +
# encrypted copy, briefly coexisting), so refuse to start below
# BACKUP_FREE_SPACE_FACTOR x rather than dying mid-run with ENOSPC.
require_free_space() {
  local dir="$1" needed="$2"
  local avail
  avail="$(df -B1 --output=avail "$dir" 2>/dev/null | tail -n 1 | tr -d '[:space:]')"
  [[ "$avail" =~ ^[0-9]+$ ]] || die "could not read free space for $dir"
  [ "$avail" -ge "$needed" ] || \
    die "only $avail bytes free under $dir, need $needed ($BACKUP_FREE_SPACE_FACTOR x the live data)"
}

# The release lock, taken WITHOUT waiting: a release in flight means
# storage/backups/production.sqlite3 may be mid-rewrite, so the backup must
# not trigger prepare-backup now. Exit 75 (EX_TEMPFAIL) so the workflow
# retries or alerts instead of recording a plain failure.
take_release_lock() {
  exec 10>"$BACKUP_RELEASE_LOCK_FILE" \
    || die "cannot open the release lock $BACKUP_RELEASE_LOCK_FILE"
  flock -n 10 || {
    printf '[%s] ERROR: a release holds %s; rerun this backup after the release finishes\n' \
      "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$BACKUP_RELEASE_LOCK_FILE" >&2
    exit "$EXIT_RELEASE_BUSY"
  }
}

release_release_lock() {
  flock -u 10 2>/dev/null || true
  exec 10>&- || true
}

# Online snapshot through SQLite's backup API. In container mode this runs
# script/admin/prepare-backup inside the app (the VM has no sqlite3 binary);
# with BACKUP_VOLUME_DIR it runs the equivalent `.backup` on the host. Both
# hold only brief page locks: writers keep writing. The release lock is held
# from before prepare-backup until the copy out of the volume completes, so
# storage/backups/production.sqlite3 is never written concurrently with a
# release; it is released again immediately after, so a backup never blocks a
# release longer than the snapshot takes.
snapshot_database() {
  local destination="$1"
  take_release_lock
  if [ -n "$BACKUP_VOLUME_DIR" ]; then
    local db="$VOLUME_DIR/$BACKUP_DB_REL"
    [ -f "$db" ] || { release_release_lock; die "database file not found: $db"; }
    command -v sqlite3 >/dev/null || { release_release_lock; die "sqlite3 CLI is required when BACKUP_VOLUME_DIR is set"; }
    sqlite3 "$db" ".backup main \"$destination\""
  else
    local container produced
    container="$(discover_container)"
    docker exec "$container" /rails/script/admin/prepare-backup
    produced="$VOLUME_DIR/backups/production.sqlite3"
    [ -f "$produced" ] || { release_release_lock; die "prepare-backup did not produce $produced"; }
    install -m 0600 "$produced" "$destination"
  fi
  release_release_lock
  [ -s "$destination" ] || die "database snapshot is empty: $destination"
}

json_escape() {
  # Enough escaping for hostnames, image refs and digests: nothing here should
  # ever contain a quote, but a hostile value must not break the manifest.
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr -d '\n\r'
}

main() {
  parse_args "$@"
  case "$BACKUP_ENCRYPTION" in
    age)
      [ -n "$BACKUP_AGE_RECIPIENT" ] || die "BACKUP_AGE_RECIPIENT is required for BACKUP_ENCRYPTION=age"
      case "$BACKUP_AGE_RECIPIENT" in
        REPLACE-*) die "BACKUP_AGE_RECIPIENT still has the example placeholder" ;;
      esac
      ;;
    gpg) [ -n "$BACKUP_GPG_RECIPIENT" ] || die "BACKUP_GPG_RECIPIENT is required for BACKUP_ENCRYPTION=gpg" ;;
  esac
  [[ "$BACKUP_FREE_SPACE_FACTOR" =~ ^[0-9]+$ ]] || die "BACKUP_FREE_SPACE_FACTOR must be a number"
  require_tools

  local stamp="${BACKUP_DATETIME:-$(date -u +%Y%m%d-%H%M%S)}"
  [[ "$stamp" =~ ^[0-9]{8}-[0-9]{6}$ ]] || die "stamp must look like YYYYMMDD-HHMMSS, not '$stamp'"

  if [ -z "$BACKUP_WORK_DIR" ]; then
    WORK_DIR="$BACKUP_STATE_ROOT/campfire-backup.$stamp"
  else
    WORK_DIR="$BACKUP_WORK_DIR"
  fi
  local work_abs output_abs
  work_abs="$(realpath -m "$WORK_DIR")"
  output_abs="$(realpath -m "$OUTPUT_DIR")"
  case "$output_abs" in
    "$work_abs"|"$work_abs"/*)
      die "--output-dir must not sit inside this run's work directory ($WORK_DIR)" ;;
  esac

  mkdir -p "$BACKUP_STATE_ROOT"
  mkdir -p "$OUTPUT_DIR"
  exec 9>"$BACKUP_LOCK_FILE"
  flock -n 9 || die "another backup holds $BACKUP_LOCK_FILE"
  trap cleanup EXIT

  prune_stale_workdirs
  refuse_once_busy
  resolve_volume

  local live_bytes needed
  live_bytes="$(live_data_bytes)"
  needed=$((live_bytes * BACKUP_FREE_SPACE_FACTOR))
  require_free_space "$BACKUP_STATE_ROOT" "$needed"
  require_free_space "$OUTPUT_DIR" "$needed"

  rm -rf "$WORK_DIR"
  mkdir -p "$WORK_DIR"
  chmod 0700 "$WORK_DIR"
  local stage="$WORK_DIR/smartfire-backup-$stamp"
  mkdir -p "$stage"

  log "snapshotting the database through the online backup API"
  snapshot_database "$stage/production.sqlite3"
  local db_sha db_bytes
  db_sha="$(sha256sum "$stage/production.sqlite3" | awk '{print $1}')"
  db_bytes="$(stat -c%s "$stage/production.sqlite3")"

  log "copying uploads ($BACKUP_FILES_REL)"
  local files_count=0 files_bytes=0
  if [ -d "$VOLUME_DIR/$BACKUP_FILES_REL" ]; then
    cp -a "$VOLUME_DIR/$BACKUP_FILES_REL" "$stage/files"
    files_count="$(find "$stage/files" -type f | wc -l)"
    files_bytes="$(du -sb "$stage/files" | awk '{print $1}')"
  else
    mkdir -p "$stage/files"
    warn "uploads directory is missing, staging an empty one: $VOLUME_DIR/$BACKUP_FILES_REL"
  fi

  # The manifest carries facts ABOUT the backup, never secrets and never
  # anything from the app environment (not even key names).
  local created_at recipient
  created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [ "$BACKUP_ENCRYPTION" = "age" ]; then recipient="$BACKUP_AGE_RECIPIENT"; else recipient="$BACKUP_GPG_RECIPIENT"; fi
  {
    printf '{\n'
    printf '  "backup_format": 1,\n'
    printf '  "created_at": "%s",\n' "$created_at"
    printf '  "stamp": "%s",\n' "$stamp"
    printf '  "hostname": "%s",\n' "$(json_escape "$(hostname)")"
    printf '  "app_host": "%s",\n' "$(json_escape "$APP_HOST")"
    printf '  "app_image": "%s",\n' "$(json_escape "$APP_IMAGE")"
    printf '  "encryption": "%s",\n' "$BACKUP_ENCRYPTION"
    printf '  "recipient": "%s",\n' "$(json_escape "$recipient")"
    printf '  "database": {"file": "production.sqlite3", "sha256": "%s", "bytes": %s},\n' "$db_sha" "$db_bytes"
    printf '  "files": {"dir": "files", "count": %s, "bytes": %s}\n' "$files_count" "$files_bytes"
    printf '}\n'
  } > "$stage/manifest.json"
  chmod 0600 "$stage/manifest.json"

  ( cd "$stage" && find . -type f ! -name 'SHA256SUMS' -print0 | LC_ALL=C sort -z | xargs -0 sha256sum > SHA256SUMS )
  chmod 0600 "$stage/SHA256SUMS"

  local base="smartfire-backup-${stamp}.tar.gz"
  local plain="$WORK_DIR/$base" encrypted
  tar -czf "$plain" -C "$WORK_DIR" "smartfire-backup-$stamp"
  chmod 0600 "$plain"

  case "$BACKUP_ENCRYPTION" in
    age)
      encrypted="${plain}.age"
      age --encrypt --recipient "$BACKUP_AGE_RECIPIENT" --output "$encrypted" "$plain"
      ;;
    gpg)
      encrypted="${plain}.gpg"
      if [ -n "$BACKUP_GPG_HOME" ]; then
        gpg --batch --yes --trust-model always --homedir "$BACKUP_GPG_HOME" \
          --encrypt --recipient "$BACKUP_GPG_RECIPIENT" --output "$encrypted" "$plain"
      else
        gpg --batch --yes --trust-model always \
          --encrypt --recipient "$BACKUP_GPG_RECIPIENT" --output "$encrypted" "$plain"
      fi
      ;;
  esac
  chmod 0600 "$encrypted"
  rm -f "$plain"

  # The encrypted archive is the ONLY thing that leaves the work directory:
  # the EXIT trap removes every byte of plaintext staging with it.
  local basename final
  basename="$(basename "$encrypted")"
  final="$OUTPUT_DIR/$basename"
  mv "$encrypted" "$final"
  chmod 0600 "$final"
  local enc_sha enc_bytes
  enc_sha="$(sha256sum "$final" | awk '{print $1}')"
  enc_bytes="$(stat -c%s "$final")"

  {
    printf '{\n'
    printf '  "stamp": "%s",\n' "$stamp"
    printf '  "created_at": "%s",\n' "$created_at"
    printf '  "file": "%s",\n' "$(json_escape "$basename")"
    printf '  "sha256": "%s",\n' "$enc_sha"
    printf '  "bytes": %s,\n' "$enc_bytes"
    printf '  "db_sha256": "%s",\n' "$db_sha"
    printf '  "db_bytes": %s,\n' "$db_bytes"
    printf '  "files_count": %s,\n' "$files_count"
    printf '  "files_bytes": %s\n' "$files_bytes"
    printf '}\n'
  } > "$BACKUP_STATE_ROOT/campfire-nightly-last.json"
  chmod 0600 "$BACKUP_STATE_ROOT/campfire-nightly-last.json"

  log "backup complete: $basename ($enc_bytes bytes, sha256 $enc_sha)"
  printf 'BACKUP_FILE=%s\n' "$final"
  printf 'BACKUP_SHA256=%s\n' "$enc_sha"
}

main "$@"

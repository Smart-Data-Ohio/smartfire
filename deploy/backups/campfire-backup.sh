#!/usr/bin/env bash
#
# Smart Data Campfire - nightly off-machine backup.
#
# Packages a consistent SQLite snapshot (taken through the online backup API,
# never a raw copy of the live database), the Active Storage uploads tree, and
# a non-secret config manifest into one tarball, compresses and encrypts it
# client-side, and uploads it to gs://<backup-bucket>/daily/ in the SEPARATE
# backup GCP project. The timer additionally uploads the same file under
# weekly/ on Sundays and monthly/ on the 1st; bucket lifecycle ages each
# prefix out (see lifecycle.json).
#
# This runs on the VM host from a systemd timer, NOT inside the app container,
# and never stops the app or freezes writes: the database snapshot uses
# SQLite's online backup API, which takes only brief page locks.
#
# Least privilege: the VM uploads with a service account that holds ONLY
# roles/storage.objectCreator on the backup bucket. It can create objects but
# can neither read, delete nor overwrite them. Promotion to weekly/monthly is
# therefore a second upload of the same local file, not a server-side copy
# (which would need read access). Upload success cannot be verified by
# reading back; the monthly restore check (backup-restore-check.yml) with a
# separate reader identity is what proves an upload is good.
#
# --- inputs ---------------------------------------------------------------
# BACKUP_BUCKET       required. Backup bucket name WITHOUT gs:// prefix.
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
# BACKUP_DATETIME     override for the stamp, YYYYMMDD-HHMMSS in UTC. Used for
#                     the object name and the weekly/monthly decision (tests).
# BACKUP_LOCK_FILE   /default /var/lock/campfire-backup.lock.
# BACKUP_UPLOAD_BIN   gcloud (the default) or gsutil. gcloud is preferred when
#                     both are installed.
# BACKUP_KEEP_WORKDIR 1 keeps the unencrypted work directory for debugging.
#
# SECURITY: the ONCE container label and its Config.Env contain secret_key_base,
# VAPID keys and LiveKit secrets. Nothing here prints or archives them: only
# the non-secret subset (host, image, env var NAMES) goes into the manifest.

set -euo pipefail

BACKUP_BUCKET="${BACKUP_BUCKET:-}"
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
BACKUP_DATETIME="${BACKUP_DATETIME:-}"
BACKUP_LOCK_FILE="${BACKUP_LOCK_FILE:-/var/lock/campfire-backup.lock}"
BACKUP_UPLOAD_BIN="${BACKUP_UPLOAD_BIN:-}"
BACKUP_KEEP_WORKDIR="${BACKUP_KEEP_WORKDIR:-0}"

WORK_DIR=""
APP_IMAGE="unknown"
APP_HOST="unknown"
VOLUME_DIR=""

log()  { printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }
warn() { printf '[%s] WARNING: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
die()  { printf '[%s] ERROR: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; exit 1; }

cleanup() {
  if [ -n "$WORK_DIR" ] && [ -d "$WORK_DIR" ] && [ "$BACKUP_KEEP_WORKDIR" != "1" ]; then
    rm -rf "$WORK_DIR"
  fi
}

require_tools() {
  for tool in tar gzip sha256sum flock; do
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

# Online snapshot through SQLite's backup API. In container mode this runs
# script/admin/prepare-backup inside the app (the VM has no sqlite3 binary);
# with BACKUP_VOLUME_DIR it runs the equivalent `.backup` on the host. Both
# hold only brief page locks: writers keep writing.
snapshot_database() {
  local destination="$1"
  if [ -n "$BACKUP_VOLUME_DIR" ]; then
    local db="$VOLUME_DIR/$BACKUP_DB_REL"
    [ -f "$db" ] || die "database file not found: $db"
    command -v sqlite3 >/dev/null || die "sqlite3 CLI is required when BACKUP_VOLUME_DIR is set"
    sqlite3 "$db" ".backup main \"$destination\""
  else
    local container produced
    container="$(discover_container)"
    docker exec "$container" /rails/script/admin/prepare-backup
    produced="$VOLUME_DIR/backups/production.sqlite3"
    [ -f "$produced" ] || die "prepare-backup did not produce $produced"
    install -m 0600 "$produced" "$destination"
  fi
  [ -s "$destination" ] || die "database snapshot is empty: $destination"
}

json_escape() {
  # Enough escaping for hostnames, image refs and digests: nothing here should
  # ever contain a quote, but a hostile value must not break the manifest.
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr -d '\n\r'
}

upload_object() {
  local src="$1" object="$2"
  local dest="gs://${BACKUP_BUCKET}/${object}"
  local bin="$BACKUP_UPLOAD_BIN"
  if [ -z "$bin" ]; then
    if command -v gcloud >/dev/null; then bin=gcloud; elif command -v gsutil >/dev/null; then bin=gsutil; fi
  fi
  case "$bin" in
    gcloud) gcloud storage cp "$src" "$dest" ;;
    gsutil) gsutil cp "$src" "$dest" ;;
    *) die "neither gcloud nor gsutil is installed (BACKUP_UPLOAD_BIN='$BACKUP_UPLOAD_BIN')" ;;
  esac
}

main() {
  [ -n "$BACKUP_BUCKET" ] || die "BACKUP_BUCKET is required"
  case "$BACKUP_BUCKET" in
    gs://*|*/*) die "BACKUP_BUCKET must be a bare bucket name, not '$BACKUP_BUCKET'" ;;
  esac
  case "$BACKUP_ENCRYPTION" in
    age) [ -n "$BACKUP_AGE_RECIPIENT" ] || die "BACKUP_AGE_RECIPIENT is required for BACKUP_ENCRYPTION=age" ;;
    gpg) [ -n "$BACKUP_GPG_RECIPIENT" ] || die "BACKUP_GPG_RECIPIENT is required for BACKUP_ENCRYPTION=gpg" ;;
  esac
  require_tools

  local stamp="${BACKUP_DATETIME:-$(date -u +%Y%m%d-%H%M%S)}"
  [[ "$stamp" =~ ^[0-9]{8}-[0-9]{6}$ ]] || die "stamp must look like YYYYMMDD-HHMMSS, not '$stamp'"
  local datestr="${stamp%%-*}"

  mkdir -p "$BACKUP_STATE_ROOT"
  exec 9>"$BACKUP_LOCK_FILE"
  flock -n 9 || die "another backup holds $BACKUP_LOCK_FILE"
  trap cleanup EXIT

  refuse_once_busy
  resolve_volume

  if [ -z "$BACKUP_WORK_DIR" ]; then
    WORK_DIR="$BACKUP_STATE_ROOT/campfire-nightly-$stamp"
  else
    WORK_DIR="$BACKUP_WORK_DIR"
  fi
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

  # The manifest carries facts ABOUT the backup, never secrets and never the
  # values of the app environment (key names only, and only when discoverable).
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
  local enc_sha enc_bytes basename
  enc_sha="$(sha256sum "$encrypted" | awk '{print $1}')"
  enc_bytes="$(stat -c%s "$encrypted")"
  basename="$(basename "$encrypted")"

  log "uploading $basename ($enc_bytes bytes, sha256 $enc_sha)"
  local -a objects=("daily/$basename")
  # GNU date parses YYYYMMDD. %u: 1=Monday..7=Sunday.
  local dow dom
  dow="$(date -u -d "$datestr" +%u)"
  dom="$(date -u -d "$datestr" +%-d)"
  [ "$dow" = "7" ] && objects+=("weekly/$basename")
  [ "$dom" = "1" ] && objects+=("monthly/$basename")

  local object
  for object in "${objects[@]}"; do
    # One upload per prefix: a create-only credential cannot read an object
    # back for a server-side copy.
    upload_object "$encrypted" "$object"
    log "uploaded gs://${BACKUP_BUCKET}/${object}"
  done

  {
    printf '{\n'
    printf '  "stamp": "%s",\n' "$stamp"
    printf '  "created_at": "%s",\n' "$created_at"
    printf '  "bucket": "%s",\n' "$(json_escape "$BACKUP_BUCKET")"
    printf '  "object": "%s",\n' "$(json_escape "$basename")"
    printf '  "prefixes": ['
    local first=1 prefix
    for object in "${objects[@]}"; do
      prefix="${object%%/*}"
      if [ "$first" = "1" ]; then first=0; else printf ', '; fi
      printf '"%s"' "$prefix"
    done
    printf '],\n'
    printf '  "sha256": "%s",\n' "$enc_sha"
    printf '  "bytes": %s,\n' "$enc_bytes"
    printf '  "db_sha256": "%s",\n' "$db_sha"
    printf '  "db_bytes": %s,\n' "$db_bytes"
    printf '  "files_count": %s,\n' "$files_count"
    printf '  "files_bytes": %s\n' "$files_bytes"
    printf '}\n'
  } > "$BACKUP_STATE_ROOT/campfire-nightly-last.json"
  chmod 0600 "$BACKUP_STATE_ROOT/campfire-nightly-last.json"

  log "backup complete: $basename in ${objects[*]}"
}

main "$@"

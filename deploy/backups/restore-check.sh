#!/usr/bin/env bash
#
# Verifies one encrypted nightly backup: decrypts it, checks the tarball and
# the inner SHA256SUMS, runs PRAGMA integrity_check on the database, counts
# rows, and optionally hands a disposable copy of the restored database to a
# local Rust Smartfire image. That container migrates the copy with the
# image's compiled-in migrations (`campfire db-migrate`), so a backup taken
# before a schema change still checks against a newer image, then runs the
# strict boot schema check and counts rows (`campfire db-check`). The
# extracted backup itself is never mounted, and is re-verified byte-identical
# afterwards.
#
# Used by the monthly restore-check workflow and runnable by hand on any
# machine holding the decryption key. Exits non-zero (loudly) on any failure.
# Never prints the key or any row contents, only counts.
#
# Usage:
#   restore-check.sh --backup FILE --work-dir DIR [options]
#
#   --backup FILE        the encrypted backup (.tar.gz.age or .tar.gz.gpg).
#   --work-dir DIR       scratch directory (created empty, kept for inspection).
#   --encryption E       age (default from the file extension), gpg, or auto.
#   --age-identity FILE  age private key file (required for age).
#   --gpg-home DIR       GnuPG home holding the private key (for gpg).
#   --min-users N        fail unless the users table holds at least N rows.
#   --min-messages N     fail unless the messages table holds at least N rows.
#   --image REF          also migrate and check a disposable copy with this
#                        local Rust image (labelled
#                        net.smartdata.campfire.runtime=rust).
#   --container-name N   name prefix for those disposable containers
#                        (default unique).

set -euo pipefail

BACKUP=""
WORK_DIR=""
ENCRYPTION="auto"
AGE_IDENTITY=""
GPG_HOME=""
MIN_USERS=""
MIN_MESSAGES=""
APP_IMAGE=""
CHECK_CONTAINER="campfire-restore-check-$$"

log() { printf '[restore-check] %s\n' "$*"; }
die() { printf '[restore-check] ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --backup) BACKUP="${2:-}"; shift 2 ;;
    --work-dir) WORK_DIR="${2:-}"; shift 2 ;;
    --encryption) ENCRYPTION="${2:-}"; shift 2 ;;
    --age-identity) AGE_IDENTITY="${2:-}"; shift 2 ;;
    --gpg-home) GPG_HOME="${2:-}"; shift 2 ;;
    --min-users) MIN_USERS="${2:-}"; shift 2 ;;
    --min-messages) MIN_MESSAGES="${2:-}"; shift 2 ;;
    --image) APP_IMAGE="${2:-}"; shift 2 ;;
    --container-name) CHECK_CONTAINER="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

[ -n "$BACKUP" ] || die "--backup is required"
[ -n "$WORK_DIR" ] || die "--work-dir is required"
[ -f "$BACKUP" ] || die "backup file not found: $BACKUP"
command -v sqlite3 >/dev/null || die "sqlite3 is not installed"
command -v tar >/dev/null || die "tar is not installed"
command -v sha256sum >/dev/null || die "sha256sum is not installed"

if [ "$ENCRYPTION" = "auto" ]; then
  case "$BACKUP" in
    *.tar.gz.age) ENCRYPTION="age" ;;
    *.tar.gz.gpg) ENCRYPTION="gpg" ;;
    *) die "cannot infer --encryption from '$BACKUP'; pass --encryption age|gpg" ;;
  esac
fi

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
chmod 0700 "$WORK_DIR"

log "decrypting $BACKUP ($ENCRYPTION)"
plain="$WORK_DIR/backup.tar.gz"
case "$ENCRYPTION" in
  age)
    [ -n "$AGE_IDENTITY" ] || die "--age-identity is required for age decryption"
    [ -f "$AGE_IDENTITY" ] || die "age identity file not found: $AGE_IDENTITY"
    command -v age >/dev/null || die "age is not installed"
    age --decrypt --identity "$AGE_IDENTITY" --output "$plain" "$BACKUP"
    ;;
  gpg)
    command -v gpg >/dev/null || die "gpg is not installed"
    if [ -n "$GPG_HOME" ]; then
      gpg --batch --yes --homedir "$GPG_HOME" --decrypt --output "$plain" "$BACKUP"
    else
      gpg --batch --yes --decrypt --output "$plain" "$BACKUP"
    fi
    ;;
  *) die "--encryption must be age or gpg, not '$ENCRYPTION'" ;;
esac

log "extracting the tarball"
tar -tzf "$plain" >/dev/null
mkdir -p "$WORK_DIR/extracted"
tar -xzf "$plain" -C "$WORK_DIR/extracted"
stage_count="$(find "$WORK_DIR/extracted" -mindepth 1 -maxdepth 1 -name 'smartfire-backup-*' | wc -l)"
[ "$stage_count" = "1" ] || die "expected exactly one smartfire-backup-* directory, found $stage_count"
stage="$(find "$WORK_DIR/extracted" -mindepth 1 -maxdepth 1 -name 'smartfire-backup-*')"
log "staged in $stage"

log "verifying SHA256SUMS"
( cd "$stage" && sha256sum -c SHA256SUMS )

db="$stage/production.sqlite3"
[ -f "$db" ] || die "production.sqlite3 is missing from the backup"

log "running PRAGMA integrity_check"
integrity="$(sqlite3 "$db" 'PRAGMA integrity_check;')"
[ "$integrity" = "ok" ] || die "integrity_check failed: $integrity"
log "integrity_check: ok"

count_table() {
  local table="$1"
  if [ "$(sqlite3 "$db" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='$table';")" = "1" ]; then
    sqlite3 "$db" "SELECT count(*) FROM \"$table\";"
  else
    echo "missing"
  fi
}

users="$(count_table users)"
rooms="$(count_table rooms)"
messages="$(count_table messages)"
log "row counts: users=$users rooms=$rooms messages=$messages"

if [ -n "$MIN_USERS" ] && { [ "$users" = "missing" ] || [ "$users" -lt "$MIN_USERS" ]; }; then
  die "users=$users is below --min-users=$MIN_USERS"
fi
if [ -n "$MIN_MESSAGES" ] && { [ "$messages" = "missing" ] || [ "$messages" -lt "$MIN_MESSAGES" ]; }; then
  die "messages=$messages is below --min-messages=$MIN_MESSAGES"
fi

if [ -n "$APP_IMAGE" ]; then
  command -v docker >/dev/null || die "docker is required for --image"
  runtime="$(docker image inspect "$APP_IMAGE" --format '{{index .Config.Labels "net.smartdata.campfire.runtime"}}')" \
    || die "cannot inspect local image $APP_IMAGE"
  [ "$runtime" = "rust" ] || die "$APP_IMAGE is not a Rust Smartfire image (runtime label '$runtime')"
  copy="$WORK_DIR/app-check"
  mkdir -p "$copy/db"
  cp "$db" "$copy/db/production.sqlite3"
  # Container uid 1000 migrates this disposable copy, so it (and SQLite's
  # journal beside it) must be writable whatever uid extracted the backup.
  chmod 0777 "$copy" "$copy/db"
  chmod 0666 "$copy/db/production.sqlite3"
  for command in db-migrate db-check; do
    log "campfire $command on a copy of the restored database"
    docker run --rm --name "$CHECK_CONTAINER-$command" --network none --memory 768m \
      -v "$copy:/rails/storage" "$APP_IMAGE" \
      campfire "$command" /rails/storage/db/production.sqlite3
  done
  log "the Rust image migrated and read the restored database; re-verifying the extracted backup"
  ( cd "$stage" && sha256sum -c SHA256SUMS )
fi

log "restore check passed: $BACKUP is intact and readable"

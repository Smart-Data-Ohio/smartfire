#!/usr/bin/env bash
#
# Verifies one encrypted nightly backup: decrypts it, checks the tarball and
# the inner SHA256SUMS, runs PRAGMA integrity_check on the database, counts
# rows, and optionally boots the Rails app against a copy of the restored
# database and counts rows through the models. Rails' SQLite adapter issues
# its own PRAGMAs on connect, so the app cannot open a file-mode read-only
# database; instead it only ever sees a disposable copy, the runner sets
# PRAGMA query_only so no model code can write, and the extracted backup is
# re-verified byte-identical afterwards.
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
#   --rails-root DIR     when set, also boot the app at DIR against a COPY of
#                        the restored database and count rows with a Rails
#                        runner, then re-verify the extracted backup is
#                        untouched. Refuses when DIR already has a
#                        storage/db/production.sqlite3, so it can never run
#                        inside a live checkout by accident.
#   --min-users N        fail unless the users table holds at least N rows.
#   --min-messages N     fail unless the messages table holds at least N rows.

set -euo pipefail

BACKUP=""
WORK_DIR=""
ENCRYPTION="auto"
AGE_IDENTITY=""
GPG_HOME=""
RAILS_ROOT=""
MIN_USERS=""
MIN_MESSAGES=""

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
    --rails-root) RAILS_ROOT="${2:-}"; shift 2 ;;
    --min-users) MIN_USERS="${2:-}"; shift 2 ;;
    --min-messages) MIN_MESSAGES="${2:-}"; shift 2 ;;
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

if [ -n "$RAILS_ROOT" ]; then
  log "booting the app against a copy of the restored database"
  [ -d "$RAILS_ROOT" ] || die "rails root not found: $RAILS_ROOT"
  [ -x "$RAILS_ROOT/bin/rails" ] || die "no executable bin/rails in $RAILS_ROOT"
  target="$RAILS_ROOT/storage/db/production.sqlite3"
  [ ! -e "$target" ] || die "$target already exists; refusing to run inside a live checkout"
  mkdir -p "$RAILS_ROOT/storage/db"
  cp "$db" "$target"
  # shellcheck disable=SC2064
  trap "rm -f '$target' '$target-wal' '$target-shm'" EXIT
  ( cd "$RAILS_ROOT" && SECRET_KEY_BASE_DUMMY=1 RAILS_ENV=production bin/rails runner \
    'ActiveRecord::Base.connection.execute("PRAGMA query_only = ON"); puts JSON.generate(users: User.count, rooms: Room.count, messages: Message.count)' )
  log "rails runner row count succeeded; re-verifying the extracted backup is untouched"
  ( cd "$stage" && sha256sum -c SHA256SUMS )
fi

log "restore check passed: $BACKUP is intact and readable"

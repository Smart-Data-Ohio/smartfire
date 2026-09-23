#!/usr/bin/env bash
#
# Installs the nightly backup timer on the VM host. Idempotent: re-running it
# updates the script and units and leaves an existing /etc/campfire-backups/
# backup.env untouched. Run as root from a checkout of this repository:
#
#   sudo deploy/backups/install-backup.sh
#
# Then edit /etc/campfire-backups/backup.env (copied from backup.env.example
# on first install) and verify with:
#
#   sudo systemctl start campfire-backup.service
#   sudo journalctl -u campfire-backup.service --since '5 min ago'
#   sudo systemctl list-timers campfire-backup.timer
#
# INSTALL_DEST_DIR, INSTALL_ENV_DIR and INSTALL_SYSTEMD_DIR override the
# install locations (for the test harness); they default to the real paths.

set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST_DIR="${INSTALL_DEST_DIR:-/opt/campfire-backups}"
ENV_DIR="${INSTALL_ENV_DIR:-/etc/campfire-backups}"
SYSTEMD_DIR="${INSTALL_SYSTEMD_DIR:-/etc/systemd/system}"

log() { printf '[install-backup] %s\n' "$*"; }
die() { printf '[install-backup] ERROR: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "must run as root (use sudo)"

command -v systemctl >/dev/null || die "systemctl is not available"
for tool in tar gzip sha256sum flock; do
  command -v "$tool" >/dev/null || die "$tool is not installed"
done
if ! command -v gcloud >/dev/null && ! command -v gsutil >/dev/null; then
  die "neither gcloud nor gsutil is installed; the backup cannot upload"
fi

install -d -o root -g root -m 0755 "$DEST_DIR"
install -o root -g root -m 0755 "$SRC_DIR/campfire-backup.sh" "$DEST_DIR/campfire-backup.sh"
install -o root -g root -m 0755 "$SRC_DIR/restore-check.sh" "$DEST_DIR/restore-check.sh"
install -o root -g root -m 0644 "$SRC_DIR/lifecycle.json" "$DEST_DIR/lifecycle.json"
install -o root -g root -m 0644 "$SRC_DIR/README.md" "$DEST_DIR/README.md"
log "installed the backup scripts in $DEST_DIR"

install -d -o root -g root -m 0700 "$ENV_DIR"
if [ -f "$ENV_DIR/backup.env" ]; then
  log "keeping the existing $ENV_DIR/backup.env"
else
  install -o root -g root -m 0600 "$SRC_DIR/backup.env.example" "$ENV_DIR/backup.env"
  log "created $ENV_DIR/backup.env from the example; edit it before the first run"
fi

install -d -o root -g root -m 0755 "$SYSTEMD_DIR"
install -o root -g root -m 0644 "$SRC_DIR/campfire-backup.service" "$SYSTEMD_DIR/campfire-backup.service"
install -o root -g root -m 0644 "$SRC_DIR/campfire-backup.timer" "$SYSTEMD_DIR/campfire-backup.timer"
systemctl daemon-reload
systemctl enable --now campfire-backup.timer
log "enabled campfire-backup.timer:"
systemctl list-timers campfire-backup.timer --no-pager || true

# The configured encryption tool must exist before the first timer run, so
# install it rather than warning: a warned-about miss becomes a failed 09:00
# run days later.
encryption="$(sed -n 's/^BACKUP_ENCRYPTION=//p' "$ENV_DIR/backup.env" | tail -n 1)"
encryption="${encryption:-age}"
if [ "$encryption" = "gpg" ]; then
  command -v gpg >/dev/null || die "gpg is configured but not installed"
elif ! command -v age >/dev/null; then
  command -v apt-get >/dev/null || die "age is configured but not installed, and apt-get is not available to install it"
  log "age is configured but not installed; installing it with apt-get"
  apt-get update
  apt-get install -y age
  command -v age >/dev/null || die "age is still not installed after apt-get install -y age"
fi
command -v docker >/dev/null || log "WARNING: docker is not installed; container-mode snapshots need it"

log "done. Next: edit $ENV_DIR/backup.env, then run: systemctl start campfire-backup.service"

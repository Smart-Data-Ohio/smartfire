#!/usr/bin/env bash
#
# Creates the nightly boot-disk snapshot schedule for the app VM and attaches
# it to the instance's boot disk. Idempotent: existing policies and attachments
# are left alone. The lead runs this (directly, or through
# setup-backup-project.sh, which calls it).
#
# This is NOT the release-time snapshot from deploy-gcp.yml. That one is taken
# by hand per release, while writes are frozen, and kept as that release's
# checkpoint. This schedule takes a crash-consistent boot-disk snapshot every
# night and GCP drops each one after SNAPSHOT_RETENTION_DAYS. The two cover
# different failures: the release snapshot rolls back a bad deploy, the
# schedule (plus the encrypted logical backup in Cloud Storage) recovers from
# a dead disk, a dead VM, or a dead project. See docs/backups.md.
#
# --- inputs ---------------------------------------------------------------
# APP_PROJECT_ID    required. The project holding the app VM.
# APP_ZONE          zone of the app VM (default us-central1-a).
# APP_INSTANCE      app VM name (default campfire).
# SNAPSHOT_REGION   region for the resource policy (default us-central1).
# SNAPSHOT_POLICY   policy name (default smartfire-nightly-boot-disk).
# SNAPSHOT_START    daily start time UTC HH:MM (default 08:00).
# SNAPSHOT_RETENTION_DAYS  snapshot retention (default 14).

set -euo pipefail

APP_PROJECT_ID="${APP_PROJECT_ID:-}"
APP_ZONE="${APP_ZONE:-us-central1-a}"
APP_INSTANCE="${APP_INSTANCE:-campfire}"
SNAPSHOT_REGION="${SNAPSHOT_REGION:-us-central1}"
SNAPSHOT_POLICY="${SNAPSHOT_POLICY:-smartfire-nightly-boot-disk}"
SNAPSHOT_START="${SNAPSHOT_START:-08:00}"
SNAPSHOT_RETENTION_DAYS="${SNAPSHOT_RETENTION_DAYS:-14}"

log() { printf '[snapshot-schedule] %s\n' "$*"; }
die() { printf '[snapshot-schedule] ERROR: %s\n' "$*" >&2; exit 1; }

[ -n "$APP_PROJECT_ID" ] || die "APP_PROJECT_ID is required"
command -v gcloud >/dev/null || die "gcloud is not installed"
command -v jq >/dev/null || die "jq is not installed"
[[ "$SNAPSHOT_START" =~ ^[0-9]{2}:[0-9]{2}$ ]] || die "SNAPSHOT_START must look like HH:MM, not '$SNAPSHOT_START'"
[[ "$SNAPSHOT_RETENTION_DAYS" =~ ^[0-9]+$ ]] || die "SNAPSHOT_RETENTION_DAYS must be a number"

if gcloud compute resource-policies describe "$SNAPSHOT_POLICY" \
    --project="$APP_PROJECT_ID" --region="$SNAPSHOT_REGION" >/dev/null 2>&1; then
  log "resource policy $SNAPSHOT_POLICY already exists in $SNAPSHOT_REGION"
else
  log "creating snapshot schedule $SNAPSHOT_POLICY (daily $SNAPSHOT_START UTC, keep $SNAPSHOT_RETENTION_DAYS days)"
  gcloud compute resource-policies create snapshot-schedule "$SNAPSHOT_POLICY" \
    --project="$APP_PROJECT_ID" --region="$SNAPSHOT_REGION" \
    --description="Smartfire nightly boot-disk snapshots; see docs/backups.md" \
    --start-time="$SNAPSHOT_START" --daily-schedule \
    --max-retention-days="$SNAPSHOT_RETENTION_DAYS" \
    --on-source-disk-delete=keep-auto-snapshots
fi

boot_disk="$(gcloud compute instances describe "$APP_INSTANCE" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
  | jq -r '.disks[] | select(.boot == true) | .source')"
if [ -z "$boot_disk" ] || [ "$boot_disk" = "null" ]; then
  die "no boot disk found on $APP_INSTANCE"
fi
disk_name="${boot_disk##*/}"
log "boot disk of $APP_INSTANCE is $disk_name"

policy_url="projects/${APP_PROJECT_ID}/regions/${SNAPSHOT_REGION}/resourcePolicies/${SNAPSHOT_POLICY}"
if gcloud compute disks describe "$disk_name" \
    --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
    | jq -e --arg p "$policy_url" '.resourcePolicies[]? | contains($p)' >/dev/null 2>&1; then
  log "policy $SNAPSHOT_POLICY is already attached to $disk_name"
else
  log "attaching $SNAPSHOT_POLICY to $disk_name"
  gcloud compute disks add-resource-policies "$disk_name" \
    --project="$APP_PROJECT_ID" --zone="$APP_ZONE" \
    --resource-policies="$policy_url"
fi

non_boot="$(gcloud compute instances describe "$APP_INSTANCE" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
  | jq -r '[.disks[] | select(.boot != true)] | length')"
if [ "$non_boot" -gt 0 ]; then
  log "WARNING: $non_boot non-boot disk(s) are NOT covered by this schedule"
fi

log "done: $disk_name snapshots nightly at $SNAPSHOT_START UTC, kept $SNAPSHOT_RETENTION_DAYS days"

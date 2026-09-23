#!/usr/bin/env bash
#
# Creates the nightly boot-disk snapshot schedule for the app VM and attaches
# it to the instance's boot disk. Idempotent: existing policies and attachments
# are left alone. The lead runs this (directly, or through
# setup-backup-project.sh, which calls it).
#
# A disk holds only ONE schedule. When the boot disk already carries one --
# production's is default-schedule-1 (daily 14:00 UTC, 14-day retention, keep
# snapshots when the disk is deleted) -- this script reports its settings and
# stops instead of failing: the existing schedule already covers the disk.
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

# Reports one schedule already attached to the disk: name, region, start
# time, retention. Best effort throughout: an undescribable policy is still
# a reason to stop, just reported with less detail.
report_schedule() {
  local disk="$1" url="$2" name region project
  # URLs come back full or partial; both end in
  # projects/<project>/regions/<region>/resourcePolicies/<name>.
  name="${url##*/}"
  region="$(printf '%s' "$url" | sed -n 's|.*/regions/\([^/]*\)/resourcePolicies/.*|\1|p')"
  project="$(printf '%s' "$url" | sed -n 's|.*/projects/\([^/]*\)/regions/.*|\1|p')"
  if [ -z "$region" ] || [ -z "$project" ] || [ -z "$name" ]; then
    log "existing schedule on $disk: $url (could not parse it; leaving it alone)"
    return 0
  fi
  local policy_json start retention on_delete
  if ! policy_json="$(gcloud compute resource-policies describe "$name" \
      --project="$project" --region="$region" --format=json 2>/dev/null)"; then
    log "existing schedule on $disk: $name in $region (could not describe it; leaving it alone)"
    return 0
  fi
  start="$(printf '%s' "$policy_json" | jq -r '.snapshotSchedulePolicy.schedule.dailySchedule.startTime // "unknown"')"
  retention="$(printf '%s' "$policy_json" | jq -r '.snapshotSchedulePolicy.retentionPolicy.maxRetentionDays // "unknown"')"
  on_delete="$(printf '%s' "$policy_json" | jq -r '.snapshotSchedulePolicy.retentionPolicy.onSourceDiskDelete // "unknown"')"
  log "existing schedule on $disk: $name in $region (daily $start UTC, keep $retention days, on-source-disk-delete $on_delete); leaving it alone"
}

boot_disk="$(gcloud compute instances describe "$APP_INSTANCE" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
  | jq -r '.disks[] | select(.boot == true) | .source')"
if [ -z "$boot_disk" ] || [ "$boot_disk" = "null" ]; then
  die "no boot disk found on $APP_INSTANCE"
fi
disk_name="${boot_disk##*/}"
log "boot disk of $APP_INSTANCE is $disk_name"

disk_json="$(gcloud compute disks describe "$disk_name" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json)"
policy_url="projects/${APP_PROJECT_ID}/regions/${SNAPSHOT_REGION}/resourcePolicies/${SNAPSHOT_POLICY}"
skipped=0
if printf '%s' "$disk_json" \
    | jq -e --arg p "$policy_url" '.resourcePolicies[]? | contains($p)' >/dev/null 2>&1; then
  log "policy $SNAPSHOT_POLICY is already attached to $disk_name"
else
  attached="$(printf '%s' "$disk_json" | jq -r '.resourcePolicies[]? // empty')"
  if [ -n "$attached" ]; then
    while IFS= read -r url; do
      [ -n "$url" ] || continue
      report_schedule "$disk_name" "$url"
    done <<<"$attached"
    log "a disk holds only one schedule; skipping $SNAPSHOT_POLICY"
    skipped=1
  else
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
    log "attaching $SNAPSHOT_POLICY to $disk_name"
    gcloud compute disks add-resource-policies "$disk_name" \
      --project="$APP_PROJECT_ID" --zone="$APP_ZONE" \
      --resource-policies="$policy_url"
  fi
fi

non_boot="$(gcloud compute instances describe "$APP_INSTANCE" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
  | jq -r '[.disks[] | select(.boot != true)] | length')"
if [ "$non_boot" -gt 0 ]; then
  log "WARNING: $non_boot non-boot disk(s) are NOT covered by this schedule"
fi

if [ "$skipped" = "1" ]; then
  log "done: left the existing schedule on $disk_name in place (no change needed)"
else
  log "done: $disk_name snapshots nightly at $SNAPSHOT_START UTC, kept $SNAPSHOT_RETENTION_DAYS days"
fi

#!/usr/bin/env bash
#
# One-time setup for rolling daily backups. The LEAD runs this from a machine
# with gcloud authenticated as a backup-project admin. It is idempotent: every
# resource is described first and created only when missing, so re-running it
# converges without duplicating anything. It never touches the live app.
#
# What it creates, all in the SEPARATE backup project:
#   1. the backup bucket (uniform access, versioning, 30-day soft delete,
#      lifecycle from lifecycle.json, public access prevention);
#   2. smartfire-backup-writer: the service account the VM uploads with,
#      granted ONLY roles/storage.objectCreator on that bucket;
#   3. smartfire-backup-reader + a Workload Identity pool/provider so the
#      monthly restore-check workflow can read (never write or delete);
#   4. the nightly boot-disk snapshot schedule on the app VM (delegated to
#      snapshot-schedule.sh, which lives in the app project by necessity).
#
# Project creation and billing linking are NOT done here: when BACKUP_PROJECT_ID
# does not exist the script prints the manual commands and stops.
#
# --- inputs ---------------------------------------------------------------
# BACKUP_PROJECT_ID   required. The separate backup project (must exist).
# BACKUP_BUCKET       required. Bare bucket name, without gs://.
# BACKUP_LOCATION     bucket location (default US-CENTRAL1).
# BACKUP_WRITER_SA    writer SA short name (default smartfire-backup-writer).
# BACKUP_READER_SA    reader SA short name (default smartfire-backup-reader).
# BACKUP_WIF_POOL     Workload Identity pool id (default smartfire-backup-pool).
# BACKUP_WIF_PROVIDER pool provider id (default github-actions).
# GITHUB_REPO         owner/repo allowed to impersonate the reader
#                     (default Smart-Data-Ohio/smartfire).
# APP_PROJECT_ID      app project, for the snapshot schedule (default
#                     smart-data-campfire).
# APP_ZONE / APP_INSTANCE  app VM location (defaults us-central1-a/campfire).
# LIFECYCLE_FILE      lifecycle JSON (default: lifecycle.json next to this
#                     script).
#
# At the end it prints the values the lead must put in GitHub variables/
# secrets and in /etc/campfire-backups/backup.env on the VM.

set -euo pipefail

BACKUP_PROJECT_ID="${BACKUP_PROJECT_ID:-}"
BACKUP_BUCKET="${BACKUP_BUCKET:-}"
BACKUP_LOCATION="${BACKUP_LOCATION:-US-CENTRAL1}"
BACKUP_WRITER_SA="${BACKUP_WRITER_SA:-smartfire-backup-writer}"
BACKUP_READER_SA="${BACKUP_READER_SA:-smartfire-backup-reader}"
BACKUP_WIF_POOL="${BACKUP_WIF_POOL:-smartfire-backup-pool}"
BACKUP_WIF_PROVIDER="${BACKUP_WIF_PROVIDER:-github-actions}"
GITHUB_REPO="${GITHUB_REPO:-Smart-Data-Ohio/smartfire}"
APP_PROJECT_ID="${APP_PROJECT_ID:-smart-data-campfire}"
APP_ZONE="${APP_ZONE:-us-central1-a}"
APP_INSTANCE="${APP_INSTANCE:-campfire}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIFECYCLE_FILE="${LIFECYCLE_FILE:-$SCRIPT_DIR/lifecycle.json}"

log() { printf '[setup-backup-project] %s\n' "$*"; }
die() { printf '[setup-backup-project] ERROR: %s\n' "$*" >&2; exit 1; }

[ -n "$BACKUP_PROJECT_ID" ] || die "BACKUP_PROJECT_ID is required"
[ -n "$BACKUP_BUCKET" ] || die "BACKUP_BUCKET is required"
case "$BACKUP_BUCKET" in
  gs://*|*/*) die "BACKUP_BUCKET must be a bare bucket name, not '$BACKUP_BUCKET'" ;;
esac
command -v gcloud >/dev/null || die "gcloud is not installed"
command -v jq >/dev/null || die "jq is not installed"
[ -f "$LIFECYCLE_FILE" ] || die "lifecycle file not found: $LIFECYCLE_FILE"

BUCKET_URL="gs://${BACKUP_BUCKET}"
WRITER_EMAIL="${BACKUP_WRITER_SA}@${BACKUP_PROJECT_ID}.iam.gserviceaccount.com"
READER_EMAIL="${BACKUP_READER_SA}@${BACKUP_PROJECT_ID}.iam.gserviceaccount.com"

# --- 0. the project must already exist --------------------------------------
if ! gcloud projects describe "$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
  cat >&2 <<EOF
[setup-backup-project] ERROR: project $BACKUP_PROJECT_ID does not exist or is not visible.
Create it and link billing by hand first; this script never does that silently:

  gcloud projects create $BACKUP_PROJECT_ID --name="Smartfire backups"
  # link billing in the Cloud console (Billing > Link a billing account),
  # or: gcloud billing projects link $BACKUP_PROJECT_ID --billing-account=BILLING_ACCOUNT_ID
  gcloud services enable storage.googleapis.com compute.googleapis.com \\
    iam.googleapis.com iamcredentials.googleapis.com sts.googleapis.com \\
    --project=$BACKUP_PROJECT_ID

Then re-run this script.
EOF
  exit 1
fi
log "project $BACKUP_PROJECT_ID exists"

log "enabling the required APIs (idempotent)"
gcloud services enable storage.googleapis.com compute.googleapis.com \
  iam.googleapis.com iamcredentials.googleapis.com sts.googleapis.com \
  --project="$BACKUP_PROJECT_ID" --quiet

# --- 1. the bucket -----------------------------------------------------------
if gcloud storage buckets describe "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
  log "bucket $BUCKET_URL already exists"
else
  log "creating bucket $BUCKET_URL in $BACKUP_LOCATION"
  gcloud storage buckets create "$BUCKET_URL" \
    --project="$BACKUP_PROJECT_ID" --location="$BACKUP_LOCATION" \
    --uniform-bucket-level-access --public-access-prevention
fi

log "applying bucket settings: versioning, 30-day soft delete, lifecycle"
gcloud storage buckets update "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" \
  --versioning --soft-delete-duration=30d --lifecycle-file="$LIFECYCLE_FILE"

# --- 2. the writer identity ---------------------------------------------------
# A dedicated SA in the BACKUP project, granted objectCreator on this bucket
# and nothing else. Even if the app project (or the VM) is fully compromised,
# the attacker gains no delete, overwrite or read on the backups.
if gcloud iam service-accounts describe "$WRITER_EMAIL" --project="$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
  log "service account $WRITER_EMAIL already exists"
else
  log "creating service account $WRITER_EMAIL"
  gcloud iam service-accounts create "$BACKUP_WRITER_SA" \
    --project="$BACKUP_PROJECT_ID" \
    --description="Smartfire nightly backup uploader (create-only); see docs/backups.md" \
    --display-name="Smartfire backup writer"
fi

if gcloud storage buckets get-iam-policy "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "serviceAccount:$WRITER_EMAIL" \
      '.bindings[] | select(.role == "roles/storage.objectCreator") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "writer already holds roles/storage.objectCreator on $BUCKET_URL"
else
  log "granting roles/storage.objectCreator on $BUCKET_URL to $WRITER_EMAIL"
  gcloud storage buckets add-iam-policy-binding "$BUCKET_URL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="serviceAccount:$WRITER_EMAIL" --role="roles/storage.objectCreator"
fi

# --- 3. the reader identity for the monthly restore check --------------------
if gcloud iam service-accounts describe "$READER_EMAIL" --project="$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
  log "service account $READER_EMAIL already exists"
else
  log "creating service account $READER_EMAIL"
  gcloud iam service-accounts create "$BACKUP_READER_SA" \
    --project="$BACKUP_PROJECT_ID" \
    --description="Smartfire monthly restore check (read-only); see docs/backups.md" \
    --display-name="Smartfire backup reader"
fi

if gcloud storage buckets get-iam-policy "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "serviceAccount:$READER_EMAIL" \
      '.bindings[] | select(.role == "roles/storage.objectViewer") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "reader already holds roles/storage.objectViewer on $BUCKET_URL"
else
  log "granting roles/storage.objectViewer on $BUCKET_URL to $READER_EMAIL"
  gcloud storage buckets add-iam-policy-binding "$BUCKET_URL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="serviceAccount:$READER_EMAIL" --role="roles/storage.objectViewer"
fi

if gcloud iam workload-identity-pools describe "$BACKUP_WIF_POOL" \
    --project="$BACKUP_PROJECT_ID" --location=global >/dev/null 2>&1; then
  log "Workload Identity pool $BACKUP_WIF_POOL already exists"
else
  log "creating Workload Identity pool $BACKUP_WIF_POOL"
  gcloud iam workload-identity-pools create "$BACKUP_WIF_POOL" \
    --project="$BACKUP_PROJECT_ID" --location=global \
    --display-name="Smartfire backups" \
    --description="GitHub Actions access to backup-project readers only"
fi

if gcloud iam workload-identity-pools providers describe "$BACKUP_WIF_PROVIDER" \
    --project="$BACKUP_PROJECT_ID" --location=global \
    --workload-identity-pool="$BACKUP_WIF_POOL" >/dev/null 2>&1; then
  log "Workload Identity provider $BACKUP_WIF_PROVIDER already exists"
else
  log "creating Workload Identity provider $BACKUP_WIF_PROVIDER"
  gcloud iam workload-identity-pools providers create-oidc "$BACKUP_WIF_PROVIDER" \
    --project="$BACKUP_PROJECT_ID" --location=global \
    --workload-identity-pool="$BACKUP_WIF_POOL" \
    --issuer-uri="https://token.actions.githubusercontent.com" \
    --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
    --attribute-condition="assertion.repository == '$GITHUB_REPO'"
fi

PROJECT_NUMBER="$(gcloud projects describe "$BACKUP_PROJECT_ID" --format='value(projectNumber)')"
WIF_PROVIDER="projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/providers/${BACKUP_WIF_PROVIDER}"
PRINCIPAL="principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/attribute.repository/${GITHUB_REPO}"
if gcloud iam service-accounts get-iam-policy "$READER_EMAIL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "$PRINCIPAL" \
      '.bindings[] | select(.role == "roles/iam.workloadIdentityUser") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "GitHub repo $GITHUB_REPO may already impersonate $READER_EMAIL"
else
  log "allowing $GITHUB_REPO to impersonate $READER_EMAIL"
  gcloud iam service-accounts add-iam-policy-binding "$READER_EMAIL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="$PRINCIPAL" --role="roles/iam.workloadIdentityUser"
fi

# --- 4. the nightly snapshot schedule (lives in the app project) -------------
log "creating the nightly snapshot schedule (delegated to snapshot-schedule.sh)"
APP_PROJECT_ID="$APP_PROJECT_ID" APP_ZONE="$APP_ZONE" APP_INSTANCE="$APP_INSTANCE" \
  "$SCRIPT_DIR/snapshot-schedule.sh"

# --- 5. what the lead must still do by hand -----------------------------------
cat <<EOF

=====================================================================
Backup infrastructure is ready. Remaining manual steps for the lead:
=====================================================================

1. Generate the age keypair OFF the VM and OFF any shared machine:

     age-keygen -o smartfire-backup-key.txt   # keep this file sealed

   The public recipient (the age1... line) goes in /etc/campfire-backups/
   backup.env on the VM as BACKUP_AGE_RECIPIENT. The PRIVATE key goes ONLY
   in the GitHub secret BACKUP_RESTORE_AGE_IDENTITY (step 4) and in the
   sealed ops store. Never copy it to the VM.

2. Attach the writer SA to the app VM (brief downtime: the VM must be
   stopped to change its service account):

     gcloud compute instances stop $APP_INSTANCE --zone=$APP_ZONE \\
       --project=$APP_PROJECT_ID
     gcloud compute instances set-service-account $APP_INSTANCE \\
       --zone=$APP_ZONE --project=$APP_PROJECT_ID \\
       --service-account=$WRITER_EMAIL --scopes=cloud-platform
     gcloud compute instances start $APP_INSTANCE --zone=$APP_ZONE \\
       --project=$APP_PROJECT_ID

   The cloud-platform scope is the coarse gate; the IAM binding above is
   what actually limits the VM to creating objects in $BUCKET_URL.

3. On the VM, install the timer (idempotent) and configure it:

     sudo deploy/backups/install-backup.sh
     sudoedit /etc/campfire-backups/backup.env   # BACKUP_BUCKET=$BACKUP_BUCKET
     sudo apt-get install age                    # if the installer warned
     sudo systemctl start campfire-backup.service
     sudo journalctl -u campfire-backup.service --since '10 min ago'

4. GitHub repository variables (Settings > Secrets and variables > Actions):

     BACKUP_GCP_PROJECT_ID       $BACKUP_PROJECT_ID
     BACKUP_GCP_BUCKET           $BACKUP_BUCKET
     BACKUP_GCP_WIF_PROVIDER     $WIF_PROVIDER
     BACKUP_GCP_READER_SA        $READER_EMAIL

   GitHub repository secret:

     BACKUP_RESTORE_AGE_IDENTITY   <contents of smartfire-backup-key.txt>

5. Run the "Monthly backup restore check" workflow by hand once (Actions >
   Monthly backup restore check > Run workflow) to prove the whole chain.
=====================================================================
EOF

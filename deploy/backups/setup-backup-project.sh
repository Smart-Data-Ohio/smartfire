#!/usr/bin/env bash
#
# One-time setup for rolling daily backups. The LEAD runs this from a machine
# with gcloud authenticated as an admin of the backup project (and a viewer of
# the app project, for the VM lookup). It is idempotent: every resource is
# described first and created only when missing, so re-running it converges
# without duplicating anything. It never touches the live app, never changes
# the VM, and never changes access scopes.
#
# What it creates, all in the SEPARATE backup project:
#   1. the backup bucket (uniform access, versioning, 30-day soft delete,
#      lifecycle from lifecycle.json, public access prevention);
#   2. a bucket-level roles/storage.objectCreator grant for the VM's EXISTING
#      service account (a cross-project binding on the bucket itself, so no
#      new identity is attached to the VM and the VM is never stopped).
#      Only when the VM has no service account, or its scopes cannot write to
#      Cloud Storage, the script instead creates smartfire-backup-writer in
#      the backup project, grants THAT, prints the one-time attach steps, and
#      exits 1: re-run it after attaching and it converges to exit 0.
#   3. smartfire-backup-reader + a Workload Identity pool/provider so the
#      monthly restore-check workflow can read (never write or delete). The
#      impersonation binding pins the exact ID-qualified subject GitHub mints
#      for this repository's main branch, matching the deployer's bindings
#      (see deploy/gcp/README.md#authentication).
#   4. the nightly boot-disk snapshot schedule on the app VM (delegated to
#      snapshot-schedule.sh, which lives in the app project by necessity).
#
# Project creation and billing linking are NOT done here: when BACKUP_PROJECT_ID
# does not exist the script prints the manual commands and stops.
#
# --- inputs ---------------------------------------------------------------
# BACKUP_PROJECT_ID   the separate backup project (default
#                     smart-data-campfire-backups; must already exist).
# BACKUP_BUCKET       required. Bare bucket name, without gs://.
# BACKUP_LOCATION     bucket location (default US-CENTRAL1).
# BACKUP_WRITER_SA    fallback writer SA short name, only created when the VM
#                     cannot upload as it stands (default
#                     smartfire-backup-writer).
# BACKUP_READER_SA    reader SA short name (default smartfire-backup-reader).
# BACKUP_WIF_POOL     Workload Identity pool id (default smartfire-backup-pool).
# BACKUP_WIF_PROVIDER pool provider id (default github-actions).
# GITHUB_REPO         owner/repo allowed to impersonate the reader
#                     (default Smart-Data-Ohio/smartfire).
# GITHUB_OWNER_ID     numeric GitHub id of the owner, for the ID-qualified
#                     subject (default 262436228).
# GITHUB_REPO_ID      numeric GitHub id of the repo, for the ID-qualified
#                     subject (default 1370426325).
# GITHUB_REF          git ref whose runs may impersonate the reader; the bound
#                     subject is repo:<owner>@<owner-id>/<repo>@<repo-id>:
#                     ref:<ref> (default refs/heads/main: scheduled runs always
#                     use the default branch, so dispatch by hand from main).
# APP_PROJECT_ID      app project, for the VM lookup and snapshot schedule
#                     (default smart-data-campfire).
# APP_ZONE / APP_INSTANCE  app VM location (defaults us-central1-a/campfire).
# LIFECYCLE_FILE      lifecycle JSON (default: lifecycle.json next to this
#                     script).
#
# At the end it prints the values the lead must put in GitHub variables/
# secrets and in /etc/campfire-backups/backup.env on the VM.

set -euo pipefail

BACKUP_PROJECT_ID="${BACKUP_PROJECT_ID:-smart-data-campfire-backups}"
BACKUP_BUCKET="${BACKUP_BUCKET:-}"
BACKUP_LOCATION="${BACKUP_LOCATION:-US-CENTRAL1}"
BACKUP_WRITER_SA="${BACKUP_WRITER_SA:-smartfire-backup-writer}"
BACKUP_READER_SA="${BACKUP_READER_SA:-smartfire-backup-reader}"
BACKUP_WIF_POOL="${BACKUP_WIF_POOL:-smartfire-backup-pool}"
BACKUP_WIF_PROVIDER="${BACKUP_WIF_PROVIDER:-github-actions}"
GITHUB_REPO="${GITHUB_REPO:-Smart-Data-Ohio/smartfire}"
GITHUB_OWNER_ID="${GITHUB_OWNER_ID:-262436228}"
GITHUB_REPO_ID="${GITHUB_REPO_ID:-1370426325}"
GITHUB_REF="${GITHUB_REF:-refs/heads/main}"
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

# --- 2. the uploader identity: the VM's own service account ------------------
# No new identity is attached to the VM and the VM is never stopped: IAM
# allows a cross-project grant on the bucket itself. Two preconditions, both
# only ever READ here, never changed: the VM must HAVE a service account, and
# its access scopes must allow Cloud Storage writes. When either is missing,
# the fallback below stages a dedicated writer SA and this script exits 1
# with the one-time attach steps; attaching any service account (or widening
# scopes) needs a VM stop, and that stays a manual, visible decision.
vm_json="$(gcloud compute instances describe "$APP_INSTANCE" \
  --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json)"
VM_SA_EMAIL="$(printf '%s' "$vm_json" | jq -r '.serviceAccounts[0].email // empty')"
vm_scopes="$(printf '%s' "$vm_json" | jq -r '[.serviceAccounts[0].scopes[]? | split("/") | last] | join(" ")')"
vm_scopes_ok=0
case " $vm_scopes " in
  *" cloud-platform "*|*" devstorage.read_write "*|*" devstorage.full_control "*) vm_scopes_ok=1 ;;
esac

VM_NEEDS_ATTACH=0
VM_SHORTFALL=""
if [ -n "$VM_SA_EMAIL" ] && [ "$vm_scopes_ok" = "1" ]; then
  UPLOAD_MEMBER="serviceAccount:$VM_SA_EMAIL"
  log "the VM already runs as $VM_SA_EMAIL with Cloud Storage write scopes; no VM change is needed"
else
  if [ -z "$VM_SA_EMAIL" ]; then
    VM_SHORTFALL="it has no service account"
  else
    VM_SHORTFALL="its service account $VM_SA_EMAIL has scopes ($vm_scopes) that cannot write to Cloud Storage"
  fi
  log "the VM cannot upload as it stands ($VM_SHORTFALL); staging the attach-once fallback"
  if gcloud iam service-accounts describe "$WRITER_EMAIL" --project="$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
    log "service account $WRITER_EMAIL already exists"
  else
    log "creating service account $WRITER_EMAIL"
    gcloud iam service-accounts create "$BACKUP_WRITER_SA" \
      --project="$BACKUP_PROJECT_ID" \
      --description="Smartfire nightly backup uploader (create-only); see docs/backups.md" \
      --display-name="Smartfire backup writer"
  fi
  UPLOAD_MEMBER="serviceAccount:$WRITER_EMAIL"
  VM_NEEDS_ATTACH=1
fi

# Whoever uploads holds ONLY roles/storage.objectCreator on this bucket: it
# can create objects but can neither read, delete nor overwrite them. Even a
# fully compromised app VM therefore cannot harm existing backups.
if gcloud storage buckets get-iam-policy "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "$UPLOAD_MEMBER" \
      '.bindings[] | select(.role == "roles/storage.objectCreator") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "$UPLOAD_MEMBER already holds roles/storage.objectCreator on $BUCKET_URL"
else
  log "granting roles/storage.objectCreator on $BUCKET_URL to $UPLOAD_MEMBER"
  gcloud storage buckets add-iam-policy-binding "$BUCKET_URL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="$UPLOAD_MEMBER" --role="roles/storage.objectCreator"
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

# The impersonation binding pins the exact ID-qualified subject GitHub mints
# for runs of this repo on GITHUB_REF, the same form as the deployer's
# per-environment bindings. The provider-level attribute condition above is
# the outer gate (assertion.repository is never ID-qualified); this subject
# is the inner one. Dispatch the workflow by hand from main: a run from any
# other ref mints a different subject and is denied.
PROJECT_NUMBER="$(gcloud projects describe "$BACKUP_PROJECT_ID" --format='value(projectNumber)')"
WIF_PROVIDER="projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/providers/${BACKUP_WIF_PROVIDER}"
GITHUB_SUBJECT="repo:${GITHUB_REPO%%/*}@${GITHUB_OWNER_ID}/${GITHUB_REPO##*/}@${GITHUB_REPO_ID}:ref:${GITHUB_REF}"
PRINCIPAL="principal://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/subject/${GITHUB_SUBJECT}"
if gcloud iam service-accounts get-iam-policy "$READER_EMAIL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "$PRINCIPAL" \
      '.bindings[] | select(.role == "roles/iam.workloadIdentityUser") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "$GITHUB_SUBJECT may already impersonate $READER_EMAIL"
else
  log "allowing $GITHUB_SUBJECT to impersonate $READER_EMAIL"
  gcloud iam service-accounts add-iam-policy-binding "$READER_EMAIL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="$PRINCIPAL" --role="roles/iam.workloadIdentityUser"
fi

# --- 4. the nightly snapshot schedule (lives in the app project) -------------
log "creating the nightly snapshot schedule (delegated to snapshot-schedule.sh)"
APP_PROJECT_ID="$APP_PROJECT_ID" APP_ZONE="$APP_ZONE" APP_INSTANCE="$APP_INSTANCE" \
  "$SCRIPT_DIR/snapshot-schedule.sh"

# --- 5. what the lead must still do by hand -----------------------------------
if [ "$VM_NEEDS_ATTACH" = "1" ]; then
  cat <<EOF

=====================================================================
Backup infrastructure is ALMOST ready: the VM cannot upload yet.
$APP_INSTANCE ($APP_PROJECT_ID/$APP_ZONE): $VM_SHORTFALL.
Attach the uploader below, then re-run this script: it converges to exit 0.
=====================================================================

--- attach the uploader to the app VM (one brief stop) ---

Attaching a service account (like widening scopes) needs the VM stopped;
that stays a manual step, never something this script does silently:

  gcloud compute instances stop $APP_INSTANCE --zone=$APP_ZONE \\
    --project=$APP_PROJECT_ID
  gcloud compute instances set-service-account $APP_INSTANCE \\
    --zone=$APP_ZONE --project=$APP_PROJECT_ID \\
    --service-account=$WRITER_EMAIL --scopes=cloud-platform
  gcloud compute instances start $APP_INSTANCE --zone=$APP_ZONE \\
    --project=$APP_PROJECT_ID

The cloud-platform scope is the coarse gate; the IAM binding above is what
actually limits the VM to creating objects in $BUCKET_URL. After the
attach, re-run this script before continuing below.
EOF
else
  cat <<EOF

=====================================================================
Backup infrastructure is ready. No VM identity change was needed.
=====================================================================
EOF
fi

cat <<EOF

--- age keypair (generate OFF the VM and OFF any shared machine) ---

  age-keygen -o smartfire-backup-key.txt   # keep this file sealed

The public recipient (the age1... line) goes in /etc/campfire-backups/
backup.env on the VM as BACKUP_AGE_RECIPIENT. The PRIVATE key goes ONLY
in the GitHub secret BACKUP_RESTORE_AGE_IDENTITY (below) and in the
sealed ops store. Never copy it to the VM.

--- install the timer on the VM (idempotent) ---

  sudo deploy/backups/install-backup.sh
  sudoedit /etc/campfire-backups/backup.env   # BACKUP_BUCKET=$BACKUP_BUCKET
  sudo systemctl start campfire-backup.service
  sudo journalctl -u campfire-backup.service --since '10 min ago'

The installer installs age itself when the configured encryption needs it.

--- GitHub repository variables (Settings > Secrets and variables > Actions) ---

  BACKUP_GCP_PROJECT_ID       $BACKUP_PROJECT_ID
  BACKUP_GCP_BUCKET           $BACKUP_BUCKET
  BACKUP_GCP_WIF_PROVIDER     $WIF_PROVIDER
  BACKUP_GCP_READER_SA        $READER_EMAIL

GitHub repository secret:

  BACKUP_RESTORE_AGE_IDENTITY   <contents of smartfire-backup-key.txt>

--- prove the whole chain once by hand ---

Run the "Monthly backup restore check" workflow once (Actions > Monthly
backup restore check > Run workflow) from the main branch: the Workload
Identity binding only trusts $GITHUB_SUBJECT.
=====================================================================
EOF

if [ "$VM_NEEDS_ATTACH" = "1" ]; then
  die "stopping here: attach $WRITER_EMAIL to $APP_INSTANCE and re-run this script"
fi

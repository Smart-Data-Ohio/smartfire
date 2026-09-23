#!/usr/bin/env bash
#
# One-time setup for rolling daily backups. The LEAD runs this from a machine
# with gcloud authenticated as an admin of the backup project (and enough
# access in the app project to manage service accounts, IAM bindings and
# Workload Identity Federation). It is idempotent: every resource is described
# first and created only when missing, so re-running it converges without
# duplicating anything. It never touches the live app, never changes the VM,
# and never attaches anything to the VM: the VM has no service account and
# needs none, because the nightly GitHub Actions workflow (not the VM) is what
# uploads to Cloud Storage.
#
# What it creates:
#   1. in the SEPARATE backup project: the backup bucket (US multi-region,
#      uniform access, versioning, 30-day soft delete, lifecycle from
#      lifecycle.json, public access prevention);
#   2. in the APP project: smartfire-backup-runner, the identity the nightly
#      workflow impersonates. It holds roles/storage.objectCreator on the
#      backup bucket (a cross-project binding on the bucket itself: create
#      objects, but neither read, delete nor overwrite them) and, ON THE
#      campfire VM ONLY, the IAP-tunnel and SSH roles it needs to run the
#      backup script: roles/iap.tunnelResourceAccessor,
#      roles/compute.osAdminLogin (OS Login with sudo, as root must run the
#      backup) and roles/compute.viewer (so ssh/scp can resolve the VM;
#      read-only metadata, no deploy rights). A Workload Identity pool/
#      provider in the app project trusts the nightly workflow's
#      ID-qualified subject for refs/heads/main, the same form the restore
#      check uses;
#   3. in the backup project: smartfire-backup-reader + a Workload Identity
#      pool/provider so the monthly restore-check workflow can read (never
#      write or delete). The impersonation binding pins the exact
#      ID-qualified subject GitHub mints for this repository's main branch,
#      matching the deployer's bindings (see deploy/gcp/README.md#authentication);
#   4. the nightly boot-disk snapshot schedule on the app VM (delegated to
#      snapshot-schedule.sh, which lives in the app project by necessity and
#      skips cleanly when the disk already has a schedule).
#
# Project creation and billing linking are NOT done here: when BACKUP_PROJECT_ID
# does not exist the script prints the manual commands and stops.
#
# --- inputs ---------------------------------------------------------------
# BACKUP_PROJECT_ID   the separate backup project (default
#                     smart-data-campfire-backups; must already exist).
# BACKUP_BUCKET       required. Bare bucket name, without gs://.
# BACKUP_LOCATION     bucket location (default US, the US multi-region: the
#                     backups must survive the loss of the app region).
# BACKUP_RUNNER_SA    backup-runner SA short name, created in the APP
#                     project (default smartfire-backup-runner).
# BACKUP_READER_SA    reader SA short name, created in the backup project
#                     (default smartfire-backup-reader).
# BACKUP_WIF_POOL     Workload Identity pool id, created in each project
#                     (default smartfire-backup-pool).
# BACKUP_WIF_PROVIDER pool provider id (default github-actions).
# GITHUB_REPO         owner/repo allowed to impersonate the SAs
#                     (default Smart-Data-Ohio/smartfire).
# GITHUB_OWNER_ID     numeric GitHub id of the owner, for the ID-qualified
#                     subject (default 262436228).
# GITHUB_REPO_ID      numeric GitHub id of the repo, for the ID-qualified
#                     subject (default 1370426325).
# GITHUB_REF          git ref whose runs may impersonate the SAs; the bound
#                     subject is repo:<owner>@<owner-id>/<repo>@<repo-id>:
#                     ref:<ref> (default refs/heads/main: scheduled runs always
#                     use the default branch, so dispatch by hand from main).
# APP_PROJECT_ID      app project, holding the VM and the backup runner
#                     (default smart-data-campfire).
# APP_ZONE / APP_INSTANCE  app VM location (defaults us-central1-a/campfire).
# LIFECYCLE_FILE      lifecycle JSON (default: lifecycle.json next to this
#                     script).
#
# At the end it prints the values the lead must put in GitHub variables/
# secrets. Nothing is installed on the VM: the nightly workflow copies the
# backup script to the VM on every run.

set -euo pipefail

BACKUP_PROJECT_ID="${BACKUP_PROJECT_ID:-smart-data-campfire-backups}"
BACKUP_BUCKET="${BACKUP_BUCKET:-}"
BACKUP_LOCATION="${BACKUP_LOCATION:-US}"
BACKUP_RUNNER_SA="${BACKUP_RUNNER_SA:-smartfire-backup-runner}"
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
RUNNER_EMAIL="${BACKUP_RUNNER_SA}@${APP_PROJECT_ID}.iam.gserviceaccount.com"
READER_EMAIL="${BACKUP_READER_SA}@${BACKUP_PROJECT_ID}.iam.gserviceaccount.com"

# --- 0. the projects must already exist -------------------------------------
if ! gcloud projects describe "$BACKUP_PROJECT_ID" >/dev/null 2>&1; then
  cat >&2 <<EOF
[setup-backup-project] ERROR: project $BACKUP_PROJECT_ID does not exist or is not visible.
Create it and link billing by hand first; this script never does that silently:

  gcloud projects create $BACKUP_PROJECT_ID --name="Smartfire backups"
  # link billing in the Cloud console (Billing > Link a billing account),
  # or: gcloud billing projects link $BACKUP_PROJECT_ID --billing-account=BILLING_ACCOUNT_ID
  gcloud services enable storage.googleapis.com \\
    iam.googleapis.com iamcredentials.googleapis.com sts.googleapis.com \\
    --project=$BACKUP_PROJECT_ID

Then re-run this script.
EOF
  exit 1
fi
log "project $BACKUP_PROJECT_ID exists"

gcloud projects describe "$APP_PROJECT_ID" >/dev/null 2>&1 \
  || die "project $APP_PROJECT_ID does not exist or is not visible; the backup runner lives there"
log "project $APP_PROJECT_ID exists"

log "enabling the required APIs (idempotent)"
gcloud services enable storage.googleapis.com \
  iam.googleapis.com iamcredentials.googleapis.com sts.googleapis.com \
  --project="$BACKUP_PROJECT_ID" --quiet
gcloud services enable iam.googleapis.com iamcredentials.googleapis.com sts.googleapis.com \
  --project="$APP_PROJECT_ID" --quiet

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

# --- helpers -----------------------------------------------------------------
ensure_service_account() {
  local name="$1" project="$2" description="$3" display="$4"
  local email="${name}@${project}.iam.gserviceaccount.com"
  if gcloud iam service-accounts describe "$email" --project="$project" >/dev/null 2>&1; then
    log "service account $email already exists"
  else
    log "creating service account $email"
    gcloud iam service-accounts create "$name" \
      --project="$project" \
      --description="$description" \
      --display-name="$display"
  fi
}

ensure_wif_pool() {
  local project="$1"
  if gcloud iam workload-identity-pools describe "$BACKUP_WIF_POOL" \
      --project="$project" --location=global >/dev/null 2>&1; then
    log "Workload Identity pool $BACKUP_WIF_POOL already exists in $project"
  else
    log "creating Workload Identity pool $BACKUP_WIF_POOL in $project"
    gcloud iam workload-identity-pools create "$BACKUP_WIF_POOL" \
      --project="$project" --location=global \
      --display-name="Smartfire backups" \
      --description="GitHub Actions access to backup identities only"
  fi

  if gcloud iam workload-identity-pools providers describe "$BACKUP_WIF_PROVIDER" \
      --project="$project" --location=global \
      --workload-identity-pool="$BACKUP_WIF_POOL" >/dev/null 2>&1; then
    log "Workload Identity provider $BACKUP_WIF_PROVIDER already exists in $project"
  else
    log "creating Workload Identity provider $BACKUP_WIF_PROVIDER in $project"
    gcloud iam workload-identity-pools providers create-oidc "$BACKUP_WIF_PROVIDER" \
      --project="$project" --location=global \
      --workload-identity-pool="$BACKUP_WIF_POOL" \
      --issuer-uri="https://token.actions.githubusercontent.com" \
      --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
      --attribute-condition="assertion.repository == '$GITHUB_REPO'"
  fi
}

# The impersonation binding pins the exact ID-qualified subject GitHub mints
# for runs of this repo on GITHUB_REF, the same form as the deployer's
# per-environment bindings. The provider-level attribute condition above is
# the outer gate (assertion.repository is never ID-qualified); this subject
# is the inner one. Dispatch the workflows by hand from main: a run from any
# other ref mints a different subject and is denied.
#
# Sets WIF_PROVIDER_RESULT to the full provider resource name (a global,
# because log lines share stdout and cannot be captured apart).
WIF_PROVIDER_RESULT=""
allow_github_subject() {
  local email="$1" project="$2"
  local number provider principal subject
  number="$(gcloud projects describe "$project" --format='value(projectNumber)')"
  provider="projects/${number}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/providers/${BACKUP_WIF_PROVIDER}"
  subject="repo:${GITHUB_REPO%%/*}@${GITHUB_OWNER_ID}/${GITHUB_REPO##*/}@${GITHUB_REPO_ID}:ref:${GITHUB_REF}"
  principal="principal://iam.googleapis.com/projects/${number}/locations/global/workloadIdentityPools/${BACKUP_WIF_POOL}/subject/${subject}"
  if gcloud iam service-accounts get-iam-policy "$email" --project="$project" --format=json \
      | jq -e --arg m "$principal" \
        '.bindings[] | select(.role == "roles/iam.workloadIdentityUser") | .members[] | select(. == $m)' >/dev/null 2>&1; then
    log "$subject may already impersonate $email"
  else
    log "allowing $subject to impersonate $email"
    gcloud iam service-accounts add-iam-policy-binding "$email" \
      --project="$project" \
      --member="$principal" --role="roles/iam.workloadIdentityUser"
  fi
  WIF_PROVIDER_RESULT="$provider"
}

ensure_instance_binding() {
  local role="$1" member="$2"
  if gcloud compute instances get-iam-policy "$APP_INSTANCE" \
      --project="$APP_PROJECT_ID" --zone="$APP_ZONE" --format=json \
      | jq -e --arg m "$member" --arg r "$role" \
        '.bindings[] | select(.role == $r) | .members[] | select(. == $m)' >/dev/null 2>&1; then
    log "$member already holds $role on $APP_INSTANCE"
  else
    log "granting $role on $APP_INSTANCE to $member"
    gcloud compute instances add-iam-policy-binding "$APP_INSTANCE" \
      --project="$APP_PROJECT_ID" --zone="$APP_ZONE" \
      --member="$member" --role="$role"
  fi
}

# --- 2. the backup-runner identity (lives in the APP project) ----------------
# The nightly workflow impersonates this account: it runs the backup script
# on the VM over IAP SSH and uploads the encrypted archive. It deliberately
# holds no snapshot, deploy or Artifact Registry rights.
ensure_service_account "$BACKUP_RUNNER_SA" "$APP_PROJECT_ID" \
  "Smartfire nightly backup runner (create-only upload, VM-scoped SSH); see docs/backups.md" \
  "Smartfire backup runner"

# The runner holds ONLY roles/storage.objectCreator on this bucket: it can
# create objects but can neither read, delete nor overwrite them. Even a
# compromised workflow run therefore cannot harm existing backups.
if gcloud storage buckets get-iam-policy "$BUCKET_URL" --project="$BACKUP_PROJECT_ID" --format=json \
    | jq -e --arg m "serviceAccount:$RUNNER_EMAIL" \
      '.bindings[] | select(.role == "roles/storage.objectCreator") | .members[] | select(. == $m)' >/dev/null 2>&1; then
  log "serviceAccount:$RUNNER_EMAIL already holds roles/storage.objectCreator on $BUCKET_URL"
else
  log "granting roles/storage.objectCreator on $BUCKET_URL to $RUNNER_EMAIL"
  gcloud storage buckets add-iam-policy-binding "$BUCKET_URL" \
    --project="$BACKUP_PROJECT_ID" \
    --member="serviceAccount:$RUNNER_EMAIL" --role="roles/storage.objectCreator"
fi

# IAP tunnel + SSH, on the campfire VM only: the same shape the deployer
# uses to reach the VM, and no deploy rights.
ensure_instance_binding "roles/iap.tunnelResourceAccessor" "serviceAccount:$RUNNER_EMAIL"
ensure_instance_binding "roles/compute.osAdminLogin" "serviceAccount:$RUNNER_EMAIL"
ensure_instance_binding "roles/compute.viewer" "serviceAccount:$RUNNER_EMAIL"

ensure_wif_pool "$APP_PROJECT_ID"
allow_github_subject "$RUNNER_EMAIL" "$APP_PROJECT_ID"
RUNNER_WIF_PROVIDER="$WIF_PROVIDER_RESULT"

# --- 3. the reader identity for the monthly restore check --------------------
ensure_service_account "$BACKUP_READER_SA" "$BACKUP_PROJECT_ID" \
  "Smartfire monthly restore check (read-only); see docs/backups.md" \
  "Smartfire backup reader"

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

ensure_wif_pool "$BACKUP_PROJECT_ID"
allow_github_subject "$READER_EMAIL" "$BACKUP_PROJECT_ID"
READER_WIF_PROVIDER="$WIF_PROVIDER_RESULT"

GITHUB_SUBJECT="repo:${GITHUB_REPO%%/*}@${GITHUB_OWNER_ID}/${GITHUB_REPO##*/}@${GITHUB_REPO_ID}:ref:${GITHUB_REF}"

# --- 4. the nightly snapshot schedule (lives in the app project) -------------
log "checking the nightly snapshot schedule (delegated to snapshot-schedule.sh)"
APP_PROJECT_ID="$APP_PROJECT_ID" APP_ZONE="$APP_ZONE" APP_INSTANCE="$APP_INSTANCE" \
  "$SCRIPT_DIR/snapshot-schedule.sh"

# --- 5. what the lead must still do by hand -----------------------------------
cat <<EOF

=====================================================================
Backup infrastructure is ready. No VM change was needed: the VM has no
service account and needs none, and nothing is installed on it. The
nightly workflow copies the backup script to the VM on every run.
=====================================================================

--- age keypair (generate OFF the VM and OFF any shared machine) ---

  age-keygen -o smartfire-backup-key.txt   # keep this file sealed

The public recipient (the age1... line) goes in the GitHub variable
BACKUP_AGE_RECIPIENT below. The PRIVATE key goes ONLY in the GitHub
secret BACKUP_RESTORE_AGE_IDENTITY (below) and in the sealed ops store.
Never copy it to the VM.

--- GitHub repository variables (Settings > Secrets and variables > Actions) ---

Nightly backup (Actions > Nightly backup):

  BACKUP_GCP_BUCKET           $BACKUP_BUCKET
  BACKUP_APP_PROJECT          $APP_PROJECT_ID
  BACKUP_APP_ZONE             $APP_ZONE
  BACKUP_APP_INSTANCE         $APP_INSTANCE
  BACKUP_RUNNER_WIF_PROVIDER  $RUNNER_WIF_PROVIDER
  BACKUP_RUNNER_SA            $RUNNER_EMAIL
  BACKUP_AGE_RECIPIENT        <the age1... public recipient from above>

Monthly restore check (Actions > Monthly backup restore check):

  BACKUP_GCP_PROJECT_ID       $BACKUP_PROJECT_ID
  BACKUP_GCP_BUCKET           $BACKUP_BUCKET
  BACKUP_GCP_WIF_PROVIDER     $READER_WIF_PROVIDER
  BACKUP_GCP_READER_SA        $READER_EMAIL

GitHub repository secret:

  BACKUP_RESTORE_AGE_IDENTITY   <contents of smartfire-backup-key.txt>

--- prove the whole chain once by hand ---

Run the "Nightly backup" workflow once (Actions > Nightly backup > Run
workflow) from the main branch, then the "Monthly backup restore check"
the same way: the Workload Identity bindings only trust
$GITHUB_SUBJECT.
=====================================================================
EOF

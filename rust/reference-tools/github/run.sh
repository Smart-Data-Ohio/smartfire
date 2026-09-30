#!/usr/bin/env bash
# Run GitHub vectors/rollback verification in our pinned Rails image; no network requests.
set -euo pipefail
RUST_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TASK_ROOT=$(cd "$RUST_ROOT/.." && pwd)
SCRATCH="$TASK_ROOT/.scratch/ws15g"
mkdir -p "$SCRATCH"
MODE=${1:-generate}
case "$MODE" in generate|verify|webhooks|claims|fetch|actions|notifier|notifier_verify|references|domain|cards|card_http|health|subscriptions|connections|health_page|discussions) ;; *) echo "unknown GitHub oracle mode: $MODE" >&2; exit 2 ;; esac
python3 - "$TASK_ROOT" "$SCRATCH/reference-hashes.json" <<'PY'
import hashlib, json, pathlib, subprocess, sys
paths = ['app/controllers/github/pull_request_threads_controller.rb','app/models/integrations/fizzy_status.rb','app/models/google/client.rb','app/models/calendar/push_channel.rb','app/models/room.rb','app/controllers/concerns/sudo_mode.rb','app/controllers/github/connections_controller.rb','app/controllers/accounts/bots/github_connections_controller.rb','app/models/github/app.rb', 'app/models/github/write_client.rb', 'app/models/github_connected_account.rb', 'app/controllers/github/app_connections_controller.rb', 'config/initializers/active_record_encryption.rb', 'config/application.rb', 'Gemfile.lock', 'app/controllers/github/webhooks_controller.rb', 'app/models/github/webhook_delivery.rb', 'app/models/github/notifier.rb', 'app/jobs/github/perform_agent_action_job.rb', 'app/jobs/fizzy/perform_agent_action_job.rb', 'app/models/audit_log.rb', 'app/models/agent_event.rb', 'app/models/github/pull_request_fetcher.rb', 'app/jobs/github/fetch_pull_request_job.rb', 'app/models/github/pull_request.rb', 'app/models/github/agent_pull_request_action.rb', 'app/models/github/review_logins.rb', 'app/models/agent.rb', 'app/models/agent_approval.rb', 'app/models/github/notification.rb', 'app/models/github/repository_subscription.rb', 'app/models/github/pull_request_reference_sync.rb', 'app/models/github/pull_request_url.rb', 'app/models/message.rb', 'app/models/channel_thread.rb', 'app/models/user/inbox_preferences.rb', 'app/jobs/github/deliver_subscription_event_job.rb', 'app/models/user.rb', 'app/models/user/bot.rb', 'app/models/membership.rb', 'app/models/thread_membership.rb', 'app/models/activity_item.rb', 'app/jobs/application_job.rb']
hashes = {p: hashlib.sha256(subprocess.check_output(['git', '-C', sys.argv[1], 'show', 'd7c7de92:' + p])).hexdigest() for p in paths}
for p in ['app/models/github/pull_request_thread.rb', 'app/helpers/github/pull_requests_helper.rb', 'app/views/github/pull_requests/_card.html.erb', 'app/views/github/pull_requests/_cards.html.erb', 'app/views/github/pull_requests/_thread_header.html.erb', 'app/views/github/pull_requests/_files_summary.html.erb']:
    hashes[p] = hashlib.sha256(subprocess.check_output(['git', '-C', sys.argv[1], 'show', 'd7c7de92:' + p])).hexdigest()
for p in ['app/controllers/rooms/github/pull_request_cards_controller.rb','app/views/rooms/github/pull_request_cards/show.html.erb','app/controllers/accounts/integrations_health_controller.rb','app/views/accounts/integrations_health/show.html.erb','app/controllers/rooms/github_subscriptions_controller.rb','app/views/rooms/github_subscriptions/_section.html.erb']:
    hashes[p] = hashlib.sha256(subprocess.check_output(['git', '-C', sys.argv[1], 'show', 'd7c7de92:' + p])).hexdigest()
pathlib.Path(sys.argv[2]).write_text(json.dumps(hashes))
PY
rm -f "$SCRATCH/$MODE.sqlite3" "$SCRATCH/$MODE.sqlite3-wal" "$SCRATCH/$MODE.sqlite3-shm"
GITHUB_FIXTURE_SECRET=$(sed -n 's/^SECRET_KEY_BASE=//p' "$RUST_ROOT/parity/.env.reference")
SCRIPT=generate.rb
if [[ "$MODE" == discussions ]]; then SCRIPT=discussions.rb; fi
if [[ "$MODE" == health_page ]]; then SCRIPT=health_page.rb; fi
if [[ "$MODE" == connections ]]; then SCRIPT=connections.rb; fi
if [[ "$MODE" == verify ]]; then SCRIPT=verify_rust.rb; fi
if [[ "$MODE" == webhooks ]]; then SCRIPT=webhooks.rb; fi
if [[ "$MODE" == claims ]]; then SCRIPT=claims.rb; fi
if [[ "$MODE" == fetch ]]; then SCRIPT=fetch.rb; fi
if [[ "$MODE" == actions ]]; then SCRIPT=actions.rb; fi
if [[ "$MODE" == notifier ]]; then SCRIPT=notifier.rb; fi
if [[ "$MODE" == notifier_verify ]]; then SCRIPT=notifier_verify.rb; fi
if [[ "$MODE" == references ]]; then SCRIPT=references.rb; fi
if [[ "$MODE" == domain ]]; then SCRIPT=domain.rb; fi
if [[ "$MODE" == cards ]]; then SCRIPT=cards.rb; fi
if [[ "$MODE" == card_http ]]; then SCRIPT=card_http.rb; fi
if [[ "$MODE" == health ]]; then SCRIPT=health.rb; fi
if [[ "$MODE" == subscriptions ]]; then SCRIPT=subscriptions.rb; fi
docker run --rm --network none --name "ws15g-github-$MODE" --entrypoint '' --user "$(id -u):$(id -g)" \
  -e RAILS_ENV=production -e RAILS_LOG_LEVEL=fatal -e SECRET_KEY_BASE="$GITHUB_FIXTURE_SECRET" \
  -e DISABLE_SSL=1 -e SKIP_TELEMETRY=true -e DATABASE_URL="sqlite3:/work/scratch/$MODE.sqlite3" \
  -e GITHUB_REFERENCE_HASHES=/work/scratch/reference-hashes.json -e GITHUB_VECTOR_PATH=/work/vectors/github.json \
  -e GITHUB_WEBHOOK_VECTOR_PATH=/work/vectors/github_webhooks.json \
  -e GITHUB_CLAIM_VECTOR_PATH=/work/vectors/github_claims.json \
  -e GITHUB_FETCH_VECTOR_PATH=/work/vectors/github_fetch.json \
  -e GITHUB_ACTION_VECTOR_PATH=/work/vectors/github_actions.json \
  -e GITHUB_NOTIFIER_VECTOR_PATH=/work/vectors/github_notifier.json \
  -e GITHUB_RUST_OUTPUT=/work/scratch/rust-account.json \
  -v "$SCRATCH:/work/scratch" -v "$RUST_ROOT/reference-tools/github:/work/scripts:ro" -v "$RUST_ROOT/vectors:/work/vectors" \
  "${WS15G_REFERENCE_IMAGE:-ws6-reference-d7c7de92}" bin/rails runner "/work/scripts/$SCRIPT"

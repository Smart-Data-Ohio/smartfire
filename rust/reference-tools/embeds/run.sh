#!/usr/bin/env bash
# Regenerate the embed contracts from the plain pinned production image.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
source "$ROOT/reference-tools/reference_image_env.sh"
MODE=${1:-generate}
case "$MODE" in
  github_transport) exec bash "$ROOT/reference-tools/github/run.sh" transport ;;
  generate|urls|twitter_fetch|opengraph_validation|linkedin_cards|link_embed|fizzy|fizzy_retries|fizzy_message_form|fizzy_cards|fizzy_agent_requests|fizzy_agent_reads|fizzy_agent_action|card_html_audit) ;;
  *) echo "unknown embed oracle mode: $MODE" >&2; exit 2 ;;
esac
OUTPUT=$MODE
if [[ "$MODE" == generate ]]; then OUTPUT=embeds; fi
REFERENCE=${CAMPFIRE_REFERENCE:-$ROOT/..}
FIZZY_HASH=$(git -C "$REFERENCE" show "$PARITY_REFERENCE_SHA:app/models/fizzy/client.rb" | sha256sum | cut -d ' ' -f 1)
exec bash "$ROOT/reference-tools/run_with_source_tests.sh" \
  "reference-tools/embeds/$MODE.rb" "/work/vectors/ws15e_$OUTPUT.json" "$FIZZY_HASH"

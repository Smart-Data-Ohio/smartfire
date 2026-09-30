#!/usr/bin/env bash
# Fresh pinned Rails vectors, compared without normalization to the committed files.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${WS8BR2_ORACLE_DIR:-$ROOT/../.scratch/verified-oracles}
mkdir -p "$OUT"
export PARITY_NAMESPACE=ws8br2 PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker
export PARITY_IMAGE=${PARITY_IMAGE:-ws8br2-reference:d7c7de92-status-2e20b24c}
run() {
  local probe=$1 seed=$2 expected=$3 label=$4
  "$ROOT/parity/bin/reference" exec --seed "$seed" --time 2026-03-02T16:00:00Z --freeze -- \
    bin/rails runner --skip-executor "/work/reference-tools/users/$probe.rb" "${@:5}" > "$OUT/$label.json" 2> "$OUT/$label.log"
  if [ "${WS8BR2_REGENERATE:-0}" = 1 ]; then cp "$OUT/$label.json" "$ROOT/$expected"; else cmp "$OUT/$label.json" "$ROOT/$expected"; fi
  rg '^Rails .* oracle:' "$OUT/$label.log"
}
run public default vectors/users_public.json public
run avatars default vectors/users_avatars.json avatars
run avatar_images default vectors/users_avatar_images.json avatar-images
run preferences default vectors/users_preferences.json preferences
run people default vectors/users_people.json people
run profiles default vectors/users_profile_settings.json profiles
run appearance default vectors/users_appearance.json appearance
run accounts default vectors/users_account_mutations.json accounts
run account_views default vectors/users_account_views.json account-views
run audit_logs default vectors/users_audit_logs.json audit-logs
run icons default vectors/users_icons.json icons
run logos default vectors/users_logos.json logos
run onboarding first_run vectors/users_first_run.json first-run
run onboarding default vectors/users_welcome.json welcome welcome
run profile_page default vectors/users_profile_page.json profile-page
run layout_preferences default vectors/users_layout_preferences.json layout-preferences
run profile_sections default vectors/users_profile_sections.json profile-sections
run status_popup default vectors/users_status_popup.json status-popup
run status_panels default vectors/users_status_panels.json status-panels
run dm_picker default vectors/users_dm_picker.json dm-picker
run joining default vectors/users_joining.json joining
run pwa default vectors/users_pwa_default.json pwa-default
run pwa first_run vectors/users_pwa_first_run.json pwa-first-run
run zones first_run crates/db/src/slash_commands/rails_zone_identifiers.json zones
run zones first_run crates/db/src/slash_commands/rails_named_zones.json named-zones named
if [ "${WS8BR2_REGENERATE:-0}" = 1 ]; then
  echo 'WS8br2 oracle generation: 25 fresh unnormalized files written'
else
  echo 'WS8br2 oracle verification: all 25 fresh files match byte for byte; no masks or normalization'
fi

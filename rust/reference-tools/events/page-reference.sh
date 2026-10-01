#!/usr/bin/env bash
# Keep Event behavior at d7c7de92; apply only the approved shared layout/asset
# delta from 2e20b24c (_common.md, status-popup Rails drift).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BUILD=${WS14E_PAGE_BUILD:-$ROOT/../.scratch/calendar-page-reference-build}
mkdir -p "$BUILD"
for file in app/views/layouts/application.html.erb app/assets/stylesheets/people.css app/javascript/controllers/profile_card_controller.js; do
  mkdir -p "$BUILD/$(dirname "$file")"
  git -C "$ROOT/.." show "2e20b24c:$file" > "$BUILD/$file"
done
cat > "$BUILD/Dockerfile" <<'DOCKERFILE'
FROM ws14e-reference:d7c7de92
USER root
COPY --chown=1000:1000 app/ /rails/app/
RUN rm -rf /rails/public/assets
USER 1000:1000
RUN SECRET_KEY_BASE_DUMMY=1 ./bin/rails assets:precompile
DOCKERFILE
docker build -t ws14e-reference:pages-2e20b24c "$BUILD"

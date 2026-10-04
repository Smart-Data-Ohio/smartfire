#!/usr/bin/env bash
# Original named tests from the pinned tree; dependencies remain Gemfile.lock locked.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
source "$ROOT/rust/reference-tools/reference_image_env.sh"
cd "$ROOT"
STORE=$ROOT/.scratch/rails-controller-tests
mkdir -p "$STORE"
WORK=$(mktemp -d -p "$STORE" run.XXXXXX)
trap 'python3 -c "import shutil,sys; shutil.rmtree(sys.argv[1])" "$WORK"' EXIT
git archive "$(cat rust/parity/reference.sha)" test | tar -x -C "$WORK"
if (( $# == 0 )); then set -- test/controllers/agent_approvals_controller_test.rb; fi
docker run --rm -e PARITY_REFERENCE_SHA --name "${PARITY_OWNER:-ws11ui}-rails-controller-tests" --cpus 2 \
  --user 0:0 --env-file rust/parity/.env.reference \
  -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -e PARITY_REDIS=1 \
  -e BUNDLE_WITHOUT=development -e BUNDLE_JOBS=1 -e BUNDLE_DEPLOYMENT=true \
  -v "$WORK/test:/rails/test:ro" "${PARITY_IMAGE:-campfire-reference}" \
  bash -c 'bundle config set --local without development; bundle install --quiet; bin/rails db:prepare >/dev/null; status=0; for file in "$@"; do echo "Rails pinned file: $file"; bin/rails test "$file" || status=1; done; exit "$status"' bash "$@"

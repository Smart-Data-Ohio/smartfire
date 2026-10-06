#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
mkdir -p target/ci-receipts
docker run --rm --user "$(id -u):$(id -g)" --volume "$repo:/src" --workdir /src \
  --env HOME=/tmp --env CARGO_BUILD_JOBS=4 --env RUST_TEST_THREADS=4 \
  "${RUST_CI_IMAGE:-campfire-toolchain}" python3 ci/compiler_ignore_mutations.py
# Ask the compiled test harnesses, with no suite/name/default-filter restriction.
# This also catches ignores introduced by cfg_attr, include!, and procedural macros.
bash ci/cargo.sh nextest list --locked --workspace --exclude html5ever \
  --build-jobs 4 --profile ci --run-ignored only --ignore-default-filter --message-format json \
  > target/ci-receipts/ignored-list.json
python3 ci/ignored_tests.py --nextest-list target/ci-receipts/ignored-list.json

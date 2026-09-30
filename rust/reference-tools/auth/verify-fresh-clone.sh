#!/usr/bin/env bash
# Clone only committed branch state; rebuild seeds and use this clone's own Cargo target.
set -euo pipefail
SOURCE=$(cd "$(dirname "$0")/../../.." && pwd)
CLONE=${1:-$SOURCE/.scratch/round-three-clean-clone}
[ ! -e "$CLONE" ] || { echo "fresh-clone: destination already exists: $CLONE" >&2; exit 1; }
branch=$(git -C "$SOURCE" branch --show-current)
git clone --local --no-hardlinks --branch "$branch" "$SOURCE" "$CLONE"
cd "$CLONE"
export TMPDIR="$CLONE/.scratch/tmp"
export CARGO_TARGET_DIR="$CLONE/.scratch/cargo-target"
export CI=1 CABLE_TEST_PORT_RANGE=51600-51699
export PARITY_NAMESPACE=ws9-clean PARITY_OWNER=ws9 PARITY_IMAGE=${WS9_REFERENCE_IMAGE:-ws9-reference:d7c7de92}
mkdir -p "$TMPDIR" .scratch/verification
printf 'fresh-clone: commit %s\n' "$(git rev-parse HEAD)"
cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 >/dev/null
python3 - <<'PY'
import pathlib, subprocess, tomllib
paths = subprocess.check_output(['rg', '--files', 'rust', '-g', 'Cargo.toml'], text=True).splitlines()
for path in paths:
    tomllib.loads(pathlib.Path(path).read_text())
root = tomllib.loads(pathlib.Path('rust/Cargo.toml').read_text())
print(f'TOML check: {len(paths)} manifests parsed; {len(root["workspace"]["dependencies"])} workspace dependency keys; no duplicate keys')
PY
rust/parity/bin/seed build default first_run > .scratch/verification/seeds.log 2>&1
cat .scratch/verification/seeds.log
for seed in default first_run; do
    rust/parity/bin/reference runner --seed "$seed" --time 2026-03-02T16:00:00Z --freeze \
        rust/reference-tools/campfire/verify_parity_seed.rb "$seed" > ".scratch/verification/seed-$seed.log" 2>&1
    cat ".scratch/verification/seed-$seed.log"
done
bash rust/reference-tools/auth/round-three.sh > .scratch/verification/rails-round-three.log 2>&1
cat .scratch/verification/rails-round-three.log
bash rust/reference-tools/auth/profile-security.sh > .scratch/verification/rails-profile.log 2>&1
cat .scratch/verification/rails-profile.log
git diff --exit-code -- rust/vectors/profile_security.json rust/vectors/round_three_security.json
[ ! -e rust/target ]
echo 'fresh-clone: rust/target absent before suite; no artifact directory prepared'
cargo test --manifest-path rust/Cargo.toml --locked -j 4 -p campfire -p campfire_db -p campfire_views -p rails_compat \
    -- --test-threads=4 > .scratch/verification/tests.log 2>&1
rg '^test result:' .scratch/verification/tests.log
[ -f rust/target/campfire_session_keys_rust_output.json ]
echo 'fresh-clone: session-keys test created its artifact directory and output'
bash rust/reference-tools/auth/profile-rollback.sh > .scratch/verification/profile-rollback.log 2>&1
rg '^(test result:|WS9)' .scratch/verification/profile-rollback.log
bash rust/reference-tools/auth/rollback.sh > .scratch/verification/auth-rollback.log 2>&1
rg '^(test result:|WS9)' .scratch/verification/auth-rollback.log
(
    cd rust
    CONTAINER_PREFIX=ws9-clean OUT="$CLONE/.scratch/db-differential" \
        bash reference-tools/db/differential.sh > "$CLONE/.scratch/verification/db-differential.log" 2>&1
)
rg '^(test result:|message_save|schema.sql|validated|rollback ok)' .scratch/verification/db-differential.log
cargo clippy --manifest-path rust/Cargo.toml --locked -j 4 --workspace --exclude html5ever --all-targets \
    -- -D warnings > .scratch/verification/clippy.log 2>&1
tail -1 .scratch/verification/clippy.log
git diff --exit-code
echo 'fresh-clone: tracked branch state unchanged; all checks passed'

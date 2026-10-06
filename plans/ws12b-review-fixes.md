# PR #187: WS12b review fixes

This change is limited to the frozen `rust/ws12b-board-writes` slice at `d886f8cf0c72064e7082b82195a3e59cacb65a1d`. It fixes the three findings in Astra's read-only review evidence. It adds no remaining WS12 features and does not touch `rust/ws12-boards` or Rails source.

## Root causes and changes

1. **P2, null title:** `Option::and_then(string_column)` turned both an absent title and a submitted JSON null into `None`, so metadata validation omitted the assignment and later persisted the result. The parser now preserves a present key as a submitted value. Null is blank for the non-null Rust title model, so the existing Rails-compatible validation returns 422 before any writes commit. The HTTP regression checks the literal error, original title/result/timestamp, and counts for source, audit, activity, and job rows.
2. **P2, quadratic recorder queries:** the work-event fanout computed recipients, then called the standalone recorder for every recipient. Each call reloaded the event and recomputed the whole authorized roster, with a user SELECT per candidate. Fanout now computes its authorized roster once, bulk-loads the users, and passes the authorized event/user snapshots to the shared recorder persistence logic. Hash maps and a set also avoid repeated linear roster searches. Standalone recorder calls still authorize against current source permissions; per-recipient transactions still run after the source commit. With 1/10/20 followers, Rust now matches Rails' membership/user SELECT counts of 3/3, 3/12 and 3/22.
3. **P3, grouped broadcasts:** the recorder broadcast every source/timestamp refresh of an existing grouped item. Rails' update callback only broadcasts changes to read, handled, type, or creation time. Grouped recording changes only the source, update time, and possibly read state, so it now broadcasts only when reopening a read item. The regression checks stable item ID, newest source, updated ordering, unread state, zero frames for an unread repoint, and one frame for reopening a read item.

## Rails oracle and nullable-field audit

The reference is `d7c7de92` plus the approved board/layout drift in `ws12-reference:boards-b908ebc2`. Both generators verify the existing three source-hash ledgers before executing. The default and first_run seeds were copied from the reviewed reference and validated through Rails: 29/0 and 4/0 checks passed/failed. The Rust tests run with `CI=1` so missing seeds cannot silently skip.

All original 82 response rows and owner coercions remain unchanged. The write oracle now has 94 complete responses, adding 12 JSON-null cases for board creation/update and ordinary thread update, including null title plus result. The Rails-generated vector is byte-identical to a fresh final regeneration, without masks. `boards_recorder.json` records actual Rails SQLite query counts at 1, 10 and 20 followers and unread/read grouped-repoint broadcast counts (0/1).

The other nullable write parameters preserve submission presence: null tags clear tags; null result clears the result; null work owner unassigns and still checks assignment permissions; null work status attempts to remove tracking and validates/authorizes accordingly; null archive casts to zero and validates (board posts reject any submission). A null lifecycle status is Rails' no-op. Board creation already rejects null titles, and ordinary updates now reject them too. All of these responses come from Rails and are checked literally in the oracle.

## Failing-first evidence

The new tests and Rails vectors ran against unchanged production source at `d886f8cf` before applying the fixes. The recorder regressions failed at actual assertions with 43/461 SELECTs at 20 followers and one extra ActivityChannel frame. The dedicated title/result rollback regression failed on 200 versus 422; the expanded HTTP oracle independently failed on `update-null-title` (200 versus 422). No compile/setup failure is counted as a regression failure. The definitive raw receipts follow.

## Raw completion receipts

`red-null-title.log`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1913 filtered out; finished in 1.10s
```

`red-recorder.log`

```text
Unread grouped repoint: Rust activity frames=1; Rails=0
Recorder fanout: followers=1; Rust membership/user SELECTs=5/5; Rails=3/3
Recorder fanout: followers=10; Rust membership/user SELECTs=23/131; Rails=3/12
Recorder fanout: followers=20; Rust membership/user SELECTs=43/461; Rails=3/22
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1203 filtered out; finished in 0.38s
```

`red-board-oracle.log`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1913 filtered out; finished in 46.36s
```

`green-recorder.log`

```text
Recorder fanout: followers=1; Rust membership/user SELECTs=3/3; Rails=3/3
Unread grouped repoint: Rust activity frames=0; Rails=0
Recorder fanout: followers=10; Rust membership/user SELECTs=3/12; Rails=3/12
Recorder fanout: followers=20; Rust membership/user SELECTs=3/22; Rails=3/22
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1203 filtered out; finished in 0.31s
```

`green-channel-threads.log`

```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 1883 filtered out; finished in 67.62s
```

`workspace.log`

```text
test result: ok. 1911 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 649.66s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.59s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1201 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 135.54s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.43s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.25s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.26s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.20s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.74s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`clippy.log`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 01s
```

`release-inputs.log`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.21s
```

`rails-write-final.log`

```text
Rails board write oracle: 94 complete HTTP responses; no masks
```

`rails-recorder-final.log`

```text
Rails recorder oracle: 3 fanout traces; unread/read repoint activity frames=0/1
```

`oracle-preservation.log`

```text
Original board oracle: 82/82 complete Rails responses unchanged
Expanded board oracle: 94 complete Rails responses; 12 nullable-parameter cases added
Rails null title plus result: status=422; body={"error":"Name can't be blank"}
```

Workspace totals, summed from the 60 raw Cargo receipts: **3828 passed, 0 failed, 12 existing ignored**. No new ignores, exclusions or normalization were added. Every final validation command exited 0.

## Reproduction and validation

Commands run from the assigned worktree root. Build jobs were 2, test threads at most 4 (2 for the new focused database regressions); the configured machine-wide rustc throttle was unchanged. No subordinate agents were launched.

```sh
# Shared native Rust test/build inputs used for final validation:
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 RUST_TEST_THREADS=4
export CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH"

# Definitive failing-first assertions, before applying the fixes:
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db work_recorder_review_test -- --test-threads=2 --nocapture
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire null_title_rejects -- --test-threads=2
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire board_writes_match_complete_rails_responses_without_masks -- --test-threads=2

# Final focused checks and the full workspace (no exclusions):
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db work_recorder_review_test -- --test-threads=2 --nocapture
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire controllers::channel_threads:: -- --test-threads=4
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --all-targets -- -D warnings
mise exec rust@1.98.1 -- rust/ci/with-release-inputs.sh cargo check --locked -p campfire --bin campfire

# Reference runners use these task-specific settings:
export PARITY_NAMESPACE=ws12b-fix PARITY_OWNER=ws12b-fix PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default
rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run
rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/boards-write-final.json
rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/recorder.rb > .scratch/boards-recorder-final.json
cmp .scratch/boards-write-final.json rust/vectors/boards_write.json
cmp .scratch/boards-recorder-final.json rust/vectors/boards_recorder.json
```

The workspace runs all test targets and doctests, including the expanded HTTP oracle. Clippy runs with warnings denied and no exclusions. The release-input guard checks the production binary with only the Docker builder's source and asset inputs. Raw logs remain in this worktree's `.scratch/logs/`.

## Cleanup

All test/build/reference commands completed before cleanup; no task-owned reference container remains. The measured 22G `.scratch/target` was removed with `cargo clean` (raw receipt: `Removed 22360 files, 22.5GiB total`). The additional 40K `rust/target` contained only four regenerated rollback/security JSON outputs; their generating test code was verified before removing those files. Both target directories are absent. The 695M temporary media copy and temporary fix scripts were also removed. Release-input temporary source directories are absent. Raw logs, Rails captures, and validated parity seeds remain. The Python model server was not touched, no stash was used, and all tracked changes stay under `rust/` on `rust/ws12b-board-writes`.

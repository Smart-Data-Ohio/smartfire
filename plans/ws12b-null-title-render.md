# PR #187: null-title failed-form rendering

This follow-up to `0a4dccc43cd05b69c22d58798766833f7934ac90` fixes Astra's remaining P3 finding. It stays on `rust/ws12b-board-writes` and changes no Rails source or remaining WS12 features.

The previous fix correctly rejected a submitted null title and rolled back the result, but converted the null to an empty string before presenting the failed board form. Rails retains nil in `app/controllers/channel_threads_controller.rb` and `app/views/channel_threads/_board_post.html.erb`: the application layout falls back to `Smartfire`, and the title input omits its value attribute.

The controller now retains an omitted/null/string title through its failed-form capture. It captures the submitted title when the metadata assignment is reached, so an earlier board auto-archive rejection keeps the original title. The non-null persisted model still validates null as blank, preserving the existing 422 and rollback. The board view's title is nullable; the existing layout and form helpers render Rails' nil behavior while retaining empty-string behavior and attribute order.

The Rails generator adds JSON PATCH requests with a null title and a result, accepting HTML and Turbo/HTML. All 94 previous response rows and owner coercions are unchanged. The pinned reference image `ws12-reference:boards-b908ebc2` regenerated 96 complete responses after checking the source-hash ledgers for `d7c7de92` plus approved board drift. The default and first_run seeds were validated with Rails: 29/0 and 4/0 checks passed/failed. Rust runs with `CI=1`.

The expanded oracle ran against unchanged controller/view source before the fix. It failed at the response assertion for both new cases, with precisely the reviewed title and input-attribute differences. The interrupted initial compilation is not counted as failing-first evidence.

```text
complete Rails response mismatches: ["update-null-title-html", "update-null-title-turbo"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2012 filtered out; finished in 83.46s
```

Validation uses Rust 1.98.1, two build jobs, four workspace test threads, and one focused-oracle test thread. The configured machine-wide rustc throttle remains unchanged. No subordinate agents were launched. Raw logs and Rails captures are retained under `.scratch/null-title-render/`.

```sh
# From the assigned worktree root, with canonical native media:
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/null-title-render/target"
export TMPDIR="$PWD/.scratch/null-title-render" CI=1 RUST_TEST_THREADS=4
export CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499
export LD_LIBRARY_PATH="$PWD/.scratch/null-title-render/rails-media/native-libs"
export PATH="$PWD/.scratch/null-title-render/rails-media/usr/bin:$PATH"
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --all-targets -- -D warnings
mise exec rust@1.98.1 -- rust/ci/with-release-inputs.sh cargo check --locked -p campfire --bin campfire
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire board_writes_match_complete_rails_responses_without_masks -- --test-threads=1

env PARITY_NAMESPACE=ws12b-null-render PARITY_OWNER=ws12b-null-render PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 \
  rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/null-title-render/rails-board-oracle.json
cmp .scratch/null-title-render/rails-board-oracle.json rust/vectors/boards_write.json
```

## Raw completion receipts

`workspace.log` (exit 0):

```text
test result: ok. 2010 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1247.51s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.28s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.41s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 1228 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 226.97s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.62s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 16.73s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.61s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.49s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
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

Workspace total: **3954 passed, 0 failed, 12 existing ignored**, summed from 60 raw Cargo summaries. No ignores, exclusions, or response normalization were added.

`clippy.log` (exit 0):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 40s
```

`release-inputs.log` (exit 0):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.50s
```

`board-oracle.log` (exit 0):

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.25s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2012 filtered out; finished in 78.46s
```

Rails capture and oracle preservation:

```text
Rails board write oracle: 96 complete HTTP responses; no masks
Original board oracle: 82/82 complete Rails responses unchanged
Reviewed board oracle: 94/94 complete Rails responses unchanged
Expanded board oracle: 96 complete Rails responses; HTML and Turbo null-title cases added
```

All four final Rust commands exited 0. Validated source hashes were unchanged throughout the run, and the Rails capture is byte-identical to the committed vector.

## Cleanup

Before removal, the private target measured 23G, the native media copy 695M, and generated diagnostics 424K. All commands completed, no process was using the private target/media paths, and no task-owned Rails reference container remained. Cargo clean exited 0:

```text
Removed 19800 files, 22.6GiB total
```

All scratch targets, the private media copy, generated `rust/target` diagnostics, and release-input temporary trees are absent. Raw logs, complete failing response pairs, Rails captures, and validated seeds are retained. The Python model server was not touched, no stash was used, and all tracked changes remain under `rust/` on the assigned branch.

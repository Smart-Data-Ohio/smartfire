# WS12 board query follow-up for #191 / #195

This review slice fixes board member/owner and linked-source query amplification. Source commit: `c331c580cb142c91779f1aa0648454e71887fc7a`. Failing-first checkpoint: `355670d30` (production unchanged from `d05e019b8`). Final published SHA is in the delivery reply and external report footer.

## Changes and boundaries

- `db/models/membership.rs`: `for_room_with_users` loads a membership roster and its users in two reads, retains membership order and Rails' missing-user handling. Both owner choices and opening notifications use it.
- `db/models/channel_thread/work.rs`: owner choices use WS11's existing `Agent::for_users` and the already-reviewed batch capability API. Active human membership, active agents, suspension, global/scoped/revoked grants, legacy posting and name sorting keep their existing policies. Opening-message recording still uses the original recipient gates, recorder, ledger and transaction.
- `db/models/calendar_event.rs` and `campfire/integrations/github/pull_requests.rs`: empty-safe bulk association reads using the same full-row decoders as `find`.
- `campfire/controllers/presenters/board_posts.rs`: link source facts preload once per pane. Labels, URLs, ordering, public-only PR titles, cancellation flags, timestamps and missing-association errors retain the old adapter behavior.
- `campfire/controllers/presenters/work_threads.rs`: the entire work page shares one link-source preload, avoiding one batch per row. Row event options remain omitted.
- `campfire/controllers/human_work_tests/query_board_followup.rs`: nine real-SQL regressions at 10/200 associations. The notification test also checks the real persisted recipient set; the index test checks the rendered row/link counts. All app probes use FrozenClock and `TestApp::without_job_runner()`.
- `reference-tools/boards/query_followup.rb` and `vectors/board_query_counts.json`: measured Rails SELECTs for the corresponding model/presenter scopes, generated from our reference image.

The agent availability map and unlinked event picker already use the previous `5c9a82709` batching fix; they are reused without duplicate implementations. The query fix commit leaves `presenters/boards.rs` untouched: the per-post room reload belongs to WS14g and enters this branch through its reviewed main merge. The query fixes change no Rails files, production include paths, golden response bytes, masks, policy rules, callbacks, queue semantics, cache keys or feature surfaces.

## Read counts

All pairs are 10 / 200 associations. Counts are actual SQLite statements, not estimates. Reader probes capture only the measured read closure/request; the opening probe traces SELECT/WITH statements on the writer during the complete model operation.

| Probe | Before | After | Rails measured here |
|---|---:|---:|---:|
| Legacy agent availability | 52 / 1,002 | 4 / 4 | 4 / 4 |
| Explicit agent availability | 62 / 1,202 | 4 / 4 | 4 / 4 |
| Human owner choices | 11 / 201 | 2 / 2 | 3 / 3 |
| Agent owner choices | 71 / 1,401 | 4 / 4 | Not measured in the Rails producer |
| Opening-message creation, total SELECTs | 70 / 260 | 61 / 61 | 17 / 17 |
| Opening-message creation, user reads | 16 / 206 | 7 / 7 | Not separately measured |
| Upcoming unlinked event choices | 2 / 2 | 2 / 2 | 2 / 2 |
| Existing linked calendar events + choices | 12 / 202 | 3 / 3 | 3 / 3 |
| Existing linked PRs + choices | 12 / 202 | 3 / 3 | 3 / 3 |
| Linked work-index rows, full HTTP request | 25 / 215 | 16 / 16 | Not measured in the Rails producer |

Availability before counts replay the exact `board_owner_active_map` method from `9467e398`; the previous fix already made the current baseline 4/4. The review's explicit-grant 26/98 measurements used 4/16 owners; this replay measures the requested 10/200 sizes. The unlinked picker was already fixed and passes on the unchanged baseline, as requested. All other before counts are from unchanged `d05e019b8` production code, with the new regression module attached.

Opening creation keeps one real recipient (Jason, following everything) while the other board members are invisible. Its overall fixed Rust read cost remains above Rails, but the membership amplification is removed. Its roster now costs two reads irrespective of member count; the six additional user reads belong to the surrounding operation. This slice does not claim that every fixed creation read matches Rails' budget.

## Failing-first evidence

Commands below ran from the assigned worktree. The six new failures are query-budget assertions after both sizes and their functional checks completed. No fixture/schema error is counted as failing-first evidence.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire query_board_followup -- --test-threads=2 --nocapture > .scratch/board-query-followup/logs/fail-first.log 2>&1
```

```text
WS12 board agent availability explicit size=10: 4 SELECTs
WS12 board agent availability legacy size=10: 4 SELECTs
WS12 board agent availability legacy size=200: 4 SELECTs
WS12 board agent availability explicit size=200: 4 SELECTs
WS12 board unlinked event/link choices size=10: 2 SELECTs
WS12 board event event/link choices size=10: 12 SELECTs
WS12 board unlinked event/link choices size=200: 2 SELECTs
WS12 board event event/link choices size=200: 202 SELECTs
WS12 board pull_request event/link choices size=10: 12 SELECTs
WS12 board opening notification size=10: 70 SELECTs; 16 user reads
WS12 board pull_request event/link choices size=200: 202 SELECTs
WS12 board opening notification size=200: 260 SELECTs; 206 user reads
WS12 board agent owner choices size=10: 71 SELECTs
WS12 board owner choices size=10: 11 SELECTs
WS12 board owner choices size=200: 201 SELECTs
WS12 board agent owner choices size=200: 1401 SELECTs
WS12 board linked work index size=10: 25 SELECTs
WS12 board linked work index size=200: 215 SELECTs
test result: FAILED. 3 passed; 6 failed; 0 ignored; 0 measured; 2295 filtered out; finished in 5.19s
```

The two availability regressions were also proved against the exact old method, temporarily replayed in this worktree and restored byte for byte in a `finally` block. All other source stayed unchanged. The driver requires both runtime test failures and the expected large-size counts, so a compile error cannot satisfy the proof. The raw driver and logs are retained under `.scratch/board-query-followup/`.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 .scratch/board-query-followup/prove-old-owner-map.py > .scratch/board-query-followup/logs/old-availability-driver.log 2>&1
```

```python
from pathlib import Path
import subprocess

path=Path('rust/crates/db/src/models/channel_thread/board.rs')
original=path.read_bytes()
old=subprocess.check_output(['git','show','9467e398:rust/crates/db/src/models/channel_thread/board.rs'],text=True)
current=original.decode()
start='    pub fn board_owner_active_map'
def function_end(text):
    opening=text.index('{',text.index(start))
    depth=1
    for i in range(opening+1,len(text)):
        depth += (text[i]=='{')-(text[i]=='}')
        if depth==0: return i+1
    raise RuntimeError('unclosed availability function')
old_function=old[old.index(start):function_end(old)]
try:
    path.write_text(current[:current.index(start)]+old_function+current[function_end(current):])
    with Path('.scratch/board-query-followup/logs/old-availability-fail-first.log').open('w') as log:
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--manifest-path','rust/Cargo.toml','--locked','-p','campfire','agent_availability_batches','--','--test-threads=2','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
finally:
    path.write_bytes(original)
assert result.returncode==101, result.returncode
proof=Path('.scratch/board-query-followup/logs/old-availability-fail-first.log').read_text()
assert 'test result: FAILED. 0 passed; 2 failed' in proof, proof[-2000:]
assert 'size=200: 1002 SELECTs' in proof and 'size=200: 1202 SELECTs' in proof
print('Old availability method from 9467e398 rejected by both regressions; restored current board.rs byte for byte.')
```

```text
WS12 board agent availability legacy size=10: 52 SELECTs
WS12 board agent availability explicit size=10: 62 SELECTs
WS12 board agent availability legacy size=200: 1002 SELECTs
WS12 board agent availability explicit size=200: 1202 SELECTs
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2302 filtered out; finished in 1.22s
Old availability method from 9467e398 rejected by both regressions; restored current board.rs byte for byte.
```

## Rails producers and response bytes

Reference is `d7c7de92` plus approved board drift. For the approved board/work/activity files, **origin/main's Rails is the reference**. The work/board producers validate their committed source-hash ledgers; the four approved-drift files also match the reference image byte for byte. Main initially remained `056ab49acffd52007334356ada0bdba3774c0fe2`, already merged by `d239c89eb`, then moved during verification to `06ce3d0352bc3eba26a3780ca807bcdb8dcb091b` (#194). Merge `7c65cedf0ba9ac64971904ef599362de79e1f874` preserves WS14g's room reuse and our new source batches without conflicts. No response corpus was changed. No masks were added, removed or widened.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/boards/query_followup.rb" > .scratch/board-query-followup/rails-query-counts.json 2> .scratch/board-query-followup/logs/rails-queries.log
cmp rust/vectors/board_query_counts.json .scratch/board-query-followup/rails-query-counts.json
```

```text
Rails board legacy size=10: 4 SELECTs
Rails board legacy size=200: 4 SELECTs
Rails board explicit size=10: 4 SELECTs
Rails board explicit size=200: 4 SELECTs
Rails board humans size=10: 3 SELECTs
Rails board humans size=200: 3 SELECTs
Rails board opening size=10: 17 SELECTs
Rails board opening size=200: 17 SELECTs
Rails board unlinked size=10: 2 SELECTs
Rails board unlinked size=200: 2 SELECTs
Rails board event size=10: 3 SELECTs
Rails board event size=200: 3 SELECTs
Rails board pull_request size=10: 3 SELECTs
Rails board pull_request size=200: 3 SELECTs
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/board-query-followup/human_work_http.json 2> .scratch/board-query-followup/logs/human-oracle.log
cmp rust/vectors/human_work_http.json .scratch/board-query-followup/human_work_http.json
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/board-query-followup/work_link_model.json 2> .scratch/board-query-followup/logs/link-oracle.log
cmp rust/vectors/work_link_model.json .scratch/board-query-followup/work_link_model.json
```

```text
Rails work link model oracle: 29 validation/persistence cases; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/board-query-followup/boards_write.json 2> .scratch/board-query-followup/logs/board-oracle.log
cmp rust/vectors/boards_write.json .scratch/board-query-followup/boards_write.json
```

```text
Rails board write oracle: 96 complete HTTP responses; no masks
```

All four producer commands and all four byte comparisons exit 0. The 96 board HTTP, 90 work HTTP and 29 link model cases retain their committed, unmasked bytes. Their consuming Rust test results appear in the final workspace receipt below.

```sh
sha256sum app/models/board_automations/nudge_pusher.rb app/views/channel_threads/_board_post.html.erb app/views/channel_threads/new.html.erb app/views/layouts/application.html.erb > .scratch/board-query-followup/logs/reference-source-hashes.log
docker run --rm --name ws12-board-query-reference-hashes --entrypoint sha256sum ws12-reference:boards-b908ebc2 /rails/app/models/board_automations/nudge_pusher.rb /rails/app/views/channel_threads/_board_post.html.erb /rails/app/views/channel_threads/new.html.erb /rails/app/views/layouts/application.html.erb > .scratch/board-query-followup/logs/image-source-hashes.log
```

```text
04c5ad74ad14463e99a987c8fadb52a02f4e26b4f83240797ff54deb00ff51ee  app/models/board_automations/nudge_pusher.rb
cdc80236fbb1ced4e70019d961e0795dae8e3a29352b8543ea8053b60aa4d4a2  app/views/channel_threads/_board_post.html.erb
a13234df714707b0d46477a924c6231866fc92b0f46f205d83f6e6ebd881190a  app/views/channel_threads/new.html.erb
53008a50df6bf20f53013c146ef0584690cdd1061b5b748d5bf2035f3fd854c9  app/views/layouts/application.html.erb
04c5ad74ad14463e99a987c8fadb52a02f4e26b4f83240797ff54deb00ff51ee  /rails/app/models/board_automations/nudge_pusher.rb
cdc80236fbb1ced4e70019d961e0795dae8e3a29352b8543ea8053b60aa4d4a2  /rails/app/views/channel_threads/_board_post.html.erb
a13234df714707b0d46477a924c6231866fc92b0f46f205d83f6e6ebd881190a  /rails/app/views/channel_threads/new.html.erb
53008a50df6bf20f53013c146ef0584690cdd1061b5b748d5bf2035f3fd854c9  /rails/app/views/layouts/application.html.erb
```

## Fresh clone and seeds

The fresh clone initially contains `c331c580` and is then fast-forwarded to merge `7c65cedf` before repeating the final gates. All three restored seeds were independently verified against Rails during this run; tests use `CI=1`, so a missing seed cannot silently skip.

```sh
git fetch origin main
git clone --local --no-hardlinks --single-branch --branch rust/ws12-boards . .scratch/board-query-clean/source
mkdir -p .scratch/board-query-clean/source/rust/parity/.seed .scratch/board-query-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run rust/parity/.seed/agents_ui .scratch/board-query-clean/source/rust/parity/.seed/
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/board-query-followup/logs/default-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/board-query-followup/logs/first-run-seed.log 2>&1
```

```text
  "passed": 4,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/board-query-followup/logs/agents-ui-seed.log 2>&1
```

```text
  "passed": 40,
  "failed": 0
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/board-query-followup/logs/metadata.json
```

Locked workspace metadata exits 0. No lockfile changes were needed.

## Google/board merge verification (7c65cedf)

Main moved during the initial gate. That initial clone passed 4,309 tests (0 failed, 14 existing ignores); initial strict clippy also passed. These intermediate checks were repeated on merged source `7c65cedf`, including WS14g's reviewed board lifecycle/query fix. No manual changes to WS14g's code were made.

```sh
git merge --no-ff origin/main -m 'Merge reviewed Google and board room batching from main' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>'
git -C .scratch/board-query-clean/source fetch origin rust/ws12-boards
git -C .scratch/board-query-clean/source merge --ff-only FETCH_HEAD
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/board-query-followup/logs/merged-metadata.json
```

Post-merge locked metadata exits 0; no lockfile change.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/board-query-followup/logs/merged-workspace-test.log 2>&1
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 16s
test result: ok. 2312 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 642.27s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.87s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1275 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 135.68s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.53s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.44s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.80s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
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
Workspace aggregate: 4324 passed; 0 failed; 14 ignored; 61 target summaries; exit 0
```

All 61 target summaries are included above; zero silent seed skips. The 14 existing ignores, unchanged by this slice:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

All three consuming oracle tests also ran in this intermediate gate:

```text
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

The query capture was also rerun on this merged clone, rather than inferred from earlier source:

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked -p campfire query_board_followup -- --test-threads=2 --nocapture > .scratch/board-query-followup/logs/merged-query-final.log 2>&1
```

```text
WS12 board agent availability legacy size=10: 4 SELECTs
WS12 board agent availability explicit size=10: 4 SELECTs
WS12 board agent availability legacy size=200: 4 SELECTs
WS12 board agent availability explicit size=200: 4 SELECTs
WS12 board unlinked event/link choices size=10: 2 SELECTs
WS12 board event event/link choices size=10: 3 SELECTs
WS12 board unlinked event/link choices size=200: 2 SELECTs
WS12 board event event/link choices size=200: 3 SELECTs
WS12 board pull_request event/link choices size=10: 3 SELECTs
WS12 board opening notification size=10: 61 SELECTs; 7 user reads
WS12 board pull_request event/link choices size=200: 3 SELECTs
WS12 board opening notification size=200: 61 SELECTs; 7 user reads
WS12 board agent owner choices size=10: 4 SELECTs
WS12 board owner choices size=10: 2 SELECTs
WS12 board agent owner choices size=200: 4 SELECTs
WS12 board owner choices size=200: 2 SELECTs
WS12 board linked work index size=10: 16 SELECTs
WS12 board linked work index size=200: 16 SELECTs
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 2308 filtered out; finished in 4.22s
```

## Huddle/enrollment merge (dc288e6cb)

Main subsequently advanced to `fa630c92037cc7524a261050685e302e71bd6bd3`, bringing #185 and #195. Merge `dc288e6cb16abf588f209f24f848661567c3f5cc` retains both sides. The only textual conflicts were module declarations in `campfire/controllers/presenters.rs` and `views/src/lib.rs`: keep our work modules and main's moved status/message-link exports once each. Reviewed huddle/enrollment functions and our association/PR snapshot behavior are preserved. Main's literal template whitespace is retained for byte parity.

```sh
git merge --no-ff origin/main -m 'Merge reviewed huddle and enrollment integrations from main' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>' > .scratch/board-query-followup/logs/huddle-merge.log 2>&1
git add rust/crates/campfire/src/controllers/presenters.rs rust/crates/views/src/lib.rs
git commit --no-edit
git -C .scratch/board-query-clean/source fetch origin rust/ws12-boards > .scratch/board-query-followup/logs/final-clone-update.log 2>&1
git -C .scratch/board-query-clean/source merge --ff-only FETCH_HEAD >> .scratch/board-query-followup/logs/final-clone-update.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/board-query-followup/logs/final-metadata.json
```

Final locked metadata exits 0. The fresh clone is now at `dc288e6cb`; final gates follow below.

## Final workspace and read-count receipts (dc288e6cb)

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8 > .scratch/board-query-followup/logs/final-workspace-test.log 2>&1
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 27s
test result: ok. 2408 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 345.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.67s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1275 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 105.66s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.35s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.88s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
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
Workspace aggregate: 4420 passed; 0 failed; 16 ignored; 61 target summaries; exit 0
```

All three unmasked consuming oracle tests and all nine query regressions ran. No silent seed skip and no ignore was added by WS12. Final main imports two opt-in huddle browser tests, so the final ignore count is 16 rather than the earlier 14; their exact reasons and all existing ignores follow:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

```text
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked -p campfire query_board_followup -- --test-threads=2 --nocapture > .scratch/board-query-followup/logs/final-query.log 2>&1
```

```text
WS12 board agent availability explicit size=10: 4 SELECTs
WS12 board agent availability legacy size=10: 4 SELECTs
WS12 board agent availability explicit size=200: 4 SELECTs
WS12 board agent availability legacy size=200: 4 SELECTs
WS12 board event event/link choices size=10: 3 SELECTs
WS12 board unlinked event/link choices size=10: 2 SELECTs
WS12 board event event/link choices size=200: 3 SELECTs
WS12 board unlinked event/link choices size=200: 2 SELECTs
WS12 board pull_request event/link choices size=10: 3 SELECTs
WS12 board opening notification size=10: 61 SELECTs; 7 user reads
WS12 board pull_request event/link choices size=200: 3 SELECTs
WS12 board opening notification size=200: 61 SELECTs; 7 user reads
WS12 board agent owner choices size=10: 4 SELECTs
WS12 board owner choices size=10: 2 SELECTs
WS12 board owner choices size=200: 2 SELECTs
WS12 board agent owner choices size=200: 4 SELECTs
WS12 board linked work index size=10: 16 SELECTs
WS12 board linked work index size=200: 16 SELECTs
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 2406 filtered out; finished in 4.24s
```

All after-counts in the table match this intermediate capture, after the Google/board and huddle/enrollment merges. The subsequent #197 merge is verified below.

## Populated-provider merge (#197)

Main moved again to `5f908337a953b8672472317b03bd0cd5de34194e`. Merge `31c4e2398` preserves its populated message event/PR providers and new WS12 consumers, with these conflict resolutions:

- `CalendarEvent` keeps both work-link batch APIs and main's room-restricted `for_message_ids` event-reference join.
- `ChannelThread::for_ids` and `WorkThreadEvent::for_ids` retain one empty-safe batch implementation each; no duplicate APIs or per-item reloads.
- The activity presenter keeps one request-wide source preload and its batched display-name cache. Main's borrowed work-event formatter and message/reminder URL query order are preserved; the latter is independently asserted by #197's Rails consumer vector. Its work/nudge room-thread destinations already agree with our version.
- The GitHub presenter keeps main's populated message-card preload and our pane header's single PR association read.

No Rails reference files changed in these main merges. No frozen #187 branch was touched.

```sh
git merge --no-ff origin/main -m 'Merge reviewed populated provider and WS12 consumer batches' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>'
git -C .scratch/board-query-clean/source fetch origin rust/ws12-boards
git -C .scratch/board-query-clean/source merge --ff-only FETCH_HEAD
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/board-query-followup/logs/provider-metadata.json
```

Locked metadata exits 0. The same fresh, isolated clone is fast-forwarded to the merge and all final gates are repeated.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked -p campfire query_board_followup -- --test-threads=2 --nocapture > .scratch/board-query-followup/logs/provider-query.log 2>&1
```

```text
WS12 board agent availability explicit size=10: 4 SELECTs
WS12 board agent availability legacy size=10: 4 SELECTs
WS12 board agent availability explicit size=200: 4 SELECTs
WS12 board agent availability legacy size=200: 4 SELECTs
WS12 board unlinked event/link choices size=10: 2 SELECTs
WS12 board event event/link choices size=10: 3 SELECTs
WS12 board unlinked event/link choices size=200: 2 SELECTs
WS12 board event event/link choices size=200: 3 SELECTs
WS12 board pull_request event/link choices size=10: 3 SELECTs
WS12 board opening notification size=10: 61 SELECTs; 7 user reads
WS12 board pull_request event/link choices size=200: 3 SELECTs
WS12 board opening notification size=200: 61 SELECTs; 7 user reads
WS12 board owner choices size=10: 2 SELECTs
WS12 board agent owner choices size=10: 4 SELECTs
WS12 board owner choices size=200: 2 SELECTs
WS12 board agent owner choices size=200: 4 SELECTs
WS12 board linked work index size=10: 16 SELECTs
WS12 board linked work index size=200: 16 SELECTs
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 2409 filtered out; finished in 4.73s
```

## Final full-workspace gate after #197

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8 > .scratch/board-query-followup/logs/provider-workspace-test.log 2>&1
```

```text
test result: ok. 2411 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 470.25s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.95s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1275 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 136.17s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.64s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.54s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.03s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.74s
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
Workspace aggregate: 4423 passed; 0 failed; 16 ignored; 61 target summaries; exit 0
```

All 16 pre-existing ignores listed in the huddle/enrollment gate remain unchanged. No additional ignore or seed skip was introduced. The full gate also includes #188 inbox vectors and #197 provider/WS12-consumer vectors.

```text
test controllers::human_work_tests::query_round_two::inbox_batches_work_events_and_associations_in_json_and_html ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::message_features::ws12_consumer_tests::ws12_populated_work_inbox_preloads_owner_events_in_constant_queries ... ok
test controllers::message_features::ws12_consumer_tests::ws12_board_message_consumers_match_rails_through_the_real_inbox ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

## Final strict clippy and production-only inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/board-query-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/board-query-followup/logs/provider-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.90s
```

Strict clippy exits 0 with zero warnings.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-query-clean/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/board-query-clean/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/board-query-followup/logs/provider-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 42.81s
```

The guard exits 0. It supplies only crates/manifests plus its explicit asset inputs; vectors, parity and reference tools are unavailable to production code. This is a production `cargo check`, not a linked release-profile build. The final verified source is `31c4e2398fb040cba7d745562df5344d8459deae`; the publication commit adds this report only.

## Cleanup and remaining work

The configured machine-wide rustc throttle was left unchanged; `CARGO_BUILD_JOBS=2`. Final workspace tests used eight threads, focused tests two. Listener ranges remained within 53400–53499. No python model-server process was touched.

Cleanup measured `.scratch/target` at 27G and `.scratch/board-query-clean/source/rust/target` at 40K. With no active WS12 compiler/test processes, these two regenerable directories were removed. Native media inputs, seeds, source clone and raw evidence logs remain. No stash, rebase, frozen-branch edit or extra feature work was performed.

```text
Removed 2 WS12 scratch target directories; no active WS12 compiler/test processes.
Remaining target directories under WS12 .scratch: 0
```

The requested query-review slice is complete. Overall WS12 remains **partial**: the remaining recorder/inbox integration, specifically the BoardSlaNudge model/recorder facts and its flagged inbox adapters, plus SLA rules, recurring SLA/nudge/digest dispatchers and settings, FrozenClock coverage, and atomic source-write emission of `BoardNudgeJob { nudge_id }` remain owned work. This is **not an owner-blocked-only stop**.

Final origin refresh retained `5f908337a953b8672472317b03bd0cd5de34194e`; `git rev-list --count HEAD..origin/main` printed `0`. The branch contains the current main tip through merge commits, keeping both reviewed message/provider behavior and WS12's query fixes.

# WS12: human work pages, handoffs and links

Branch: `rust/ws12-boards`. **WS12 is partial: this slice implements the human handoff/link pages, work index and ordinary work pane. Recorder/inbox integration and automations remain unblocked owned work. This is not an owner-blocked-only stop.** The previous agent work services and tag assignment slice remains intact.

Final production source: `a54aac32d1febe4e404bf7a75b1150bd8853863b`. The final workspace, strict-clippy and release-input checks use a fresh local clone at that source SHA. The final report commit changes documentation only. Source commits were pushed as the slice progressed; no PR was opened.

## Changes by file

| Files under `rust/` | Behavior |
| --- | --- |
| `crates/db/src/models/channel_thread/work_listing.rs` | Public `ChannelThread::visible_work_threads`: active human viewers, current parent-room membership, unbounded relation, open/all/done/agents/boards filters, and Rails `updated_at DESC, id DESC` order. No invented live-room filter. |
| `crates/db/src/models/work_handoff.rs` | Shared `WorkHandoff::receivers_for` uses the real member-agent policy, post/manage/read grants and current owner exclusion, then Ruby Unicode name ordering. The existing transactional handoff writer remains shared with the agent services. |
| `crates/db/src/models/work_thread_link.rs`, `models.rs` | Public `NewWorkThreadLink` and `WorkThreadLink` read/validate/create/destroy APIs. Actual Rails association, enum, per-kind presence/uniqueness and same-room event validations; Unicode title truncation before validation. Link writes do not touch the thread or create activity/audit/history. Thread destruction removes its links. |
| `crates/campfire/src/controllers/work_threads.rs`, `controllers.rs` | Work HTML/JSON index and human handoff GET/POST. Ordered before-actions check human room access, tracking, then manager/current-owner rights before receiver/package validation. The shared writer rechecks rights under the database write. HTML redirect/notice and reset invalid form; JSON fresh real WS11 work payload plus handoff. Rails scalar/list and ActiveRecord finder coercions are preserved, including single-value array recursion and non-flattened multi-value arrays. |
| `controllers/work_threads/links.rs` | Link panel/create/destroy; any human room member may link tracked work. Events are scoped to the parent room, including past/cancelled events. Pull request parsing, identity and claim use real WS15 APIs. Link save, fetch claim and durable `FetchPullRequestJob` are one transaction. Drive URL parsing and credentialed title reads use the merged WS14g APIs; title failures fall back to the URL. |
| `controllers/channel_threads.rs`, `controllers/channel_threads/page_tests.rs`, `controllers/presenters/board_posts.rs` | Replace ordinary work-pane 501s with the shared header/status/owner/history/links/steps/handoff presentation. Reuse board history and link presentation. The previously pending work page now asserts a successful authorized response and inaccessible nonmember response. Existing ordinary non-work and board output remains covered by its complete response vectors. |
| `crates/views/src/work_threads.rs`, `channel_threads.rs`, `lib.rs`; work/handoff/link templates | Rails work index rows and filters, work header, handoff form, link panel, success/invalid Turbo streams. The shared link box supports the standalone panel and existing board header without changing the latter's bytes. |
| `crates/db/src/tests/work_read_test.rs`, `work_thread_link_test.rs`, `tests.rs` | Unbounded 105-row membership/filter/order checks, inactive/bot rejection, and 29 actual Rails link validation/persistence cases. Verify title bytes, uniqueness, cross-room rejection, destruction, unchanged thread timestamps and no activity. |
| `controllers/human_work_tests.rs` | Six seeded FrozenClock tests with the job runner stopped. Compare 90 complete HTTP responses and test authorization before validation, committed handoff history/audit/ledger/jobs, queue rollback, real PR deduped fetching, and real encrypted Google-account title requests/fallbacks. |
| `app.rs`, `app/google_test_support.rs`, three Google socket tests | Expose the merged recorded HTTP transport fixture to the work tests. Make the three existing Google socket tests use the configured test port range, preserving their existing standalone fallback. No Google production client/model code is duplicated. |
| `reference-tools/work/{human_http.rb,link_model.rb,source-hashes.json,discriminate.py}`, `vectors/{human_work_http.json,work_link_model.json}` | Actual Rails producers, 27-source hash ledger, 90 complete HTTP cases, 29 validation/persistence cases and four assertion-level policy discriminators. No response masks or whitespace normalization. |
| `reference-tools/users/ws12_inventory.py`, `plans/ws12-rails-cases.json` | Per-original-declaration port/defer ledger: 488 declarations, 199 ported, 3 existing peer tests, 286 deferred. This slice closes 51 declarations: work index 5, handoffs 13, links 22 and link model 11. |

Domain code does not render HTML. Model writes use the existing transaction environment and FrozenClock. Controllers reuse real WS11 handoff/ledger/delivery and real WS15/WS14g APIs. No new dependency, ignore, production include outside crates, parity mask, pixel work or Rails edit was added.

### Callback and transaction details

The handoff operation persists package, ownership/status event, audit, real sender/receiver ledger snapshots and durable webhook jobs together. A rejected durable receiving-agent enqueue rolls all of those back. Current manager/current-owner rules remain with the shared work writer; the human endpoint does not reimplement agent grant/ledger logic.

Pull request identity is created before link validation, matching Rails. The subsequent successful link save, fetch-request claim and durable fetch enqueue commit atomically, as required by the port's durable-queue contract. A queue rejection leaves the earlier identity but rolls back the new link and claim. This deliberately applies the approved atomic-queue rule to Rails' later enqueue; ordinary successful/error HTTP responses remain byte-identical. Two work threads linking the same PR produce one claimed fetch. Deleting a link retains the shared PR identity.

Drive title resolution checks the linking user's own encrypted Google account, usability and Drive scope, and calls the actual merged Google API. There is no cross-viewer title cache or temporary Drive adapter. The credentialed Rails producer substitutes only `Net::HTTP.start` external transport; the real Google client/controller/account policy runs and its bearer/target are asserted. Rust substitutes only the merged recorded HTTP transport and runs the real client, encryption/account and controller paths. Complete configured-success and configured-failure Turbo bodies are in the 90-case vector.

## Main merges and Rails reference

This run first merges `origin/main` twice with merge commits: `790e9b899` incorporates `3ab3a4db5` (#184 Slack import), and `66bcc65ec8b2829318e3b08f4ddd603009714122` incorporates `0700e59d43cb0a0122036a6c3a8ea06ad3d9fc1a` (#190 Google). Slack import suppression is retained alongside WS12's deferred opener fanout/tag callbacks. Locked metadata passed after all three merges and on the final fresh checkout. The passing pre-#187 full tests/clippy/release-input receipts are preserved separately; final receipts below are rerun on the merged source.

The final checked main is `7c23b0978` (#187). Merge commit `a54aac32d1febe4e404bf7a75b1150bd8853863b` adopts #187's null-title validation and failed-form title rendering, single-recipient computation, and grouped broadcast conditions. Its recorder implementation is identical to main; the only additional work-event code is the previously exposed handoff event constructor. The new human presentation and committed tag-callback reloads are retained. #188 has not merged into this checked main. The frozen `rust/ws12b-board-writes` branch was never checked out or edited. When #188 merges, integrate its ActivityItem callers and removed activity-access SQL with the remaining recorder/inbox work. Already-merged #181 dirty-column writes and snapshot-based broadcasts remain intact.

Pinned Rails is `d7c7de92` plus `_common.md`'s approved drift. Reference image: `ws12-reference:boards-b908ebc2`. Board drift goldens use origin/main's approved board files: #162's nudge tag, #164's body classes and #165's message template/current-room meta. The human work/link sources in this slice have not drifted after the pin; the 27-source ledger verifies the producers' reference sources, including the real Google account/client. No later unrelated Rails drift is silently adopted.

## Remaining owned work and peer integration

- **Recorder/inbox:** extend the shared typed recorder beyond Message/WorkThreadEvent to AgentBudgetNotice, HuddleGrant and BoardSlaNudge through their owning domains' recipient APIs; verify remaining recipient/idempotency/grouping/repointing declarations and replace the flagged peer adapters. Integrate #188's ActivityItem callers once merged. Existing activity APIs and prior work grouping are preserved, but this slice does not complete the recorder or claim all inbox cases.
- **Automations:** implement BoardSlaRule, BoardSlaNudge and BoardStaleDigest models/controllers, dispatchers, push integration and recurring registration. Commit `BoardNudgeJob { nudge_id }` atomically with its source write, and use FrozenClock for every clock boundary. None of these automations is newly implemented here.
- **Parity ledger:** 286 declarations remain explicitly deferred with reason/owner in `plans/ws12-rails-cases.json`; an aggregate HTTP comparator does not claim every original system declaration. Pixel work remains cut. WS11-API owns its REST/MCP surface integration; the previously exposed WS12 model functions remain available.
- **Main coordination:** #187 is merged and its review fixes are adopted. Integrate #188's inbox callers when merged. Its pending PR does not block the unimplemented owned recorder/automation work.

No new owner question is needed for the next slice. **This stop is a coherent partial slice, not owner-blocked-only.**

## Verification receipts

Commands below ran in this session from the assigned worktree. Logs are retained under `.scratch/logs/`. Cargo jobs were 2 under the configured machine-wide rustc throttle; tests used 4 threads and listeners used 53400–53499. The default and first_run seeds were independently verified and copied into the fresh checkout. Final broad checks use the final committed production source, rather than an earlier partially built tree.

### Fresh checkout and locked metadata

```sh
git clone --local --no-hardlinks --single-branch --branch rust/ws12-boards . .scratch/human-work-clean/source
mkdir -p .scratch/human-work-clean/source/rust/parity/.seed .scratch/human-work-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run .scratch/human-work-clean/source/rust/parity/.seed/
git -C .scratch/human-work-clean/source fetch origin
git -C .scratch/human-work-clean/source merge --ff-only origin/rust/ws12-boards
git -C .scratch/human-work-clean/source rev-parse HEAD
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/human-work-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/logs/human-work-fresh-metadata.json
```

```text
a54aac32d1febe4e404bf7a75b1150bd8853863b
```

The clone was fast-forwarded after each correction. Final metadata exits 0 and emits JSON without a textual summary. Its only untracked directory is regenerable `.scratch/`.

### Verified Rails seeds and byte-identical oracle recapture

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/logs/human-work-default-seed-final.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/logs/human-work-first-run-seed.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/human-work-final-oracle.json 2> .scratch/logs/human-work-oracle-final.log
cmp .scratch/human-work-final-oracle.json rust/vectors/human_work_http.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/human-work-links-model-recaptured.json 2> .scratch/logs/human-work-link-model-oracle-final.log
cmp .scratch/human-work-links-model-recaptured.json rust/vectors/work_link_model.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/human-old-board-write-oracle.json 2> .scratch/logs/human-old-board-write-oracle.log
cmp .scratch/human-old-board-write-oracle.json rust/vectors/boards_write.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/recorder.rb > .scratch/human-work-recorder-oracle.json 2> .scratch/logs/human-work-recorder-oracle.log
cmp .scratch/human-work-recorder-oracle.json rust/vectors/boards_recorder.json
```

```text
  "passed": 29,
  "failed": 0
```

```text
  "passed": 4,
  "failed": 0
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
Rails work link model oracle: 29 validation/persistence cases; 0 masks
Rails board write oracle: 96 complete HTTP responses; no masks
Rails recorder oracle: 3 fanout traces; unread/read repoint activity frames=0/1
```

All commands exit 0; all four `cmp` checks exit 0 with no output. The expanded board vector preserves all original 82 responses unchanged and adds 14 #187 review cases. Response comparisons include complete body bytes, status, location, content type and cache-control. Fixed nonce/form tokens use the existing approved test helpers; the Rails capture skips CSRF verification only for its scripted reference writes, and Rust HTTP tests use real token CSRF. This is behavior/response parity, not pixel work.

### Original board responses retained through #187

```sh
python3 - <<'PY' > .scratch/logs/human-work-original-board-preservation.log
import json,subprocess
from pathlib import Path
original=json.loads(subprocess.check_output(['git','show','1d04bc61b:rust/vectors/boards_write.json']))
current=json.loads(Path('rust/vectors/boards_write.json').read_text())
assert len(original['rows'])==82
assert original['coercions']==current['coercions']
assert original['reference']==current['reference']
for row in original['rows']:
    assert row in current['rows'],row['name']
print('Original board oracle: 82/82 complete Rails responses unchanged')
print(f"Expanded board oracle: {len(current['rows'])} complete Rails responses; {len(current['rows'])-82} review cases added")
PY
```

```text
Original board oracle: 82/82 complete Rails responses unchanged
Expanded board oracle: 96 complete Rails responses; 14 review cases added
```

### Security-relevant failures before correction

```sh
python3 rust/reference-tools/work/discriminate.py > .scratch/logs/human-work-discrimination-final.log 2>&1
```

```text
human-membership: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2019 filtered out; finished in 1.82s
human-manager: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2019 filtered out; finished in 1.24s
link-room: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1245 filtered out; finished in 1.27s
work-active-human: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1245 filtered out; finished in 0.10s
WS12 human work discriminators: 4 broken implementations rejected at actual assertions; 0 compile/setup failures; sources restored
```

These are actual assertions with successful compilation/setup, not compiler failures counted as discrimination. Sources were restored; the demonstrated guards remain in the final source. An earlier compiler-SIGKILL attempt is not cited as a successful discriminator run; retry kept the existing throttle.

The complete Rails HTTP comparator also failed before the finder-coercion correction (receiver array: Rust 422, Rails 201), and before ordered callback denials received Rails' HTML content type (Rust `application/json`, Rails `text/html`). The durable PR test failed at the actual persisted link count before link save and fetch enqueue were made atomic. These were executed earlier in this session on the then-current source, not rerun against the fixed source as purported failures:

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire human_work_http_matches_complete_rails_responses -- --test-threads=4 > .scratch/logs/human-work-coercion-red.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/human-work-clean/source/rust/Cargo.toml --locked -p campfire human_work_http_matches_complete_rails_responses -- --test-threads=4 > .scratch/logs/human-work-header-red.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/human-work-clean/source/rust/Cargo.toml --locked -p campfire human_pr_link_and_fetch_claim_roll_back_when_the_durable_job_is_rejected -- --test-threads=4 > .scratch/logs/human-work-pr-atomic-red.log 2>&1
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2019 filtered out; finished in 34.98s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2019 filtered out; finished in 42.75s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2020 filtered out; finished in 1.62s
```

The earlier broad attempts were intentionally interrupted to add those missing checks; their logs are not passing workspace receipts. The first Google focused run inherited an existing seed grant in its negative fixture; account setup was isolated before the final fresh-source full run. No failing/partial check is represented below as passing.

### Full workspace on final committed source

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/human-work-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/logs/human-work-workspace-test.log 2>&1
```

```text
test result: ok. 2129 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 604.91s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.36s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1269 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 196.98s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.64s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.48s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.37s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.47s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.62s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.29s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.70s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Exit 0. Sum of all 61 raw summaries: **4125 passed, 0 failed, 14 existing ignored**; includes vendored html5ever. All six new app tests and both new database tests ran; none silently skipped for missing seeds. The merged #187 regressions and prior concurrent stale-instance/idempotency tests also pass. Selected actual test lines:

```text
test controllers::channel_threads::board_read_tests::board_post_forms_pages_and_panes_match_complete_rails_http_responses ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::human_work_tests::human_drive_title_uses_the_real_google_api_and_linkers_encrypted_account ... ok
test controllers::human_work_tests::human_handoff_authorization_precedes_receiver_validation_and_creates_nothing ... ok
test controllers::human_work_tests::human_handoff_http_commits_history_audit_ledger_and_job_together ... ok
test controllers::human_work_tests::human_links_save_no_activity_and_claim_one_real_pr_fetch_across_threads ... ok
test controllers::human_work_tests::human_pr_link_and_fetch_claim_roll_back_when_the_durable_job_is_rejected ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_mutations_test::concurrent_recorders_are_idempotent_and_keep_handled_sources_handled ... ok
test tests::work_mutations_test::concurrent_stale_instances_produce_one_event_for_identical_work_changes ... ok
test tests::work_read_test::workspace_work_relation_rechecks_membership_is_unbounded_and_rejects_inactive_or_bot_viewers ... ok
test tests::work_recorder_review_test::recorder_unread_repoint_broadcasts_only_when_read_state_changes ... ok
test tests::work_recorder_review_test::recorder_fanout_queries_stay_within_rails_counts ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

Existing ignored declarations (none added by WS12; two pending WS11-API Google consumer tests came from merged #190):

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

### Strict clippy and production-only inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/human-work-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/logs/human-work-clippy-final.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/human-work-clean/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/human-work-clean/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/logs/human-work-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.90s
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.92s
```

Both checks exit 0 on the final production source. Strict clippy has zero warnings. The release guard checks the production crate with only crates/manifests and allowed separate asset inputs, without test vectors, parity seeds or reference tools; this is `cargo check`, not a linked release-profile build.

### Inventory

```sh
python3 rust/reference-tools/users/ws12_inventory.py > .scratch/logs/human-work-inventory-final.log
```

```text
WS12 Rails inventory: 488 declarations; 286 deferred; 3 existing peer tests; 199 ported
```

## Cleanup and stop

All test/build/reference commands completed before cleanup. The WS12 Docker-name check is empty, and the 53400–53499 listener check contains only its header. Measured and removed the 39G `.scratch/target` plus the 40K diagnostic-only `.scratch/human-work-clean/source/rust/target`. Deletion was bounded by explicit resolved paths inside this worktree, Cargo marker checks for the build target, and the exact four regenerable JSON filenames for the diagnostic target; active process checks use executable names/paths rather than matching shell command text. Both directories are verified absent. Raw cleanup lines:

```text
WS12 active cargo/rustc/test processes: 0
Removed /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target; exists=False
Removed /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/human-work-clean/source/rust/target; exists=False
Remaining target directories under WS12 .scratch: 0
```

Logs, reference captures, source checkout, native media tools and verified seeds remain. No test process is left running. The Python model server was not touched, no stash was used, and the frozen #187 branch was not edited. The report commit is documentation only. **Overall WS12 remains partial with unblocked owned recorder/inbox and automation work, not owner-blocked-only.**

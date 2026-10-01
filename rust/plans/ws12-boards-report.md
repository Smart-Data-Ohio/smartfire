# WS12 continuation: board post reads — partial

Branch: `rust/ws12-boards`. Post-read implementation: `8dddfd4e373a75d03cb1ae9bb4116f50e5631da4`. Verified source includes integration/fixture commit `e2e3633b7bb6fe91df421c591d5048bc321be3b0` (pushed).

**Partial; not owner-blocked-only.** Substantial WS12-owned work remains. The room/list slice is preserved in the [previous report](https://github.com/Smart-Data-Ohio/smartfire/blob/0df11aec95f49c6f1d62dfa91a3cd3a9948e8116/rust/plans/ws12-boards-report.md). Historical commands there are not represented as rerun here.

## Changes by file

- `crates/db/src/models/channel_thread/work.rs`: shared read policies for active human parent-room members; manager/owner work controls; manager-only assignment; human/agent choices. Uses merged WS11's `Agent::for_user`, `active` and `can("post_messages", room)`, including revoked grants, suspension, membership removal and the existing legacy-grant fallback.
- `crates/campfire/src/controllers/channel_threads.rs` and `channel_threads/writes.rs`: complete board HTML show/new forms, Turbo frames, access denial and initial Markdown query coercion. Post/work mutations remain unfinished.
- `controllers/presenters/board_posts.rs`: result Markdown through existing rich-text APIs, run URL, owner availability, audit-history display kinds, messages/agent steps/composer, linked Drive/PR/event displays and unlinked upcoming-event choices. Private PR titles stay hidden.
- `crates/views/src/channel_threads/board.rs`, `templates/channel_threads/{new,board_post}.html`, `templates/threads/work/links/_box.html`: exact layout/forms, tags/status/owner/lifecycle controls, pinned result, history/links, pending-message template/current-room metadata, streams and composer. `_conversation.html` preserves Rails' empty-collection whitespace and normal/anchored panes.
- `controllers/messages/payload.rs`: shared work read policies and real Agent APIs replace duplicate eligibility SQL.
- `crates/db/src/database.rs`, `models/channel_thread.rs`, `models/channel_thread/board.rs`, `models/thread_tag.rs`: main-merge integration preserves WS11 ordered per-record callback chains and WS12 last-save/coalescing callbacks under distinct API names. Board removal frames share the thread's ordered callback record and run before a failing agent-ledger hook. Savepoint rollback and committed-job wakeup behavior remain intact.
- `crates/db/src/tests/work_read_test.rs`: four real DB tests for manager/owner/membership policy, agent eligibility and legacy fallback, unavailable owners, and destruction callback failure order. Fixture SQL creates read states; it does not prove owner mutations.
- `crates/db/src/tests/agent_lifecycle_test.rs`: commit approval creation/inbox fanout before applying oracle IDs, then assert the retargeted inbox row exists. This fixes a WS11/WS12 fixture interaction without changing production validation or lifecycle expectations.
- `controllers/channel_threads/{board_read_tests,page_tests}.rs`: 29 complete Rails HTTP responses and updated board/ordinary-work seam expectation. Response status/body comparisons are literal; mismatch files are test-created diagnostic outputs.
- `reference-tools/boards/{post_views.rb,post-source-hashes.json,post_browser.mjs,post_discriminate.py}`, `browser.py --posts`, `vectors/boards_post_read.json`: actual Rails oracle, hash ledger, browser interactions and deliberately broken implementations. Browser selectors use stable board IDs.
- `reference-tools/users/ws12_inventory.py`, `plans/ws12-rails-cases.json`: per-declaration owner/evidence. **488 declarations: 76 ported, 3 existing peer cases, 409 deferred.** Three newly closed declarations cover board/channel/nonmember new forms, board header/result/manage controls, and unavailable human/agent owners. Agent-candidate cases that call work writes remain deferred.

No Rails, schema, dependency, lockfile, asset, mask or normalization changes, and no new ignores. No pixel work or PR. The Python model server was not touched.

## Reference and merges

Reference is `d7c7de92` plus approved status drift `2e20b24c` and board drift #162/#164/#165. The board/work/activity files changed since the pin are:

| File | Reference |
| --- | --- |
| `app/models/board_automations/nudge_pusher.rb` | origin/main, #162 notification tag |
| `app/views/channel_threads/_board_post.html.erb` | origin/main, #164 body class and #165 message template/current-room meta |
| `app/views/channel_threads/new.html.erb` | origin/main, #164 body class |

Those actual origin/main files are in `ws12-reference:boards-b908ebc2`; other sources remain pinned. The post probe validates the existing 18-source ledger plus nine post/controller/helper/link/conversation hashes before generating responses. #176 and #183 contain no newer Rails drift in these files.

Requested main `2b051607` was already included. Merged #176/main `7442031dfefe4454a6fc67743a74de322e978ad4` with merge commit `1709790a`, resolving callback and thread create/destroy overlaps while retaining both sides' behavior. Merged #183/main `59ad94de3d0c53dfc15cae95901453f6506a7e83` with merge commit `cb0fa7c5`; that merge only changes test-runner coordination. Locked metadata was rerun after each merge; no lockfile change.

#181 remains OPEN. Its UserStar/ActivityItem dirty-column writes and operation-snapshot broadcasts remain on `rust/ws12a-stars-activity`; this branch does not duplicate those review fixes. Take main's versions when #181 merges. WS11 is merged: agent mutation integration is now unblocked.

## Design and writer audit

Policies/owner choices are HTML-free. Presenters gather read facts; templates format them. Session CSRF/nonces remain request-local; no new cache or broadcast carries them.

| Model | Production writer/validation/callback boundary | Remaining owner |
| --- | --- | --- |
| ChannelThread | Read policies/display; existing main writers retained. Destruction callback order changed and tested with a failing committed agent hook. | WS12: post/result/work mutations, full validations, status stamps, independent stale-instance events and deletion side effects |
| User/Agent/AgentGrant/Membership | Read through existing APIs; tests use actual grant revoke and membership destruction | Existing writer owners; eligible-owner assignment validation remains WS12 |
| WorkThreadEvent/WorkHandoff/WorkThreadLink | Legal read fixtures only; no new production writers | WS12: authorization, validations, audit writes and callbacks |
| SLA/nudges/digests/jobs | No new writers | WS12 with WS17: validations/callbacks and source-transaction BoardNudgeJob persistence |
| ActivityItem/recorder/UserStar | Earlier slice retained without edits; upstream review fixes awaited | WS12 full recorder; WS11-UI consumes domain APIs |

Earlier board room/thread/tag validation coverage remains documented in the previous report. Read-fixture SQL does not close mutation behavior. No new enqueue is introduced; pending BoardNudgeJob must be persisted inside its source transaction.

## Precisely remaining, in requested order

1. **Board mutations (WS12):** post creation with first message/owner/tags; metadata/status/owner/lifecycle/delete endpoints; result edit/clear with timestamps, events, recipients and run URL writes; tag coercion/assignment/auto-assignment. Original four board system scenarios that submit creation/result writes remain deferred; the new read browser checks do not close them.
2. **Human work (WS12):** convert/remove tracking; eligible owner validation/writes; manager-only assignment/removal and owner status writes; status stamps; independent stale-instance events and concurrency tests; handoffs, link writes and full audit trail. Ordinary-room work HTML remains unfinished.
3. **Automations (WS12 with WS17):** SLA rules, nudges/digests, forms/controllers, recurring dispatchers, NudgePushJob and atomic `BoardNudgeJob { nudge_id }`. Queue-insert failure through HTTP, FrozenClock timing, recipients, retries/idempotency and cleanup remain.
4. **Recorder/inbox (WS12 with WS11-UI/WS13):** source authorization/fallback association, notification/agent_work/followed-member recipients, grouping/repointing/source hooks, concurrent idempotency, UI access/state adapter replacement, inbox/helper/indicator/browser parity and invitation integration.
5. **Agent work (WS12 with WS11-API, now unblocked):** real WS11 board-post/work-thread/handoff/working-presence services and the ten WS8b-m thread-work declarations. Read owner choices are complete; mutations are not.

Every remaining Rails declaration has an individual owner/reason in `plans/ws12-rails-cases.json`. No unresolved product question is blocking this read slice.

## Verification receipts

Commands below were run for this continuation. Reference-built `default`/`first_run` seeds are present; `CI=1` rejects silent seed skips. Render-only nonce/token inputs are fixed identically on both sides, without output masks or normalization. Browser sessions and assets are real.

### Actual Rails HTTP oracle

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/post_views.rb > .scratch/board-post-views-final.json 2>.scratch/logs/posts-oracle-final.log
cmp .scratch/board-post-views-final.json rust/vectors/boards_post_read.json
```

```text
Rails board post read oracle: 29 complete HTTP responses; approved board drift; no masks
```

Both exit 0. Cases cover manager/member/frame/prefilled forms, channel/nonmember denial; manager/member/owner/observer post controls, closed/locked/empty/unavailable human or agent, suspended/revoked agents, result/run/history/Drive/public-private PR/event/upcoming choices; normal/anchored/empty panes. The probe requires expected 200/404 statuses, rejecting broken-fixture goldens.

### Real browser behavior

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 rust/reference-tools/boards/browser.py --posts > .scratch/logs/posts-browser.log 2>&1
```

```text
Rails board read browser scenarios:
new-post-form-and-cancel: passed
post-discussion-template-and-controls: passed
real-pane-fetch-and-anchor: passed
WS12 browser board posts: 3 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, navigation, controls and pane fetches
Rust board read browser scenarios:
new-post-form-and-cancel: passed
post-discussion-template-and-controls: passed
real-pane-fetch-and-anchor: passed
WS12 browser board posts: 3 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, navigation, controls and pane fetches
```

Owner/tag choices, Cancel navigation, pending-message template/current-room meta, result disclosure/controls and real pane/anchor fetches ran. Unfinished write forms were not submitted. Initial selector failures also occurred on Rails; those harness fixes are not counted as product regressions.

### Failing first and rejecting regressions

The HTTP comparison first reached Rust's missing 501 response versus Rails 200:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1833 filtered out; finished in 2.20s
```

```sh
env CARGO_TARGET_DIR="$PWD/.scratch/target" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 rust/reference-tools/boards/post_discriminate.py > .scratch/logs/posts-discrimination.log 2>&1
```

```text
WS12 post mutation assignment-policy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1169 filtered out; finished in 0.07s
WS12 post mutation revoked-agent-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1169 filtered out; finished in 0.08s
WS12 post mutation deletion-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1169 filtered out; finished in 0.09s
WS12 post discrimination: 3 mutations rejected; sources restored
```

Mutations permit owner reassignment, ignore revoked grants or defer removal frames behind a failing ledger. Each reaches a real failing assertion; sources restore in finally. Compilation/harness failures are not counted.

### Seed verification

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/logs/posts-default-seed-check.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/logs/posts-first-run-seed-check.log 2>&1
```

```text
default:
  "passed": 29,
  "failed": 0
first_run:
  "passed": 4,
  "failed": 0
```

Both exit 0; checked again against actual Rails during this continuation.

### WS11 fixture integration failure and fresh main baseline

The first fresh workspace run at `8dddfd4e` executed the app successfully, then three DB lifecycle tests failed in fixture setup with `RecordInvalid(Errors([("source", "must exist")]))`:

```text
test result: ok. 1831 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 799.30s
test result: FAILED. 1163 passed; 3 failed; 4 ignored; 0 measured; 0 filtered out; finished in 140.56s
```

The WS11 fixture renamed approvals' primary keys inside their creation transaction; the captured after-commit fanout still used the now-absent original IDs. A separate no-hardlinks clone was explicitly checked out at origin/main `59ad94de`, with the same seeds, before diagnosing the interaction:

```sh
git clone --quiet --local --no-hardlinks --branch main . .scratch/posts-main-baseline
git -C .scratch/posts-main-baseline checkout --detach 59ad94de3d0c53dfc15cae95901453f6506a7e83
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/posts-main-baseline/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/posts-main-baseline/rust/Cargo.toml --locked -p campfire_db agent_lifecycle_test:: -- --test-threads=4 > .scratch/logs/posts-origin-main-lifecycle.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1144 filtered out; finished in 0.34s
```

This is a WS11/WS12 fixture interaction, not an inherited main failure. Commit `e2e3633b` commits normal approval creation/fanout before applying oracle IDs, and asserts real retargeted inbox rows exist. It preserves production validation and all lifecycle vectors/assertions. The complete fresh workspace rerun below includes this correction.

### Fresh-checkout broad verification

A no-hardlinks local clone at `.scratch/posts-clean` began at committed `8dddfd4e`; only the verified `default`/`first_run` seeds were copied. After main integration and the fixture correction it was fast-forwarded to committed `e2e3633b7bb6fe91df421c591d5048bc321be3b0`. No test input depends on .scratch or earlier diagnostic outputs; tests create transient DB/files themselves. All builds use one assigned Cargo target, the configured rustc throttle, two build jobs and four test threads.

```sh
git clone --quiet --local --no-hardlinks --branch rust/ws12-boards . .scratch/posts-clean
git -C .scratch/posts-clean fetch origin rust/ws12-boards
git -C .scratch/posts-clean merge --ff-only FETCH_HEAD
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/posts-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/posts-clean/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/logs/posts-final-workspace-test.log 2>&1
```

```text
test result: ok. 1831 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 585.36s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.95s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1166 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 141.99s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.49s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.99s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.28s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.36s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.55s
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
WS12 fresh workspace totals: 3708 passed; 0 failed; 12 existing ignores; CI seeds present
```

Exit 0. All 3,708 tests executed; no seeded silent skips. This includes the 29-response test, four work-read DB tests, room/list and prior stars/activity coverage, merged WS11 tests, all workspace unit/integration suites and doctest commands. The 12 existing ignores are app/library Rails Cable recordings, explicit Node gateway suite, push latency measurement, four Rails scenario/export/fixture/two-factor rollback cases, Pebble TLS/ACME, mail Rails export, and two kit documentation examples. No ignores were added.

### Fresh Clippy

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/posts-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/posts-clean/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/logs/posts-fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 10s
```

Exit 0, all workspace/test targets, without allowances/exclusions.

### Production release-input guard

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/posts-clean/.scratch" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" bash .scratch/posts-clean/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -p campfire > .scratch/logs/posts-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.71s
```

Exit 0. The temporary copy contains only Cargo manifests/crates plus the declared asset build context. Production code builds without vectors, parity seeds, reference tools or undeclared Rails files. The guard removed the temporary input copy. No Rust source changed after the complete fresh verification.

### Inventory, lockfile and formatting

```sh
python3 rust/reference-tools/users/ws12_inventory.py
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" mise exec rust@1.98.1 -- cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 > .scratch/logs/posts-final-metadata.json
mise exec rust@1.98.1 -- rustfmt --edition 2024 --config skip_children=true --check rust/crates/db/src/models/channel_thread/work.rs rust/crates/db/src/tests/work_read_test.rs rust/crates/db/src/tests/agent_lifecycle_test.rs rust/crates/campfire/src/controllers/channel_threads/board_read_tests.rs rust/crates/campfire/src/controllers/presenters/board_posts.rs rust/crates/views/src/channel_threads/board.rs
git diff --check
```

```text
WS12 Rails inventory: 488 declarations; 409 deferred; 3 existing peer tests; 76 ported
```

Metadata exits 0 (13 workspace members), formatting exits 0 on the six listed modules and diff check exits 0; the silent checks produce no summary text. No lockfile change. Final main check is `59ad94de`; #181 remains OPEN.

## Cleanup and final boundary

```text
WS12 post cleanup: no listeners on 53400-53499; no running ws12 containers
34G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target
3.4M	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/rust/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/posts-clean/rust/target
WS12 post cleanup: removed .scratch/target; moved rust/target and .scratch/posts-clean/rust/target captures into .scratch/posts-response-diffs; no release-input copy remains
```

The Cargo target is deleted; fresh/working target diagnostic outputs were moved into this worker's .scratch, so neither target directory remains. Fresh source clones, pinned media runtime, raw logs and response captures remain for review. No own listeners/containers/test processes or release-input copy remain. All code/test/tooling commits are pushed; the final report commit only changes documentation. This remains **partial and not owner-blocked-only**: the WS12-owned mutations/services listed above remain. Agent integration is unblocked by #176; #181 review fixes remain upstream.

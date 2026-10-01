# WS12 continuation: board rooms, lists and commit callbacks — partial

Verified source/pushed code: `759d9d5395fa6735128da063c0cbdf33d4e405ca`, branch `rust/ws12-boards`.
This continues the separately reviewed stars/activity slice at `6f211929` (`rust/ws12a-stars-activity`).
The full WS12 assignment is **partial**. WS12-owned implementation remains; this is **not owner-blocked-only**.

## What changed

- `crates/campfire/src/controllers/rooms/boards.rs`, controller bindings and board dispatch in `rooms.rs`: board room show/new/create/edit/update, creation restrictions, creator/admin updates, memberships, room/membership audits, board-only scope, icon validation, correct forms and sidebar/header broadcasts. Board rooms retain their STI type; actors may remove their own memberships as Rails permits.
- `crates/db/src/models/channel_thread/board.rs`: full-relation status/owner/tag filters, cumulative 50-row windows plus a probe, page clamp/coercion, grouped tags/replies/links, owner availability, and board creation/update/deletion row callbacks. New posts mark only visible, disconnected, unmuted non-creators unread and publish only those unread signals.
- `crates/db/src/database.rs`: record callback registration/coalescing, keeping first callback position and the last save's flags; savepoint rollback restores earlier registrations. This after-commit API is for callbacks/broadcasts. Durable jobs must still be persisted inside the source transaction.
- `crates/db/src/models/channel_thread.rs`, `thread_tag.rs`, `broadcasts.rs`: board callbacks registered from actual model writes; tag parent validation and tag create/destroy row replacements; dependent tag deletion does not replace deleted post rows. `Partial::BoardRow` supplies plain rendering inputs.
- `crates/campfire/src/channels/sink.rs`, `controllers/presenters/boards.rs`: background row rendering through the same presenter as HTTP. Tags/reply/link counts and human owner facts are grouped. Agent availability reads use existing public Agent APIs, with one lookup per distinct agent; no WS11 code was copied.
- `crates/views/src/rooms/boards.rs`, board index/nav/row/new/edit/form templates, `rooms.rs`, `messages.rs`, and room header overflow: explicit board kind and paths; exact list/column/form HTML; empty/filtered boards, cumulative pagination, unavailable owners, tags/counts, digest display, member panel, sidebar and real signed Cable stream. The temporary shared edit-layout extension from the first commit was removed; the shared layout now matches main again.
- `crates/db/src/tests/board_test.rs`, `controllers/rooms/boards_{read_tests,domain_tests,rails_cases}.rs`: 14 new DB tests and 23 new app tests. The reusable socket helper in `opens_rails_cases.rs` gained a channel argument while preserving its existing Turbo-channel entry point.
- `reference-tools/boards/{build_reference.py,source-hashes.json,read_views.rb,domain.rb,browser.py,browser.mjs,discriminate.py}` and `vectors/boards_{read,domain}.json`: real Rails reference/hash checks, complete HTTP and fragment byte vectors, callback/filter oracle, browser interactions and rejecting policy/pagination mutations.
- `reference-tools/users/ws12_inventory.py`, `plans/ws12-rails-cases.json`: each Rails declaration is individually ported or deferred with its owner/evidence.

No Rails edits, schema/dependency/lockfile changes, asset overrides, masks, normalization, allowlist changes or new ignores. No screenshots/pixel comparisons or PR. The Python model server was not touched.

## Rails reference and main integration

Reference: `d7c7de92`, plus approved status drift `2e20b24c` and board drift #162/#164/#165.
The board/work/activity source diff from the pin to `origin/main` contains exactly:

| File | Reference |
| --- | --- |
| `app/models/board_automations/nudge_pusher.rb` | origin/main: approved #162 notification tag |
| `app/views/channel_threads/_board_post.html.erb` | origin/main: approved #164 body class and #165 message template/current-room metadata |
| `app/views/channel_threads/new.html.erb` | origin/main: approved #164 body class |

Those three files were copied from `origin/main` into the actual Rails image. Every other board source in the 18-file ledger is pinned to `d7c7de92`. The cached image tag `ws12-reference:boards-b908ebc2` retains its original name; the rebuilt image's sources are identical to current `origin/main` at `2b05160758307effa035b7d2581618014c46b755`. Both Ruby probes validate the full ledger before running. The approved pane/new-post files are included in the reference; implementing those responses remains below.

Merged main's room timezone/cache fixes and PR-refresh deflake with merge commit `fe2048db`. Locked metadata passed after the merge; no conflict or lockfile change. #176 was checked and remains OPEN, with no merge commit. Eligible-agent owner writes and agent services still wait for that merge.

## Validation/callback boundary for written rows

| Writer | Implemented / retained | Remaining |
| --- | --- | --- |
| Board room create/update | Existing Rails Room validations, icon normalization/resolution, STI and creator rules; explicit grantees; default mentions; membership change audit; actual room/sidebar/header broadcasts | Generic room destruction is existing WS8 code; automation-dependent cleanup remains with WS12 automations |
| Board ChannelThread create/settings/tags/destroy | Existing creator/room/name/archive/tracking/tag validations and status creation stamp; board prepend/replace/remove and unread recipients; tag callbacks/no-op/rollback/savepoint behavior | Work status update stamp and eligible-owner writes, pinned result/event/recorder behavior, tag auto-assignment and work-deletion webhooks |
| ThreadTag create/destroy | Required real parent, presence/format/length/per-post uniqueness; individual after-commit row replacements; no parent touch | No direct tag-update endpoint introduced; HTTP comma-separated tag handling remains with board post endpoints |
| WorkThreadLink / BoardStaleDigest | Read/group/display only; legal rows in isolated fixture setup | No production writer added or accepted for these models in this slice |
| SLA/nudges/digests/jobs | No production writer introduced | Full automation validations/callbacks and atomic `BoardNudgeJob` emission remain WS12 |

Coalescing is proved for repeated saves of the same model instance and savepoint/transaction rollback. This is not acceptance of independent stale-instance work updates, which remain the next domain slice.

## Precise remaining work, in the lead's order

1. **Finish boards (WS12):** board post new/create/show/edit/update/delete HTTP behavior and thread pane, initial/reply content, tags' request coercion, tag assignment rules/auto-assignment, pinned-result write/event/recipients/rendering, run URL writes, and byte vectors for every post/pane/result fragment. Port the four original board system scenarios; the three browser scenarios below cover only the completed read/list behavior.
2. **Human work (WS12):** convert/remove tracking, eligible human owners, manager versus owner policy, status stamps/changes, handoffs, links, work audit trail, and one event per actual stale-instance/independent-writer change. Existing WS8 helpers are not acceptance of this surface.
3. **Automations (WS12 with WS17 contract):** SLA rules/nudges/digests and recurring dispatchers, `NudgePushJob`, and source-transaction `BoardNudgeJob { nudge_id }` persistence. Test queue-insert failure through HTTP, clocks, retries, recipients and idempotency; implement automation forms/controllers and cleanup.
4. **Full recorder/inbox (WS12 with WS11-UI):** all source authorization and fallback association behavior; recipient/notification/agent_work/followed-member policy; grouping/repointing and source hooks; recorder concurrency; replace UI access/state adapters with the existing domain APIs; full inbox/helper/indicator/browser response parity and WS13 invitation integration.
5. **After WS11 #176 (WS12 with WS11-API):** merge main and use WS11's real models for eligible-agent owner writes and board-post/work-thread/handoff/working-presence services; close the ten WS8b-m thread-work declarations. Read-only seed owner labels use existing APIs; no duplicate WS11 implementation.

The inventory contains **488 Rails declarations: 73 ported, 3 existing WS7 peer cases, 412 deferred**. This adds 33 completed declarations to the prior 40: 16 board room controller, 3 board sidebar, 4 board room model, 2 tag model and 8 board-thread declarations. Each other declaration is deferred individually with its owner. The agent grant/candidate mutation cases are explicitly not closed by seed owner-display comparisons.

## Verification receipts

All commands here were rerun. `default` and `first_run` seeds are present; `CI=1` makes absence fail. Render-only oracle tokens/nonces are fixed identically at input; the response bodies are compared as raw strings with no masks or normalization. HTTP comparisons assert raw response bodies, statuses and redirect Locations. Rust write tests use real session-bound CSRF. Rails form-golden writes bypass only the oracle controller's CSRF verification so deterministic fixture token strings can be rendered; browser and actual controller tests use real security paths.

### Actual reference and seed verification

```sh
python3 rust/reference-tools/boards/build_reference.py
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run
```

```text
WS12 board reference: 18 source hashes verified; d7c7de92 plus approved status/board drift
default:
  "passed": 29,
  "failed": 0
first_run:
  "passed": 4,
  "failed": 0
```

### Rails byte/domain oracles

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/read_views.rb > .scratch/board-read-views.json 2> .scratch/logs/boards-read-oracle-final.log
cmp .scratch/board-read-views.json rust/vectors/boards_read.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/domain.rb > .scratch/board-domain-final.json 2> .scratch/logs/boards-domain-oracle-final.log
cmp .scratch/board-domain-final.json rust/vectors/boards_domain.json
```

```text
Rails board read oracle: 22 row fragments and 25 complete HTTP responses; approved board drift; no masks
Rails board domain oracle: 9 filters and 8 committed/rolled-back callback sequences; reference d7c7de92
```

Both literal comparisons exit 0 without output. Full response cases include list/columns, odd array/page query shapes, filters/empty boards, cumulative pagination, digest, Turbo frames, new/edit forms for admin/member, redirects, blank-name writes and invalid-icon errors. Rows cover all four statuses, escaped titles, no owner, inactive/revoked human owners, closed/locked lifecycle and existing links. FrozenClock domain vectors compare actual filters, ordering and transaction callback sequences; the query oracle uses Rails' actual loaded relation, not a join-duplicating pluck shortcut.

### Focused seeded tests

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire boards_ -- --test-threads=8
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db board_test:: -- --test-threads=8
```

```text
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 1680 filtered out; finished in 35.13s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 960 filtered out; finished in 0.73s
```

37 new owned Rust tests ran (23 app, 14 DB); the app filter also selects 4 existing peer tests. None skipped/ignored. The live socket test subscribes to the real authorized RoomMessagesChannel, checks create/tag/destroy HTML and rollback silence. Separate controller sockets verify memberships' sidebar/header frames.

### Browser behavior (completed read slice)

```sh
env CARGO_TARGET_DIR="$PWD/.scratch/target" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 rust/reference-tools/boards/browser.py
```

```text
Rails:
WS12 browser board reads: 3 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, GET forms and Stimulus
Rust:
WS12 browser board reads: 3 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, GET forms and Stimulus
```

Real filter submissions, list/column navigation and all four columns match. The Stimulus check inserts seeded row shapes with changed status/owner/tag attributes, verifies rejection and acceptance, and uses the original controller; actual server socket delivery is tested separately. This is not a claim to have ported the original new-post/result/pane scenarios.

### Rejecting regressions / fail first

```sh
env CARGO_TARGET_DIR="$PWD/.scratch/target" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 rust/reference-tools/boards/discriminate.py
```

```text
WS12 board mutation manager: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1706 filtered out; finished in 1.10s
WS12 board mutation room-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1706 filtered out; finished in 1.03s
WS12 board mutation page-probe: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 973 filtered out; finished in 0.10s
WS12 board discrimination: 3 mutations rejected; sources restored
```

Every mutation reaches a failing assertion; no compile/fixture failure is counted. Sources restore in finally. Historical pre-implementation receipts (preserved raw logs):

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 960 filtered out; finished in 0.11s
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 1673 filtered out; finished in 1.97s
```

Those failures expose missing creation/tag/destruction row callbacks and the ordinary-room response at the board URL; the complete HTTP mismatch was 115829 bytes versus Rails' 88063 bytes.

### Fresh-checkout broad verification

A new local clone at `.scratch/boards-clean` was made from committed `759d9d53` with no hardlinks. Only the required reference-built `default`/`first_run` seeds were copied. Tests create their own transient DB/files; no fixture inputs depend on .scratch or prior targets. All builds use the same explicitly assigned worker Cargo target and `CARGO_BUILD_JOBS=2`; the fresh workspace suite uses four test threads.

```sh
git clone --quiet --local --no-hardlinks --branch rust/ws12-boards . .scratch/boards-clean
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/boards-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/boards-clean/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4
```

```text
Verified board fresh checkout/source: 759d9d5395fa6735128da063c0cbdf33d4e405ca
test result: ok. 1704 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 684.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.03s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 970 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 106.71s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.71s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.51s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.50s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
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
WS12 fresh workspace totals: 3385 passed; 0 failed; 12 existing ignores; CI seeds present
```

Exit 0. All 3,385 tests executed, including the new board tests, prior stars/activity tests, all workspace unit/integration suites and doctest commands. No seeded local skips. The 12 pre-existing ignores are the app/library Rails Cable recordings, explicit Node gateway suite, push latency measurement, four Rails scenario/export/fixture/two-factor rollback cases, Pebble TLS/ACME case, mail Rails export, and two kit documentation examples. No ignores were added.

### Fresh Clippy

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/boards-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/boards-clean/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 42.89s
```

Exit 0, including html5ever and all test targets; no lint allowance/exclusion.

### Production release-input guard

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/boards-clean/.scratch" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" bash .scratch/boards-clean/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -p campfire
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

Exit 0. The temporary copy contained only Cargo manifests/crates and the explicitly declared asset build context. Production code built without vectors, parity seeds, reference tools or other undeclared Rails files. The copy was removed by the guard. The only subsequent tooling change added an explicit ws12-prefixed name to the one-shot source-hash Docker probe; its actual build/18-hash check was rerun successfully. No Rust source changed after this fresh verification.


### Inventory

```sh
python3 rust/reference-tools/users/ws12_inventory.py
```

```text
WS12 Rails inventory: 488 declarations; 412 deferred; 3 existing peer tests; 73 ported
```

Locked metadata after merging main and on final source exits 0 (13 workspace members). Rustfmt --edition 2024 --config skip_children=true --check on all eight new Rust modules and git diff --check exit 0. No Cargo.lock change. Final main/WS11 check: origin/main remains 2b051607; #176 is OPEN with no merge commit. Cleanup receipts follow.

## Cleanup and final scope

Cleanup followed the completed suites, stopped browser runners and successful guard. Exact targets and raw receipts:

```text
WS12 board cleanup: no listeners on 53400-53499; no running parity.owner=ws12 containers
33G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/boards-clean/rust/target
WS12 board cleanup: removed .scratch/target and .scratch/boards-clean/rust/target; moved own rust/target failure captures into .scratch/boards-response-diffs; no release-input copy remains
```

The fresh source checkout, raw logs and response mismatch captures remain in this worker's .scratch for review; Cargo targets and release-input copies are gone. All own listeners and containers stopped. The source/test/tooling commits and this report are pushed; no PR is opened. WS12-owned board post/result, human work, automation and full recorder/inbox work remains. Only the agent-owned integration is waiting for #176; this is not owner-blocked-only.

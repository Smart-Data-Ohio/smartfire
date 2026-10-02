# WS12 wave 4 report — PARTIAL

Branch: `rust/ws12-boards`. Verified source slice: `fabd4d9f9f8b258704b7afefd9801bc9efa55eed` (pushed). This is a coherent stars + inbox-domain slice, not the completed WS12 deliverable. WS12-owned implementation remains; this is not an owner-blocked-only handoff.

## Reference and sequencing

Stars and activity use Rails `d7c7de92`, with checked source hashes in `reference-tools/users/*-source-hashes.json`. Current `origin/main` is `b908ebc2`; merge commit `60a9e6e0` brought in WS13b and preserves both the activity state API and huddle callbacks. Locked Cargo metadata was rerun successfully after the merge.

Approved board drift after the pin is present in origin/main: `app/models/board_automations/nudge_pusher.rb` (#162 tag), `app/views/channel_threads/_board_post.html.erb` (#164 body class), and `app/views/channel_threads/new.html.erb` (#165 message template/current-room-id metadata). Board work must use origin/main's Rails for those files, not the old pinned versions. This slice does not implement or claim comparison of those board paths. The stars browser reference also contains approved #163 status drift; stars/activity source hashes still match the pin.

WS11 #176 remains open at verification time. Agent work services and eligible-agent owners were not copied or implemented ahead of the instructed merge. Stars may target agent users through existing authentication/fixture paths. WS11-UI inbox controller/presenter adapters are absent on this main snapshot, so their integration is deferred.

## Changes by file

Paths below are relative to `rust/`.

- `crates/db/src/models/user_star.rs`: private UserStar create/find/find-or-create/remove and User star readers. `models.rs` exports the model; `tests.rs` registers its tests. Reads/deletes are scoped to the viewer, writes are idempotent, and timestamps are unchanged on repeated stars.
- `crates/campfire/src/controllers/users/stars.rs`, `controllers/users.rs`, `controllers.rs`: real POST/DELETE routing, Rails authentication order, JSON anonymous 401, self rejection, active-human policy, target lookup, create race rescue, JSON/Turbo/HTML responses. Early Rails head responses retain bare text/html even for JSON. Bot key/session and agent Bearer requests are rejected; runtime headers are assembled from fixture inputs.
- `crates/views/src/users/people.rs` and `crates/views/templates/users/stars/_toggle.html`: shared star button plus exact partial and Turbo replace body; existing profile-card formatting is preserved.
- `crates/db/src/models/activity_item/access.rs` and `access.sql`: domain-only accessible relation for all eleven supported polymorphic source types, live room membership and agent owner/admin rules, active human/viewer scope, exact filters/counts and updated_at/id cursor ordering. SQL lives inside the crate, not a controller or external production include.
- `crates/db/src/models/activity_item.rs`: read/unread/handled/unhandled model state functions, unchanged-state timestamp/broadcast suppression, Rails 8 association validation on creation of supported sources and event validation on actual state saves. Methods keep WS13's immutable receiver/returned-row API and huddle invitation/ring-job broadcast branch.
- `crates/campfire/src/controllers/activity_domain_tests.rs`: real seed/model/FrozenClock assertions over all source types, membership revocation, inactive users and agents, owner transfer/no owner, foreign security sources, literal state vectors, exact non-huddle Cable payloads, rollback, cursors, association validation and independent SQLite-writer message-recorder idempotency. This checks the existing message recorder and does not claim a full generic recorder port.
- `crates/db/src/tests/user_star_test.rs` and `crates/campfire/src/controllers/users/stars_tests.rs`: five model tests and seven request/render/security tests, including independent writer and HTTP request concurrency, both dependent star deletions, deactivation retention and no star notifications/broadcasts.
- `reference-tools/users/stars.rb`, `activity_domain.rb`, source hash files and `vectors/users_stars.json`, `activity_domain.json`: fresh outputs from our actual pinned Rails models/controllers/views. Stars compare 21 HTTP response cases plus two fixed-input partials/streams; activity compares nine access snapshots, nine FrozenClock state transitions and fourteen validation/state-save cases. No response normalization, mask or allowlist changes.
- `reference-tools/users/browser_stars.py` and `.mjs`: original four starred-people system interactions on real Rails and Rust listeners, signed Jason session, real CSRF and original Stimulus. Covers profile-card star/unstar and Starred group refetch; inline member row with mouse/keyboard, Escape/focus and self menu suppression; card Escape; phone star/presence/no overflow. Original inline avatar/identity alignment is a DOM assertion only; no screenshots or pixel diff.
- `reference-tools/users/media_runtime.sh`: optional caller owner/namespace and named Docker container, preserving WS8b-r2 defaults; reuses pinned media libraries for native byte comparisons without changing the host.
- `reference-tools/users/discriminate_ws12.py`: seven deliberate source mutations, each required to reach a failing assertion, with restoration in finally.
- `reference-tools/users/ws12_inventory.py`, `plans/ws12-rails-cases.json`: each owned/adjacent Rails declaration listed with file, original line/title, port/defer status, named owner and evidence.

## Validation and callbacks on written rows

| Writer | Rails validation/callback | Status |
| --- | --- | --- |
| UserStar create | Required user and starred_user; viewer/target uniqueness; cannot star self; unique SQLite index | Ported and discriminated; model permits inactive targets and bot starrers as Rails does; HTTP human policy is separate |
| UserStar remove | Viewer-scoped delete_all; no touch or model notifications | Ported; repeated deletion does nothing; other viewers retain their rows |
| User removal/deactivation | Both star associations deleted on hard removal; stars survive deactivation | Existing user lifecycle preserved and checked on real rows |
| ActivityItem supported-source creation | Required user/source, event_type inclusion, unique user/source key | Ported for all eleven supported inbox sources; after-commit broadcast preserved |
| Activity state/type refresh | Inclusion validation; Rails 8 skips unchanged non-null belongs_to checks; update timestamp only on saved change | Ported; disappearing source still permits state update, as the oracle proves; nine exact state/timestamp steps |
| Activity broadcast | Active humans only; user_ID_activity; after commit; only saved watched fields | Checked for exact ordinary payload, changed/no-op state, creation and rollback; preserves WS13 huddle payload/ring path |
| Activity sign-in email callback | after_create_commit configured mail | Existing WS9 authentication + WS10 durable-mail path is retained; centralizing generic creation callback remains WS12 with WS10 |
| Arbitrary unsupported polymorphic source construction | Rails constant/model association validation | Explicitly deferred to WS12 generic recorder; this slice validates the eleven supported source writers and retains the pre-existing fallback behavior |
| Boards/work/tags/SLA/digests/new generic recorder writers | Their full validations, touches, recipients, job and broadcast callbacks | Deferred to WS12; no new writes to those source tables in production code in this slice |

## Precisely remaining

1. Board domain and full board response parity: room board behavior, post create/update/delete announcements and row broadcasts, thread tags/assignments/auto assignment, pinned result, list/pane, board/new-post rendering, sidebar board integration and approved drift reference goldens.
2. Human work tracking: convert/remove work, eligible human owners, status updates, manager-vs-owner authorization, handoffs, links, work audit events and stale-instance/independent-writer change concurrency. Existing WS8 human helpers and flags are not acceptance of this remaining surface.
3. Full ActivityItems::Recorder: source authorization, supported/fallback association behavior, notification/agent_work/followed-member recipient rules, grouping/repointing, work/SLA/source hooks, callback centralization and source-specific idempotency. The added concurrency proof covers the existing message recorder only.
4. WS11-UI integration: replace its flagged access/query/state adapters through these model APIs after its controller/presenters land; finish full inbox render byte vectors, cursor HTTP semantics, helpers, indicator/inbox interactions and the WS13 invitation endpoint cases. Existing ActivityChannel implementation is kept; generic recorder/channel integration remains.
5. Automations: SLA rule/nudge/stale digest models and Rails validations, recurring SLA/digest dispatchers, NudgePushJob and atomic BoardNudgeJob emission, recipient/clock/retry/idempotency coverage and automation views/controllers.
6. After WS11 #176 merges: merge origin/main, use its real models/APIs for eligible-agent owners, agent board-post/work-thread/handoff/working-presence services and payloads; close the ten WS8b-m thread-work declarations and agent REST/MCP delivery integration with WS11-API. No duplicate WS11 implementation was introduced.

`plans/ws12-rails-cases.json` lists all 488 declarations individually: 40 ported, 3 existing WS7 channel cases, 445 deferred. These are Rails declaration counts, not Rust test execution counts. The new owned Rust tests comprise 15 app tests (7 stars + 8 activity) and 5 DB tests; the ws12_ filter also includes one existing peer test.

## Verification receipts

All commands below were executed in this worktree (or the explicit fresh clone within it); summaries are raw.

### Clean checkout and seeded inputs

```sh
git clone --quiet --local --no-hardlinks --branch rust/ws12-boards . .scratch/clean
git -C .scratch/clean pull --ff-only origin rust/ws12-boards
```

```text
Verified checkout/source: fabd4d9f9f8b258704b7afefd9801bc9efa55eed
```

A new clone was created at aacb9139, then fast-forwarded for the media helper and final two-line Clippy cleanup. Only the required `default` and `first_run` Rails seeds were copied into `clean/rust/parity/.seed/`; tests create their own temporary rows/files. No tests depend on the worker's .scratch fixtures. All Cargo targets used the one explicitly assigned worker target.

Both seeds were built from the pinned archive before implementation. The actual Rails validators were rerun, separately on each seed:

### Rails seed validators

```sh
env CAMPFIRE_REFERENCE="$PWD/.scratch/reference" PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws19b-ci-reference rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default
env CAMPFIRE_REFERENCE="$PWD/.scratch/reference" PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws19b-ci-reference rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run
```

```text
default:
  "passed": 29,
  "failed": 0
first_run:
  "passed": 4,
  "failed": 0
```



### Pinned native media runtime

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_IMAGE=ws19b-ci-reference bash rust/reference-tools/users/media_runtime.sh
```

```text
ws12 pinned media runtime: image ws19b-ci-reference; libvips, FFmpeg tools and libraries extracted; no host libraries changed
```

This reuses the existing Rails media extraction helper. The tests load its private libraries/tools; no host libraries were changed and no PNG assertions were loosened.

### Full seeded app and database suites

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/clean/rust/Cargo.toml --locked -p campfire -p campfire_db --no-fail-fast -- --test-threads=8
```

```text
test result: ok. 1670 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 640.07s
test result: ok. 956 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 106.99s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Run at 42f0bedf: 2,626 tests executed, seven existing explicit ignores, no failures. `CI=1` and both required seeds were present, so this is not a missing-seed green result. The seven ignores are the reference Cable recording, explicit Node gateway suite, push latency measurement, Ruby scenario comparison, Rails export, Ruby fixture row comparison and Rails-mutated two-factor rollback fixture. No ignores were added. DB doctests contain zero declarations.

The only subsequent production change was removing two needless generic-argument borrows in star_stream. The final-source WS12 rerun below verifies literal star fragments/streams and all new domain/security tests again.

### Final-source WS12 tests

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/clean/rust/Cargo.toml --locked -p campfire -p campfire_db ws12_ -- --test-threads=8
```

```text
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 1657 filtered out; finished in 14.65s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 955 filtered out; finished in 0.19s
```

Run at fabd4d9f. All 20 new owned Rust tests ran (15 app + 5 DB); the filter also selects one pre-existing peer app test, hence the app summary is 16. No missing-seed skips or own ignores.

### Full workspace Clippy

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/clean/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 32s
```

Exit 0 at fabd4d9f, including html5ever; no exclude flag or lint allowance. The first Clippy attempt identified the two new star-stream borrows, both fixed and pushed before this successful rerun.

### Release input guard

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" bash .scratch/clean/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -p campfire
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 23s
```

Exit 0 at fabd4d9f. The guard copied only Cargo manifests/crates and explicitly declared asset inputs into its temporary source tree, built the production binary, then removed the copy. It had no vectors, parity seeds, reference tools or other Rails files available.

### Actual Rails oracles and literal file comparisons

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws19b-ci-reference rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/users/stars.rb > .scratch/verified-oracles/users_stars.final.json 2> .scratch/logs/stars-oracle-final.log
cmp .scratch/verified-oracles/users_stars.final.json rust/vectors/users_stars.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws19b-ci-reference rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/users/activity_domain.rb > .scratch/verified-oracles/activity_domain.final.json 2> .scratch/logs/activity-oracle-final.log
cmp .scratch/verified-oracles/activity_domain.final.json rust/vectors/activity_domain.json
```

```text
Rails stars oracle: 2 fragments and streams, 4 validations, 21 HTTP responses; reference d7c7de92
Rails activity domain oracle: 9 access snapshots over all 11 source types, 9 state transitions, 14 validation cases; reference d7c7de92
```

Both cmp commands exited 0 with no output. These are fresh literal Rails outputs, not Rust-authored expected values. Token inputs for render-only goldens are fixed identically; actual HTTP/browser tests use real session-bound CSRF.

### Starred people browser parity

```sh
env CARGO_TARGET_DIR="$PWD/.scratch/target" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" python3 .scratch/clean/rust/reference-tools/users/browser_stars.py
```

```text
Rails:
WS12 browser stars: 4 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, CSRF and Stimulus
Rust:
WS12 browser stars: 4 passed; 0 failed; Chromium 153.0.8010.12; real signed sessions, CSRF and Stimulus
```

Run after the release-input guard, using its newly built Rust binary. All four original scenarios include real signed sessions, CSRF, original Stimulus, inline member-row/focus assertions and phone overflow. Both runners stopped; no screenshots or pixel diff.

### Discriminating regressions

```sh
env CARGO_TARGET_DIR="$PWD/.scratch/target" CI=1 python3 rust/reference-tools/users/discriminate_ws12.py
```

```text
WS12 mutation star-self: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.14s
WS12 mutation star-unique: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.13s
WS12 mutation star-association: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.28s
WS12 mutation star-viewer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.33s
WS12 mutation activity-viewer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1672 filtered out; finished in 0.83s
WS12 mutation activity-source: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1672 filtered out; finished in 0.86s
WS12 mutation activity-event: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1672 filtered out; finished in 1.26s
WS12 discrimination: 7 mutations rejected; sources restored
```

The script fixes CARGO_BUILD_JOBS=2, uses at most eight test threads, and restores every source in finally. The four star mutations reject missing self/uniqueness/association validation and cross-viewer deletion; the three activity mutations reject cross-viewer access, missing source validation and invalid-event state saves. Every mutation reached a failing assertion, not a compile/fixture failure.

Historical fail-first receipts before implementation (preserved logs; final-source green rerun is above):

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1578 filtered out; finished in 1.03s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1585 filtered out; finished in 0.73s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1672 filtered out; finished in 1.00s
```

The first three HTTP tests failed because the endpoints were still 501 (private/idempotent mutations, authorization/CSRF and concurrent requests). The activity state test failed on a duplicate same-instant broadcast; the association test failed when a missing Message source was inserted. These are separate from the deliberately broken mutation runs.

### Initial broad-run environment failure and corrected reruns

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53499 MAIL_TEST_PORT_RANGE=53400-53409 GITHUB_TEST_PORT_RANGE=53413-53419 mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/clean/rust/Cargo.toml --locked -p campfire -p campfire_db -- --test-threads=8
```

```text
test result: FAILED. 1662 passed; 8 failed; 3 ignored; 0 measured; 0 filtered out; finished in 481.60s
```

Seven failures were exactly "GitHub test port range exhausted" because the initial allocation had only seven ports for eight parallel tests. The eighth was moon.jpg's full PNG logo bytes under host libvips 8.18. This is reported as a worker harness/environment issue, not an inherited-source exception. No source test was ignored or assertion normalized. The corrected port pool and pinned media both reproduced green cases, followed by the complete successful seeded suites above.

### GitHub wider-port rerun

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/clean/rust/Cargo.toml --locked -p campfire controllers::github:: -- --test-threads=8
```

```text
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 1629 filtered out; finished in 72.92s
```



### Logo pinned-media rerun

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/clean/rust/Cargo.toml --locked -p campfire stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers -- --test-threads=8
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1672 filtered out; finished in 1.83s
```



### Per-declaration Rails inventory

```sh
python3 rust/reference-tools/users/ws12_inventory.py
```

```text
WS12 Rails inventory: 488 declarations; 445 deferred; 3 existing peer tests; 40 ported
```



Other rerun checks: locked Cargo metadata exited 0 (13 workspace members); git diff --check and rustfmt --edition 2024 --check on all eight changed Rust modules exited 0. No lockfile changes.


## Cross-workstream touches and limits

- WS8b-r2: reused existing profile card/member/sidebar behavior and closed its four star browser criteria. Its media extraction helper gained optional worker namespace/owner settings, with default behavior retained. No frontend assets or overrides were changed.
- WS13b: merged returned-row state API, huddle invitation enqueue/broadcast branch and peer ring jobs intact; complete inbox invitation integration remains WS11-UI + WS12.
- WS17: checked its existing message recorder across two independent database writers, including preserving handled state on duplicate delivery. No board nudge job implementation is claimed.
- WS9/WS10: standard CSRF/authentication and sign-in mail job paths are preserved.
- WS11/WS11-UI/WS11-API: consume their existing auth infrastructure; service/controller integration remains as listed.
- No Rails source edits, schema changes, dependency/lockfile changes, masks, allowlists, new ignores or unverified pixel claims. No PR opened.

Cleanup ran after all checks/browser listeners stopped. Exact deletion list: `.scratch/target` (26G Cargo output) and `.scratch/clean/rust/target` (4.0K test-session output). Raw receipts:

```text
WS12 cleanup: no listeners on 53400-53499; no running parity.owner=ws12 containers
26G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target
4.0K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/clean/rust/target
WS12 cleanup: removed own .scratch/target and .scratch/clean/rust/target; no release-input copy remains
```

The fresh checkout and raw logs remain under this worker's .scratch for review; build/test targets and release-input copies are gone. No listeners remain in 53400–53499 and no parity.owner=ws12 containers are running. All branch commits are pushed; no PR was opened.

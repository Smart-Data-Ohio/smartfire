# WS12 #201 review fixes

Both requested P2s are fixed on `rust/ws12-work-reads`. Verified final source: `ce334fce87237cbee3246705302fffacb8488cc3` (includes source fixes `6fab956ed9494c263975337e906fabcff8314f25` and main merge `d27fb97a786794f0afb1dd67c1531a671358444e`); failing-first regression commit: `34e575140`, based on unchanged `3cc1918105caed67e5a008418473beac3e71469d` production. The publication commit adds only this report.

The interrupted automation work is saved, unpushed, as `2d09780b556eea6e17db3877fb1ede164cfb6f44` on `rust/ws12-board-automations-2`, with message **WIP: board automations (interrupted)**. It retains the prior ID-0 fix `25e45a80` and all automation models/tests/reference tools/vectors. This round does not resume automation features or change #200's branch.

## Implementation

The seven affected ID loaders use one JSON-array bind through SQLite's existing `json_each` pattern: `Room::for_ids`, `User::where_ids`, `Agent::for_users`, `ChannelThread::for_rooms`, `ChannelThread::board_reply_counts`, `ChannelThread::board_link_counts`, and `ThreadTag::for_threads`. The opening board notification membership UPDATE is bounded the same way. The other new work-page facts (memberships, membership counts, icon names and agent grants) already use JSON-array binds. Empty-list behavior, ordering, visibility, recipient selection and recorded facts are preserved. A test-support-only API lowers the real reader connection bind limit; it is absent from production builds.

`messages::payload` now resolves brand image paths with `campfire_assets::asset_path` in both its ordinary user serializer and the batched work serializer. GitHub becomes `/assets/icons/brands/github-07b84d6c.svg`. Custom icon URLs remain `/icons/<name>`, and brand names still take precedence over custom name collisions.

## Failing-first boundary and icon regressions

All four new regressions ran against `3cc19181` loaders/serializers before production changes, in `34e575140` with test-only support and corrected committed Rails vectors. The production diff for those loaders and serializers is empty. The receipt is `.scratch/pr201-fixes/logs/before-corrected.log`.

| Probe | Before | After |
|---|---|---|
| Real SQLite compile limit 32,766; 32,767 tracked rows | HTTP 500 | HTTP 200; all 32,767 rows; all 47,703,422 Rails bytes equal |
| Real reader limit 64; 10 / 100 rows, shared room/owner | 200 / 500 | 200 / 200; complete response equal |
| Real reader limit 64; 10 / 100 distinct human rooms/owners | 200 / 500 | 200 / 200; complete response equal |
| Real reader limit 64; 10 / 100 distinct agent rooms/owners | 200 / 500 | 200 / 200; complete response equal |
| Real reader limit 64; 10 / 100 distinct custom icons | 200 / 500 | 200 / 200; complete response equal |
| Each of seven ID loaders at real limit 64; 10 / 100 IDs | Success / too many SQL variables | Success / success |
| 72 complete icon responses/helper payloads | 18 differences, confined to brand and brand-collision cases | 0 differences |

The large Rails response is stored as a deterministic 590,352-byte gzip vector. The Rust test decompresses and compares the entire body, then checks every row is present. It also asserts the actual compiled SQLite parameter ceiling, so it exercises the review's real boundary. The smaller two-size tests lower SQLite itself to 64; they do not substitute a mock limit or assert only a generated SQL string. All apps use `TestApp::boot_frozen().without_job_runner()`; no sleeps or active queue races.

Raw failing-first summaries:

```text
  left: 500
 right: 200
WS12 PR201 SQLite limit=64: "shared" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "shared" rows=100; HTTP 500 Internal Server Error
WS12 PR201 SQLite limit=64: "humans" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "humans" rows=100; HTTP 500 Internal Server Error
WS12 PR201 SQLite limit=64: "agents" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "agents" rows=100; HTTP 500 Internal Server Error
WS12 PR201 SQLite limit=64: "icons" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "icons" rows=100; HTTP 500 Internal Server Error
WS12 PR201 icon callers: 72 complete Rails responses/helper payloads; 18 differences; 0 masks
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 2433 filtered out; finished in 31.90s
```

All four pass again on the final merged fresh-clone source, including the additional preloaded agent/public user checks:

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr201-merged-clean/source/rust/Cargo.toml --locked -p campfire pr201_review -- --test-threads=2 --nocapture > .scratch/pr201-fixes/logs/merged-focused.log 2>&1
```

```text
WS12 PR201 loader limit=64: Room::for_ids rows=10: Ok(10)
WS12 PR201 loader limit=64: User::where_ids rows=10: Ok(10)
WS12 PR201 loader limit=64: Agent::for_users rows=10: Ok(10)
WS12 PR201 loader limit=64: ChannelThread::for_rooms rows=10: Ok(10)
WS12 PR201 loader limit=64: board_reply_counts rows=10: Ok(0)
WS12 PR201 loader limit=64: board_link_counts rows=10: Ok(0)
WS12 PR201 loader limit=64: ThreadTag::for_threads rows=10: Ok(0)
WS12 PR201 loader limit=64: Room::for_ids rows=100: Ok(100)
WS12 PR201 loader limit=64: User::where_ids rows=100: Ok(100)
WS12 PR201 loader limit=64: Agent::for_users rows=100: Ok(100)
WS12 PR201 loader limit=64: ChannelThread::for_rooms rows=100: Ok(100)
WS12 PR201 loader limit=64: board_reply_counts rows=100: Ok(0)
WS12 PR201 loader limit=64: board_link_counts rows=100: Ok(0)
WS12 PR201 loader limit=64: ThreadTag::for_threads rows=100: Ok(0)
WS12 PR201 large work JSON: 32767 threads; HTTP 200; 47703422 bytes; byte-identical Rails response; 0 masks
WS12 PR201 SQLite limit=64: "shared" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "shared" rows=100; HTTP 200 OK
WS12 PR201 SQLite limit=64: "humans" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "humans" rows=100; HTTP 200 OK
WS12 PR201 SQLite limit=64: "agents" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "agents" rows=100; HTTP 200 OK
WS12 PR201 SQLite limit=64: "icons" rows=10; HTTP 200 OK
WS12 PR201 SQLite limit=64: "icons" rows=100; HTTP 200 OK
WS12 PR201 icon callers: 72 complete Rails responses/helper payloads; 0 differences; 0 masks
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2542 filtered out; finished in 33.76s
```

## Every shared payload caller

The six icon fixtures are GitHub, a GitHub/custom-name collision, custom image, emoji, unknown name and blank name. The 72-vector suite compares 66 full HTTP bodies/statuses and six direct live agent-adapter payloads with Rails (each also checks its merged preloaded/batched adapter and the public bot-user serializer); no fields are masked. The public serializer entry points and all their current callers are:

| Shared serializer | Callers | Brand coverage |
|---|---|---|
| `work_threads` / batched `work_user` | `work_threads::index` JSON | Complete `/work.json?state=all` bodies |
| `thread` | `channel_threads::index`; standalone/root-backed create; membership responses from join/read/unread; nested message context/root message summary; root actions thread summary | Complete thread index/detail, nested message, root actions, forwards and direct root-message adapter payloads exercise the same serializer |
| `thread_details` | `channel_threads::show`; `channel_threads::writes::update` JSON | Complete thread detail includes history actors and owner options |
| `message` | `messages::update` JSON; parent-message fields in thread show/create; `message_forwards::create`; live `SharedPayload`/`Presenter::agent_message_payload` | Complete parent-message/forward responses plus direct live agent adapter, using Rails MessagePayloadHelper |
| `thread_message` | `channel_thread_messages::index`, `show`, `create`, `update`; thread show nested messages | Complete nested index/show, reply author/context and forwarded nested messages |
| `actions` | Root and nested message actions | Complete root and nested action bodies |

These callers reuse the exact serializer entry points; no controller or adapter-specific icon code is added. Forward-destination metadata is also compared as an unaffected control. The new icon suite exercises the shared serializers through readers and actual forward POSTs; it does not claim a new icon-specific HTTP mutation vector for every wrapper action. Those existing write-response vectors are covered by the full workspace tests. Root `MessagesController#show` has no pinned JSON template (Rails returns 406), so root message data is compared through its supported adapter, parent-message and forward callers rather than inventing a show endpoint.

The main merge adds further real callers of the same adapter: agent event polling (`agents::poll`), agent room/message/thread reader operations (`agents::reads::operation`), conversation context/post/direct-message services (REST and MCP), and legacy bot message index/create/update. Legacy bot boost creation directly calls the now-public `payload::user`. They retain main's reviewed preload/authorization and response wrappers; their committed Rails vectors run in the final workspace gate. The new direct adapter fixtures also check the preloaded batch path and compare the public booster-user payload with the Rails root creator payload for all six icons.

## Normal-size read growth and recorded facts

The original normal-size read regressions remain flat after the bind changes and main merge. All eight complete JSON bodies and all 208 recorder rows (including IDs, source/event identities, read/handled state and timestamps) still equal Rails. JSON uses 10 / 100 rows; opening uses 9 / 199 recipients. Rails counts are freshly regenerated in this round.

| Probe | Rust final | Rails |
|---|---:|---:|
| Work JSON, shared room/owner | 12 / 12 | 38 / 308 |
| Work JSON, distinct human rooms/owners | 12 / 12 | 58 / 508 |
| Work JSON, distinct agent rooms/owners | 14 / 14 | 88 / 808 |
| Work JSON, distinct custom icons | 13 / 13 | 58 / 508 |
| Opening creation, total SELECT/WITH reads | 74 / 454 | 41 / 611 |
| Opening creation, user reads | 5 / 5 | Not separately measured |

Opening read growth is 380 for 190 additional recipients, below Rails' 570. This claims the required growth parity, not equality of the fixed opening cost.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr201-merged-clean/source/rust/Cargo.toml --locked -p campfire query_read_growth -- --test-threads=2 --nocapture > .scratch/pr201-fixes/logs/query-final.log 2>&1
```

```text
WS12 opening recipients=9: 74 SELECTs; 5 user reads
WS12 work JSON shared rows=10: 12 reader SQL
WS12 opening recipients=199: 454 SELECTs; 5 user reads
WS12 work JSON shared rows=100: 12 reader SQL
WS12 work JSON humans rows=10: 12 reader SQL
WS12 work JSON humans rows=100: 12 reader SQL
WS12 work JSON agents rows=10: 14 reader SQL
WS12 work JSON agents rows=100: 14 reader SQL
WS12 work JSON icons rows=10: 13 reader SQL
WS12 work JSON icons rows=100: 13 reader SQL
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2544 filtered out; finished in 3.98s
```

## Rails reference and regeneration

Pinned Rails is `d7c7de92` plus approved board drift. **For approved board/work/activity files, origin/main Rails is the reference.** The 10-file icon/large-list source ledger and seven-file read-growth ledger match `origin/main`, and the reference image validates them before generating outputs. No masks are added or widened. The 90/29/96 corpora and the existing eight work JSON/208 opening-recorder facts regenerate unchanged.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/work/pr201-review.rb > .scratch/pr201-fixes/icons-regenerated.json 2> .scratch/pr201-fixes/logs/icons-regenerated.log
cmp rust/vectors/pr201_icons.json .scratch/pr201-fixes/icons-regenerated.json
```

```text
Rails shared message/thread/work icon JSON: 72 complete responses; brand/collision/custom/emoji/unknown/blank; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/work/pr201-review.rb large > .scratch/pr201-fixes/large-regenerated.json.gz 2> .scratch/pr201-fixes/logs/large-regenerated.log
cmp rust/vectors/work_large.json.gz .scratch/pr201-fixes/large-regenerated.json.gz
```

```text
Rails large work JSON: 32767 threads; HTTP 200; 47703422 bytes; 590352 gzip bytes; complete response; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/work/read_growth.rb > .scratch/pr201-fixes/read-growth-regenerated.json 2> .scratch/pr201-fixes/logs/read-growth-regenerated.log
cmp rust/vectors/work_read_growth.json .scratch/pr201-fixes/read-growth-regenerated.json
```

```text
Rails work JSON shared size=10: 38 SELECTs; complete response; 0 masks
Rails work JSON shared size=100: 308 SELECTs; complete response; 0 masks
Rails work JSON humans size=10: 58 SELECTs; complete response; 0 masks
Rails work JSON humans size=100: 508 SELECTs; complete response; 0 masks
Rails work JSON agents size=10: 88 SELECTs; complete response; 0 masks
Rails work JSON agents size=100: 808 SELECTs; complete response; 0 masks
Rails work JSON icons size=10: 58 SELECTs; complete response; 0 masks
Rails work JSON icons size=100: 508 SELECTs; complete response; 0 masks
Rails board opening recipients=9: 41 SELECTs; 9 complete recorder facts
Rails board opening recipients=199: 611 SELECTs; 199 complete recorder facts
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/pr201-fixes/human-regenerated.json 2> .scratch/pr201-fixes/logs/human-regenerated.log
cmp rust/vectors/human_work_http.json .scratch/pr201-fixes/human-regenerated.json
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/pr201-fixes/links-regenerated.json 2> .scratch/pr201-fixes/logs/links-regenerated.log
cmp rust/vectors/work_link_model.json .scratch/pr201-fixes/links-regenerated.json
```

```text
Rails work link model oracle: 29 validation/persistence cases; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/pr201-fixes/boards-regenerated.json 2> .scratch/pr201-fixes/logs/boards-regenerated.log
cmp rust/vectors/boards_write.json .scratch/pr201-fixes/boards-regenerated.json
```

```text
Rails board write oracle: 96 complete HTTP responses; no masks
```

All six producer runs and all six complete-file comparisons exit 0, including the compressed large-list vector. Rust consumes those exact committed vectors in the fresh-clone tests. The root/default, first-run and agent UI seeds were independently reverified in this round; their raw summaries follow.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/pr201-fixes/logs/default-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/pr201-fixes/logs/first-run-seed.log 2>&1
```

```text
  "passed": 4,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/pr201-fixes/logs/agents-ui-seed.log 2>&1
```

```text
  "passed": 40,
  "failed": 0
```

## Fresh clone and final gates

The first fresh clone passed all 4,445 tests (61 summaries, 16 explicit ignores). Before publication, main moved from `b98904b8` to `4fd0a74ccb0d596a0687557f3cc1f52a5a58ad08` (#192). Merge `d27fb97a7` resolves the shared payload conflict by retaining main's preloaded room/user/thread/member/agent-policy reads, public user serializer and committed-resource callbacks, together with the separate flat work-page facts and common brand asset resolver. No reviewed preloads are discarded. Regression extension `ce334fce8` covers the newly merged adapter paths. A second fresh clone of that final committed source runs all final gates below.

```sh
git clone --no-hardlinks --single-branch --branch rust/ws12-work-reads "$PWD" .scratch/pr201-merged-clean/source
mkdir -p .scratch/pr201-merged-clean/source/rust/parity/.seed .scratch/pr201-merged-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run rust/parity/.seed/agents_ui .scratch/pr201-merged-clean/source/rust/parity/.seed/
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/pr201-merged-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/pr201-fixes/logs/merged-metadata.json
```

Clone, seed copies and locked metadata exit 0. The clone's implementation is clean at `ce334fce8`; only runtime scratch files are untracked. `CI=1` makes missing seeds a failure. The configured rustc throttle is unchanged, Cargo uses two jobs, full tests use eight threads and focused tests use two. Listeners stay in 53400–53499. No stash, pixel work or Python model-server operations.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr201-merged-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8 > .scratch/pr201-fixes/logs/merged-workspace-test.log 2>&1
```

Raw summaries:

```text
test result: ok. 2539 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 646.69s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.51s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1282 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 175.02s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.68s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.58s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.04s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.18s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s
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

```text
Workspace aggregate: 4558 passed; 0 failed; 16 ignored; 61 raw test summaries; fresh clone ce334fce8
```

All 16 inherited explicit ignores are disclosed below. None was added here, and none of the requested limit/icon/read-count tests is ignored. The two WS14g polling cases retain their owner's stale pre-#192 ignore annotations; this round does not alter those owned tests. Browser/reference-export/measurement/Pebble cases and documentation examples remain separate optional harnesses. No fixture consumer silently skips.

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

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/pr201-merged-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/pr201-fixes/logs/clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 34s
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr201-merged-clean/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/pr201-merged-clean/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/pr201-fixes/logs/release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 12s
```

Both compiler checks exit 0 with no warnings. The release-input script checks production code using only crates plus the explicitly permitted asset build inputs; it excludes vectors/parity/reference tools. This is a source-input Cargo check, not a claim that an optimized release binary was linked.

The final fetch still reports `origin/main=4fd0a74ccb0d596a0687557f3cc1f52a5a58ad08`; it is an ancestor of the tested branch. No subsequent main movement requires another merge.


## Remaining scope and cleanup

The requested #201 fixes are complete. The saved automation WIP remains paused for the later round: SLA rule/settings integration, recurring SLA/nudge/digest completion and scheduler wiring still require finishing and validation. These are WS12-owned follow-ups, not a claim that only owner-blocked work remains.

All owned Cargo/rustc work had stopped before deletion. The three exact owned scratch targets were measured and deleted; a final target-directory scan returns none. Seeds, native tooling, source clones and receipts are preserved.

```text
29G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/pr201-clean/source/rust/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/pr201-merged-clean/source/rust/target
Scratch Cargo targets removed: 3; remaining: 0; no active owned Cargo/rustc processes
Preserved source clones, logs, vectors, verified seeds and matched native media tools
```

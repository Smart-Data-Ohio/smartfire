# WS8bm2 message features C — PR #198 review fixes

Verification source: **7dd675d74e9661a4c77996687e4c71fdd5933e25**, merging current main **033ab0ca9** (#193) into `rust/ws8bm2-message-features-c`. Fix commit: **c4bf6fbbc**. The final report commit changes documentation only; its pushed SHA is in the final reply. This review slice is complete and stops here as requested. Fresh-clone gates: **4431 passed, 0 failed, 16 existing ignores**; strict clippy, release-input binary build and locked metadata exited 0. Oracle replay: **33/34 exact**, with one existing Rails read-count scalar difference and no response/container/model-data differences in that fixture. The wider provider/date scope remains partial; unblocked work still remains. Rust-only; no deployment or PR creation.

## Complete

1. **P2: bounded callback preloads.** `integrations/message_batches.rs` selects referencing messages in ascending ID order, using a keyset and Rails' 1,000-row `find_each` batch size. `link_embed/store.rs`, `channels/github_cards.rs` and `presenters/link_embeds.rs` consume these batches. The sibling-fetch set lives outside the writer's batch loop, preserving one claim/job per embed across batches. Requests and durable jobs still share the metadata transaction; publications still run after commit. The 32,767 suppressed-message reproduction now commits and broadcasts every frame.
2. **Fixture authorization.** `older_provider_tests.rs` and `reference-tools/messaging/older_provider_callbacks.rb` define fixture token constants and construct their authorization headers at runtime. The credential-source regression fails on b251962e. The existing 360-frame recorded vector is unchanged and independently replays exactly after this fix.
3. **Optional fixed-read reduction.** The generic/LinkedIn callback no longer uses a full search/message presenter. It loads referencing messages, rooms and visible link references only, then renders the same owner card partials. `reference_components` serves both the ordinary message presenter and callbacks; `views/messages.rs::cards_for_client_id` shares the original container markup without loading unrelated body/boost/pin/poll/agent/avatar/provider/cache facts. The callback renders only its own provider kind. Response frames stay identical.
4. **Main integration.** The merge keeps #193's `activity::Sources`, deduplicated thread/actor preloads and room-name cache together with this branch's HTML-only actor/room loads. The JSON payload loader retains the P3 savings. Locked metadata passed immediately after the resolved merge. Only the two activity files required conflict resolution.

## Bind-limit audit

All branch-added/changed callback `IN` arguments are now bounded: `Message::for_ids`, `Room::for_ids`, `PullRequest::for_messages`, `PullRequestThread::for_messages` and `Reference::for_messages` receive at most 1,000 IDs per call. The newly selected public-card discussion lookup in `searches/preloads.rs` also chunks its argument defensively, retaining zero queries for private/unknown-only pages and one lookup for ordinary mixed pages. The branch's callback-only `message_cards_for_messages` is called inside the bounded GitHub publisher. The keyset selector itself uses three scalar binds, not one bind per reference.

Inherited main reader internals and board files were left alone. The forbidden `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, and `board_posts.rs` have no diff against the merged main. Main's owner changes are retained.

## Rails proof and read counts

The new `bounded_provider_callbacks.rb` runs real pinned Rails model updates, checking stream, target order, metadata persistence and fetch jobs. Its compact vector records first/last HTML and SHA256 over **every original HTML byte plus a newline**, in delivery order. Rust's registered callbacks run over real WebSockets and compare those complete ordered digests, with no masks or retry. The large cases are generic 32,767 suppressed messages, GitHub 32,767 suppressed unknown-private messages, and LinkedIn 2,001 visible messages with a shared stale sibling and an opposite-kind sibling. All **67,535 frames** match. LinkedIn creates exactly one stale-sibling job across three batches; the opposite-kind card does not appear. Existing 360-frame public/private/unknown, suppression, negative/empty metadata and durable GitHub job vectors also pass.

Counts below include physical reader and triggering-writer SELECT/WITH statements at both 4 and 16 references. The production callbacks and renderer are exercised, not an isolated preload test.

| Operation / privacy | Rust before 4 / 16 | Rust after 4 / 16 | Rails 4 / 16 |
|---|---:|---:|---:|
| GitHub public | 14 / 14 | 14 / 14 | 8 / 20 |
| GitHub private or unknown | 13 / 13 | 13 / 13 | 7 / 19 |
| Generic/LinkedIn/negative/empty, public PR sibling | 37 / 37 | 9 / 9 | 7 / 19 |
| Generic/LinkedIn/negative/empty, private or unknown PR sibling | 36 / 36 | 9 / 9 | 7 / 19 |

The GitHub durable-job test separately proves six real jobs, 60 exact frames and 24 authenticated owner HTTP requests. This turn does not independently recount total job SQL reads; Astra's prior 16/15 measurement is not presented as a new measurement.

## Failing-first receipts

All baseline commands ran at **b251962e production**, with only new tests/vectors registered. Two build jobs, four test threads and the configured rustc throttle were retained. Evidence is under the assigned worktree's `.scratch/ws8bm2-c-review/`.

- `bounded-before.log`: the generic metadata transaction fails with `too many SQL variables`; GitHub receives no callback frame and fails at the unchanged 30-second WebSocket wait; the credential guard finds the literal fixture authorization header. The 2,001-message LinkedIn preservation case already passes before batching and is not claimed as a failing-first defect.
- `older-before.log`: all 360 existing frame bytes and six durable jobs match; the new card-only read-budget assertion fails at 37/36 reads. This is a fixed-cost reduction; the old cost was already flat at 4/16.
- `features-after.log`: all 182 message-feature tests pass, including the three large callback cases and all prior 360 frames. `search-audit-after.log`: the six production searches still match 60 card containers and retain their 48/49-read costs after the defensive discussion chunking.

Baseline commands (source `.scratch/ws8bm2-c-review/ci-env.sh` first):

```sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::bounded_provider_tests -- --test-threads=4 --nocapture
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::older_provider_tests -- --test-threads=4 --nocapture
```

```text
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 2397 filtered out; finished in 30.85s
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 2399 filtered out; finished in 4.63s
test result: ok. 182 passed; 0 failed; 0 ignored; 0 measured; 2219 filtered out; finished in 44.45s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2400 filtered out; finished in 1.65s
```

## Fresh-clone verification

The verification source was cloned with `git clone --no-hardlinks --branch rust/ws8bm2-message-features-c "$PWD" .scratch/ws8bm2-c-review/fresh`. Only the Cargo registry cache was copied; seeds were independently rebuilt in that clone with `parity/bin/seed build default first_run agents_ui`. No untracked source, vector or local fixture input was copied. CI=1 makes missing seeds fail. All Cargo commands used the existing two build jobs, four test threads, assigned port range and unchanged machine-wide rustc throttle; no timing threshold, retry or ignore was changed. The normal native target cache was reused, with the fresh clone mounted at `/src`.

Exact Cargo commands, executed from the fresh clone after sourcing the assigned-root `.scratch/ws8bm2-c-review/ci-env.sh` and setting `CARGO_TARGET_DIR=/native-target`:

```sh
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
bash rust/ci/cargo.sh metadata --locked --format-version 1
```

Raw full workspace summary lines (`fresh-workspace.log`), including all packages, html5ever and doctests:

```text
WS8bm2 fresh workspace aggregate: 4431 passed; 0 failed; 16 ignored; 61 test-result summaries
test result: ok. 2419 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 588.91s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.66s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1275 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 160.57s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.96s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.97s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.46s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.36s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.76s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.88s
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

Strict clippy (`fresh-clippy.log`), release-input build (`fresh-release.log`), and command exits (`fresh-gates.log`):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 34s
WS8bm2 fresh workspace exit: 0
WS8bm2 fresh strict clippy exit: 0
WS8bm2 fresh release-input build exit: 0
WS8bm2 fresh locked metadata exit: 0
```

Both merge and fresh-clone locked metadata JSON parse successfully and contain the workspace members. The merge command completed before its locked metadata invocation.

## Independent Rails replay

From the fresh clone: `python3 rust/reference-tools/messaging/verify_oracles.py` replays 28 fixtures, and `python3 rust/reference-tools/messaging/features-reference-check.py` checks the pin. The six additional fixtures use the same rebuilt default seed, pinned image, frozen time and `cmp` of untouched original JSON files:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_RUNTIME=docker PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash rust/parity/bin/reference runner --storage "$storage" --time 2026-03-02T16:00:00Z --freeze "rust/reference-tools/messaging/$name.rb" "/rails/storage/db/$name.json"
cmp "rust/vectors/messaging/$name.json" "$storage/db/$name.json"
```

The names are `ws12_consumers`, `provider_batch`, `private_provider_pages`, `search_headers`, `older_provider_callbacks`, and `bounded_provider_callbacks`. Exact executed loops are `.scratch/ws8bm2-c-review/replay.sh` and `remaining-oracles.sh`; outputs are `fresh-oracles.log` and `fresh-remaining-oracles.log`. The first loop stops at the private-provider scalar difference; the second continues the three not-yet-run fixtures. No failed fixture is retried.

Raw replay/source receipts:

```text
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 oracle replay: review_saved_race.json byte-identical
WS8bm2 oracle replay: review_dates.json byte-identical
WS8bm2 oracle replay: date_compact_widths.json byte-identical
WS8bm2 oracle replay: providers.json byte-identical
WS8bm2 oracle replay: provider_edits.json byte-identical
WS8bm2 oracle replay: event_cards.json byte-identical
WS8bm2 oracle replay: date_coercions.json byte-identical
WS8bm2 oracle replay: composer.json byte-identical
WS8bm2 oracle replay: composer_sti.json byte-identical
WS8bm2 oracle replay: twitter_preloads.json byte-identical
WS8bm2 oracle replay: twitter_cards.json byte-identical
WS8bm2 oracle replay: twitter_text.json byte-identical
WS8bm2 oracle replay: provider_callbacks.json byte-identical
WS8bm2 oracle replay: agent_command.json byte-identical
WS8bm2 oracle replay: user_coercions.json byte-identical
WS8bm2 oracle replay: date_years.json byte-identical
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 review oracle replay: ws12_consumers.json byte-identical
WS8bm2 review oracle replay: provider_batch.json byte-identical
WS8bm2 review oracle replay: search_headers.json byte-identical
WS8bm2 review oracle replay: older_provider_callbacks.json byte-identical
WS8bm2 bounded Rails embed: 32767 references; title=after; 32767 ordered frames; 0 fetch jobs; sha256=206900da1d0f6faedab48dbcfac3cb391447f796c60d0382ee505b8fc05cc73e
WS8bm2 bounded Rails linkedin: 2001 references; title=after; 2001 ordered frames; 1 fetch jobs; sha256=bc18f21379d958716b61aba60a7caba8238e27a981c5ceb643d7d6105a36c0d9
WS8bm2 bounded Rails github: 32767 references; title=after; 32767 ordered frames; 0 fetch jobs; sha256=8f06770af732b42bba92cbc7b4e48dcd7a9a8e126a0a032532a440d21a5ceb21
WS8bm2 review oracle replay: bounded_provider_callbacks.json byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 private-provider replay difference: $.pages[3].reads 32 -> 33; all 60 card containers and all other data identical; not retried
WS8bm2 review oracle replay: 33/34 fixtures byte-identical; 1 existing Rails read-count scalar differs
```

**Flagged replay difference:** `private_provider_pages.json` differs only at `$.pages[3].reads`: the independently regenerated Rails unknown-private, 16-result page counts 33 reads instead of the pinned 32. A recursive comparison confirms that all 60 original card containers, model rows and every other field are identical. This remains a failed byte comparison, not a masked pass. The fixture and Rust query thresholds were not changed; it was not retried. The previously accepted pinned fixture is unchanged. The Rust production-search regression passed, with flat 48/49 reads and exact original card bytes.

## Remaining scope and ownership

The requested #198 changes are complete, including the optional fixed-read trim. Broader scope is still partial: mapped PR thread headers/reply windows; older-window calendar-event, Fizzy and X callbacks/jobs; generic/LinkedIn network fetch-job paths; and broader stale-sibling rollback/job permutations remain to expand. These include unblocked follow-ups; this is **not** a claim that only owner-blocked work remains.

Date/coercion expansion remains **on hold for WS11-UI #196**; this slice builds no second parser. The read-only `BoardSlaNudge` (WS12) and `AgentBudgetNotice` (WS11) fact-reader seams remain named until typed owner APIs are exported. Real WS17 push, WS11 agent dispatch and WS13 huddle APIs remain wired; no stand-in replaces a landed API. #179's viewer-zone/cache behavior is unchanged.

Browser inventory: polls 4; pins/saves 7; slash 26; search/files 4; scheduling 4 — **45 cases**. Prior accepted checkpoint: 45/45 on both Rails and Rust. Browser suites were not rerun in this review slice and no new browser harness was built. Previously accepted controller/deferred Rails-file pass inventories remain available in the report history at 950e5de and b251962e; this slice adds four regression functions and one compact Rails vector rather than claiming new ports of deferred files.

## Cleanup and final state

The fresh clone's target directory contained exactly four generated JSON exports, inspected before deletion. All four were removed by exact name, and its empty target directory was removed. The normal native Cargo cache remains; no extra scratch build target remains. The release guard removed its temporary input tree, and no owned test/reference container remains running. The fresh clone has no source edits; its untracked `.scratch/` contains output-only evidence. The configured rustc throttle and Python model server were untouched; no stash was used.

```text
WS8bm2 scratch cleanup: four generated JSON exports removed; fresh clone target absent
```

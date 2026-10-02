# WS8bm2 message features D — provider checkpoint (partial)

Verification source: merge **febda63f7babb1a628311121b8b2225b854d4fc8**, on `rust/ws8bm2-message-features-d`, incorporating main **34b3cd40c7043150cd659397f513626b2729253f** (WS11-API PR #202, including #192). D started from **b98904b88**, PR #198's actual merge. The final report commit changes documentation only; the pushed SHA is in the final reply. This is a coherent **partial** checkpoint: unblocked provider work remains. Rust-only; no deployment or PR creation.

## Complete in this checkpoint

1. **Stable private-provider generator.** After the real warm-up, `private_provider_pages.rb` warms the real Icons cache outside the measured SQL region, temporarily holds only `@custom_cache_at` at infinity, and restores its timestamp afterward. It neither changes the production model nor freezes the process clock. The deterministic expiry probe invalidates Icons after each warm-up without sleeping. Both ordinary independent regenerations and both forced-expiry regenerations match the original pinned fixture byte-for-byte. The pinned fixture remains unchanged: private/unknown 32 reads at 4 and 16 results; mixed 33 at both sizes.
2. **Mapped PR headers and reply windows.** The GitHub callback now loads discussion mappings in bounded, ascending-ID keyset batches, preloads their threads once, and reuses the same private-safe owner header card. It preserves orphan skips, malformed mapping behavior, target order and the owner partial. Six public/private/unknown groups at 4/16 mappings cover 60 original headers, six standalone thread-page headers, six older HTTP reply-card fragments, 36 callback window checks and 18 job window checks. Every callback/job frame is compared over real WebSockets, without masks. Six registered GitHub durable jobs exercise all five owner HTTP endpoints, including files, with 30 runtime-built authenticated requests.
3. **Older-window Fizzy and X callbacks and jobs.** Both callback renderers select references in bounded message-ID order and batch only the rooms and provider facts their partials consume. Fizzy still broadcasts neutral lazy frames; private cache payloads never enter shared frames. X uses the owner's numeric card ordering and one account/icon snapshot, avoiding the full body/boost/pin/poll/agent/cache presenter. Four groups cover 4/16 references split across roots and replies, outside both current windows, with reversed sibling-card ordering and suppression. Twelve durable jobs exercise Fizzy 200/403/401 and X 200/404/401 responses, exact original frame bytes, persisted fetch errors and runtime-built auth headers. The registered test handlers call the same owner fetch functions and missing-record classification, replacing only the network transport with injected resolver/dialer and a real local TLS server; no external network is used.
4. **Rollback and regression guards.** Four rollback probes use the same Rails model writes: private Fizzy cache saves and X post saves. They prove no frames and no persisted cache/text change. These are basic rollback probes, not a claim that the broader stale-sibling matrix is complete. The deliberately misordered X batch implementation fails the real frame comparison. Query-cost regressions fail against the original renderers, then pass with the bounded paths.

Commits: `e660d68b9` (Icons generator), `4cac192d9` (provider batches, runtime tests and two Rails vectors), `56a6df688` (align rollback probe operations with Rails and assert persisted state). `e2e01f119` removes two redundant borrows and compiles the legacy header wrapper only for tests, following the first strict-clippy result. Commit `d8f2fb64a` breaks the inherited Cable/database ownership cycle with a deterministic failing-first drop regression. Merge `febda63f7` keeps current main without conflicts; final gates use that source in a fresh clone.

## Changed files and design boundaries

| Files under `rust/` | Change |
|---|---|
| `reference-tools/messaging/private_provider_pages.rb`, `private_provider_cache_expiry.rb` | Hold fixture Icons state; deterministic expiry regression. Existing private-provider JSON is unchanged. |
| `crates/campfire/src/channels/github_cards.rs`, `controllers/presenters/github.rs` | Bounded mapping/thread preloads; reuse header facts without per-header discussion queries. |
| `controllers/presenters/fizzy_cards.rs`, `twitter_cards.rs`, `integrations/message_batches.rs`, `integrations/twitter/post.rs` (all under `crates/campfire/src/`) | Provider-only batch rendering; bounded writer-side sibling claims with one cross-batch deduplication set. |
| `crates/views/src/twitter/cards.rs`, `twitter/mod.rs` | Pure card-container entry point, retaining the ordinary message adapter and owner markup. |
| `controllers/message_features/{mapped_provider_tests,older_owner_tests,quote_integration_tests}.rs`, `controllers/message_features.rs` (under `crates/campfire/src/`) | Four runtime regressions, vector import membership support and test registration. |
| `reference-tools/messaging/{mapped_provider_callbacks,older_owner_callbacks}.rb`, matching `vectors/messaging/*.json`, `verify_oracles.py` | Two independently regenerated corpora; expand replay from 28 to all 36 owned fixtures. |
| `crates/cable/src/{server,lib}.rs`, `crates/campfire/src/jobs.rs`, `jobs/tests.rs` | Non-owning Cable sink handle; deterministic shutdown lifetime regression. Live app/router/subscription owners remain strong. |

This touches WS15 consumer adapters and the X sibling-claim helper, retaining their landed owner fetch APIs. No owner network/parser implementation is replaced, and no WS12 protected file changes.

## Rails differentials and query counts

`mapped_provider_callbacks.rb` and `older_owner_callbacks.rb` independently generate the pinned vectors from Rails **d7c7de92**. Their fixture rows, complete owner HTML, stream order, reply windows, errors and fixed network responses are committed. Rust imports those rows into real SQLite and exercises actual controllers, registered events, durable queues and WebSockets. The 360 mapped frames and 160 Fizzy/X frames match all original bytes. There are no masks, timing retries or external calls.

Callback counts include physical reader plus triggering-writer SELECT/WITH statements. Job counts below measure owner fetch **reader** statements, excluding durable-queue polling and writer statements. The report does not label those partial job counts as total job SQL.

| Operation | Rust before, 4 / 16 | Rust after, 4 / 16 | Rails callback, 4 / 16 |
|---|---:|---:|---:|
| Mapped public PR | 30 / 78 | 15 / 15 | 23 / 83 |
| Mapped private or unknown PR | 25 / 61 | 14 / 14 | 19 / 67 |
| Fizzy callback | 14 / 50 | 5 / 5 | 7 / 19 |
| X callback | 117 / 453 | 9 / 9 | 7 / 19 |
| Fizzy job reader, each status | 13 / 49 | 4 / 4 | Not independently measured |
| X job reader, each status | 114 / 450 | 6 / 6 | Not independently measured |

The owner baseline restores the Fizzy/X callback renderer files verbatim from `e660d68b9` (identical to starting main) while retaining the bounded pending-sibling writer preparation. An earlier whole-original run measured X 116/452; the one extra bounded-selector read explains 117/453 in the definitive restored-renderer receipt. All original frame bytes still pass before optimization; the new flat-cost assertions fail. Mapped callback baseline uses the unmodified starting-main callback.

Bind-limit audit: `Message::for_ids` and Fizzy `Card::for_messages` receive at most 1,000 IDs; X `Post::for_messages` chunks at 900; `Room::for_ids` and `ChannelThread::for_ids` use one JSON bind. Mapping and reference keysets use three scalar binds. The X pending-sibling fetch set remains outside the batch loop, preserving global deduplication. No branch-added callback preload has an unbounded `IN` list.

## Failing-first evidence

Evidence is in `.scratch/ws8bm2-d/`. The deterministic Icons baseline fails the byte comparison solely at all six query-count scalars (32 to 33, or 33 to 34). It is not repinned. The provider baseline commands use two build jobs, four test threads and the unchanged machine-wide throttle:

```sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::mapped_provider_tests -- --test-threads=4 --nocapture
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::older_owner_tests -- --test-threads=4 --nocapture
```

Raw receipts from `icons-before.log`, `mapped-before-final.log`, and `owners-before-final.log`:

```text
WS8bm2 private provider Rails: private 4=33 reads; private 16=33 reads; unknown 4=33 reads; unknown 16=33 reads; mixed 4=34 reads; mixed 16=34 reads; 60 card containers
mapped header callback N+1: ["false: 30 -> 78", "true: 25 -> 61", "null: 25 -> 61"]
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 2431 filtered out; finished in 5.94s
owner callback N+1: ["fizzy: 14 -> 50", "twitter: 117 -> 453"]
owner job renderer N+1: ["\"fizzy\" 200: 13 -> 49", "\"fizzy\" 403: 13 -> 49", "\"fizzy\" 401: 13 -> 49", "\"twitter\" 200: 114 -> 450", "\"twitter\" 404: 114 -> 450", "\"twitter\" 401: 114 -> 450"]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2433 filtered out; finished in 5.97s
```

Both fixed ordinary and forced-expiry pairs are separate Rails executions from separate seeded storage directories. The exact reference runner invocation uses `--time 2026-03-02T16:00:00Z --freeze` with `private_provider_pages.rb` or `private_provider_cache_expiry.rb`; each output is compared directly with `rust/vectors/messaging/private_provider_pages.json`.

## Fresh-clone verification

Main merged twice without conflicts: `cdc3f3819` incorporates WS11-API #192, and final `febda63f7` incorporates #202. Locked metadata ran immediately after both merges. Final source was cloned with `git clone --no-hardlinks --branch rust/ws8bm2-message-features-d "$PWD" .scratch/ws8bm2-d/release-checkpoint`. Only the Cargo registry cache was copied. Default, first-run and agents-UI seeds were independently rebuilt there. No untracked source, vectors or fixtures were copied. CI=1 makes absent seeds fail. Thirteen local workspace packages were explicitly cleaned from the shared native cache before final gates, retaining dependency artifacts and preventing a different fresh checkout's `/src` artifacts from being reused.

From the fresh clone, using the assigned-root `.scratch/ws8bm2-d/ci-env.sh`, `CARGO_TARGET_DIR=/native-target`, two build jobs, four test threads and the unchanged configured rustc throttle:

```sh
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

Independent oracle replay and the source guard also run from that final fresh clone:

```sh
WS8BM2_ORACLE_SCRATCH="$PWD/.scratch/ws8bm2-d/verified-oracles" python3 rust/reference-tools/messaging/verify_oracles.py
python3 rust/reference-tools/messaging/features-reference-check.py --self-test
```

Three earlier fresh-clone suites at `4cac192d9`, `56a6df688` and `e2e01f119` also passed (4,443 tests each). The `e2e01f119` clippy and release-input build also passed. They are supplementary evidence; final summaries below belong to main merge `febda63f7`, including the lifetime fix. The unsuccessful first merged run and fresh-main control are recorded separately below. The first strict-clippy run rejected two redundant borrows and a test-only legacy wrapper; those are fixed without lint suppressions. No timing threshold was widened, concurrency lowered, retry added or new ignore introduced.

## Inherited shutdown ownership cycle: failing first and fixed

The first post-#192 fresh workspace run exhausted Docker's unchanged 65,536 soft file limit. SQLite database opens then failed, although disk and temporary directories were healthy. An exact, separately seeded fresh clone of main `4fd0a74cc` reproduced the same failure. Its peak sampled descriptor count was 65,529, including 64,699 deleted SQLite descriptors. This is an ownership leak, not a timing test or a reason to raise limits.

The database Env owns its event sink; that Jobs sink strongly owned Cable; Cable's authenticator/channel dependencies owned the same database. The cycle kept every stopped TestApp's SQLite connections alive. The app sink was already weak. Jobs now also keeps a weak Cable handle, upgrading it only during delivery. Live apps, routers and subscriptions still own Cable strongly, so active delivery uses the same server. Events emitted after server destruction are harmless.

The new regression installs a drop-observed authenticator, attaches the event sink, then drops the final external Cable owner. Before the fix it deterministically fails because the authenticator has not dropped; afterward it passes and checks a post-shutdown disconnect event. No sleeps, retries, file-limit changes or timing changes are used. Existing real-WebSocket differential tests continue to verify live delivery.

The baseline test was run on `cdc3f3819` with only the regression added; `lifecycle-before.log` records failure. The fixed run is `lifecycle-after.log`:

```sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 jobs::tests::event_sink_does_not_keep_a_stopped_cable_server_alive -- --exact --test-threads=4 --nocapture
```

Raw failed full-run/control and deterministic regression receipts (these are not the final gate):

```text
test result: FAILED. 1702 passed; 835 failed; 7 ignored; 0 measured; 0 filtered out; finished in 796.15s
test result: FAILED. 1676 passed; 857 failed; 7 ignored; 0 measured; 0 filtered out; finished in 633.05s
WS8bm2 main FD probe: pid=3961168 open=65529 sqlite=64714 deleted_sqlite=64699; Max open files            65536                524288               files
the event sink retained Cable and its database-owning authenticator
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2544 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2544 filtered out; finished in 0.00s
```

An earlier lifetime attempt reused incompatible main artifacts and failed compilation; `lifecycle-cache-mismatch.log` is retained but is not failing-first evidence. The valid baseline above follows a local-workspace package cleanup. One cleanup helper had a syntax error that cleared more of this worktree's rebuildable Cargo cache than intended; it changed no source, fixture or other worker target. Final verification rebuilt those artifacts.

Raw successful provider and Icons summaries:

```text
WS8bm2 older-owner callbacks: 40/40 exact frames; flat 4/16 reads; 4/4 silent rollbacks; roots and replies outside both windows
WS8bm2 mapped-provider jobs: 6/6 registered durable jobs; 180/180 exact frames; 18/18 reply windows; 30/30 authenticated owner API reads
WS8bm2 older-owner jobs: 12/12 durable jobs with injected owner transports; 120/120 exact frames; 12/12 recorded errors; 6/6 runtime-built auth headers; no external network
WS8bm2 mapped-provider callbacks: 60/60 headers; 6/6 thread-page headers; 6/6 older HTTP reply cards; 36/36 reply-window checks; 180/180 exact frames; flat 4/16 reads
WS8bm2 Icons independent regeneration 1: byte-identical pinned fixture; 6/6 read counts; 60/60 cards
WS8bm2 Icons independent regeneration 2: byte-identical pinned fixture; 6/6 read counts; 60/60 cards
WS8bm2 Icons expiry regeneration 1: byte-identical pinned fixture; 6/6 read counts; 60/60 cards
WS8bm2 Icons expiry regeneration 2: byte-identical pinned fixture; 6/6 read counts; 60/60 cards
```

Raw full workspace summaries (`verified-workspace.log`):

```text
WS8bm2 verified fresh workspace aggregate: 4569 passed; 0 failed; 16 ignored; 61 test-result summaries
test result: ok. 2550 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 936.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1282 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 214.89s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.87s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.81s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.68s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.49s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.98s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.26s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.06s
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

Strict clippy (`verified-clippy.log`), release-input build (`verified-release.log`) and exits:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 09s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
WS8bm2 verified fresh metadata exit: 0
WS8bm2 verified fresh strict clippy exit: 0
WS8bm2 verified fresh workspace exit: 0
WS8bm2 verified fresh release-input build exit: 0
WS8bm2 verified locked metadata: 13 workspace members; lockfile unchanged
```

Independent fresh-clone Rails replay (`verified-oracles.log`) and source guard:

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
WS8bm2 oracle replay: ws12_consumers.json byte-identical
WS8bm2 oracle replay: provider_batch.json byte-identical
WS8bm2 private provider Rails: private 4=32 reads; private 16=32 reads; unknown 4=32 reads; unknown 16=32 reads; mixed 4=33 reads; mixed 16=33 reads; 60 card containers
WS8bm2 oracle replay: private_provider_pages.json byte-identical
WS8bm2 oracle replay: search_headers.json byte-identical
WS8bm2 oracle replay: older_provider_callbacks.json byte-identical
WS8bm2 oracle replay: bounded_provider_callbacks.json byte-identical
WS8bm2 mapped-provider Rails: 6 groups; 60 headers; 18 reply windows; 360 callback/job frames
WS8bm2 oracle replay: mapped_provider_callbacks.json byte-identical
WS8bm2 older-owner Rails: 4 groups; 40 callback frames; 12 network jobs; 120 job frames; 4 silent rollbacks
WS8bm2 oracle replay: older_owner_callbacks.json byte-identical
WS8bm2 oracle replay: 36/36 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Independently rebuilt seeds (`verified-seeds.log`):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

Bounded final-process descriptor samples (`verified-fds.log`):

```text
WS8bm2 verified FD sampling: 95 samples; peak sample:
WS8bm2 verified FD probe: pid=230710 open=2590 sqlite=1764 deleted_sqlite=1734; Max open files            65536                524288               files
Final sampled app descriptors:
WS8bm2 verified FD probe: pid=230710 open=908 sqlite=54 deleted_sqlite=9; Max open files            65536                524288               files
```

## Remaining scope and ownership

**Partial, with unblocked work remaining.** The next provider slice still needs:

- Older-window calendar-event callbacks and jobs, including their query growth.
- Generic and LinkedIn actual network fetch-job paths (the existing callback matrix is retained, but is not evidence for those network paths).
- Broader stale-sibling claim, rollback and job permutations beyond the four basic owner rollback probes and previously accepted cross-batch deduplication cases.

Exceptional scheduling/reminder/slash date and coercion matrices remain **on hold for WS11-UI #196**. No second parser or date expansion was built in D. The final PR state is recorded with the checkpoint evidence below.

The `/work.json` read-growth P2 belongs to #193/WS12. This slice does not change `work_threads.rs`, `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, or `board_posts.rs`. The branch ownership guard confirms zero forbidden-file changes. Main already exports and consumes typed `BoardSlaNudge` facts, so that old report flag is retired. The read-only `AgentBudgetNotice` (WS11-API) fact-reader seam remains explicitly tagged in `activity.rs` until its typed owner model is exported. Real WS17 push, WS11 agent dispatch and WS13 huddle APIs remain wired; #179 viewer-zone/cache behavior is retained.

Browser inventory is unchanged: polls 4; pins/saves 7; slash 26; search/files 4; scheduling 4 — **45 cases**. Prior accepted checkpoint: 45/45 on both apps. Browser suites were **not rerun in D**, and no new browser harness was built. Previously accepted controller and deferred Rails-file inventories remain in report history at `950e5de` and `b251962e`; D adds four provider runtime regression functions, one Cable lifetime regression and two Rails vectors, without claiming new ports of those deferred files.

## Cleanup and final state

All five feature-verification clones and the fresh-main control scratch target directories were inspected before removing their generated exports by exact name, then removing the empty directory. The normal native Cargo cache remains; no extra scratch build target remains. Logs, fresh clones, independently rebuilt seeds and oracle storage remain as evidence. The release guard removes its temporary input tree. No owned test/reference container remains running. The configured rustc throttle and Python model server were untouched, and no stash was used.

Final ownership/date/cleanup receipts:

```text
WS8bm2 source ownership guard: 0 changes in all 5 WS12 protected files versus merged main 34b3cd40c
WS8bm2 pinned private-provider fixture guard: unchanged versus merged main; original 32/33 counts retained
WS8bm2 verified locked metadata: 13 workspace members; lockfile unchanged
WS8bm2 WS11-UI PR196 checkpoint state: OPEN; mergedAt=None
WS8bm2 owned running containers: 0; configured rustc throttle unchanged; Python model server untouched; no stash used
WS8bm2 cleanup fresh: scratch target already absent
WS8bm2 cleanup fresh-final: scratch target already absent
WS8bm2 cleanup checkpoint: scratch target already absent
WS8bm2 cleanup merged: removed campfire_session_keys_rust_output.json (2397 bytes)
WS8bm2 cleanup merged: removed kit_security_rust_output.json (1425 bytes)
WS8bm2 cleanup merged: removed rails_compat_rust_output.json (7602 bytes)
WS8bm2 cleanup merged: removed rails_compat_smartfire_rust_output.json (21382 bytes)
WS8bm2 cleanup main-control: removed campfire_session_keys_rust_output.json (2397 bytes)
WS8bm2 cleanup release-checkpoint: removed campfire_session_keys_rust_output.json (2397 bytes)
WS8bm2 cleanup release-checkpoint: removed kit_security_rust_output.json (1425 bytes)
WS8bm2 cleanup release-checkpoint: removed rails_compat_rust_output.json (7602 bytes)
WS8bm2 cleanup release-checkpoint: removed rails_compat_smartfire_rust_output.json (21382 bytes)
WS8bm2 cleanup: 6/6 clone scratch targets absent; release-input staging absent; normal native Cargo cache retained
```

# WS8b-m2 F: CI delivery-order and observed HTTP Host review fixes

Validated source: `98fe6ac796c07c065d3002d516d9ca5bd3dca93b`, branch `rust/ws8bm2-message-features-f`. Fix commit: `f0813862eefbe86fecebcf11b132d945ae2cf4bb`. Main `941a5ef4a` was merged as `23823276d`; interrupted helpers were preserved in WIP `3013bacd4`. Main advanced to `2fd002490` (#189) during the first green gates, and was merged as `98fe6ac79`. Locked metadata passed after both merges. Both sides are retained. The final report commit changes documentation only.

**Both requested review items are complete. This is a coherent PR checkpoint, partial for cutover and not owner-blocked-only.** No product ordering, sibling claim, renderer, queue, query, time parser, timeout or retry policy was changed. Final Rust harnesses use four test threads; builds use two Cargo jobs and the existing machine-wide rustc throttle. Test launches wait for shared capacity. The separate immutable legacy executable is recognized by the shared `/debug/deps/` capacity counter. Its optional sweep was parked between batches when required checks needed capacity, and stopped on its first failure. No test's executing native process was paused. No stash, deploy, model-server operation or new browser harness was used.

## 1. Whose order: ordered publications versus unordered wire arrivals

The original CI log was fetched in full, and the requested excerpt command was rerun:

```sh
gh run view 37108558359 --log | grep -A40 older_embed_children_tests
```

CI's Generic/private_image frame target was `...generic-16-2`, while the comparison expected `...generic-16-1`. This was an arrival-order assertion, not different metadata.

**Actual local failing-first reproduction:** an immutable executable rebuilt with the original two ordered job comparators, unchanged production code and vectors, failed at numeric layout seed **136**. Generic/redirect delivered target `...generic-16-12` where the comparator expected `...generic-16-11`. The two frames have the same metadata and stale-child content; their message targets differ. The runner stopped on that failure, preserving its complete stdout. Earlier ordinary runs did not reproduce it: ten fresh uncontrolled processes, 50 one-thread-runtime layouts, 50 two-thread-runtime layouts, then loaded layouts 50–135 passed. Layout seed controls physical row insertion, group order and subscription order; it does **not** fix OS scheduling or Tokio's internal random choices. We do not claim old seed 136 fails deterministically on every run.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2707 filtered out; finished in 9.13s
```

**Independent Rails proof:** `embed_wire_order.rb` imports the existing rows into an isolated pinned `d7c7de92` server, creates a fresh signed session and runs 20 real parents and their 20 actually queued children. The fixture network permits only its two hosts; production Rails jobs/models/renderers are unchanged. The broadcast recorder calls the original method, so every frame goes through actual Redis and Puma's authenticated Cable connection. Every publication exactly matches the existing 400-frame oracle in publication order.

```text
WS8bm2 Rails wire jobs: 20 parents; 20 actual queued children; 400 ordered publications byte-identical
WS8bm2 Rails wire seed=0: 400/400 exact envelopes; publication order exact; arrival order DIFFERENT
WS8bm2 Rails wire seed=1: 400/400 exact envelopes; publication order exact; arrival order identical
WS8bm2 Rails wire seed=2: 400/400 exact envelopes; publication order exact; arrival order DIFFERENT
WS8bm2 Rails wire seed=0: 400/400 exact envelopes; publication order exact; arrival order identical
```

The identical seed-0 subscription layout produced both ordered and unordered deliveries on separate executions. In its unordered capture, the Generic/16/success job alone arrives in relative publication-index order `43,44,45,40,41,42,46,...,71`; this is also a reorder **within one job**, not merely interleaving separate jobs. Two captures reorder frames within individual subscriptions too. Rails' exact bundled `ActionCable::Channel::Streams#worker_pool_stream_handler` dispatches each callback through `worker_pool.async_invoke`, backed by a concurrent executor. Rust's ordinary connection path merges ready subscription streams through `SelectAll`; sequence sorting in `flush` is for the disconnect cutoff, not a global arrival-order guarantee. The parent/sibling publication loops use ordered message IDs; no hash iteration determines child execution/publication order. The fixture generator's synchronous broadcast replacement records a valid publication transcript but cannot establish an arrival-order contract.

**Fix:** `comparison_support::frames` receives actual socket envelopes, retains the complete stream identifier and every HTML byte, and compares sorted vectors as a multiset. Multiplicity is preserved. Existing exact expected counts and 250 ms extra-frame silence checks remain; missing frames still use the unchanged socket deadline. Helper controls reject a missing frame, duplicate replacement, changed HTML and changed stream identity.

`older_embed_wire_orders.json` pins two actual legal Rails delivery permutations as indices into the unchanged publication corpus. Capture SHA256 values are provenance, not credentials. `recorded_rails_delivery_orders_preserve_all_queued_child_frames` replays these observed orders through a real Rust socket with five authorized subscriptions and the unchanged 256-frame capacity (room maximum 200). This is a transport/comparator regression; it does not replace the real network/job/state matrix. The old comparator fails this recorded seed-0 replay before the fix:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2707 filtered out; finished in 0.57s
WS8bm2 legacy ordered Rails delivery replay exit: 101
```

**Sweep:** the parent-only embed matrix, mapped PR callback/job frames, older GitHub/generic/LinkedIn callback frames, older Fizzy/X callback/job frames, and older Calendar callback/MeetLink frames used the same positional wire assumption. They now share the full-envelope multiset check. Direct publication-receiver/bounded-provider ordering assertions remain ordered. Final-state sibling cases have at most one frame; Calendar execution cases assert silence; OOO's badge/notice test already sorts complete observed envelopes. No expected value is substituted on the actual side.

**After:** seeds 0–49 pass, then the fresh-clone source/rebuilt-seed run explicitly includes the failing layout 136 and 49 neighbors (136–185). Every process runs all four embed tests, including both real parent/child job matrices, fatal jobs and the recorded-order replay. Each cohort verifies 600 real-job frames plus 400 replay envelopes per process. These are new seeded executions, not retries of failures.

```text
WS8bm2 fixed seeds: 50/50 passed; 200 tests; 50000 exact frames/envelopes; no failures, retries or timing changes
WS8bm2 fixed bad-seed range: 50/50 passed (136..185); 200 tests; 50000 exact frames/envelopes; fresh-clone source and rebuilt seeds
```

## 2. Observe the actual HTTP Host

Both embed job matrices previously recorded the fixture destination as the observed host. They now read `Received.header("Host")`, requiring it to be present, alongside actual method/path. This corpus uses lowercase DNS authorities on default HTTPS port 443, so no normalization or fallback is needed. Cookie and Authorization absence assertions remain.

`check_embed_host_mutant.py` temporarily mutates the real production `Request::transport` to emit `wrong-host.example.test` only during each original `http_error` case. DNS destination and TLS SNI stay unchanged. All production/test source bytes are restored in `finally`, and no fault selector is retained. Before fixing the observation, both complete matrices passed despite four wrong parent headers and eight wrong parent/child headers. The unmatched virtual host's fallback 404 produced the same negative-cache behavior as the intended 502. Afterward, both matrices fail specifically at `actual wire HTTP calls`; the failure witness is required by the driver, so compilation failures or unrelated assertions cannot earn rejection credit.

Executed in the worker checkout, first with the legacy observation and then with the corrected one:

```sh
python3 rust/reference-tools/messaging/check_embed_host_mutant.py --expect legacy -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/check_embed_host_mutant.py --expect reject -- bash .scratch/ws8bm2-f-order216/run-test.sh
```

Before:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2707 filtered out; finished in 9.91s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2706 filtered out; finished in 10.34s
WS8bm2 HTTP Host mutant: both full matrices escaped the legacy comparison
```

After (expected failures, driver succeeds only when both requested comparisons reject):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2707 filtered out; finished in 1.66s
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 2706 filtered out; finished in 1.70s
WS8bm2 HTTP Host mutant: both full matrices rejected by received-header comparisons
```

## Changed files and read counts

- `controllers/message_features/comparison_support.rs`: exact envelope multiset, four integrity controls, numeric fixture/subscription-layout seeds.
- `older_embed_children_tests.rs`, `older_embed_job_tests.rs`: exact multiset delivery checks and physically received Host; child module also replays recorded legal Rails orders.
- `mapped_provider_tests.rs`, `older_provider_tests.rs`, `older_owner_tests.rs`, `older_calendar_tests.rs`: remove the same invalid wire-order assumption and add stream-identity validation.
- `reference-tools/messaging/embed_wire_order.rb`, `check_embed_wire_order.py`: actual Rails job/publication/Redis/Cable observations without replacing delivery.
- `check_embed_host_mutant.py`: real transport fault, phase-specific rejection and byte-for-byte restoration.
- `vectors/messaging/older_embed_wire_orders.json`: two observed delivery permutations; the original 46 deterministic corpora remain unchanged.

| Consumer, both Generic and LinkedIn, all five outcomes | Rails reads 4 / 16 refs | Rust before 4 / 16 | Rust after 4 / 16 |
| --- | --- | --- | --- |
| Parent fetch (parent-only and dispatched-child matrices) | 14 / 26 | 10 / 10 | 10 / 10 |
| Actually dispatched stale sibling | 10 / 22 | 10 / 10 | 10 / 10 |

Representative physical-query lines from the final fresh suite:

```text
WS8bm2 older-embed job Rust generic success size=16: 10 consumer reads; Rails=26; 32 exact frames
WS8bm2 older-embed child Rust generic success size=16: 10 consumer reads; Rails=22
WS8bm2 older-embed job Rust generic success size=4: 10 consumer reads; Rails=14; 8 exact frames
WS8bm2 older-embed child Rust generic success size=4: 10 consumer reads; Rails=10
WS8bm2 older-embed job Rust linkedin success size=4: 10 consumer reads; Rails=14; 8 exact frames
WS8bm2 older-embed child Rust linkedin success size=4: 10 consumer reads; Rails=10
WS8bm2 older-embed job Rust linkedin success size=16: 10 consumer reads; Rails=26; 32 exact frames
WS8bm2 older-embed child Rust linkedin success size=16: 10 consumer reads; Rails=22
WS8bm2 older-embed job Rust generic success size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust generic success size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin success size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin success size=16: 10 consumer reads; Rails=26; 16 exact frames
```

No product SQL or new `IN` list was added. Existing bounded batches and fetch deduplication remain. No own edit touches WS12's prohibited board/work files, WS11's ledger or the read-only review evidence. Main's reviewed behavior, viewer-zone logic and real owner APIs are retained by the merge.

## Final fresh-clone commands and raw receipts

The first green full suite/clippy/release receipts before #189 are preserved under `.scratch/ws8bm2-f-order216/pre-pr189/`. They are superseded by the final gates below. Final source is a second `--no-hardlinks` clone at `98fe6ac79`, `.scratch/ws8bm2-f-order216/fresh-merged`; default, first_run and agents_ui were rebuilt there from tracked inputs. Its source, assets and rebuilt seeds are used for all final gates. Only the pre-existing owned Cargo dependency/compiler cache is reused. No untracked source/fixture from the worker supplies the clone.

Commands were executed via the existing CI Docker bridge, its unchanged rustc throttle, two build jobs and the worker's ports. Compilation and native-target mutation tests are serialized; the optional original-comparator executable is an independent immutable copy. Compilation finishes before the capacity check that launches the final test suite.

```sh
rust/parity/bin/seed build default first_run agents_ui
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j 2 --no-run
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j 2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j 2 -- -D warnings
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

The deterministic oracle replay and source self-test also ran from the first fresh clone at fix commit `f0813862e`; #189 changes no consumed Rails oracle file, deterministic generator or owned vector. Their source check still covers the exact pinned image. Two observed wire-order pins are legal nondeterministic outcomes and are not claimed to regenerate in identical order. The 46 ordinary generated corpora do regenerate byte-identically. The existing X quote-URL corpus uses its reviewed post-pin `955af4c3781bef07b97b7aefce12c376110a812c` image; all other deterministic corpora use `d7c7de92`.

```sh
WS8BM2_ORACLE_SCRATCH="$PWD/../fresh-oracles" python3 rust/reference-tools/messaging/verify_oracles.py
python3 rust/reference-tools/messaging/features-reference-check.py --self-test
```

Full workspace: **4760 passed, 0 failed, 14 inherited ignored**, across 61 literal summary lines. No newly ignored or silently skipped seeded case.

```text
test result: ok. 2703 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 946.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.98s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1319 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 144.47s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.50s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.96s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.21s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.44s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.45s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.00s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.70s
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
WS8bm2 full fresh workspace exit: 0
```

Strict clippy:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 34s
WS8bm2 fresh strict clippy exit: 0
```

Release-input build:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 16s
WS8bm2 fresh release input build exit: 0
```

```text
WS8bm2 fresh locked metadata exit: 0
WS8bm2 fresh workspace compile exit: 0
WS8bm2 oracle replay: 46/46 independently replayed fixtures byte-identical
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

## Remaining cutover scope, with owner and reason

1. **WS8b-m2 / WS15e:** broader Rails enqueue-adapter rejection differentials. Existing SQLite rejection tests prove stronger atomic metadata/claim/job/frame rollback, but do not replace the wider Rails adapter matrix.
2. **WS8b-m2 / WS14g:** InboundSync/SyncEntry transport and 429 retry/exhaustion permutations. Full ordinary/fatal entry rows, permanent 503 outcomes and dispatched children are already covered; MeetLink retry tests are not evidence for these two consumers' transient exhaustion paths.
3. **WS8b-m2:** exceptional relative-duration overflow, leading/trailing legacy time-split input and unsampled top-level structured parameter containers. Existing absolute grammar/coercion/DST corpora pass again. Reuse the merged shared `rails_compat` parser for later work; no second parser is introduced.
4. **Lead/system-test phase:** final both-app cutover browser run. The previously accepted 45/45 behavior inventory is retained below; browsers were not rerun in this review checkpoint and no pixel result is claimed.
5. **WS12:** broader board/work parity stays with its owner; this slice does not alter those implementations.

The old **WS11 AgentBudgetNotice reader seam is resolved by merged main**: the real typed reader and its activity consumer are present. Physical push, agent invocation/auth and huddles already use their owners' real integrations. Remaining items 1–3 are unblocked proof/implementation work; this is **not owner-blocked-only**.

## Retained named Rails ports and browser inventory

All 35 named slash comparisons and 140 named controller behaviors remain inventoried, and their Rust ports run in the final workspace suite. This is not a claim that all Rails test files ran as suites; evidence is the independently replayed Rails vectors and named Rust tests. Existing earlier command mutations are historical evidence, not claimed as rerun this round.

| Rails name | Rust test name |
| --- | --- |
| registry holds every shipped command with metadata | `builtin_registry_holds_every_shipped_command_with_metadata` |
| command_text? matches slash commands but not escapes or play passthrough | `builtin_command_text_matches_slash_commands_but_not_escapes_or_play_passthrough` |
| huddle starts a call when configured | `builtin_huddle_starts_a_call_when_configured` |
| huddle errors when unconfigured | `builtin_huddle_errors_when_unconfigured` |
| event opens the prefilled form url | `builtin_event_opens_the_prefilled_form_url` |
| event without a time prefills the title only | `builtin_event_without_a_time_prefills_the_title_only` |
| event rejects past times | `builtin_event_rejects_past_times` |
| bare event opens the blank form | `builtin_bare_event_opens_the_blank_form` |
| poll opens the builder in channels but not threads | `builtin_poll_opens_the_builder_in_channels_but_not_threads` |
| remind posts and saves with a reminder | `builtin_remind_posts_and_saves_with_a_reminder` |
| remind rejects unusable input without posting | `builtin_remind_rejects_unusable_input_without_posting` |
| status sets emoji and text until end of day | `builtin_status_sets_emoji_and_text_until_end_of_day` |
| status rejects blank arguments | `builtin_status_rejects_blank_arguments` |
| dnd toggles, takes durations, and turns off | `builtin_dnd_toggles_takes_durations_and_turns_off` |
| dnd rejects garbage durations | `builtin_dnd_rejects_garbage_durations` |
| ooo sets an end with a note, and off clears it | `builtin_ooo_sets_an_end_with_a_note_and_off_clears_it` |
| ooo takes week durations, dates, and datetimes | `builtin_ooo_takes_week_durations_dates_and_datetimes` |
| ooo bare tomorrow and weekdays run to the end of the day | `builtin_ooo_bare_tomorrow_and_weekdays_run_to_the_end_of_the_day` |
| ooo bare dates run to the end of the day | `builtin_ooo_bare_dates_run_to_the_end_of_the_day` |
| ooo bare month dates roll to next year when this year's passed | `builtin_ooo_bare_month_dates_roll_to_next_year_when_this_year_s_passed` |
| ooo day durations stay exact | `builtin_ooo_day_durations_stay_exact` |
| ooo broadcasts the badge and the notice | `builtin_ooo_broadcasts_the_badge_and_the_notice` |
| ooo off while calendar OOO covers says the calendar still shows it | `builtin_ooo_off_while_calendar_ooo_covers_says_the_calendar_still_shows_it` |
| ooo rejects blank arguments, garbage, past times, and long notes | `builtin_ooo_rejects_blank_arguments_garbage_past_times_and_long_notes` |
| shrug posts with the shrug | `builtin_shrug_posts_with_the_shrug` |
| posting commands in a board answer an error without posting | `builtin_posting_commands_in_a_board_answer_an_error_without_posting` |
| posting commands in a board thread still post | `builtin_posting_commands_in_a_board_thread_still_post` |
| slash posts in threads skip the legacy webhook fanout | `builtin_slash_posts_in_threads_skip_the_legacy_webhook_fanout` |
| slash posts in channels fan out to legacy webhooks | `builtin_slash_posts_in_channels_fan_out_to_legacy_webhooks` |
| slash posts in threads process attachments once | `builtin_slash_posts_in_threads_process_attachments_once` |
| me posts an action line | `builtin_me_posts_an_action_line` |
| me requires an action | `builtin_me_requires_an_action` |
| play posts through the normal message path | `builtin_play_posts_through_the_normal_message_path` |
| slash posts never start a stream | `builtin_slash_posts_never_start_a_stream` |
| unknown commands error with the available list | `builtin_unknown_commands_error_with_the_available_list` |

| File under `test/controllers/` | Named behaviors covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16/16 |
| `messages/pins_controller_test.rb` | 6/6 |
| `rooms/pins_controller_test.rb` | 3/3 |
| `saved_items_controller_test.rb` | 12/12 |
| `scheduled_messages_controller_test.rb` | 19/19 |
| `searches_controller_test.rb` | 36/36 |
| `rooms/slash_commands_controller_test.rb` | 10/10 |
| `autocompletable/icons_controller_test.rb` | 6/6 |
| `autocompletable/slash_commands_controller_test.rb` | 7/7 |
| `autocompletable/users_controller_test.rb` | 5/5 |
| `rooms/message_links_controller_test.rb` | 12/12 |
| `rooms/files_controller_test.rb` | 8/8 |

| Browser file under `test/system/` | Prior accepted Rails/Rust behavior results |
| --- | --- |
| `polls_test.rb` | 4/4 each |
| `pins_saved_test.rb` | 7/7 each |
| `slash_commands_test.rb` | 26/26 each |
| `search_files_test.rb` | 4/4 each |
| `scheduled_messages_test.rb` | 4/4 each |
| Total | 45/45 each; not rerun this checkpoint |


Cleanup: generated scratch build-output directories and the extra immutable legacy executable are removed after verification. The pre-existing owned compiler cache remains. Logs, source-check evidence and legal Rails wire captures remain for review. No owned test, reference server, generator, forwarder or parked controller is left running.

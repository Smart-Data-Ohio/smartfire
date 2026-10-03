# WS11 API next-4: merged WS8 dispatcher comparisons

Branch: `rust/ws11api-next-4`. Verified Rust source snapshot: `bed2e1748b5ff7d8305bf2eeae9ee954d5464731`.
Pinned Rails: `d7c7de9264c63015be398001d7a1094e7695a6db`.
Main #216 merged at `699ea1c4a` through merge `210eca9c8`. Main advanced during verification,
so #215 at `1d3b69b8f` was also included through merge `393f9b0bb` before the final gates.
There was no tracked work in progress at the start; only untracked scratch artifacts.

## Outcome and ownership

Closed the 34 previously deferred WS8 built-in dispatcher comparisons. Fresh Rails executes
all 35 built-in cases and 63 observations; the whole generated JSON file is byte-identical
with #216's committed oracle. The already mapped OOO broadcast case is replayed and counted
only once. The 34 newly mapped names cover 62 observations. The dispatcher ledger now maps
all 40 pinned names: 34 WS8 built-ins, the earlier WS11-ui OOO broadcast comparison and
five WS11 custom-agent command comparisons.

No behavior difference was found in WS11-API or WS8 code for these cases. No application
behavior fix, owner service change, mask, allowlist, timing limit or test concurrency change
was needed. The comparisons use WS8's merged app adapter, actual attachment-processing
reads, committed legacy webhook hooks, persisted user/message/reminder state and the
complete returned dispatcher payload. The OOO case also checks real socket HTML frames.

The broader inventory is 362 compared names / 378 total, with 16 deferred. The exact remaining
names stay in `reference-tools/agents/deferred-domain-cases.json` and
`plans/ws11api-remaining-scope.md`: WS12's eight eligibility/viewer and eight mutation/validation
comparisons, plus its Recorder read-cost review. They remain deferred as explicitly requested
for this WS8-only follow-up. #215 landed during verification, so they are no longer blocked on
an unmerged service. The new owner tests execute as part of the workspace suite but are not
mapped or counted as fresh WS11 API differentials in this round.

## Changed files

- `reference-tools/agents/case-ports.json`: map all 34 new names to #216's existing tests;
  preserve pinned order and the existing OOO mapping, with no duplicate comparison.
- `reference-tools/agents/check-case-ports.py`: discover the WS8 `named!(test, index)` tests.
  Existing CI seed preparation already invokes this checker and its controls.
- `reference-tools/agents/test-case-ports.py`: new actual-source macro-discovery regression;
  removing the invocations and supplying a malformed index are rejected.
- `reference-tools/agents/deferred-domain-cases.json`: regenerate the inventory; only the
  16 WS12 names remain deferred.
- `plans/ws11api-remaining-scope.md`: update totals and the actual main/dependency state.

No WS8 production file was modified by this workstream. No WS8-owned mismatch or reproduction
remains to hand back.

## Failing-first checker evidence

Adding the 34 mappings to merged #216 exposed an existing discovery gap. Before adding
the two-line macro-discovery rule, the normal checker and the new regression both failed:

```text
AssertionError: rust/crates/campfire/src/controllers/message_features/slash_named_tests.rs
WS11 WS8 macro discovery before fix: exit 1
FAIL: test_slash_macro_and_missing_invocation (__main__.Discovery.test_slash_macro_and_missing_invocation)
Ran 3 tests in 0.001s
FAILED (failures=1)
WS11 WS8 macro regression before fix: exit 1
```

Commands executed before the fix:

```sh
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/test-case-ports.py
```

After the fix, both pass, including the missing-invocation negative controls. The mapped
Rust tests below were already ported in #216; this round freshly replays their Rails inputs
and verifies their executions rather than claiming a new application failing-first fix.

## Newly closed named cases

All tests below live in
`crates/campfire/src/controllers/message_features/slash_named_tests.rs`.

| Pinned Rails comparison | Rust test |
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

## Verification commands and raw results

The final gates ran from `.scratch/next-4-ws8-fresh`, an independent local clone:

```sh
git clone --local --no-hardlinks --single-branch --branch rust/ws11api-next-4 . .scratch/next-4-ws8-fresh
```

The clone was fast-forwarded from the source branch before the final gates to the verified
snapshot named above. Both main merges were followed by this successful locked check:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
```

The test/build environment, preserving the existing machine-wide rustc throttle:

```sh
export CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8
export CARGO_TARGET_DIR="$PWD/rust/target" TMPDIR="$PWD/.scratch/tmp"
export CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 GITHUB_TEST_PORT_RANGE=52920-52949
export PARITY_OWNER=ws11api-next4-ws8 PARITY_NAMESPACE=ws11api-next4-ws8 PARITY_IMAGE=ws11api-reference:d7c7de92
```

Fresh-clone seed commands:

```sh
rust/parity/bin/ci-seed prepare
rust/parity/bin/seed build default first_run agents_ui
```

Raw seed build lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

The fresh Rails replay ran from the source worktree, using a private copy of the fresh
clone's default seed:

```sh
mkdir -p .scratch/next-4-ws8/rails-slash
cp -a .scratch/next-4-ws8-fresh/rust/parity/.seed/default/. .scratch/next-4-ws8/rails-slash/
export PARITY_SEED_DIR="$PWD/.scratch/next-4-ws8-fresh/rust/parity/.seed"
python3 rust/reference-tools/messaging/features-reference-check.py
rust/parity/bin/reference runner --storage "$PWD/.scratch/next-4-ws8/rails-slash" --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/slash_named.rb /rails/storage/db/slash_named.json
cmp rust/vectors/messaging/slash_named.json .scratch/next-4-ws8/rails-slash/db/slash_named.json
```

Raw source/replay summaries:

```text
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 named slash Rails: 35/35 built-in cases; 63 observations; actual attachment TracePoint and committed webhook/badge/notice hooks
WS11 fresh WS8 slash byte comparison: exit 0
```

Checker and deferred-inventory commands rerun after the discovery fix:

```sh
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/test-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
```

Raw summaries:

```text
WS11 named ports: test/models/channel_thread_agent_assignment_test.rb: 13 mapped cases; 16 unmapped case names
WS11 source case files: 26 pinned Git files matched; 0 checkout mismatches
...
----------------------------------------------------------------------
Ran 3 tests in 0.001s

OK
WS11 deferred case inventory: 1 pinned files; 16 named source cases; owners recorded per file
```

Final strict clippy and Rust-only release-input commands, after the #215 main merge:

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -j2 --bin campfire
```

Raw summary lines, in that order:

```text
    Finished `dev` profile [unoptimized] target(s) in 47.00s
    Finished `dev` profile [unoptimized] target(s) in 2m 16s
```

The full workspace run used the existing byte-exact pinned-media runner. All non-media
tests ran natively; six application media tests and the storage vectors ran in the pinned
image. Native and pinned portions ran sequentially, with at most eight test threads.
Fourteen deliberately ignored tests were not executed. Filtered counts in the media
summaries represent the other portion of that split run, not missing tests; all six
application media tests passed. No assertion, byte field or timing threshold was relaxed.

Commands in the fresh clone:

```sh
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 rust/reference-tools/agents/summarize-tests.py ../next-4-ws8/workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py ../next-4-ws8/workspace.log
```

The last two scripts were also rerun in the source worktree with its corresponding
`.scratch/next-4-ws8/workspace.log` path. Raw rollups:

```text
WS11 workspace totals: 4802 passed; 0 failed; 14 ignored; 60 result summaries
WS11 missing-seed skips: 0
WS11 workspace command exit: 0
WS11 named comparison totals: 362 passed; 0 failed; 16 deferred
```

Raw per-file executed comparison counts:

```text
WS11 named comparisons: test/models/agent_test.rb: 41 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 40 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 13 passed; 0 failed; 16 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 23 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 15 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 9 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
```

Every raw workspace test-result summary:

```text
test result: ok. 2712 passed; 0 failed; 5 ignored; 0 measured; 6 filtered out; finished in 483.44s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2717 filtered out; finished in 2.08s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.71s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1355 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 99.62s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.91s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.61s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.12s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
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
```

## Cleanup and remaining scope

The sole target directory created for this round was removed after verification:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml
test ! -d rust/target
```

Raw cleanup output:

```text
Removed 19247 files, 12.7GiB total
```

Remote main was checked after the app run and still matched included merge `1d3b69b8f`.
All 34 requested comparisons are closed. Only the 16 WS12 names and Recorder read-cost
review remain deferred under the requested scope; #215 is merged and available for a
separate fresh replay. No new WS11-owned implementation gap was found. No Python model
server, other worktree, or peer branch was modified.

# WS8br Wave 4 continuation report — PARTIAL

Main is merged through `ea630861` (#170), including WS15e (#166), the shared asset-golden helper (#168), and board drift (#164/#165). The latest tested source is `f255912d`; the final report commit changes documentation only. This continuation integrates the real merged provider factories, closes three more missing native card slots and preserves constant/query-free preload behavior. The final independently seeded fresh-clone workspace has **2702 passes, 0 failures and 11 existing ignores across 58 harness summaries**. The app has no failures or silent seed skips. Full room-page byte parity is still partial: one GitHub card slot, owner panel/configuration integrations and ten original controller mappings remain. Browser interaction acceptance remains inventoried for the end-to-end phase. No screenshot or pixel-diff work is on the remaining list.

## Pushed coherent slices

| Commit | Change |
| --- | --- |
| `b4e888df` | Merge main at `8952bed4` with a merge commit. Compose main's provider code with the actual WS8bm/WS8bm2, WS11 and WS17 integrations already present. Locked metadata and all Cargo TOML duplicate-key checks pass. |
| `5f87dfb9` | Compare complete Fizzy, generic-link and LinkedIn card containers through the actual room HTTP page; independently compile each omitted seam and require rejection. Record the sole remaining native card difference. |
| `7db9ec99` | Batch provider facts for message and quote/reply-source preloads, retain the full merged scheduler roster, and commit the pinned-media test-executable runner. |
| `0968fe8e` | Merge main again at `ea630861` (#170), which advanced during verification. Preserve native room adapters, M2 recent searches, WS11 task coverage and WS17 submitted-settings preferences. Rerun the complete fresh-clone verification on this source. |
| `f255912d` | Regenerate the original thread-page golden from pinned Rails with the approved #163 layout overlay. Twenty-eight request cases retain all non-layout bodies/facts; eight full-page captures adopt the status action and corresponding live assets. |

## Changes by file and cross-workstream boundaries

- `controllers/presenters.rs`, `presenters/link_embeds.rs`, `presenters/rich_text.rs`, `controllers/messages.rs`, `messages/rendered.rs`, `searches/preloads.rs`, `controllers/rooms.rs`: call WS15e's real Twitter, Fizzy, generic-link and LinkedIn factories from main while retaining WS8bm2 quote references and WS8bm message-list/composer behavior. Preloaded child presenters share pending fetch sets and Twitter facts with their parent. PageResolver delegates Twitter existence and signed image paths to the main resolver. Room rendering returns pending link/Twitter IDs from the reader and invokes main's enqueue seam on the writer. A rejected enqueue clears incomplete cached fragments so retry rendering registers those fetches again; claim atomicity and provider jobs remain the owner implementations.
- `integrations/fizzy/cards.rs::Card::for_messages`, `integrations/link_embed/store.rs::Reference::for_messages`, `searches/preloads.rs`, `presenters/fizzy_cards.rs`: small read-only batch seams retain owner row parsing/order/rendering. Load provider facts for the selected messages and their quote/reply sources, and remember empty provider results. The existing quote regression renders without queries; four versus sixteen complete search-message renders retain the same query count. Lazy rendering still uses the owner lookups. This does not claim every arbitrary legacy inline-X URL is query-free.
- `channels/sink.rs`, `channels/tests/hub_test.rs`, `controllers.rs`, `jobs/tests.rs`, views module/message registrations: union the main provider event/render paths with the actual M2 features and WS11/WS17 events. Preserve the stronger ordered edit-stream assertion and every registered periodic task. Shared message-partial changes are merge reconciliation only; no replacement message/composer implementation was written here.
- `db/models/user.rs`: restore main's linked-account disconnect callback inside the existing deactivation transaction, alongside the real WS11 suspension/grant callbacks. No duplicate lifecycle SQL primitive is used.
- `fizzy_message_cards/tests.rs`: the legacy bot webhook fixture now creates an actual legacy bot via `User::create_bot`, rather than treating the seed's now-agent Bender as a legacy bot. The expected one webhook job is unchanged; WS11 agent behavior is not weakened.
- `integrations/action_claims/tests.rs`: assert the complete combined first-tick roster of thirteen tasks and the seven tasks due after thirty seconds. Main alone's smaller roster is insufficient once WS11 is merged. No scheduler interval or task filtering is changed.
- `controllers/presenters/view_context.rs`: merge WS17's mutable submitted-settings preference rendering with M2's recent-search chrome. The later WS17 Unicode, notification/push and profile fixes come directly from main; this worker does not implement R2's users/account pages.
- `rooms/native_integration_tests.rs`: the new real-HTTP assertion checks every merged-provider container against pinned Rails bytes, including empty containers, and requires exactly three populated seed bodies. It does not serialize the DOM or substitute golden HTML in production.
- `reference-tools/rooms/native_provider_discrimination.py`, the existing boundary/remaining-case scripts, `native_residual.py`, `pinned_media_runner.py`, and `plans/ws8br-owner-integration.md`: reproducible integration checks, exact residual bytes, two build jobs and explicit owner inputs. No timing limits are widened.
- `reference-tools/rooms/regenerate_thread_layout.py`, `vectors/messaging/thread-pages.json`: reproduce WS8bm's original Rails request corpus against the explicitly approved status/layout source overlay. No message-list or composer internal implementation is edited.

`rust/test-support/asset_goldens.rs` is main's shared helper, used directly for page goldens containing current local asset URLs. Actual digests are validated in identified URL fields; surrounding bytes stay exact. The separate strict native comparator applies no masks. Rails board changes `541c0f69` and `8952bed4` are merged reference drift owned by WS12. The post-pin status-popup drift remains WS8br2's responsibility.

## Native byte result and exact remaining bytes

All six root composer/pending-template regions and both DM lists remain exact. The actual room-page GET now asserts the complete merged Fizzy, generic-link and LinkedIn container bytes. Fresh-clone strict result:

```text
FAIL room 654632876 message_list: Rust 287734 bytes, Rails 288823 bytes
PASS room 654632876 composer: 10373 exact bytes
PASS room 654632876 pending_template: 1449 exact bytes
PASS room 186869642 message_list: 19547 exact bytes
PASS room 186869642 composer: 10369 exact bytes
PASS room 186869642 pending_template: 1449 exact bytes
PASS room 699448329 message_list: 6559 exact bytes
PASS room 699448329 composer: 8596 exact bytes
PASS room 699448329 pending_template: 1464 exact bytes
Native room component acceptance: 8 exact matches; 1 differences; no masks
```

```text
github_pr_cards_message_dea0071a-bcd7-5548-a81a-8afdd467d12d: Rust 101 bytes; Rails 1190 bytes
Native residual inventory: 1 empty owner card slots explain the complete remaining difference; strict acceptance still fails
```

The remaining Designers list is **Rust 287734 bytes versus Rails 288823 bytes**, a 1089-byte shortfall. `github_pr_cards_message_dea0071a-bcd7-5548-a81a-8afdd467d12d` is Rust 101 bytes (the empty target) versus Rails 1190 bytes (the complete public PR card and discussion link). Both complete strings are tracked in `rust/plans/ws8br-native-residual.json`. The diagnostic proves this one empty owner region explains every remaining list byte; it neither changes a render nor accepts the difference. Actual/expected/diff files are retained in the fresh clone's `.scratch/native-components-diff/`.

WS15g's GitHub-card factory is still absent from the merged main version. Its unmerged owner API is `github::message_cards(conn, app, message)` plus `github::cache_stamp(conn, message)`, carried by `MessageComponents.github_cards_html` and `github_cards_stamp`. The lead must reconcile these with M2 quotes and the now-merged other provider fields. No local GitHub-card port is substituted. The strict checker deliberately exits 1. A passing seeded app suite is not a claim that this ninth native region or the complete page is byte-identical.

## Exact shell inputs passed to owners

The stable entry points and full contract are in `rust/plans/ws8br-owner-integration.md`:

1. Actual reader connection/app state, request host, verified request origin as cache base URL, and the app's shared fragment store.
2. Original root message records: last forty, or up to forty before a same-room root anchor, the anchor and forty after (eighty-one). Missing, foreign and thread anchors fall back to the last page. No IDs/content/timestamps/feature facts are replaced.
3. `room_native::message_list(&presenter, &messages, divider.message_id, divider.count)` calls unchanged `Presenter::room_message_list`. Viewer divider facts derive from membership `last_read_message_id` and `unread_at`, outside shared per-message fragments. Only the rooms/show caller adds `"\n    \n"`; standalone owner lists retain their bytes.
4. `composer_facts(&room, &viewer, None, drive_flow)`: room ID/kind/viewer display name, root thread `None`, static slash commands followed by room agent commands, and Drive `None`/`Metadata`. Picker availability remains explicitly `false`, pending WS14. Open/Closed/Direct kinds are native; Voice/Stage/Board use the earlier Closed fallback until their owners' screen integration. Header and refresh STI keys remain complete.
5. Actual request `ViewContext`, account/preferences/chrome, user ID/name/title/fresh signed avatar, CSRF and CSP values. M2's scheduled `ComposerButton {ctx, room_id, thread_id: None}` is passed as M1's trusted scheduled control. M1 `PendingTemplate {ctx, user}` is mounted once. Root footer removes one owner initial newline and adds two spaces; pending template retains Rails' final newline.
6. Room/header/involvement facts, selected cached fragments, updated timestamp, original/unpaged invitation, join code, signed messages stream, divider jump/scroll facts, and real WS17 per-viewer OOO facts. Main provider graphs and shared pending-fetch IDs reach the main writer enqueue seam after rendering.

Pin refresh continues invoking WS8bm2's actual count/list seam, including full STI stream targets and viewer-owned Unpin tokens. It contains no local pin policy. Still missing: M1's root thread-panel and poll-builder factories, M2's complete pins-panel factory (count/list exists), WS13 configured huddle header/sidebar adapters, and WS14 configured Picker availability. These are the remaining owner integrations before full native page acceptance.

## Original controller mappings, largest files first

Pinned Rails `d7c7de92`: fourteen owned files, 161 original declarations, **161 actually executed and passed in Rails**. Rust: **151 individually named original mappings passed; ten deferred**. Two additional members regressions and one additional pins regression remain separate (154 named mapping tests total). No new original-controller mappings are claimed in this continuation. All previously delivered CRUD/auth/membership/DM forms/errors/notes, unread/category/favorite/involvement, refresh/pins, members JSON and inbound-address cases execute in the final fresh-clone suite.

| Rails file | Original Rust passes | Deferred |
| --- | ---: | ---: |
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 0 |
| `test/controllers/rooms_controller_test.rb` | 27 | 2 |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 0 |
| `test/controllers/users/sidebars_controller_test.rb` | 6 | 8 |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 0 |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 0 |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 0 |
| `test/controllers/room_categories_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 0 |
| `test/controllers/switchers_controller_test.rb` | 5 | 0 |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 0 |

Exact remaining original cases:

`test/controllers/rooms_controller_test.rb`:

1. `show renders collapsed work-thread guidance in the new-thread panel` — WS8bm thread-panel rendering.
2. `destroy succeeds when the queue is down and the sweep recovers the room` — Lead decision 2 requires atomic queue rollback; native fault-injection coverage is separate.

`test/controllers/users/sidebars_controller_test.rb`:

1. `channel row shows the live huddle stack with names and count` — WS13 huddle grant/presence integration and full-request query instrumentation.
2. `board row shows the live huddle stack with names and count` — WS13 huddle grant/presence integration and full-request query instrumentation.
3. `direct row shows the live huddle stack when the peer is in the call` — WS13 huddle grant/presence integration and full-request query instrumentation.
4. `quiet rows keep an empty stack target with no visible presence` — WS13 huddle grant/presence integration and full-request query instrumentation.
5. `direct row re-renders when a participant joins` — WS13 huddle grant/presence integration and full-request query instrumentation.
6. `group direct rooms render member names and a huddle stack` — WS13 huddle grant/presence integration and full-request query instrumentation.
7. `no channel or DM stacks without huddle configuration` — WS13 huddle grant/presence integration and full-request query instrumentation.
8. `sidebar query count does not grow with quiet channels, DMs, boards, and stages` — WS13 huddle grant/presence integration and full-request query instrumentation.

The quiet huddle query case specifically requires exactly one `huddle_grants` SELECT; the completed ordinary group-DM full-request query measurement does not cover it. The cached direct-row participant case must invalidate when the participant joins, including the renamed peer facts. Do not manufacture WS13 grants/stacks locally. The queue-down destroy declaration conflicts with fixed decision 2 (atomic queue enqueue/rollback); native HTTP rollback fault coverage exists separately and is not mislabeled as that Rails declaration. This conflict remains a lead decision, not an accepted parity exemption.

The broader inventory records 58 original files and 512 source declarations, including other owners' files and unexecuted system cases; that is not 512 executed tests. Browser behavior remaining for the end-to-end phase includes DM picker/create/settings, member selection, room/navigation/header/sidebar/switcher interactions, per-viewer unread behavior, native list/composer/panels, keyboard/focus/drawer interactions, configured huddle states, and inbound-email reveal/enable/regenerate/disable/non-admin denial. Previously shipped DM/inbound browser probes remain tracked but were not rerun this continuation. No screenshot/pixel comparison or visual matching project is deferred.

## Failures shown before fixes and discrimination

The initial merged WS15e run exposed the lost linked-account deactivation callback, incomplete cached render retry requests, and the seed's agent-versus-legacy-bot fixture mismatch. Those were fixed before the main/provider slice was pushed. The initial room-page card assertion also addressed the wrong room constant; it now reads the Designers room ID from the unchanged Rails fixture and asserts its viewer. That test-setup error is not counted as production regression evidence. Raw provider run summaries:

```text
test result: FAILED. 190 passed; 4 failed; 0 ignored; 0 measured; 1100 filtered out; finished in 96.09s
```

```text
test result: ok. 194 passed; 0 failed; 0 ignored; 0 measured; 1100 filtered out; finished in 121.58s
```

The first full fresh-clone run then found the two preload SQL regressions (three queries versus zero; twenty-seven versus sixty-three), the obsolete seven-task roster versus thirteen actual merged tasks, and a WS11 approval-queue inspection assertion (zero versus one). Batch provider facts and the complete roster fixed the first three. The unchanged WS11 test passed in the same four-thread focused run and subsequent complete runs. It is recorded as an observed queue-inspection race/flake, with no concurrency or deadline change, not called inherited. Host media validation separately rejected libvips 8.18.6 / ffmpeg n9.0.2 against pinned 8.16.1 / ffmpeg 7.1.5-0+deb13u1. No version guard was removed.

```text
test result: FAILED. 1289 passed; 4 failed; 2 ignored; 0 measured; 0 filtered out; finished in 385.12s
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.35s
```

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1291 filtered out; finished in 4.57s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1293 filtered out; finished in 1.40s
```

After the second main merge, the complete thread-page golden correctly rejected the adopted `user-status:changed@window->member-panel#refreshPresence` layout action missing from its older expected bytes. The app run had one failure:

```text
test result: FAILED. 1296 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 642.36s
```

The golden is regenerated from Rails, not patched from the Rust response. The committed script verifies ten approved #163 source files at `2e20b24c` and six thread/message source files at `d7c7de92`, then calls WS8bm's original exporter. It requires every non-layout body and fixture fact to remain identical. Eight raw complete layout bodies change only for the approved action and the two adopted live asset URLs. Diagnostic comparison during generation does not normalize production output or the test; the test still invokes main's strict asset-field verifier. Initial invocation omitted the original exporter's `--skip-executor` option and failed before capture; the corrected invocation uses that existing harness option. The failed setup log is retained. Command actually rerun:

```sh
python3 rust/reference-tools/rooms/regenerate_thread_layout.py
```

```text
Rails thread layout source: 10 approved #163 files at 2e20b24c; 6 thread/message files at d7c7de92
WS8bm thread-pages oracle: 28 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
Rails thread layout capture: 28 original request cases; 8 complete layout renders updated; all other facts and response bodies unchanged
```

Compiled discrimination independently omits each merged provider seam, then independently omits the list/composer/template caller boundary. Existing unsafe preview and per-membership SQL mutations must also fail. Sources are restored in `finally`, and no mutant is committed. The preview mutation rejects all three security-relevant cases; full HTTP query tracing rejects per-row SQL. No authorization policy was changed in this continuation; the existing denial coverage runs in the full suite. Commands actually rerun from the canonical worktree:

```sh
RUSTC_BOOTSTRAP=1 CARGO_BUILD_JOBS=2 python3 rust/reference-tools/rooms/native_provider_discrimination.py
RUSTC_BOOTSTRAP=1 CARGO_BUILD_JOBS=2 python3 rust/reference-tools/rooms/native_boundary_discrimination.py
RUSTC_BOOTSTRAP=1 CARGO_BUILD_JOBS=2 python3 rust/reference-tools/rooms/remaining_cases_discrimination.py
```

```text
fizzy_cards: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 3.80s
link_embed_cards: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 2.62s
linkedin_cards: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 2.40s
Native provider discrimination: three independently omitted owner card seams rejected; source restored
```

```text
list: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 0.73s
composer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 0.64s
template: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1294 filtered out; finished in 0.61s
Native boundary discrimination: three independently compiled unadapted owner seams rejected; source restored
```

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1292 filtered out; finished in 1.47s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1293 filtered out; finished in 1.80s
Remaining case discrimination: unsafe previews and per-row HTTP queries rejected; source restored
```

Canonical focused commands, with CI=1, TMPDIR="$PWD/.scratch", CARGO_BUILD_JOBS=2, CARGO_PROFILE_TEST_DEBUG=0, CARGO_PROFILE_DEV_DEBUG=0, CABLE_TEST_PORT_RANGE=52100-52149, MAIL_TEST_PORT_RANGE=52100-52149 and RUSTC_BOOTSTRAP=1:

```sh
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire ws15e_ -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire -- preloaded_quote_cards_render_without_queries_for_distinct_direct_rooms full_message_preloads_keep_queries_constant preloaded_complete_messages_match_rails_and_lazy_presenter native_room_page_provider_cards_match_rails_bytes --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire -- github_claim_registered_periodic_task_executes_and_obeys_its_interval ws11_approval_queue_failure_rolls_back_decision_ledger_and_inbox --test-threads=4
```

Their raw summaries above are retained in `.scratch/merge10-provider-tests-final.log`, `.scratch/bulk-provider10-tests.log` and `.scratch/queue-roster10-tests.log`. The complete fresh-clone run below executes these cases again on the final source.

## Fresh-clone verification and runtime dependencies

Fresh clone: `.scratch/fresh-continue10/repo`, created with `git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-continue10/repo`. Its target initially was empty. Independently build default and first-run seeds with tracked tooling and the pinned Rails reference image; no canonical target, seed, captured response or scratch fixture was copied in. The clone was fast-forwarded to each pushed source fix, including final `f255912d`, before the complete rerun. Only generated scratch/seed output is untracked. The prior complete `7db9ec99` run (2662 passes, zero failures, eleven existing ignores) is retained separately; the summaries below are one complete new run on `f255912d`, including doctests, not a union of selected reruns.

`reference-tools/rooms/pinned_media_runner.py` is committed. Cargo still compiles every Rust executable on the host through the normal machine-wide rustc wrapper. Only the already compiled storage-vector executable runs in the pinned Rails/media container at the same fresh-clone absolute path. Its dependency file identifies `storage/tests/vectors.rs`; all other executables run natively. CI stays enabled, the strict media-version guard and every byte assertion stay enabled, and all eight vectors run, including the nineteen generated media outputs. This supplies the required media runtime rather than skipping or masking its corpus. The reference image is the same pinned dependency independently used to build the seeds. No rustc runs in Docker or bypasses the host throttle.

From the fresh clone, commands actually rerun sequentially (Cargo test finishes before clippy; clippy finishes before native capture):

```sh
PARITY_NAMESPACE=ws8br-fresh10-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 rust/parity/bin/seed build default first_run
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire native_component_capture_matches_rails_root_selection -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-capture.log
python3 rust/reference-tools/rooms/native_residual.py --capture-log .scratch/native-capture.log
```

Every final verification command above exits zero except the strict native comparator, which exits one for the explicitly reported GitHub difference. Metadata intentionally produces no stdout. `RUSTC_BOOTSTRAP=1` enables libtest timing output on stable 1.98.1. Disabling debug symbols saves disk, without changing debug assertions or timing deadlines. Build jobs stay two; harness threads stay four (at most eight permitted). Seed/key/clippy/native-capture raw lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
    Finished `dev` profile [unoptimized] target(s) in 39.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1298 filtered out; finished in 0.61s
```

Raw final workspace summaries (all 58 harnesses, including doctests):

```text
test result: ok. 1297 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 489.52s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.08s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 705 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 56.21s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.62s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.36s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.90s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
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

Existing ignores, distinct from silent seed skips:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
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

## Runtime observations

No coverage is cut, test concurrency lowered or timing threshold widened. These are shared-host observations, not controlled benchmark comparisons. The prior `7db9ec99` app run took 514.54 seconds. Final app raw summary:

```text
test result: ok. 1297 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 489.52s
```

Final app slowest six tests:

- `controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope`: 51.336 s.
- `controllers::messages::paging_tests::root_formats_and_destroy_side_effects_match_rails`: 44.057 s.
- `controllers::messages::paging_tests::validators_observe_related_rows_and_older_unpins_without_message_touches`: 43.713 s.
- `controllers::messages::paging_tests::page_anchors_require_alive_membership_and_a_root_message`: 43.344 s.
- `controllers::messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes`: 41.744 s.
- `controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix`: 41.000 s.

Final owned room slowest six tests:

- `controllers::rooms::rooms_rails_cases::show_keeps_the_last_page_when_the_first_unread_fell_off_it_and_links_the_pill_to_it`: 11.056 s.
- `controllers::rooms::direct_rename_tests::direct_rename_coercions_and_rejections_match_real_rails_requests`: 4.638 s.
- `controllers::rooms::closeds_rails_cases::updating_the_icon_replaces_sidebar_rows_and_headers_for_members_only`: 2.803 s.
- `controllers::rooms::direct_selection_tests::direct_selection_queries_match_rails_and_commit_notes_audits_and_flash`: 2.437 s.
- `controllers::rooms::closeds_rails_cases::create_case`: 2.284 s.
- `controllers::rooms::direct_forms_tests::invalid_direct_rename_renders_the_attempted_name_without_writes`: 1.705 s.

The last-page/off-page unread-pill case performs many serial page requests and is retained in full. No claim is made that these few timings explain the entire earlier 588-second run; exact current per-test evidence is provided rather than reducing coverage.

## Pinned Rails execution and mapping receipts

From the canonical worktree, commands actually rerun:

```sh
python3 rust/reference-tools/rooms/check_controller_files.py
python3 rust/reference-tools/rooms/check_controller_files.py --inject-source-drift
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh-continue10/repo/.scratch/workspace-tests-final.log --rails-log .scratch/controller-reference10.log
```

Raw per-file Rails executions (separate from Rust mappings):

```text
test/controllers/rooms_controller_test.rb
29 runs, 153 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/opens_controller_test.rb
15 runs, 55 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/closeds_controller_test.rb
12 runs, 68 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/directs_controller_test.rb
29 runs, 167 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/involvements_controller_test.rb
8 runs, 63 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/refreshes_controller_test.rb
4 runs, 24 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/reads_controller_test.rb
7 runs, 25 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/members_controller_test.rb
13 runs, 57 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/categories_controller_test.rb
5 runs, 13 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/favorites_controller_test.rb
6 runs, 21 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/inbound_email_addresses_controller_test.rb
8 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/room_categories_controller_test.rb
6 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/switchers_controller_test.rb
5 runs, 33 assertions, 0 failures, 0 errors, 0 skips
test/controllers/users/sidebars_controller_test.rb
14 runs, 76 assertions, 0 failures, 0 errors, 0 skips
WS8br Rails controller reference: 14 files passed; reference counts only
```

```text
Rails controller source-pin injection: wrong hash rejected before tests
```

Raw named Rust mapping receipts:

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/directs_controller_test.rb: 29 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms_controller_test.rb: 27 Rust cases passed, 2 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/opens_controller_test.rb: 15 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/closeds_controller_test.rb: 12 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/members_controller_test.rb: 13 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/refreshes_controller_test.rb: 4 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/users/sidebars_controller_test.rb: 6 Rust cases passed, 8 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/involvements_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/reads_controller_test.rb: 7 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/favorites_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/room_categories_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/categories_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/switchers_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

## Retained evidence and cleanup

Final logs are under `.scratch/fresh-continue10/repo/.scratch/`: `workspace-tests-final.log`, `workspace-clippy.log`, `native-capture.log`, `native-comparison.log`, `native-residual.log`, and `workspace-keys.log`. The initial failed fresh-clone run is `workspace-tests-initial.log`; successful pre-WS17 runs have the `-7db9ec99` suffix. The second main-merge golden failure is `workspace-tests-0968fe8e.log`. Canonical seed/reference/discrimination/focused follow-up and Rails thread-layout logs are under `.scratch/`, as named above. Exact native bytes/diffs remain outside the target directory. Target diagnostics are retained in `.scratch/retained-fresh10-diffs/`. The old fresh-continue9 target and the current fresh-continue10 target are deleted after retaining evidence; no extra target or owned test process is left running.

```text
Fresh-clone targets: fresh-continue9 and fresh-continue10 deleted; logs and byte diagnostics retained
Owned Cargo, rustc and test executable processes: none running
Owned ws8br Docker test/reference containers: none running
```

Tracked mirror: `rust/plans/ws8br-wave4-report.md`. Required external report: `/home/riels/Projects/SD-Labs/Campfire/.claude/delegation/rust-port/wave4/ws8br-report.md`. The reports are identical. Final report commit contains only this report and the owner integration contract; tested source remains `f255912d`.

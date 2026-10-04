# WS8br2 report — #175 merge verification

## Cutover reconciliation (2026-10-04)

The preceding checkpoint is historical. Main `78b9b1546` has merged the source
APIs it described as absent. The current, **partial** result is recorded in
[ledger-ws8br-ws17-ws11ui-report.md](ledger-ws8br-ws17-ws11ui-report.md);
[the remaining manifest](ledger-ws8br-ws17-ws11ui-remaining.json) enumerates every
original clause still lacking a verified receipt. No old owner-held paragraph is
credited as a present missing implementation. Original run/count history below
is retained; it is not a new cutover execution claim.


Verified implementation: `0b009d1087c25cb2d72b20c77663108151d9a29a`.
Merge commit: `9380d068139e5123c1e90049a03a1ee40e5cf25b`, parents `ad32fff5` and main `434d1c14` (#175). The following test-only commits retain Astra's owner-list precedence probe and remove an automatically duplicated fixture initializer. Locked metadata passes after the merge and in the fresh clone.

**Requested merge scope complete.** Both sides' behavior and tests remain. Broader WS8br2 is **partial: 14 original criteria remain**, listed below. No pixel-diff work.

## Notable conflict resolutions

| Boundary | Resolution |
|---|---|
| `controllers/rooms.rs`, `presenters/room_native.rs`, `views/src/rooms.rs`, `templates/rooms/show.html` | Use #175's native room load/render path and owner whole-list slot. Initialize `ShowView.unread_divider_index` from numeric domain message IDs before presentation. Retain the cached-message fallback; a supplied whole list wins immediately. The divider is request/viewer state outside shared message fragments, with the same Rails partial and surrounding whitespace. |
| Native composer and owner panels | Keep Schedule send, the pending template, thread/poll/pins panels and GitHub thread provider. No request-local composer or viewer facts enter shared message caches. Four complete populated pages, four empty HTTP responses and the owner component corpora are compared with Rails. |
| `channels/sink.rs` | Keep main's centralized message-feature dispatcher and general domain-partial fallback, including Events. Remove the older duplicate message/Event dispatch branches and duplicate status dispatch entries. No producer or audit callback is added. |
| `controllers.rs`, `presenters.rs`, `views/src/users.rs`, `users/summary.rs` | Union controller routes/test endpoint coverage and presenter/view modules. Keep the owned profiles, status popup, account/icon/audit controllers, Google identity controls and escaping fix, with main's room/message/thread features. Route the status update once. Account/icon commit-before-audit ordering and exactly-once producer checks remain unchanged. |
| `presenters/view_context.rs`, `rich_text.rs` | Preserve the owned persisted notification/Calendar/Drive projection and request-local unsaved settings, while filling main's configured Google Picker and searches-controller query. Main's icon-name entry point delegates to the canonical brand/alias/database registry, avoiding a duplicate unused implementation. |
| Direct picker controller/model/template | Keep main's `DirectPickerUser` adapter and picker/settings template. Adapt the owned tests to that plain view model, retaining every assertion and all four exact Rails picker bodies. People/cards retain their richer owned presentation model. |
| Test helpers and modules | Retain the union of test modules, frozen clocks, configured app boot and failure artifact helpers. Consolidate duplicate frozen-clock helpers. The first fresh build rejected an auto-merged duplicate `ooo_notice_members` fixture field; remove only the repeated empty initialization, retaining both parents' assertions. |

## Review regressions retained

The original escaping, icon-create audit failure, icon-delete audit failure and unread-divider regressions remain unchanged. Their failing-first proof against `739a6056` is historical evidence in the report committed at `ad32fff5`; this run verifies their merged implementations, rather than claiming a new baseline run.

Both independent Astra probes are now committed:

- `controllers/rooms/review_cache_tests.rs`: David has six unread and Jason two, interleaved through six warm-cache HTTP requests. Each page has exactly one divider directly before its own first unread message. Every shared fragment remains divider-free. Clearing Jason's unread boundary produces zero dividers for Jason while David still has one.
- `views/tests/room_shell.rs`: #175's real owner-list renderer wins over intentionally conflicting fallback index/count values. Each message renders once, and the list contains one divider or none according to its owner facts.

The original eight HTTP divider cases still compare the exact Rails partial, indentation, neighboring messages, scrolling flag and jump control, cold and cached. No expectations, masks or assertions were weakened.

## Rails provenance and owner goldens

Everything stays pinned to `d7c7de9264c63015be398001d7a1094e7695a6db`, with exactly the ten approved #163 status/controller/layout inputs from `2e20b24c3f2be9db8a646a1352c159b4afacad0e`. The status image validator checks all 2076 source/fixture/gem inputs. The owned upstream controller re-diff to merged main `434d1c14` returns only `app/controllers/users/statuses_controller.rb`. Board-only approved drift remains with its owners.

`verify_room_merge.py` reruns the existing room/message/Event owners' recorders in worker-private containers. Complete JSON values and every rendered string compare unchanged; it does not rewrite a golden. Event recorders load fixtures into the declared empty `first_run` seed, matching their original empty-database harness. Loading those fixtures over the populated seed was diagnosed as foreign-key violations and corrected in the harness without altering the recorder or expected bytes.

## Fresh-clone commands

The new GitHub clone is `.scratch/merge175-fresh`. It was fast-forwarded through the pushed test-only corrections to the verified SHA above before the successful workspace run. Tracked sources and unexpected untracked inputs are clean. Seeds, the archived pin and native media tools/libraries were generated afresh. Only the compiler target cache is shared with the worker's preflight; no fixture inputs were copied from an earlier run. `CI=1` prevents missing-seed skips. Two Cargo jobs, four test threads and one extra compiler target were used.

From the worker worktree:

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/merge175-fresh
```

From the new clone:

```sh
git pull --ff-only origin rust/ws8br2-users-accounts
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed prepare
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_NAMESPACE=ws8br2-merge-seed PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/reference-tools/users/media_runtime.sh
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/../merge175/target" CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" TMPDIR="$PWD/.scratch/tmp" CI=1 MAIL_TEST_PORT_RANGE=52640-52669 CABLE_TEST_PORT_RANGE=52670-52699
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bin campfire
python3 rust/reference-tools/users/verify_goldens.py
python3 rust/reference-tools/users/verify_room_merge.py
python3 rust/reference-tools/users/file_counts.py ../merge175/fresh-workspace-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
```

The release-input guard copies only Docker builder source and declared asset inputs, without vectors, parity files or reference tools, and builds the normal binary. It passed without additional Cargo jobs or an optimized release build. Fixed render-only tokens are lent identically to both renderers; real HTTP tests retain real sessions/CSRF. No startup timeout recurred. The 11 existing explicit ignores remain; no inherited-failure exception was used.

## Raw verification summaries

Pin, fresh seeds and native runtime:

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-9259468407ef675b49c8961d58752d30a197f666305cd34b0f0295c7ae2e3fd5
seed_key=rust-parity-seed-v1-925ec754eb3bc8d2a456564880d5aa3c4568f4ad4ef1f99bc1afc09f83cddcde
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
WS8br2 pinned media runtime: image ws8br2-reference:d7c7de92; libvips, FFmpeg tools and libraries extracted; no host libraries changed
```

Locked metadata exited 0 (361 resolved packages). Every workspace target summary follows, unchanged from the log:

```text
test result: ok. 1576 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 695.85s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.52s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 739 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 90.53s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.78s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.86s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.04s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.95s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.26s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.32s
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
WS8br2 fresh-clone workspace totals: 3015 passed; 0 failed; 11 ignored; 58 targets
```

Clippy:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 29s
```

Release-input build guard:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 08s
```

The sole local skip message and expected CI panic belong to the passing `missing_seed_may_skip_locally` / `missing_seed_fails_in_ci` harness tests, which deliberately use an empty temporary root. All seeded application tests ran with the fresh seeds and `CI=1`. No runtime test skipped for missing input.

Rails regeneration summaries:

```text
WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes
WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte
WS8br2 status image source verification: 2076 Rails source/fixture/gem files checked; exactly 10 approved 2e20b24c inputs, all others d7c7de92; non-runtime bin/release omitted
WS8br2 worker harness provenance: pinned Rails assertions unchanged; only stdin source loading differs
WS8br2 oracle verification: all 33 fresh files match byte for byte; no masks or normalization
WS8br2 shared core verification: all 29 generated files match byte for byte
WS8br2 shared routes verification: complete generated JSON matches byte for byte
WS8br2 shared recognition verification: complete generated JSON matches byte for byte
WS8br2 shared sidebar verification: complete generated JSON matches byte for byte
WS8br2 shared WS9 verification: all 15 complete auth pages match byte for byte
WS8br2 golden verification: sources, both fresh seeds, 33 owned oracles and shared core/routes/sidebar/auth pages passed; no masks or normalization
WS8br2 room merge sources: 10 pinned/approved files verified
WS8br2 room merge oracle full-pages: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle empty-shell: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle panels: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle pins: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle pr-thread: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle picker: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle direct-forms: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle native-components: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle event-pages: all recorded values and rendered bytes unchanged
WS8br2 room merge oracle event-fragments: all recorded values and rendered bytes unchanged
WS8br2 room merge goldens: 10 owner corpora passed; rendered bytes identical; no masks or normalization
```

Per-file executed owned test counts:

```text
controllers/rooms/review_cache_tests.rs (Astra single-divider/two-viewer cache probe): 1 passed; 0 failed; 0 ignored
controllers/rooms/parity_tests.rs (new list-rendering regression): 1 passed; 0 failed; 0 ignored
app/round_four_security_tests.rs (new production failure oracle): 1 passed; 0 failed; 0 ignored
controllers/users/fizzy_profile_tests.rs: 6 passed; 0 failed; 0 ignored
controllers/users/agent_profile_tests.rs: 11 passed; 0 failed; 0 ignored
controllers/accounts/attachment_tests.rs: 3 passed; 0 failed; 0 ignored
controllers/public_pages/sign_in_google_tests.rs: 1 passed; 0 failed; 0 ignored
controllers/users/profile_effective_ooo_tests.rs: 12 passed; 0 failed; 0 ignored
controllers/users/layout_preferences_tests.rs: 12 passed; 0 failed; 0 ignored
controllers/users/avatar_image_tests.rs: 2 passed; 0 failed; 0 ignored
controllers/users/ban_lifecycle_tests.rs: 3 passed; 0 failed; 0 ignored
controllers/public_pages.rs: 4 passed; 0 failed; 0 ignored
controllers/users/profile_security_tests.rs: 7 passed; 0 failed; 0 ignored
controllers/rooms/directs/picker_tests.rs: 5 passed; 0 failed; 0 ignored
controllers/users/status_popup_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/users/profile_sections_tests.rs: 12 passed; 0 failed; 0 ignored
controllers/users/joining_tests.rs: 2 passed; 0 failed; 0 ignored
controllers/users/profile_page_tests.rs: 6 passed; 0 failed; 0 ignored
controllers/first_runs/tests.rs: 3 passed; 0 failed; 0 ignored
controllers/welcome/tests.rs: 2 passed; 0 failed; 0 ignored
controllers/accounts/audit_logs/tests.rs: 5 passed; 0 failed; 0 ignored
controllers/accounts/icons/tests.rs: 11 passed; 0 failed; 0 ignored
controllers/accounts/logos/tests.rs: 2 passed; 0 failed; 0 ignored
controllers/accounts/view_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/users/people_tests.rs: 13 passed; 0 failed; 0 ignored
controllers/users/profile_settings_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/accounts/mutation_tests.rs: 12 passed; 0 failed; 0 ignored
WS8br2 file accounting: 149 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 4 icon bodies/navs, 9 logo PNG responses; 14 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 24 account/ban cases, 9 individual exactly-one audit checks and one owner-removal with exactly two distinct Rails audits, 14 account rows, 2 account bodies/navs/footers, 2 invites, 3 CSS bodies; 11 complete bot profile HTML/nav cases; 6 complete Fizzy fragments and real profile HTTP/side-effect cases; 10 signed icon/logo assignments, durable rollback and 3 NullAnalyzer/audit after-commit states; 5 production account and 2 icon audit-failure states; 2 complete profile pages (seed and markup); 8 unread HTTP list states, each cold and cached; exactly one divider for each of two viewers across six warm requests, then zero for the read viewer and one for the unread viewer, with divider-free shared fragments
```

## Flagged owner seams

| Owner | Current state / remaining seam |
|---|---|
| WS9 | Direct merged security, passwords, sudo, sessions and audit producers. Nine individual normal mutations assert exactly one complete Rails audit row. Account post-save failure boundaries are now corrected through those producers; no duplicate audit path. |
| WS15g | Direct merged `presenters::github::connection` and `campfire_views::github::connections::profile`. The previous placeholder call site is closed. Other GitHub APIs use imported owner code; this worker does not claim new live GitHub acceptance. |
| WS15e | Fizzy profile usable-token/decryption/disconnection seam is closed through its model. Slack import link renders, but no merged Slack import backend was found in this checkout; live import remains flagged. |
| WS17 | Profile status/effective OOO and push-notification form/save seams use merged APIs directly. Unsaved invalid values render. `layout_preferences::fill` still projects cached ISO windows; broader `Time.zone.parse` grammar and live cable partial delivery remain unproved here. Seed full-page/fragment/browser status checks do not prove live push delivery. |
| WS14g | Raw Google account/scopes/configuration, identity-email and `fetch_error` cache projections in `profile_sections.rs`/`profiles.rs` need owner APIs. Configured sign-in/Calendar/Drive metadata display is covered; OAuth start/callback, grants/refresh, unusable-token handling and live Calendar remain flagged. Main now supplies `Chrome::google_picker` from its typed public configuration; eight configured root-composer goldens cover this integration. |
| WS11 | Agent user-profile facts, owner/admin visibility and the specific owner-deactivation lifecycle are covered. Inbox preferences, agent approval/work projections, three read-only budget aggregates and lifecycle snapshot validation still need owner API consolidation. Broad agent REST/UI revocation acceptance is not claimed. |
| WS13 | Call mode/key/error fields render from profile settings. Actual global `Chrome::huddle_configured`, live huddle/ringing/transport and group-huddle criteria remain flagged. |
| WS8b-m / WS8br | The native whole message list, composer/Schedule send, pending template, thread/poll/pins panels, member JSON and group mutation routes now use the merged owners directly. Four complete populated pages, four empty HTTP pages, owner panel/PR-thread/Picker corpora and the two-viewer cache probe verify these seams. The 14 original browser interaction criteria below remain unclaimed; do not equate HTTP and directory/card coverage with member-panel browser coverage. |
| WS12 | Four star/member-row/phone criteria remain. Star mutations/room-member integration are owner work. Board-only approved drift is not imported into this owned oracle. |

Broader Ruby `Date.parse` grammar and Expat versus Nokogiri XML syntax/encoding outside the committed oracle inputs remain unproved. No new policy decision needs a user answer. No pixel work is pending. These boundaries are not counted as additional completed criteria.


## Original deferred criteria — unchanged, largest files first

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 57 | 0 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 14 | 5 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 2 | 0 |
| `test/controllers/users_controller_test.rb` | 226 | 20 | 20 | 0 |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 217 | 15 | 15 | 0 |
| `test/system/starred_people_test.rb` | 182 | 4 | 0 | 4 |
| `test/system/icons_test.rb` | 169 | 4 | 0 | 4 |
| `test/controllers/accounts/icons_controller_test.rb` | 103 | 6 | 6 | 0 |
| `test/system/service_worker_test.rb` | 102 | 2 | 2 | 0 |
| `test/controllers/users/bans_controller_test.rb` | 99 | 8 | 8 | 0 |
| `test/controllers/users/profiles_two_factor_test.rb` | 96 | 7 | 7 | 0 |
| `test/system/first_run_tour_test.rb` | 89 | 4 | 4 | 0 |
| `test/controllers/users/cards_controller_test.rb` | 87 | 7 | 7 | 0 |
| `test/controllers/workspace_icons_controller_test.rb` | 78 | 7 | 7 | 0 |
| `test/controllers/pwa_controller_test.rb` | 66 | 1 | 1 | 0 |
| `test/system/timezone_detection_test.rb` | 64 | 2 | 2 | 0 |
| `test/controllers/accounts_controller_test.rb` | 61 | 4 | 4 | 0 |
| `test/system/workspace_icons_test.rb` | 60 | 1 | 0 | 1 |
| `test/controllers/first_runs_controller_test.rb` | 57 | 4 | 4 | 0 |
| `test/controllers/accounts/logos_controller_test.rb` | 56 | 6 | 6 | 0 |
| `test/system/audit_log_test.rb` | 49 | 2 | 2 | 0 |
| `test/controllers/accounts/users_controller_test.rb` | 36 | 3 | 3 | 0 |
| `test/controllers/users/avatars_controller_test.rb` | 34 | 2 | 2 | 0 |
| `test/controllers/accounts/custom_styles_controller_test.rb` | 30 | 3 | 3 | 0 |
| `test/controllers/accounts/join_codes_controller_test.rb` | 21 | 2 | 2 | 0 |
| `test/controllers/welcome_controller_test.rb` | 21 | 2 | 2 | 0 |

`test/system/people_group_dms_test.rb` — **WS8br2 people/cards; WS8br DM actions**:
- the member panel multi-select starts a huddle with exactly that set
- group members rename, add, and leave with system notes in the timeline
- starting a group huddle rings every other member
- Esc closes only the profile card inside the mobile member panel
- Tab cycles within the profile card opened from the member panel

`test/system/starred_people_test.rb` — **WS12 stars; WS8br2 cards/directory integration**:
- starring from the profile card floats the person into a Starred group
- the member row menu stars and unstars, by mouse and keyboard
- Escape still dismisses the row menu after the profile card takes focus
- the Starred group works on a phone

`test/system/icons_test.rb` — **WS8br2 workspace icons/profile names; WS8b-m message icons**:
- colon autocomplete inserts a brand shortcode that renders in both themes
- room icon picker sets an icon that shows in the sidebar and header
- icon rooms suppress the search arrow marker
- lobehub brand icons render visibly in both themes

`test/system/workspace_icons_test.rb` — **WS8br2**:
- upload post in both themes then delete falls back to the shortcode

Deferred inventory: 14 named Rails controller/system cases remain from the original 194; 180 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.


## Cleanup and evidence

```text
Deleted target: 26G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2/.scratch/merge175/target
Deleted target: 40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2/.scratch/merge175-fresh/rust/target
WS8br2 cleanup: 0 scratch target directories; 0 owned active native build/app/media processes; 0 running owner-labeled containers
```

Only the two listed regenerable targets were deleted, after verifying process/container ownership and exact paths. The fresh clone and raw logs remain. Docker containers with other owners were left alone.

Current evidence is in `.scratch/merge175/`: `fresh-head.txt`, `fresh-metadata.json`, `fresh-workspace-tests.log`, `fresh-workspace-totals.log`, `fresh-clippy.log`, `fresh-release-input-build.log`, `fresh-owned-goldens.log`, `fresh-room-goldens.log`, `fresh-file-counts.log`, `fresh-inventory.log`, `fresh-owned-rediff.log`, `fresh-source-clean.log`, and `fresh-cleanup.log`. The rejected duplicate-field build is preserved separately as `fresh-workspace-tests-duplicate-fixture.log`; it is not included in passing counts. The external report and its tracked mirror contain identical bytes.

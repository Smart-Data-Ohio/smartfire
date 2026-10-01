# WS8br2 report — PR #177 review fixes

Verified implementation: `667eba33720637739d18529cb182a3766d395917`.
Main merge: `3f80a64b` imports `ae43b34d` (#174 Events), with a merge commit. Locked metadata passed after the merge and in the fresh clone. Both sides' controller routes, socket producers and tests are retained.

**Review scope complete:** all three Astra findings have pinned Rails regressions that fail against `739a60562ea55295181aacb3cc1868d6968bc0fa`. Broader WS8br2 remains **partial: 14 original criteria remain**, with the owner seams below. No pixel-diff work.

## Changes by file

| Files under `rust/` | Change |
|---|---|
| `crates/views/src/users/agent_profile.rs` | Send room labels through `link_to_text(name, url, attrs)`. Stored markup becomes text, exactly as Rails does. Applies to all room display-name strings. |
| `plans/ws8br2-escaping-review.md`; `reference-tools/users/{agent_profiles,people,profile_page,account_views,icons,audit_logs}.rb`; corresponding vectors and controller tests | Sweep every owned raw/HTML-trusting boundary and trace its input. Add actual tag/ampersand/quote vectors for names, bios, statuses, agent provider/runtime/description/note, account/membership names, DND person/title, member email, icon title/creator/form values, audit labels/details/IP/user-agent and textarea-closing custom CSS. Complete bodies/navs/profile application page/CSV remain compared without masks. The agent-room label was the only direct stored-text-to-trusting-helper sink found. Rendered owner fragments remain typed trusted HTML; Rails deliberately renders application custom CSS raw, while its editor escapes it. |
| `crates/campfire/src/controllers/accounts/icons.rs` | Commit the icon/attachment create or deletion and durable callbacks first, then record the audit. Audit failure returns 500 and retains the mutation. Validation/unique-index errors still return 422 and roll back. Normal audit actor/target/action/details and exactly-once checks stay intact. No duplicate shared attachment or WS9 producer. |
| `reference-tools/users/account_audit_failures.rb`, `vectors/users_account_audit_failures.json`, `controllers/accounts/{icons/tests,attachment_tests}.rs` | Extend the existing five production account failures with icon create/delete. Compare status/location, persisted name/title/creator, blob filename/MIME/size/checksum/metadata, zero audits and analysis/purge queue counts. The create uses a real signed PNG blob; the delete removes an attached PNG icon. Rails runs outside any wrapping transaction; a guard rejects a create that fails before its audit. Rust workers stop before setup to match Rails' non-performing test queue adapter, while real synchronous callbacks and durable enqueues remain active. Attachment helpers become visible only to sibling tests. |
| `crates/campfire/src/controllers/rooms.rs`, `crates/views/src/rooms.rs`, `crates/views/templates/messages/_unread_divider.html`, `crates/views/tests/room_shell.rs` | Resolve the divider position from numeric domain IDs before making cached message fragments. Insert the Rails partial before the first unread message, outside the shared cache; preserve the real-message fallback and optional owner whole-list override. Match the partial and surrounding whitespace exactly, including Askama's explicit trailing newline. |
| `reference-tools/rooms/unread_shell.rb`, `vectors/room_shell_unread.json`, `controllers/rooms/parity_tests.rs` | Retain eight boundary-fact scenarios and add actual authenticated Rails/Rust room GETs. Check six/five/three unread, legacy/deleted pointers, off-page and absent boundaries; exact divider bytes, prefix/suffix whitespace, neighboring message IDs, scroll flag and complete jump control. Run every state twice through one cache to catch viewer-local data leaking into shared fragments. |
| `reference-tools/users/{run_oracles,verify_goldens,file_counts}` | Regenerate 33 owned oracles, shared source/core/routes/sidebar/auth files; assert 148 executed owned Rust groups and retain original 194-criterion accounting. |
| `crates/campfire/src/{controllers,channels/sink}.rs` and imported main files | Resolve the main merge by retaining the owned user route/socket paths and main's Events routes/message/card producers. No tests removed or weakened. |

## Rails pin and #175

Everything remains on `d7c7de9264c63015be398001d7a1094e7695a6db`, except the ten approved #163 status/controller/layout inputs from `2e20b24c3f2be9db8a646a1352c159b4afacad0e`. The status-image check verifies all 2076 inputs and exactly that overlay. Board-only drift remains with its owners.

The owned upstream controller re-diff returns only `app/controllers/users/statuses_controller.rb`.

#175 (`rust/ws8br-rooms-http`) was checked before the fix and again at the end: **OPEN**, merge commit null. Its current `room_message_list` returns the optional owner fragment or an empty value; it does not provide the divider. It is therefore not merged here. The corrected real-message fallback must survive its eventual merge. The owning message-list integration still needs to supply the same boundary facts when it supplies the complete list.

## Failing-first evidence

Baseline clone: `.scratch/main-integration-clone`, actual HEAD `739a60562ea55295181aacb3cc1868d6968bc0fa`. Only the declared regression/helper/vector overlay was copied in; the four production implementations remained from that commit. Final regression predicates and vectors were rerun there after the whitespace assertions were added. Overlay SHA256 hashes were checked before restoring only those copies. No stash.

With the documented native environment, run:

```sh
WS8BR2_DIFF_DIR="$PWD/../review-fixes/baseline-diffs" CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire review_ -- --nocapture --test-threads=4
```

Raw summary:

```text
test result: FAILED. 25 passed; 4 failed; 0 ignored; 0 measured; 1081 filtered out; finished in 17.03s
```

The four failures:

- `review_agent_markup_matches_rails`: Rust emitted `<a ...><b>Room & "</b></a>`; Rails emitted `&lt;b&gt;Room &amp; &quot;&lt;/b&gt;` as the label.
- `review_icon_create_audit_failure_matches_rails`: both responses are 500, but reviewed Rust has no saved icon/blob/job; Rails retains the icon, identified PNG and one queued analysis job.
- `review_icon_destroy_audit_failure_matches_rails`: reviewed Rust keeps the icon/attachment and queues no purge; Rails removes them and queues one purge.
- `review_unread_divider_render_and_cached_page_match_rails`: six-unread scroll is enabled, but reviewed Rust has no divider; Rails does.

Markup-bearing card and complete-profile controls already pass against the reviewed commit. With the fixes applied, the same filter passes:

```text
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 1081 filtered out; finished in 22.09s
```

Logs and raw differences: `.scratch/review-fixes/{baseline-head.txt,baseline-tests.log,baseline-diffs,agent-markup-before.diff,fixed-regressions.log}`. All four also pass in the complete fresh-clone suite below.

## Fresh-clone verification

A **new GitHub clone** of the pushed branch was made at `.scratch/review-fresh`, HEAD `667eba33720637739d18529cb182a3766d395917`; tracked source and unexpected untracked inputs are clean. Its declared `.scratch/` contains only generated runtime media/tmp/oracle outputs, locally excluded. Tests read committed vectors or their freshly generated private inputs. Seeds and the archived pin were rebuilt in this new clone, not copied from the baseline clone. The existing compiled target cache was reused; fixture inputs were not.

The clone command ran from the worker worktree; subsequent commands ran from that new clone. Logs are in `.scratch/review-fixes/` in the worker worktree. No extra Cargo jobs, no release build, at most four test threads.

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/review-fresh
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed prepare
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_NAMESPACE=ws8br2-review-seed PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/reference-tools/users/media_runtime.sh
```

Raw summaries:

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

Native environment, generated afresh from the declared image and seeds (the target path is the one reused compiler cache):

```sh
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/../main-integration-clone/rust/target" CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" TMPDIR="$PWD/.scratch/tmp" CI=1 MAIL_TEST_PORT_RANGE=52640-52669 CABLE_TEST_PORT_RANGE=52670-52699
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

Locked metadata exited 0. Raw application/database test and clippy summary lines, plus counted workspace total:

```text
test result: ok. 1128 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 452.94s
test result: ok. 694 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 56.15s
WS8br2 fresh-clone workspace totals: 2517 passed; 0 failed; 11 ignored; 53 targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 13s
```

All 53 test-result lines are preserved in `final-workspace-tests.log`. The 11 explicit ignores are the existing reference/export/rollback hooks, measurement, ACME fixture and ignored doctests; no seeded test silently skips (`CI=1`). The expected `missing_seed_fails_in_ci` panic passes. No startup timeout or other failure recurred. No inherited-failure exception was used.

```sh
python3 rust/reference-tools/users/verify_goldens.py
python3 rust/reference-tools/users/file_counts.py ../review-fixes/final-workspace-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
```

Raw regeneration/accounting summaries:

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
WS8br2 file accounting: 148 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 4 icon bodies/navs, 9 logo PNG responses; 14 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 24 account/ban cases, 9 individual exactly-one audit checks and one owner-removal with exactly two distinct Rails audits, 14 account rows, 2 account bodies/navs/footers, 2 invites, 3 CSS bodies; 11 complete bot profile HTML/nav cases; 6 complete Fizzy fragments and real profile HTTP/side-effect cases; 10 signed icon/logo assignments, durable rollback and 3 NullAnalyzer/audit after-commit states; 5 production account and 2 icon audit-failure states; 2 complete profile pages (seed and markup); 8 unread HTTP list states, each cold and cached

```

The oracle verification checks the approved overlay, both fresh seeds, 33 owned files, 29 shared core files, routes/recognition/sidebar and 15 complete WS9 auth pages byte for byte. Render-only comparisons lend the same explicit `GLOBAL`, `method:path` and `NONCE`; production HTTP keeps real sessions/CSRF. Neither screenshots nor browser pixel diffs were run. Prior slice browser observations are historical and are not claimed as rerun in this review fix.

## Flagged owner seams

| Owner | Current state / remaining seam |
|---|---|
| WS9 | Direct merged security, passwords, sudo, sessions and audit producers. Nine individual normal mutations assert exactly one complete Rails audit row. Account post-save failure boundaries are now corrected through those producers; no duplicate audit path. |
| WS15g | Direct merged `presenters::github::connection` and `campfire_views::github::connections::profile`. The previous placeholder call site is closed. Other GitHub APIs use imported owner code; this worker does not claim new live GitHub acceptance. |
| WS15e | Fizzy profile usable-token/decryption/disconnection seam is closed through its model. Slack import link renders, but no merged Slack import backend was found in this checkout; live import remains flagged. |
| WS17 | Profile status/effective OOO and push-notification form/save seams use merged APIs directly. Unsaved invalid values render. `layout_preferences::fill` still projects cached ISO windows; broader `Time.zone.parse` grammar and live cable partial delivery remain unproved here. Seed full-page/fragment/browser status checks do not prove live push delivery. |
| WS14g | Raw Google account/scopes/configuration, identity-email and `fetch_error` cache projections in `profile_sections.rs`/`profiles.rs` need owner APIs. Configured sign-in/Calendar/Drive metadata display is covered; OAuth start/callback, grants/refresh, unusable-token handling, live Calendar, and `Chrome::google_picker` remain flagged. |
| WS11 | Agent user-profile facts, owner/admin visibility and the specific owner-deactivation lifecycle are covered. Inbox preferences, agent approval/work projections, three read-only budget aggregates and lifecycle snapshot validation still need owner API consolidation. Broad agent REST/UI revocation acceptance is not claimed. |
| WS13 | Call mode/key/error fields render from profile settings. Actual global `Chrome::huddle_configured`, live huddle/ringing/transport and group-huddle criteria remain flagged. |
| WS8b-m / WS8br | Optional whole `show.shell.message_list`/composer override, `/rooms/:id/members.json` (501), member panel, group rename/add/leave timeline and room icon/composer flows remain flagged. Existing messages and the viewer-local unread divider now render through the cached-message fallback; actual author-card DM/focus flow is covered. Do not equate directory/card coverage with member-panel coverage. |
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


The three review findings are closed; the original 14 criteria and listed integration seams are still partial. The compiler target cache and the small parity-output `target` directory generated by the tests were deleted after verification. Paths and process/container ownership were checked first; no other output or source was removed.

## Cleanup

```text
WS8br2 cleanup: deleted .scratch/main-integration-clone/rust/target (18G build cache) and .scratch/review-fresh/rust/target (40K generated parity outputs); 0 scratch target directories; 0 owned active target processes; 0 owned running containers
```

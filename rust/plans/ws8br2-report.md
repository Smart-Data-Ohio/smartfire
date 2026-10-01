# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-10-01. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Verified implementation: `b1ffb3eca42cacb41fab0109c5a60cbdf25eb56b`; the final documentation-only commit carries this report. Each implementation slice is pushed. No stash, PR, deployment, release build, screenshot or pixel-diff work.

This continuation closes **17 original criteria, 31 → 14 deferred**: ten agent-profile cases, one agent-owner deactivation case, two message-author/card browser cases, and all four first-run-tour cases. **180/194** original criteria now have equivalent Rust coverage. The per-file mapping and every remaining name appear below. This mapping does not claim that the original Ruby test files ran. Owner seams are listed separately and are not additional criteria counted as complete.

## Pushed slices

| Commit | Slice |
|---|---|
| `f3c89dc6` | Merge `origin/main` at `65ad0d39` with a merge commit; integrate #167, #170, #171 and #173. Resolve 22 content conflicts while retaining the owned Rails-exact pages and main's producers/tests. Locked metadata succeeds. |
| `c8333975` | Restore the main room-message assertion omitted during conflict resolution. The real cached message renderer remains the fallback when the WS8b-m whole-list override is absent. |
| `c403be88` | Complete agent user profiles through the WS11 model: ten exact Rails HTML/nav cases, private-room filtering and owner/admin-only controls. |
| `cd07ca83` | Execute the Rails agent-owner self-deactivation vector through the merged lifecycle, including session removal, suspension and exactly two distinct audit rows. |
| `df1deef9` | Actual room-message author/card keyboard and DM flows, and all four first-run-tour browser scenarios, in both Rails and Rust. Add first-failure browser diagnostics. |
| `23dbdd68` | Read Fizzy profile token usability through the merged owner model, including unreadable-token disconnection. Six exact fragments and real profile HTTP/persisted-side-effect cases. |
| `56a19da0` | Match Rails account audit ordering after attachment save/after-commit callbacks. NullAnalyzer failure preserves the primary logo write but prevents the later audit. |
| `88d47ca9` | Strengthen signed-blob filename regressions with a namespace-prefixed SVG whose identification depends on the sanitized extension. Ten signed icon/logo HTTP cases. |
| `e186beb1` | Fix the three stray bytes emitted by the merged message-list fallback for empty rooms. Existing Rails empty-shell goldens remain unchanged. |
| `b1ffb3ec` | Capture five production-style Rails audit-failure responses/saved snapshots outside a wrapping transaction. Commit join-code, styles and logo changes before auditing; keep WS9 as the sole producer and normal exactly-once checks. |

Prior delivered users/cards/preferences, account views/mutations, audit HTML/CSV, icons/logos, welcome/first run, public/PWA/QR, complete profile and approved #163 status popup remain covered by the new complete fresh-clone run.

## Changes by file and design

Paths below are relative to `rust/`; abbreviated `controllers/...`, `app/...`, `account_security.rs` and `channels.rs` paths are under `crates/campfire/src/`, and `views/...` abbreviates `crates/views/...`. Main's provider/domain work is imported by the merge, not reimplemented here.

| Files | Changes and evidence |
|---|---|
| `controllers/users/{profiles,statuses,notification_settings,appearance_settings}.rs`, `controllers/presenters/{profile_sections,status_settings,layout_preferences}.rs`, `views/src/users/{profiles,profile_sections,status_popup}.rs`, corresponding profile/status/settings templates | Preserve the complete pinned profile and approved popup while using main's WS9 security/session/password paths, WS17 `UserStatusSettings/save_status`, effective OOO readers and notification form data. Invalid requests retain unsaved form values. Popup success is frame 303/page 302; invalid form/page is 422. Main standalone settings templates remain at `users/settings/...`; the approved profile composition stays exact. |
| `controllers/users/profiles.rs`, `controllers/presenters/github.rs`, `views/src/github/connections.rs` | Call merged `presenters::github::connection` and `campfire_views::github::connections::profile` directly. Preserve Rails' connected-before-usable evaluation: do not overwrite the outer verified flag after a fragment usability check disconnects an unusable token. Main's GitHub controller/client/job tests are retained. |
| `controllers/presenters/agent_profile.rs`, `controllers/users.rs`, `views/src/users/agent_profile.rs`, `views/templates/users/agents/{_profile,_status_badge}.html` | Thin user-page adapter over WS11 `Agent::for_user`, kind/grant/activity summaries and room memberships. Private rooms are included only when the viewer has membership; action links require the Rails owner/admin capability. Daily usage follows the viewer's zone through WS11's public daily-window function; three read-only aggregate queries remain flagged for owner consolidation. No lifecycle/policy producer duplicated. |
| `controllers/users/agent_profile_tests.rs`, `reference-tools/users/agent_profiles.rb`, `vectors/users_agent_profiles.json` | Ten exact complete HTML/nav states: capabilities, provider/runtime/status/identity, room lists, grants, owner/admin activity, peer privacy, private rooms, suspended/minimal agents. Actual authenticated GETs also exercise the integrated page. Render-only comparisons lend the declared Rails renderer tokens; HTTP checks keep actual CSRF. |
| `controllers/accounts/mutation_tests.rs` | Execute the existing `deferred_agent_owner_cases` Rails row (historical field name retained) rather than skipping it. David deactivation suspends his agent, removes his sessions and writes one `agent.suspend` plus one `user.deactivate`, with exact actor/target/action/details/IP/UA. The fixture precondition removes Kevin sessions to reproduce the Rails scenario's prior sequence. |
| `controllers/presenters/fizzy_profile.rs`, `controllers/presenters/profile_sections.rs`, `controllers/users/profiles.rs`, `views/src/users/profile_sections.rs`, `views/templates/users/profiles/_fizzy_connection.html` | Call WS15e `integrations::fizzy::accounts::Account::{for_user,usable_token}` inside the write boundary required for unreadable-token disconnection, then reload its reason. No raw credential display or reason-only usability inference. Rails blank-reason semantics retained. |
| `controllers/users/fizzy_profile_tests.rs`, `reference-tools/users/fizzy_profile.rb`, `vectors/users_fizzy_profile.json` | Six complete fragments and actual profile HTTP/readback states: missing, usable, blank token, blank reason, unreadable ciphertext, provider rejection. Assert disconnection reason persists and no token leaks. The oracle inserts corrupt ciphertext using SQL: `update_column` encrypts it and would not test unreadability. Full seed profile still matches without changed expected bytes. |
| `controllers/presenters/attachments.rs`, `controllers/{accounts.rs,accounts/icons.rs,users.rs,users/profiles.rs,first_runs.rs}` | Integrate #171's fallible assignment/signed-ID identification and durable analysis producer directly; remove duplicate in-memory `analyze_later`. Preserve first-run Account commit before User creation, as Rails does. Existing main restart/repeated-delivery/analyzer/rollback/readback checks remain intact. |
| `controllers/accounts/attachment_tests.rs`, `reference-tools/users/{attachment_endpoints,first_run_attachment_failure}.rb`, `vectors/users_{attachment_endpoints,first_run_attachment_failure}.json`, `vectors/workspace_icons/namespaced.svg` | Ten signed-ID assignments compare raw and Rails-sanitized filenames, MIME, complete metadata, reused blob identity, redirects, durable-job counts and audit deltas. Rejecting durable enqueue rolls back icon/attachment/identification/audit. Three text-logo NullAnalyzer/audit cases assert exact after-commit persistence and audit behavior. Production-style Rails first-run failure is 500 with account/user/attachment/blob counts `[1,0,0,0]`, no rooms or sessions; correct the shared test's unsupported all-zero rollback assumption against this oracle. |
| `controllers/accounts.rs`, `account_security.rs`, `controllers/accounts/{join_codes,custom_styles,logos}.rs` | Rails saves account/attachment first and calls AuditLog afterward. Keep WS9's same producer functions, split mutation and audit transaction boundaries, and record each normal audit exactly once. A callback/audit failure returns 500 without undoing an already committed save. Styles still audit only real changes and retain exact UTF-8 sizes/digest prefixes. |
| `app/round_four_security_tests.rs`, `reference-tools/users/account_audit_failures.rb`, `vectors/users_account_audit_failures.json` | Replace the handwritten rollback expectation with five independently observed production HTTP failures and committed account/blob/audit snapshots. Assert response/location, identity/creation time, name/styles/restriction/join-code change, attachment presence, filename/MIME/size/checksum/metadata, and zero audit rows. The oracle has no wrapping transaction. Stop Rust workers before fixture setup to match Rails' queued, non-performing test adapter; synchronous NullAnalyzer callbacks remain active. Normal account-audit tests are retained. |
| `crates/views/src/rooms.rs`, `controllers/rooms.rs`, `app/tests.rs` | Preserve real nonempty cached messages and main's 40/41-message assertions; empty collection produces zero bytes, matching Rails. Keep room shell/unread/OOO integration and the optional WS8b-m override. Shared room shell changes are small integration fixes; whole member-panel/composer work remains flagged. |
| `channels.rs`, `controllers/presenters/test_support.rs`, `views/templates/pwa/service_worker.js`, `controllers/pwa.rs` | Merge main's GitHub/Fizzy/X/link and WS17 channel/status paths while retaining directory messaging partials, seeded frozen/env/first-run helpers, owned PWA tests and main PWA vectors. Preserve held ephemeral listeners from #173. |
| `reference-tools/users/browser_{room,tour}.mjs`, `browser_people.py`, `browser_diagnostics.mjs`, existing six browser drivers | Two actual room-author/card scenarios and four tour scenarios. Tour private seed declares completion/null preconditions, then tests all five steps, arrows, finish/skip persistence and help-menu restart. On the first failure, record JS/console/network failures and registered/connected Stimulus modules. No retries, DOM replacement, screenshot diffing or response masking. |
| `reference-tools/users/{discrimination.py,run_oracles.sh,verify_goldens.py,file_counts.py,deferred_inventory.py}` | Compiled mutation probes, 32 regenerated owned oracle files, shared complete layout/route/sidebar/auth goldens, 142 executed owned Rust groups, and exact 194-criterion accounting. All input fixtures are committed or generated privately from declared inputs. |

## Rails inputs and upstream re-diff

Pin: `d7c7de9264c63015be398001d7a1094e7695a6db`.
Approved owned drift: `2e20b24c3f2be9db8a646a1352c159b4afacad0e` (#163), exactly these ten files:

- `app/controllers/users/statuses_controller.rb`
- `config/routes.rb`
- `app/views/users/statuses/edit.html.erb`
- `app/views/users/statuses/_fields.html.erb`
- `app/views/users/profiles/_status.html.erb`
- `app/views/users/cards/show.html.erb`
- `app/views/users/sidebars/show.html.erb`
- `app/views/layouts/application.html.erb`
- `app/assets/stylesheets/people.css`
- `app/javascript/controllers/profile_card_controller.js`

The owned status image checks 2076 Rails source/fixture/gem inputs: exactly ten approved changes; all others pinned. Non-runtime `bin/release` is absent. No authored Rails app changes or additional overlay. Main's board-only drifts (#162/#164/#165) remain their owners' work and are not used for these oracles. Render-only comparisons lend `GLOBAL`, `method:path` and `NONCE` at both renderer boundaries; actual HTTP/browser mutations retain real sessions and CSRF. Complete outputs are compared without masks or normalization.

The upstream controller re-diff was rerun from the fresh clone with `git diff --name-only d7c7de9264c63015be398001d7a1094e7695a6db 2e20b24c3f2be9db8a646a1352c159b4afacad0e --` followed by `app/controllers/accounts_controller.rb`, `app/controllers/accounts`, `app/controllers/users_controller.rb`, `app/controllers/users`, `app/controllers/first_runs_controller.rb`, `app/controllers/welcome_controller.rb`, `app/controllers/public_pages_controller.rb`, `app/controllers/pwa_controller.rb`, `app/controllers/qr_code_controller.rb`, `app/controllers/workspace_icons_controller.rb`. Its sole output is `app/controllers/users/statuses_controller.rb`; the harness asserts that exact list.

## #171 review findings closed

The lead's historical Astra review of `a37883d6` is not an independent review of this continuation. Its two shared attachment P2s are closed by merged #171 plus these regressions: signed-ID assignment, Rails-sanitized identification/copies and durable analysis. No separate attachment producer was added. Main's real analysis restart/repeated-delivery and Rails readback tests are retained in the full run.

A subsequent actual Rails failure oracle exposed the account audit-order mismatch, now fixed. Primary attachment save, synchronous after-commit NullAnalyzer and later controller audit have distinct failure boundaries. A rejected durable enqueue still rolls back the primary transaction; a failed after-commit NullAnalyzer or later audit preserves the primary save, matching Rails.

## Flagged owner seams and precise remaining integration

| Owner | Current state / remaining seam |
|---|---|
| WS9 | Direct merged security, passwords, sudo, sessions and audit producers. Nine individual normal mutations assert exactly one complete Rails audit row. Account post-save failure boundaries are now corrected through those producers; no duplicate audit path. |
| WS15g | Direct merged `presenters::github::connection` and `campfire_views::github::connections::profile`. The previous placeholder call site is closed. Other GitHub APIs use imported owner code; this worker does not claim new live GitHub acceptance. |
| WS15e | Fizzy profile usable-token/decryption/disconnection seam is closed through its model. Slack import link renders, but no merged Slack import backend was found in this checkout; live import remains flagged. |
| WS17 | Profile status/effective OOO and push-notification form/save seams use merged APIs directly. Unsaved invalid values render. `layout_preferences::fill` still projects cached ISO windows; broader `Time.zone.parse` grammar and live cable partial delivery remain unproved here. Seed full-page/fragment/browser status checks do not prove live push delivery. |
| WS14g | Raw Google account/scopes/configuration, identity-email and `fetch_error` cache projections in `profile_sections.rs`/`profiles.rs` need owner APIs. Configured sign-in/Calendar/Drive metadata display is covered; OAuth start/callback, grants/refresh, unusable-token handling, live Calendar, and `Chrome::google_picker` remain flagged. |
| WS11 | Agent user-profile facts, owner/admin visibility and the specific owner-deactivation lifecycle are covered. Inbox preferences, agent approval/work projections, three read-only budget aggregates and lifecycle snapshot validation still need owner API consolidation. Broad agent REST/UI revocation acceptance is not claimed. |
| WS13 | Call mode/key/error fields render from profile settings. Actual global `Chrome::huddle_configured`, live huddle/ringing/transport and group-huddle criteria remain flagged. |
| WS8b-m / WS8br | Optional whole `show.shell.message_list`/composer override, `/rooms/:id/members.json` (501), member panel, group rename/add/leave timeline and room icon/composer flows remain flagged. Existing messages now render through main's cached fallback; actual author-card DM/focus flow is covered. Do not equate directory/card coverage with member-panel coverage. |
| WS12 | Four star/member-row/phone criteria remain. Star mutations/room-member integration are owner work. Board-only approved drift is not imported into this owned oracle. |

Broader Ruby `Date.parse` grammar and Expat versus Nokogiri XML syntax/encoding outside the committed oracle inputs remain unproved. No new policy decision needs a user answer. No pixel work is pending. These boundaries are not counted as additional completed criteria.

## Fresh-clone verification

The fresh checkout is `.scratch/main-integration-clone`. It was created with `git clone --no-local --single-branch --branch rust/ws8br2-users-accounts . .scratch/main-integration-clone` from the committed branch; its origin was then set to GitHub and final sources fast-forwarded from the pushed branch. At clone creation, no untracked fixture, seed or reference archive was copied from the original checkout. Both parity seeds and the pinned source archive were generated privately. Later declared preflight source/vector copies were verified and replaced by the pushed commits before final checks. A previously compiled target cache was moved into this clone; this is **not** an empty-target claim. Final native runs use pinned Rust 1.98.1, the clone's absolute reference path, generated seeds, and clean GitHub-backed source at `b1ffb3ec`. The earlier plain `cargo` preflights used 1.98.0 and are not the final full-suite evidence. No test reads untracked scratch fixtures.

Preflight source copies were verified byte for byte, explicitly restored, and replaced by a fast-forward pull before final checks. `CI=1` makes missing seeds fatal. Four test threads, `CARGO_BUILD_JOBS=2`, no extra jobs or throttle bypass. Matching libvips/FFmpeg tools and dependencies are extracted into a private directory; image and video byte assertions remain enabled. Browser checks use the locked Playwright image and actual signed sessions/CSRF. Final source checks are clean. Owned scratch target directories are removed after all verification processes finish; seeds/oracle/log artifacts remain for review.

Final native/browser/oracle commands run from the clone root unless noted. Exact setup commands rerun:

```sh
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed check-image
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_NAMESPACE=ws8br2-merge-main-seed PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
bash rust/reference-tools/users/media_runtime.sh
```

Common native environment:

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/rust/target"
export CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference"
export TMPDIR="$PWD/.scratch/tmp"
export CI=1
export MAIL_TEST_PORT_RANGE=52640-52669
export CABLE_TEST_PORT_RANGE=52670-52699
export PATH="$PWD/.scratch/rails-media/usr/bin:$PATH"
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs"
```

Commands rerun (Docker/host oracle tools run without the native library overlay; browser driver retains it only where needed):

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
python3 rust/reference-tools/users/discrimination.py account-audit-order logo-null-analysis fizzy-token-usability agent-private-room agent-owner-actions signed-blob-filename attachment-durable-enqueue
python3 rust/reference-tools/users/discrimination.py join-audit-rollback styles-audit-rollback logo-audit-rollback
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -p campfire
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
env -u LD_LIBRARY_PATH python3 rust/reference-tools/users/verify_goldens.py
python3 rust/reference-tools/users/browser_people.py
python3 rust/reference-tools/users/browser_people.py --picker
python3 rust/reference-tools/users/browser_people.py --status
python3 rust/reference-tools/users/browser_people.py --pwa
python3 rust/reference-tools/users/browser_people.py --audit
python3 rust/reference-tools/users/browser_people.py --timezone
python3 rust/reference-tools/users/browser_people.py --room
python3 rust/reference-tools/users/browser_people.py --tour
python3 rust/reference-tools/users/file_counts.py ../merge-main/final-workspace-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
git pull --ff-only --no-rebase
git diff --exit-code
git diff --cached --exit-code
```

The first seven compiled probes ran in this fresh clone at `88d47ca9`; the three additional boundary probes and final suite/clippy/browser/oracle checks use `b1ffb3ec`. Their raw lines are retained separately below rather than presenting all ten as one source-state run. Each probe restores its source in `finally`; final tests compile the restored source.

## Failure-first evidence and verification limits

Compiled regressions deliberately remove private-room/owner controls, usable-token checking, sanitized filename consumption, durable enqueue, NullAnalyzer execution or correct account audit ordering. Each relevant HTTP/golden test fails; the restored sources pass. The first eight simple signed-ID filename cases alone did not catch a raw-filename mutant because magic-byte identification was sufficient. An independently evaluated namespace-prefixed SVG adds a filename-dependent case for each endpoint; now the unchanged raw-filename mutant fails. Expectations were strengthened by new Rails observations, not changed to fit Rust.

The first full fresh-clone run found a handwritten WS9 rollback assertion inconsistent with production Rails and three stray empty-room bytes. New Rails failure snapshots and the empty-collection implementation fix resolve them. No failure is dismissed as inherited. Image-analysis metadata racing the new failure snapshot was a harness mismatch: stop workers before fixture setup to match Rails' explicit test queue adapter, without changing the oracle or dropping metadata assertions. Durable execution/restart remains exercised by main's real-runner tests.

The earlier reported browser startup timeout did not recur in this continuation's final scenarios. Its historical cause is still unknown, not claimed fixed. Every browser scenario now captures first-failure network/module/Stimulus diagnostics; a future recurrence should be diagnosed from those artifacts rather than retried. The tour help action is an actual Rails `menuitem`, not a button; the harness uses that role after inspecting the pinned menu.

Raw summaries below are copied from this continuation's rerun logs under `.scratch/merge-main/`; full logs and generated oracles remain available.

## Raw setup and source summaries

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-9259468407ef675b49c8961d58752d30a197f666305cd34b0f0295c7ae2e3fd5
seed_key=rust-parity-seed-v1-925ec754eb3bc8d2a456564880d5aa3c4568f4ad4ef1f99bc1afc09f83cddcde
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
WS8br2 pinned media runtime: image ws8br2-reference:d7c7de92; libvips, FFmpeg tools and libraries extracted; no host libraries changed
Already up to date.
Fresh clone final implementation: b1ffb3eca42cacb41fab0109c5a60cbdf25eb56b
Fresh clone tracked and staged source: clean
WS8br2 upstream controller re-diff: only users/statuses_controller.rb changed from d7c7de92 to approved 2e20b24c; all other owned controller paths unchanged
```

`ci-seed check-image` and locked metadata exited 0 without summary output. Setup was rerun again after native verification; pin/image/seed keys equal the earlier fresh-clone setup. No rendered input changed. Native versions verified: `cargo 1.98.1 (797e8a9bc 2026-08-05)` and `rustc 1.98.1 (48a229cea 2026-09-01)`.

## Raw compiled regression probes

At `88d47ca9`:

```text
account-audit-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 1.72s
logo-null-analysis: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 1.06s
fizzy-token-usability: test result: FAILED. 10 passed; 2 failed; 0 ignored; 0 measured; 1092 filtered out; finished in 3.71s
agent-private-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 0.83s
agent-owner-actions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 0.74s
signed-blob-filename: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 2.16s
attachment-durable-enqueue: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 1.75s
WS8br2 discrimination: 7 compiled regressions detected; sources restored
```

At `b1ffb3ec`:

```text
join-audit-rollback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 4.45s
styles-audit-rollback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 5.61s
logo-audit-rollback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 7.92s
WS8br2 discrimination: 3 compiled regressions detected; sources restored
```

## Raw Rails oracle verification

```text
WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes
WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte
WS8br2 status image source verification: 2076 Rails source/fixture/gem files checked; exactly 10 approved 2e20b24c inputs, all others d7c7de92; non-runtime bin/release omitted
WS8br2 worker harness provenance: pinned Rails assertions unchanged; only stdin source loading differs
Rails public oracle: 15 page bodies, 31 policy inputs, 4 QR cases; reference d7c7de92
Rails avatar oracle: 15 initials SVG bodies; reference d7c7de92
Rails avatar images oracle: 2 real uploads, complete WebP and fallback SVG bodies with cache headers; libvips 8.16.1; reference d7c7de92
Rails preference oracle: 17 time-zone cases, 1 tour touch; reference d7c7de92
Rails Fizzy profile oracle: 6 complete fragments and token-usability side effects; reference d7c7de92
Rails agent profile oracle: 10 complete HTML/nav states; reference d7c7de92
Rails people oracle: 13 cards, 2 directories; reference d7c7de92
Rails profile settings oracle: 31 PATCH cases; reference d7c7de92
Rails appearance oracle: 4 bodies, 135 zone choices; reference d7c7de92
Rails account mutation oracle: 23 HTTP cases with audit snapshots, 1 deferred agent-owner case; reference d7c7de92
Rails account views oracle: 13 rows, 2 settings bodies/navs/footers, 2 invites, 2 CSS bodies; reference d7c7de92
Rails audit logs oracle: 65 rows, 14 complete HTML/nav/CSV cases, 15 date parses; reference d7c7de92
Rails icons oracle: 39 validation cases, 3 complete HTML/nav cases; reference d7c7de92
Rails attachment endpoint oracle: 10 signed icon/logo assignments; 3 after-commit NullAnalyzer responses and persisted snapshots; reference d7c7de92
Rails account audit failure oracle: 5 production HTTP responses and committed account/blob snapshots; reference d7c7de92
Rails first-run attachment failure oracle: status 500; counts [1, 0, 0, 0]; rooms 0; sessions 0; reference d7c7de92
Rails logos oracle: 9 complete PNG bodies with response/cache headers; libvips 8.16.1; reference d7c7de92
Rails first run oracle: 1 complete body, 7 HTTP/persisted-state cases; reference d7c7de92
Rails welcome oracle: 1 complete body/sidebar, 2 visible-room redirects; reference d7c7de92
Rails full profile oracle: 1 complete application page and body; real seed memberships and WS9 security; reference d7c7de92, status/layout templates 2e20b24c
Rails Google sign-in display oracle: 8 complete sign-in bodies and configuration cases; reference d7c7de92
Rails effective OOO profile oracle: 12 persisted Calendar/manual/boundary/zone cases; complete status panel bytes; status template 2e20b24c, model files d7c7de92
Rails layout preferences oracle: 34 persisted settings/cache cases; complete sound and Drive meta bytes; reference d7c7de92
Rails profile sections oracle: 9 complete Google Calendar fragments; reference d7c7de92
Rails status popup oracle: 5 complete popup bodies, 6 HTTP update/state cases; status files 2e20b24c, other files d7c7de92
Rails status panels oracle: 11 complete status/meeting/OOO fragments; status template 2e20b24c, model files d7c7de92
Rails DM picker oracle: 4 complete picker bodies; reference d7c7de92
Rails joining oracle: 1 complete signup body; 6 HTTP cases with user, room and session state; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails zone oracle: 485 case-sensitive TZInfo identifiers; reference d7c7de92
Rails named-zone oracle: 152 names, 2 unavailable; reference d7c7de92
WS8br2 oracle verification: all 32 fresh files match byte for byte; no masks or normalization
WS8br2 shared core verification: all 29 generated files match byte for byte
WS8br2 shared routes verification: complete generated JSON matches byte for byte
WS8br2 shared recognition verification: complete generated JSON matches byte for byte
WS8br2 shared sidebar verification: complete generated JSON matches byte for byte
WS8br2 shared WS9 verification: all 15 complete auth pages match byte for byte
WS8br2 golden verification: sources, both fresh seeds, 32 owned oracles and shared core/routes/sidebar/auth pages passed; no masks or normalization
```

## Raw fresh-clone native summaries

App summary first; all 51 target summaries follow in Cargo output order. The accounting line is the sum of these raw Cargo lines. No failure or filtered test. Eleven explicit ignores are listed separately.

```text
test result: ok. 1102 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 464.51s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.29s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 660 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 93.48s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.07s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.91s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.95s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.76s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.37s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.30s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.11s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.49s
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
WS8br2 workspace accounting: 51 target summaries; 2451 passed; 0 failed; 11 ignored; 0 measured; 0 filtered out
```

Raw debug build and clippy completion, respectively:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 25s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 30s
```

Raw explicit ignores:

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

The printed local missing-seed skip belongs to the intentionally simulated `missing_seed_may_skip_locally` unit check; it is not a skipped seeded app test. The corresponding `missing_seed_fails_in_ci - should panic` and `built_seed_is_found_in_ci` checks both pass. Missing real seeds remain fatal with `CI=1`.

## Raw browser summaries

Each pair is Rails then Rust. **31 scenarios per implementation, 62 passing observations**, plus the six PWA worker mutations caught across both runs.

```text
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
WS8br2 browser PWA: 3 passed; 0 failed; 3 worker regressions detected; Chromium 153.0.8010.12; actual served worker and offline shell
WS8br2 browser PWA: 3 passed; 0 failed; 3 worker regressions detected; Chromium 153.0.8010.12; actual served worker and offline shell
WS8br2 browser audit: 2 passed; 0 failed; Chromium 153.0.8010.12; real signed session, filters, sudo and CSV download
WS8br2 browser audit: 2 passed; 0 failed; Chromium 153.0.8010.12; real signed session, filters, sudo and CSV download
WS8br2 browser timezone: 2 passed; 0 failed; Chromium 153.0.8010.12; missing-token guard, real CSRF PATCH and persisted-zone readback
WS8br2 browser timezone: 2 passed; 0 failed; Chromium 153.0.8010.12; missing-token guard, real CSRF PATCH and persisted-zone readback
WS8br2 browser room cards: 2 passed; 0 failed; Chromium 153.0.8010.12; real message author triggers, signed session and CSRF
WS8br2 browser room cards: 2 passed; 0 failed; Chromium 153.0.8010.12; real message author triggers, signed session and CSRF
WS8br2 browser tour: 4 passed; 0 failed; Chromium 153.0.8010.12; keyboard, persistence and help-menu behavior; real signed sessions and CSRF
WS8br2 browser tour: 4 passed; 0 failed; Chromium 153.0.8010.12; keyboard, persistence and help-menu behavior; real signed sessions and CSRF
```

## Actual owned Rust per-file pass counts

```text
app/round_four_security_tests.rs (new production failure oracle): 1 passed; 0 failed; 0 ignored
controllers/users/fizzy_profile_tests.rs: 6 passed; 0 failed; 0 ignored
controllers/users/agent_profile_tests.rs: 10 passed; 0 failed; 0 ignored
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
controllers/users/profile_page_tests.rs: 5 passed; 0 failed; 0 ignored
controllers/first_runs/tests.rs: 3 passed; 0 failed; 0 ignored
controllers/welcome/tests.rs: 2 passed; 0 failed; 0 ignored
controllers/accounts/audit_logs/tests.rs: 5 passed; 0 failed; 0 ignored
controllers/accounts/icons/tests.rs: 9 passed; 0 failed; 0 ignored
controllers/accounts/logos/tests.rs: 2 passed; 0 failed; 0 ignored
controllers/accounts/view_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/users/people_tests.rs: 12 passed; 0 failed; 0 ignored
controllers/users/profile_settings_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/accounts/mutation_tests.rs: 12 passed; 0 failed; 0 ignored
WS8br2 file accounting: 142 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 24 account/ban cases, 9 individual exactly-one audit checks and one owner-removal with exactly two distinct Rails audits, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 10 complete bot profile HTML/nav cases; 6 complete Fizzy fragments and real profile HTTP/side-effect cases; 10 signed icon/logo assignments, durable rollback and 3 NullAnalyzer/audit after-commit states; 5 production account audit-failure states
```

## Original deferred criteria — largest files first

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

## Cleanup

```text
WS8br2 cleanup: deleted .scratch/main-integration-clone/rust/target (32G); 0 scratch target directories; 0 owned active target processes; 0 owned running containers
```

**Partial:** 14 original criteria remain, named above, plus the precise owner seams. Full cutover parity and new independent review are not claimed. All implementation slices are pushed; this report is mirrored at `rust/plans/ws8br2-report.md`.

# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Verified implementation: `b978d93cca5a433584989ca2d9f86f1c3c4e950c`. The final documentation-only commit carries this report. Implementation slices are pushed. No PR, deployment, release build or cutover.

The received 90 deferred criteria are now **57**: 33 additional criteria have equivalent coverage. Across the original 194-case inventory, 137 are covered and 57 remain. These are criterion mappings, not claims that the original Ruby test files ran. The complete per-file mapping and remaining names follow below.

## Pushed slices this continuation

| Commit | Completed slice |
|---|---|
| `f7639d50` | Remaining Google Calendar profile markup, exact WS15g GitHub fragment call-site flag, WS17 push-settings flag. |
| `851fd507` | Approved #163 status fields/popup, edit route, Current-user update scope, card/sidebar/layout controls and frame/error response paths. |
| `ca7424a3` | Public Google grant/configuration projection, nine complete Calendar fragments and eleven complete status/meeting/OOO fragments; 14 more profile criteria. |
| `6295b1ce` | Real Rails/Rust directory selection, cards and three-person DM browser checks; six more original system criteria. |
| `5f6bdcd7` | Replace obsolete DM autocomplete with the exact pinned people picker, four complete bodies and five browser criteria. |
| `40fd8876` | Seven profile security criteria using merged WS9 code directly: devices, reauthentication, linked/unlinked Google display, setup link, password revocation and name preservation. |
| `70ce22c7` | Merge main after PR #168 landed (`eaba80d5`); immediately run locked metadata. |
| `18b5fc7f` | Serve exact approved CSS/JS bytes, strict asset checks, ten-file hybrid Rails oracle, regenerated shared-layout/auth/sidebar/route goldens, five popup browser scenarios per app. |
| `a43d1936` | Fix fresh-clone verification defects: stable byte-slice assertion and the approved sidebar source hash. |
| `31585875` | Sidebar avatar keyboard opening, Escape and returned focus in both apps; one more original system criterion. |
| `ff3a8eff` | Use a literal character array for the same six Ruby Google scope separators; satisfy clippy without changing parsing behavior. |
| `b978d93c` | Scope the quote-refresh runner assertion to completion of its inserted quote job; preserve failure detection while periodic retention runs independently. Flagged WS3/WS8a test touch. |

Previously received work remains: users/cards/preferences; account mutations and views; WS9 audit deduplication and nine exactly-once checks; icons and logo uploads; audit-log HTML/CSV; welcome/first run; public/PWA/QR/signup pages. WS9's `account_security.rs` and `authentication.rs` are byte-identical to `4278cb1e`. Role, deactivation, account-settings/join-code/custom-style/logo-removal and ban/unban audit producers remain WS9's. Per-mutation checks assert one row with Rails' actor, target, action and metadata; no duplicate producer was restored.

## Changes by file

Paths are relative to `rust/`.

| Files | Behavior and evidence |
|---|---|
| `crates/views/src/users/profile_sections.rs`, profile templates `_google_calendar`, `_status`, `_inbox_calls`, `_notifications`, `_integrations`, `_github_connection`, `show` | Pure typed display inputs; complete seeded profile page, direct WS9 security/session sections, nine Calendar and eleven status fragments. Shared status fields are rendered on the profile and popup. GitHub and push seams are explicitly flagged. |
| `crates/campfire/src/controllers/presenters/profile_sections.rs`, `controllers/users/profiles.rs`, `config.rs` | Read-only Google email/scopes/disconnection facts, separate WS9 sign-in configuration, client+secret Calendar configuration, meeting/cache notice and manual OOO return date. Ruby whitespace/scope semantics, partial grants, Drive preservation and failed-form previews are exercised. No OAuth client or token decryption added. |
| `crates/views/src/users.rs`, `templates/users/cards/show.html`, `templates/users/statuses/{edit,_fields}.html` | Exact approved card/status-popup bytes, escaped fields, errors and cancel/clear/save controls. |
| `crates/campfire/src/controllers/users/statuses.rs`, controller dispatch; `crates/db/src/models/user/status_form.rs` | Current-user-only four-field popup writer, existing User save validation, expiry/clear, atomic save and after-commit status event. Real auth/CSRF and failed-write rollback checks. Frame success 303/card, ordinary success 302/profile, frame errors 422/bare popup and ordinary error profile rendering. Calendar/OOO mutations remain explicitly 501 through the WS17 seam below. |
| `crates/views/src/rooms/sidebar.rs`, `templates/users/sidebars/show.html`, `templates/layouts/application.html`; `crates/routes/routes.json`, `vectors/campfire_routes.json` | Own-card sidebar trigger, global status-change action, approved edit route and Rails recognition order. Five complete sidebar frames and 312 named-route/534 recognition entries regenerated and compared. Shared-owner touch is limited to these approved controls. |
| `crates/assets/overrides/{people.css,controllers/profile_card_controller.js}`, `OVERRIDES.md`, `crates/assets/tests/reference.rs` | Exact `2e20b24c` source/served bytes. If an approved override equals the reference fingerprint it stays in the ordinary strict compiled-byte checks. No new asset exclusion or golden mask. Fresh-clone checks include both full bodies and all nine reference tests. |
| `crates/views/src/rooms.rs`, `templates/rooms/directs/new.html`; `controllers/rooms/directs.rs`, `directs/picker_tests.rs` | Exact pinned people picker, shared multi-select bar and existing directory reader. Four complete bodies: seed, starred, escaped and empty. Five HTTP/render groups and five real browser flows. DM writes still use WS8b-r's existing domain. |
| `crates/campfire/src/jobs/tests.rs` | WS3/WS8a cross-workstream test-only correction: wait for and assert quote-job completion independently of periodic retention; inserted-row and failed-job checks remain. The original fresh-clone failure is included below. |
| `controllers/users/profile_sections_tests.rs`, `profile_security_tests.rs`, `status_popup_tests.rs` | Twelve owner-display/HTTP groups, seven direct WS9 integration groups and four popup groups covering five complete bodies and six HTTP/state cases, plus real CSRF and rollback. |
| `reference-tools/users/{browser_people.py,browser_people.mjs,browser_picker.mjs,browser_status.mjs}` | Committed isolated browser harness and scenarios; eight directory/sidebar, five picker and five popup scenarios per app. Original Stimulus modules, real signed sessions and CSRF; no DOM/response normalization. Test-added accented picker users live only in a derived private seed. |
| `reference-tools/users/post-pin/*`, `post_pin.rb`, `status_image.sh`, `verify_post_pin.py`, `verify_status_image.py`, `{status_core,status_auth_pages,status_routes}.rb`; updated vectors/core/sidebar goldens | Exactly ten authorized post-pin sources, Git-object and source-ledger checks, pin-based hybrid image with assets recompiled. Full engine-aware route reload. Other Rails inputs remain pinned. |
| `reference-tools/users/{run_oracles.sh,discrimination.py,file_counts.py,deferred_inventory.py}` | Twenty-three exact oracle comparisons, six new compiled regression probes, 87 executed Rust test-group accounting and all remaining named criteria, largest Rails files first. |

Retained earlier file changes are also exercised by the fresh-clone suite:

| Files | Retained behavior |
|---|---|
| `crates/db/src/models/audit_log/browsing.rs`, `controllers/accounts/audit_logs.rs`, audit-log views/templates | Filters, 50-row pages, 5000-row export cap, truncated filename, CSV quoting/formula neutralization, headers and WS9 sudo. Fourteen complete HTML/nav/CSV outputs and fifteen date inputs. |
| `crates/db/src/models/workspace_icon.rs`, `crates/storage/src/workspace_icon.rs`, account/private icon controllers and views | Normalization/uniqueness race, staged SVG/PNG validation, atomic attachments/audits, purge, exact serving bytes/cache/ETag/CSP/nosniff and authentication/enrollment gates. Thirty-nine validation inputs, nineteen committed media files and three complete view/nav cases. |
| `controllers/accounts/logos.rs`, logo tests/vectors; `crates/kit/src/ctx.rs` | Upload/replacement/removal/no-op audit behavior; nine exact stock/JPEG/BMP PNG bodies and cache headers. Rails flash ETag expansion. Canonical vips 8.16 libraries extracted into private scratch without changing the host. |
| `controllers/accounts/{mutation_tests,view_tests}.rs`, existing account/security models/views | Twenty-three HTTP/state/audit cases, nine individual exactly-one audit checks, settings/user rows/invites/custom-style views and authorization/sudo rollback. Agent-owner deactivation is still deferred. |
| `crates/db/src/models/first_run.rs`, `controllers/{first_runs,welcome}.rs`, corresponding views/tests | Optional credentials, exact setup/welcome bodies, repeats and races; preserve Rails' account-save-before-room/user transaction and missing-name/empty-name distinction; seven first-run state cases plus concurrent Rust requests. |
| `controllers/users/{joining_tests,people_tests,profile_settings_tests,profile_page_tests,avatars,time_zones,tours}.rs`, public/PWA/QR controllers/views | Signup/join grants/session state, thirteen card bodies/two directories, thirty-one profile PATCH vectors, appearance/zone preferences, initials SVG, public policy, manifest/service-worker/offline bodies and QR behavior. Uploaded avatar variants and browser cases below remain partial. |

## Rails inputs and upstream re-diff

Pin: `d7c7de9264c63015be398001d7a1094e7695a6db`.
Approved drift: `2e20b24c3f2be9db8a646a1352c159b4afacad0e` (#163), **only** these ten files:

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

`ws8br2-reference:d7c7de92-status-2e20b24c` derives from the owned pin image, copies exactly those sources and recompiles its assets. Fresh-clone verification checks 2076 Rails source/fixture/gem/runtime-bin inputs: precisely ten approved changes, all others pinned. Non-runtime `bin/release` is absent from the runtime image. Render-only comparisons lend `GLOBAL`, `method:path` and `NONCE` at both renderer boundaries. Real HTTP/browser checks retain actual authentication and CSRF. No authored Rails app changes or broader post-pin overlay. The unrelated board-nudge Rails drift is not imported into this oracle. Re-diff of 31 owned/account-nested controller files between the pin and `2e20b24c` was empty; the changed/new statuses controller is handled separately.

Main merge occurred only after #168 was merged; its asset-fingerprint helper is used directly. The report snapshot's merged main is `eaba80d5`; subsequently fetched `origin/main` is `b66199b7` (WS15e). That later provider integration is not merged into this snapshot. PR #167 remains open at this check.

## Flagged seams and remaining integration

| Owner | Exact seam and closure needed |
|---|---|
| WS9 | Security/profile sessions, sudo and audits are direct merged APIs, not fallback implementations. The render-only configured-Google test supplies the owner's reauthentication configuration interface; it never starts OAuth. |
| WS15g | `ProfileShow::github_connection` explicitly flags `campfire_views::github::connections::profile(&Connection)` from `rust/ws15g-github` / PR #167. Map `linked/usable/login/reason/app_token/app_configured` from its domain. Current fallback matches the pinned seed; replace reason-based projection with owner token-usability/decryption/disconnect side effects. |
| WS17 push | `_notifications.html` explicitly flags push settings, effective notification policy and the update controller. Push subscription delivery/test actions are not implemented here. |
| WS17 status / WS14g Calendar | `controllers/users/statuses.rs` returns 501 with `x-campfire-unported: users/statuses#WS17-calendar` when Calendar/OOO keys are submitted. Replace `StatusForm` with owner `UserStatusSettings/save_status`, then call public `after_save` / `render_invalid` to preserve #163 frame/page behavior. Meeting/OOO opt-in refresh, clearing caches, claims and broadcasts are owner work. |
| WS17 effective facts | Presenter return dates are **manual-only**. Pure eleven-state render checks include Calendar/overlapping OOO facts supplied at the rendering boundary; the live effective Calendar/later-end reader is not claimed. `Partial::UserStatus` after-commit event rendering/delivery is not proved by the browser's local `user-status:changed` event. |
| WS14g | Public Google metadata and all nine Calendar display branches are integrated. Replace adapter/configuration with owned GoogleAccount display facts and complete OAuth/grants/Drive/live Calendar composition. Sign-in identity/configuration already comes from WS9 directly. |
| WS11 | Inbox preferences/agent approval/work display inputs, agent profile/capabilities/activity, agent-owner deactivation and agent revocation remain owner seams. |
| WS13 | Call mode/key/error inputs are rendered; join owned call/huddle behavior and global configuration. Directory huddle button eligibility is checked, actual call ringing is deferred. |
| WS15e | Fizzy reason-based connection input, usable-token/decryption/mutation behavior and Slack import link need integration with its newly merged APIs. No provider client was duplicated. |
| WS6 / WS8b-r / WS14g / WS17 | Request-time `Layout::load` currently leaves owner chrome at defaults. Fill brand/recent-search/Drive/huddle/meeting/OOO/sound-window facts. Full profile golden supplies actual seed/render facts, as shared WS9/core goldens do; live HTTP profile HTML with all owner defaults is not claimed byte-identical. |
| WS8b-m / WS8b-r / WS13 / WS17 | Room `show.shell.message_list` is not populated by this branch, and `/rooms/:id/members.json` remains 501. Message-author and member-panel card/focus/group/huddle/timeline cases remain deferred. Standalone directory and sidebar coverage does not close those room-specific criteria. |

Other partial boundaries: broader Ruby `Date.parse` partial/relative grammar is unproved; SVG parser is Expat rather than Nokogiri/libxml2, so XML syntax/encoding beyond the committed cases is unproved; uploaded avatar variants, tours, timezone browser reporting, icon/audit browser flows and service-worker event execution remain deferred. Astra’s independent review of `a37883d6` completed: no authorization bypass; 19/19 oracle files matched; Rails read back Rust rows and both logo variants; profile sections were present; 5/5 deliberate mutations were detected. The lead relayed two P2 gaps in shared attachment code on main, pending `rust/durable-attachment-analysis`.

## Review findings pending the shared attachment fix

The lead relayed Astra’s two P2 findings in shared `crates/campfire/src/controllers/presenters/attachments.rs`:

- `ActiveStorage::AnalyzeJob` uses an in-memory queue and is not durable. Attachment analysis can be lost on process exit; the shared fix must persist its job atomically with the attachment write.
- Assigning an existing blob by signed blob ID fails: icon upload returns 422 and logo upload returns 500, while Rails returns 302. Normal multipart-upload coverage does not prove this assignment path.

Both are owned by `rust/durable-attachment-analysis`; no fix is authored here. Do not merge main until the lead requests it after that fix lands. Then add icon/logo HTTP regressions on top, including signed-ID assignment and analysis-job durability/rollback. These gaps are additional review findings, not removed from the original 194-case mapping or counted as covered.

The updated common rule excludes browser screenshot/pixel diffs. The remaining list contains interaction, HTTP/state and DOM behavior checks only; theme/visibility criteria use behavior checks without screenshot comparisons. Board-only drift `541c0f69` / `8952bed4` belongs to WS12; this branch does not share their board-page partials and keeps its existing pin/ten-file status overlay.

## Fresh-clone verification

The branch was cloned from GitHub into `.scratch/status-final-clone` at `18b5fc7f`, with an empty target and no copied seeds, media runtime, source archive or scratch fixtures. The clone pulled pushed verification fixes `a43d1936` and browser-only `31585875`; then pulled `ff3a8eff` (equivalent separator syntax) and `b978d93c` (job-test assertion scope). The final four-thread full suite and clippy run against that pushed snapshot. DB/kit/view/asset implementation is unchanged across the final verification fixes. Both seeds and the pin archive were generated there from tracked scripts; media libraries were extracted there from the source-verified owned image. Playwright uses the committed lockfile/Dockerfile. Missing real seeds are fatal under `CI=1`. Private target/TMPDIR and ports 52610–52612; no release profile.

Verification defects found by these checks were fixed and pushed: the new asset assertion used unstable slice `as_slice`, the sidebar oracle expected its pre-drift source hash, and clippy required the scope splitter character array. The repeated four-thread suite also observed the quote-refresh test asserting an entirely empty queue while a separate periodic `Retention::PruneJob` remained running. That failed log is retained. The focused retry and a serialized whole suite passed before the correction. `b978d93c` then corrected the assertion to require the enqueued quote job to be absent; a failed quote row still fails, and the initial inserted-row check remains. Periodic retention may legitimately coexist with that job. No production job code, timeout or ignore changed. The final four-thread whole suite passes after that correction. The initial failed log and raw line remain below; this is not labeled a proved inherited failure.

The obsolete earlier-clone target was removed after process checks. The final clone target is removed after completing verification, retaining raw logs and inputs. No test servers are left running.

Commands below were rerun for this delivery. Their raw summaries follow; paths/logs are under this worktree's `.scratch/`.

Common environment for native Rust commands (run in `.scratch/status-final-clone`):

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/rust/target"
export CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference"
export TMPDIR="$PWD/.scratch/tmp"
export CI=1
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/lib/x86_64-linux-gnu:$PWD/.scratch/rails-media/usr/lib/x86_64-linux-gnu"
```

Rust commands:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -j 4 -p campfire -p campfire_db -p campfire_kit -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -j 4 -p campfire_assets -p campfire_views --test core --test reference -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -j 4 -p campfire jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner -- --exact --nocapture --test-threads=1
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -j 4 -p campfire
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -j 4 -- -D warnings
```

Oracle environment: `CAMPFIRE_REFERENCE` as above, `PARITY_RUNTIME=docker`, `PARITY_OWNER=ws8br2`, `PARITY_NAMESPACE=ws8br2-final`, `PARITY_IMAGE=ws8br2-reference:d7c7de92-status-2e20b24c`. Host media overlay is omitted from host forwarding commands. All comparisons use raw bytes; generated JSON/HTML is not normalized.

```sh
python3 rust/reference-tools/users/verify_post_pin.py
python3 rust/reference-tools/users/verify_status_image.py
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" bash rust/reference-tools/users/run_oracles.sh
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default
rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run
rust/parity/bin/reference runner --seed first_run --time 2026-02-10T12:00:00Z --freeze rust/reference-tools/users/status_core.rb > ../final-core.json
python3 rust/reference-tools/views/core/split.py ../final-core.json .scratch/verified-core
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/users/status_routes.rb > ../final-routes.json
cmp ../final-routes.json rust/crates/routes/routes.json
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/users/status_routes.rb campfire > ../final-recognition.json
cmp ../final-recognition.json rust/vectors/campfire_routes.json
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/rooms/sidebar_page.rb > ../final-sidebar.json
cmp ../final-sidebar.json rust/crates/views/tests/golden/sidebar/page.json
rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -e WS9_FULL_PAGE_GOLDENS=/rails/storage/db/auth_full_pages.json -- bin/rails runner --skip-executor 'output=$stdout; begin; $stdout=$stderr; load File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/status_auth_pages.rb"); ensure; $stdout=output; end; puts File.read("/rails/storage/db/auth_full_pages.json")' > ../final-auth-pages.json
cmp ../final-auth-pages.json rust/vectors/auth_full_pages.json
python3 rust/reference-tools/users/browser_people.py
python3 rust/reference-tools/users/browser_people.py --picker
python3 rust/reference-tools/users/browser_people.py --status
python3 rust/reference-tools/users/discrimination.py status-prefix status-redirect status-csrf status-scope profile-calendar dm-picker-view
python3 rust/reference-tools/users/file_counts.py ../final-rust-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
git diff --exit-code
git diff --cached --exit-code
```

Core comparison additionally checks every generated `.scratch/verified-core` file against `rust/crates/views/tests/golden/core` using `Path.read_bytes()` equality (29 files); route/helper/sidebar/auth JSON use the exact `cmp` commands above. Six compiled regression probes ran before the restored full suite, detected every inserted regression and restored sources; no historical discrimination count is carried forward as fresh proof.

## Raw verification summaries

### Source verification

```text
WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes
WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte
WS8br2 status image source verification: 2076 Rails source/fixture/gem files checked; exactly 10 approved 2e20b24c inputs, all others d7c7de92; non-runtime bin/release omitted
```

### Fresh seed validator: default

```text
  "passed": 29,
  "failed": 0
```

### Fresh seed validator: first_run

```text
  "passed": 4,
  "failed": 0
```

### All 23 Rails oracle comparisons

```text
Rails public oracle: 15 page bodies, 31 policy inputs, 4 QR cases; reference d7c7de92
Rails avatar oracle: 15 initials SVG bodies; reference d7c7de92
Rails preference oracle: 17 time-zone cases, 1 tour touch; reference d7c7de92
Rails people oracle: 13 cards, 2 directories; reference d7c7de92
Rails profile settings oracle: 31 PATCH cases; reference d7c7de92
Rails appearance oracle: 4 bodies, 135 zone choices; reference d7c7de92
Rails account mutation oracle: 23 HTTP cases with audit snapshots, 1 deferred agent-owner case; reference d7c7de92
Rails account views oracle: 13 rows, 2 settings bodies/navs/footers, 2 invites, 2 CSS bodies; reference d7c7de92
Rails audit logs oracle: 65 rows, 14 complete HTML/nav/CSV cases, 15 date parses; reference d7c7de92
Rails icons oracle: 39 validation cases, 3 complete HTML/nav cases; reference d7c7de92
Rails logos oracle: 9 complete PNG bodies with response/cache headers; libvips 8.16.1; reference d7c7de92
Rails first run oracle: 1 complete body, 7 HTTP/persisted-state cases; reference d7c7de92
Rails welcome oracle: 1 complete body/sidebar, 2 visible-room redirects; reference d7c7de92
Rails full profile oracle: 1 complete application page and body; real seed memberships and WS9 security; reference d7c7de92, status/layout templates 2e20b24c
Rails profile sections oracle: 9 complete Google Calendar fragments; reference d7c7de92
Rails status popup oracle: 5 complete popup bodies, 6 HTTP update/state cases; status files 2e20b24c, other files d7c7de92
Rails status panels oracle: 11 complete status/meeting/OOO fragments; status template 2e20b24c, model files d7c7de92
Rails DM picker oracle: 4 complete picker bodies; reference d7c7de92
Rails joining oracle: 1 complete signup body; 6 HTTP cases with user, room and session state; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails zone oracle: 485 case-sensitive TZInfo identifiers; reference d7c7de92
Rails named-zone oracle: 152 names, 2 unavailable; reference d7c7de92
WS8br2 oracle verification: all 23 fresh files match byte for byte; no masks or normalization
```

### Shared core/routes/sidebar/WS9 pages

```text
WS8br2 core oracle verification: all 29 regenerated files match byte for byte
WS8br2 route helpers verification: 312 regenerated entries match byte for byte
WS8br2 route recognitions verification: 534 regenerated entries match byte for byte
WS8br2 sidebar frames verification: 5 regenerated entries match byte for byte
WS8br2 WS9 full pages verification: 15 regenerated entries match byte for byte
```

### Full Rust suite: app, DB, kit unit/front/http/params/Rails vectors, DB/kit doctests

```text
test result: ok. 540 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 112.10s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 63.25s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Asset reference and core view suites

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
```

### Focused quote-job check

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.20s
```

### Debug build

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.58s
```

### Workspace clippy, -D warnings

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.21s
```

All active checks in the final run passed. Existing ignores: reference-recording opt-in, WS11 `manages_bots`, push-latency measurement; four DB external Ruby/export/rollback opt-ins; two kit example doctests. No ignored tests were added. The missing-seed guard intentionally tests a missing temporary fixture; real seeds were validated and used under `CI=1`.

The initial concurrent run failed before the job-test assertion correction; its command was the same full-suite four-thread command above. Raw failure preserved from `final-rust-tests-concurrent-failure.log`:

### Initial concurrent failure (resolved by b978d93c)

```text
test result: FAILED. 539 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 159.38s
```

Compiled regressions were deliberate mutations, not delivery failures. Each failed the relevant actual Rust check before restored-source validation.

### Six new discrimination checks

```text
dm-picker-view: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.16s
status-prefix: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.12s
status-redirect: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.75s
status-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.85s
status-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.88s
profile-calendar: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 542 filtered out; finished in 0.76s
WS8br2 discrimination: 6 compiled regressions detected; sources restored
```

### Browser people: separate Rails and Rust runs

```text
Rails directory browser scenarios:
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
Rust directory browser scenarios:
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
```

### Browser picker: separate Rails and Rust runs

```text
Rails directory browser scenarios:
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
Rust directory browser scenarios:
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
```

### Browser status: separate Rails and Rust runs

```text
Rails directory browser scenarios:
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
Rust directory browser scenarios:
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
```

Browser coverage is 18 scenarios per app (36 successful executions): eight directory/sidebar, five picker, five popup. It does not count message-author or member-panel flows as passing. Five popup cases are outside the original 194-case inventory. Original Rails system files were inspected and their criteria translated; the Ruby files themselves were not run.

## Actual Rust per-file pass counts

### Counts from the final complete app log

```text
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
controllers/accounts/mutation_tests.rs: 11 passed; 0 failed; 0 ignored
WS8br2 file accounting: 87 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred
```

## Rails criterion mapping and all deferred names

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 45 | 12 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 12 | 7 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 0 | 2 |
| `test/controllers/users_controller_test.rb` | 226 | 20 | 10 | 10 |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 217 | 15 | 15 | 0 |
| `test/system/starred_people_test.rb` | 182 | 4 | 0 | 4 |
| `test/system/icons_test.rb` | 169 | 4 | 0 | 4 |
| `test/controllers/accounts/icons_controller_test.rb` | 103 | 6 | 6 | 0 |
| `test/system/service_worker_test.rb` | 102 | 2 | 0 | 2 |
| `test/controllers/users/bans_controller_test.rb` | 99 | 8 | 5 | 3 |
| `test/controllers/users/profiles_two_factor_test.rb` | 96 | 7 | 7 | 0 |
| `test/system/first_run_tour_test.rb` | 89 | 4 | 0 | 4 |
| `test/controllers/users/cards_controller_test.rb` | 87 | 7 | 7 | 0 |
| `test/controllers/workspace_icons_controller_test.rb` | 78 | 7 | 7 | 0 |
| `test/controllers/pwa_controller_test.rb` | 66 | 1 | 0 | 1 |
| `test/system/timezone_detection_test.rb` | 64 | 2 | 0 | 2 |
| `test/controllers/accounts_controller_test.rb` | 61 | 4 | 4 | 0 |
| `test/system/workspace_icons_test.rb` | 60 | 1 | 0 | 1 |
| `test/controllers/first_runs_controller_test.rb` | 57 | 4 | 4 | 0 |
| `test/controllers/accounts/logos_controller_test.rb` | 56 | 6 | 6 | 0 |
| `test/system/audit_log_test.rb` | 49 | 2 | 0 | 2 |
| `test/controllers/accounts/users_controller_test.rb` | 36 | 3 | 2 | 1 |
| `test/controllers/users/avatars_controller_test.rb` | 34 | 2 | 0 | 2 |
| `test/controllers/accounts/custom_styles_controller_test.rb` | 30 | 3 | 3 | 0 |
| `test/controllers/accounts/join_codes_controller_test.rb` | 21 | 2 | 2 | 0 |
| `test/controllers/welcome_controller_test.rb` | 21 | 2 | 2 | 0 |

`test/controllers/users/profiles_controller_test.rb` — **WS8br2 integration; WS9 security panels, WS17 status/notifications, WS13 calls, WS14/WS15 Google, WS15g GitHub seams**:
- the layout sends meeting windows for the live sound gate
- the layout sends future meeting windows before the meeting starts
- the layout sends no meeting windows without cached intervals
- the layout sends no meeting windows when meeting status itself is off
- the layout sends OOO windows for the live sound gate
- the layout sends future calendar OOO windows before the OOO starts
- the layout sends no OOO windows when keeping notifications while out
- the layout leaves sounds alone for meetings when quiet-during-meetings is off
- layout carries the Drive previews meta tag only with the Drive scope
- the layout carries the theme, time zone, and sound state
- the layout mutes sounds for the DND presence
- the layout sends the quiet-hours window and zone for the sound gate

`test/system/people_group_dms_test.rb` — **WS8br2 people/cards; WS8br DM actions**:
- clicking a message author opens their profile card and Message lands in the DM
- the profile card opens by keyboard, traps focus, and returns it on Esc
- the member panel multi-select starts a huddle with exactly that set
- group members rename, add, and leave with system notes in the timeline
- starting a group huddle rings every other member
- Esc closes only the profile card inside the mobile member panel
- Tab cycles within the profile card opened from the member panel

`test/controllers/public_pages_controller_test.rb` — **WS9 sign-in page integration**:
- sign-in page links the public pages without OAuth
- sign-in page keeps public links beside Google sign-in when configured

`test/controllers/users_controller_test.rb` — **WS8br2; agent/bot presentation facts from WS11**:
- bot profile links to capability grants for admins
- bot profile links to capability grants for the agent owner
- bot profile hides capability grants from anyone else
- bot profile shows agent identity, status, rooms, and grants to a member
- bot profile shows the 24-hour activity line to the owner
- bot profile shows the 24-hour activity line to an admin
- bot profile hides the 24-hour activity line from another member
- bot profile hides rooms the viewer is not a member of
- suspended agent profile shows Suspended
- bot without an agent keeps the minimal profile

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

`test/system/service_worker_test.rb` — **WS8br2**:
- the worker caches static assets and never authenticated responses
- the offline shell renders with working retry behavior

`test/controllers/users/bans_controller_test.rb` — **WS8br2; WS9 sudo, WS11/WS13 revocation seams**:
- create succeeds when the user has a pending two-factor setup secret
- create enqueues RemoveBannedContentJob
- RemoveBannedContentJob deletes messages

`test/system/first_run_tour_test.rb` — **WS8br2 with WS8b-m composer/room integration**:
- a new member is walked through the tour by keyboard and finishing persists
- escape skips the tour and it never auto-starts again
- the tour restarts from the help menu
- members who completed the tour never see it auto-start

`test/controllers/pwa_controller_test.rb` — **WS8br2 service-worker event harness**:
- service worker fetch and notification logic

`test/system/timezone_detection_test.rb` — **WS8br2**:
- the browser does not report its zone without a CSRF token
- the browser reports its detected zone once

`test/system/workspace_icons_test.rb` — **WS8br2**:
- upload post in both themes then delete falls back to the shortcode

`test/system/audit_log_test.rb` — **WS8br2; WS9 export sudo seam**:
- admin browses filters and exports the audit log
- audit log stays usable at phone width

`test/controllers/accounts/users_controller_test.rb` — **WS8br2; WS9 sudo/security metadata seams**:
- destroy

`test/controllers/users/avatars_controller_test.rb` — **WS8br2**:
- show image
- show initials when image cannot be resized

Deferred inventory: 57 named Rails controller/system cases remain from the original 194; 137 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.

## Remaining work

Close the 57 named criteria above and each flagged seam. Largest remaining files are profile layout projection (12 criteria) and people room/member-panel integration (7). Complete agent/owner integration, live Calendar/OOO and status delivery, avatar variants and the listed browser/PWA cases. Integration lead should replace the exact WS15g call site when #167 merges and use WS15e’s newly merged provider APIs. No broader grammar/parser parity, clean original Ruby-system run or production readiness is claimed.

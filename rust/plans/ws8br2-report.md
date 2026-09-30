# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Verified implementation: `938fb64afde1f3931448a3ad081357ca0ded3cbf`; the final documentation-only commit carries this report. All implementation slices are pushed. No PR, deployment, release build or new main merge.

This continuation closes **13** more criteria: **57 → 44** deferred. Across the original 194-case inventory, **150** now have equivalent coverage and **44** remain. Relative to the previously received 90-case list, 46 have been closed. These are criterion mappings, not claims that the Ruby controller/system files ran. The complete mapping and every remaining name follow below, largest files first.

## Pushed slices this continuation

| Commit | Coherent slice |
|---|---|
| `ff2635f5` | Record Astra's two attachment P2s as pending the shared fix; retain behavior checks and remove screenshot-diff scope. |
| `a2815c51` | Real worker responses, the 19 original Rails Node assertions, static-only caching, offline retry and three worker regression probes per app. Three deferred criteria closed. |
| `cdb7d509` | Rails-produced audit fixtures; browser filters, phone scrolling and real sudo-confirmed CSV download against exact Rails bytes. Two criteria closed. |
| `6129ae93` | Missing-token guard and real CSRF timezone detection, persisted readback and no second report after navigation. Two criteria closed. |
| `3db7afc6` | Pending 2FA setup cleanup, atomic ban-job enqueue/rollback, real-runner message removal and unconfigured sign-in public links. Four criteria closed; four compiled regression probes detected. |
| `938fb64a` | Actual avatar uploads, complete WebP/BMP fallback bodies and Rails' flash-dependent ETag transition. Two criteria closed; two more compiled probes detected. One tracked command rebuilds all 24 owned and shared goldens. |

Prior delivered work is retained: users/cards/preferences; account mutations and views; WS9 audit deduplication and nine exactly-once checks; icon/logo multipart uploads; audit-log HTML/CSV; welcome/first run; public/PWA/QR/signup pages; full profile rendering with owner inputs; #163 status popup and exact approved CSS/JS; directory/sidebar/picker interactions; direct WS9 profile security/session panels. Earlier profile/status slices remain at `f7639d50`, `851fd507`, `ca7424a3`, `6295b1ce`, `5f6bdcd7`, `40fd8876`, `18b5fc7f` and `31585875`. Main was merged at `70ce22c7` only after #168 landed, with locked metadata checked; there is no later main merge. Existing verification corrections `a43d1936`, `ff3a8eff` and `b978d93c` remain tested.

WS9's `account_security.rs` and `authentication.rs` remain the producers from `4278cb1e`. No duplicate audit producer was restored. Nine individual mutation checks still assert exactly one row with Rails' actor, subject, action and metadata. This continuation changes tests, reference tooling and reports; production controller/domain behavior is unchanged.

## Changes by file and design

Paths are relative to `rust/`.

| Files | Change and evidence |
|---|---|
| `reference-tools/users/{browser_pwa.mjs,service_worker_harness.mjs,verify_worker_harness.py}` | The pinned Rails harness is byte-identical except its worker-source loading line: stdin receives the actual HTTP worker response. Nineteen event assertions run against Rails and Rust. Chrome checks real registration through the served initializer, app-link navigation, cache inventory and retry reload. A linked stylesheet is fetched after control to exercise caching without relying on Chrome's asset memory cache. Mutations break authenticated-cache exclusion, offline fallback and existing-tab focus; all are caught. No worker production code changed. |
| `parity/seeds/ws8br2_browser_audit.rb`, `reference-tools/users/browser_audit.mjs` | Fresh fixture rows go through Rails `AuditLog.record!`; expected CSV comes from its actual formatter in the user's zone. Both browsers filter, follow Export CSV, confirm the real password through WS9 sudo, download and compare complete bytes. A changed download byte fails comparison. Phone checks retain controls and a keyboard-focusable scrolling region. |
| `reference-tools/users/browser_timezone.mjs`, `browser_people.py` | A private derived seed clears only David's detected/explicit zone. Actual Stimulus sends one CSRF-authenticated PATCH, and a separate server-rendered readback proves persistence. A second navigation sends no report. The missing-token negative input removes the CSRF meta tag before the real controller connects, supplying the same absent-token DOM condition as Rails' forgery-disabled system test; it does not disable production CSRF or mask any parity output. No request lands during Rails' two-second settle window. Six browser modes use private seed/storage copies and clean up their servers. |
| `crates/campfire/src/controllers/users/ban_lifecycle_tests.rs`, `controllers/users.rs` | Three full HTTP tests use merged WS9 and the real durable runner. Pending setup secrets/sessions disappear on ban. A SQLite trigger rejects the actual job insert and proves user, sessions, bans and audits all roll back. An observing trigger records exactly one real `RemoveBannedContentJob` insert, its queue and typed user ID despite the live runner consuming it. The real runner deletes all of the subject's messages. Test addresses are public because parity's loopback session would correctly fail Rails' private-IP ban validation. No producer, schema or runner changed. |
| `crates/campfire/src/controllers/public_pages.rs` | Explicit unconfigured sign-in criterion uses WS9's actual page: About, Privacy and Terms links retain `_blank`/`noopener`; no Google button is invented. Configured Google remains WS14g/WS9 integration. |
| `crates/campfire/src/controllers/users/{avatar_image_tests.rs,avatars.rs}`, `reference-tools/users/avatar_images.rb`, `vectors/users_avatar_images.json` | Two actual multipart profile uploads and avatar GETs compare complete WebP/SVG bodies, input fixture SHA256, content type, cache policy and ETags. The uploaded notice changes the first ETag: the next conditional GET is 200, followed by 304 with the stable validator, exactly as Rails. Existing committed logo fixtures supply the same JPEG/BMP bytes. The media runtime is the oracle's libvips 8.16.1. These tests do not close either shared attachment P2. |
| `reference-tools/users/{discrimination.py,file_counts.py,deferred_inventory.py,run_oracles.sh,verify_goldens.py}` | Six new compiled regression probes, four-thread bound, 96 executed owned Rust groups, complete 194-case mapping, 24 raw oracle comparisons and shared source/core/routes/sidebar/WS9 page verification. No comparison mask or allowlist changed. |

Retained rendering/controller files include `controllers/users/{profiles,cards,statuses,time_zones,tours}.rs`, `controllers/accounts{,/*}.rs`, `controllers/{first_runs,welcome,public_pages,pwa,qr_code,workspace_icons}.rs`, their view modules/templates and vectors. Profile owner inputs, approved status fields, direct WS9 panels and the previous exact account/icon/logo/audit/onboarding render comparisons remain exercised by the complete suite and regenerated oracles. The prior WS3/WS8a quote-runner test correction is test-only: it asserts completion of its quote job while allowing independent periodic retention; no ignore, timeout or production job code changed.

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

The last merged main remains `eaba80d5` through `70ce22c7`. Per the lead's latest instruction, main is not merged again. Board-only drift `541c0f69` (#164) and `8952bed4` (#165) belongs to WS12 and does not share this branch's changed partials; no additional overlay was added. Board-nudge drift remains outside this worker's scope. Browser screenshot/pixel diffing is excluded by the updated common rule and is not a remaining item.

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

The configured-Google sign-in criterion needs WS14g/WS9's sign-in view configuration/button integration. First-run tour and icon posting/removal flows require WS8b-m/WS8b-r's room/composer integration. Broader Ruby `Date.parse` partial/relative grammar and Expat versus Nokogiri XML syntax/encoding outside the committed cases remain unproved. Full profile bytes are verified at the complete render boundary with real seed/owner facts; request-time HTML with all still-default owner chrome is not claimed byte-identical. Calendar/OOO mutations, owner token usability and real call/push/status delivery are partial as listed above.

## Astra findings pending the shared attachment fix

The lead relayed Astra's independent review of `a37883d6`: no authorization bypass; 19/19 oracle files byte-identical; Rails read back Rust rows and both logo variants; profile sections present; 5/5 mutations detected. That review is historical evidence for that snapshot, not an independent review of these new tests.

Two P2s remain in shared `controllers/presenters/attachments.rs` on main:

- `ActiveStorage::AnalyzeJob` uses the in-memory queue. Analysis can be lost on exit; the shared fix must enqueue durably and atomically with the attachment write.
- Existing-blob assignment by signed blob ID fails: icon 422 and logo 500 versus Rails 302. Ordinary multipart upload coverage does not prove this assignment path.

Both are pending `rust/durable-attachment-analysis`. No fix is authored here, and no new main merge was performed. After the lead requests integration of that fix, merge main with a merge commit, run locked metadata, and add icon/logo HTTP regressions for signed-ID assignment and analysis durability/rollback. These are additional review findings outside the original 194-case accounting, not criteria counted as covered.

## Fresh-clone verification and exact commands

A new GitHub clone was created under `.scratch/behavior-clone`, initially at `ff2635f5` with an empty target and no copied seeds, source archive, media libraries or scratch fixtures. It pulled the pushed slices through `938fb64a`. Preflight source copies were removed after byte checks; the final run uses clean tracked sources from the remote branch. Both seeds were regenerated again at this final snapshot from tracked tooling. The pin archive and media extraction were regenerated there too. No existing local seed, untracked fixture, pre-existing target fixture or output normalization is required. `CI=1` makes absent seeds fatal. Playwright uses the committed lockfile/Dockerfile and the owned source-verified image; Chrome 153.0.8010.12. Native test threads are four, never above eight. No release profile.

Commands actually rerun for this delivery are below. Setup was invoked from the worktree and then the clone. All verification commands run from `.scratch/behavior-clone`; logs live in the owning worktree's `.scratch/behavior-*-final.log`.

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/behavior-clone
git -C .scratch/behavior-clone pull --ff-only --no-rebase
```

Clone setup (Docker/host tooling runs without the native media `LD_LIBRARY_PATH`):

```sh
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed check-image
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_NAMESPACE=ws8br2-behavior-seed PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
bash rust/reference-tools/users/media_runtime.sh
```

Common native environment:

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/rust/target"
export CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference"
export TMPDIR="$PWD/.scratch/tmp"
export CI=1
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/lib/x86_64-linux-gnu:$PWD/.scratch/rails-media/usr/lib/x86_64-linux-gnu"
```

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
python3 rust/reference-tools/users/discrimination.py ban-setup-cleanup ban-job-enqueue ban-job-perform sign-in-public-link avatar-webp-size avatar-bmp-fallback
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -j 4 -p campfire -p campfire_db -p campfire_kit -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -j 4 -p campfire_assets -p campfire_views --test core --test reference -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -j 4 -- -D warnings
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -j 4 -p campfire
python3 rust/reference-tools/users/file_counts.py ../behavior-rust-tests-final.log
python3 rust/reference-tools/users/deferred_inventory.py
```

Goldens run without the host media overlay; the browser driver removes it from Rails/host forwarding subprocesses while retaining it for Rust:

```sh
python3 rust/reference-tools/users/verify_goldens.py
python3 rust/reference-tools/users/browser_people.py
python3 rust/reference-tools/users/browser_people.py --picker
python3 rust/reference-tools/users/browser_people.py --status
python3 rust/reference-tools/users/browser_people.py --pwa
python3 rust/reference-tools/users/browser_people.py --audit
python3 rust/reference-tools/users/browser_people.py --timezone
git diff --exit-code
git diff --cached --exit-code
```

Locked metadata succeeded and Cargo.lock stayed unchanged. The final clean-tree checks succeeded. The target directory was deleted after confirming no process executed from it; raw logs and generated oracle inputs are retained. No fresh-clone target directory or owned test server remains. No new ignore, comparison exclusion, golden mask or approval exception was introduced.

## Raw verification summaries

### Fresh-clone setup

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-9259468407ef675b49c8961d58752d30a197f666305cd34b0f0295c7ae2e3fd5
seed_key=rust-parity-seed-v1-a617da9ce849701c73417e788781ebd334010a7e99fabe78672c530aa363f3a6
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
WS8br2 pinned media runtime: image ws8br2-reference:d7c7de92; libraries extracted; no host libraries changed
```

### Fresh source/oracle checks

```text
WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes
WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte
WS8br2 status image source verification: 2076 Rails source/fixture/gem files checked; exactly 10 approved 2e20b24c inputs, all others d7c7de92; non-runtime bin/release omitted
WS8br2 worker harness provenance: pinned Rails assertions unchanged; only stdin source loading differs
Rails public oracle: 15 page bodies, 31 policy inputs, 4 QR cases; reference d7c7de92
Rails avatar oracle: 15 initials SVG bodies; reference d7c7de92
Rails avatar images oracle: 2 real uploads, complete WebP and fallback SVG bodies with cache headers; libvips 8.16.1; reference d7c7de92
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
WS8br2 oracle verification: all 24 fresh files match byte for byte; no masks or normalization
WS8br2 shared core verification: all 29 generated files match byte for byte
WS8br2 shared routes verification: complete generated JSON matches byte for byte
WS8br2 shared recognition verification: complete generated JSON matches byte for byte
WS8br2 shared sidebar verification: complete generated JSON matches byte for byte
WS8br2 shared WS9 verification: all 15 complete auth pages match byte for byte
WS8br2 golden verification: sources, both fresh seeds, 24 owned oracles and shared core/routes/sidebar/auth pages passed; no masks or normalization
```

### Both freshly generated seed validators — default, then first_run

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

### App, DB, kit units/integrations and DB/kit doctests

```text
test result: ok. 546 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 105.64s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 59.71s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Asset reference and core views

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
```

### Workspace clippy, -D warnings

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 25s
```

### Normal debug binary

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.30s
```

These suites execute **1,216 active Rust tests**, with **7 pre-existing runtime ignores and 2 ignored doctests**. No seeded case silently skipped; the local-missing-seed message belongs to its intentional guard test. The ignored entries are:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::presenters::accounts::tests::manages_bots ... ignored, WS11: resetting Bender's bot key leaves the original seeded key visible in the account bot list
test jobs::tests::push_latency ... ignored, a measurement, not a test
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

### Six compiled regression probes — expected failures, then restored full suite

```text
avatar-webp-size: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.45s
avatar-bmp-fallback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.47s
sign-in-public-link: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.40s
ban-setup-cleanup: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 1.57s
ban-job-enqueue: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.92s
ban-job-perform: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 10.87s
WS8br2 discrimination: 6 compiled regressions detected; sources restored
```

Each deliberately broken production implementation failed its named test, not compilation. Sources were restored in `finally` and the complete restored suite passed. Preflight mistakes were corrected before delivery: ban fixtures initially retained loopback target sessions (which correctly fail Rails validation), and the public-link test initially borrowed a temporary body. The avatar conditional request initially assumed 304; the Rails oracle instead proved the flash-dependent 200 → stable 304 sequence, which the final tests assert. No such failures are dismissed as inherited.

### people — Rails first, Rust second

```text
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser people: 8 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
```

### picker — Rails first, Rust second

```text
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
WS8br2 browser picker: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF
```

### status — Rails first, Rust second

```text
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
```

### pwa — Rails first, Rust second

```text
served-worker-event-harness: 19 passed
worker discrimination authenticated-cache-write: detected
worker discrimination offline-fallback: detected
worker discrimination notification-focus: detected
WS8br2 browser PWA: 3 passed; 0 failed; 3 worker regressions detected; Chromium 153.0.8010.12; actual served worker and offline shell
served-worker-event-harness: 19 passed
worker discrimination authenticated-cache-write: detected
worker discrimination offline-fallback: detected
worker discrimination notification-focus: detected
WS8br2 browser PWA: 3 passed; 0 failed; 3 worker regressions detected; Chromium 153.0.8010.12; actual served worker and offline shell
```

### audit — Rails first, Rust second

```text
audit CSV: byte-identical to the pinned Rails producer after real password confirmation
audit CSV discrimination: changed response byte detected
WS8br2 browser audit: 2 passed; 0 failed; Chromium 153.0.8010.12; real signed session, filters, sudo and CSV download
audit CSV: byte-identical to the pinned Rails producer after real password confirmation
audit CSV discrimination: changed response byte detected
WS8br2 browser audit: 2 passed; 0 failed; Chromium 153.0.8010.12; real signed session, filters, sudo and CSV download
```

### timezone — Rails first, Rust second

```text
WS8br2 browser timezone: 2 passed; 0 failed; Chromium 153.0.8010.12; missing-token guard, real CSRF PATCH and persisted-zone readback
WS8br2 browser timezone: 2 passed; 0 failed; Chromium 153.0.8010.12; missing-token guard, real CSRF PATCH and persisted-zone readback
```

The browser runs total **25 scenarios per app**, 50 executions. Each PWA run includes the 19 original event assertions and detects three altered worker responses; each audit run rejects a changed CSV byte. The missing-token timezone scenario uses the declared negative DOM input, not altered application code. Actual call ringing, room member-panel integration and push transport are not claimed.

### Actual owned Rust per-file pass counts

```text
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
controllers/accounts/mutation_tests.rs: 11 passed; 0 failed; 0 ignored
WS8br2 file accounting: 96 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred
```

### Cleanup

```text
WS8br2 fresh-clone target cleanup: removed 13591222527 bytes from .scratch/behavior-clone/rust/target; no executing target binaries; no fresh-clone target directories remain
```

## Rails criterion mapping and every remaining name

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 45 | 12 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 12 | 7 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 1 | 1 |
| `test/controllers/users_controller_test.rb` | 226 | 20 | 10 | 10 |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 217 | 15 | 15 | 0 |
| `test/system/starred_people_test.rb` | 182 | 4 | 0 | 4 |
| `test/system/icons_test.rb` | 169 | 4 | 0 | 4 |
| `test/controllers/accounts/icons_controller_test.rb` | 103 | 6 | 6 | 0 |
| `test/system/service_worker_test.rb` | 102 | 2 | 2 | 0 |
| `test/controllers/users/bans_controller_test.rb` | 99 | 8 | 8 | 0 |
| `test/controllers/users/profiles_two_factor_test.rb` | 96 | 7 | 7 | 0 |
| `test/system/first_run_tour_test.rb` | 89 | 4 | 0 | 4 |
| `test/controllers/users/cards_controller_test.rb` | 87 | 7 | 7 | 0 |
| `test/controllers/workspace_icons_controller_test.rb` | 78 | 7 | 7 | 0 |
| `test/controllers/pwa_controller_test.rb` | 66 | 1 | 1 | 0 |
| `test/system/timezone_detection_test.rb` | 64 | 2 | 2 | 0 |
| `test/controllers/accounts_controller_test.rb` | 61 | 4 | 4 | 0 |
| `test/system/workspace_icons_test.rb` | 60 | 1 | 0 | 1 |
| `test/controllers/first_runs_controller_test.rb` | 57 | 4 | 4 | 0 |
| `test/controllers/accounts/logos_controller_test.rb` | 56 | 6 | 6 | 0 |
| `test/system/audit_log_test.rb` | 49 | 2 | 2 | 0 |
| `test/controllers/accounts/users_controller_test.rb` | 36 | 3 | 2 | 1 |
| `test/controllers/users/avatars_controller_test.rb` | 34 | 2 | 2 | 0 |
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

`test/system/first_run_tour_test.rb` — **WS8br2 with WS8b-m composer/room integration**:
- a new member is walked through the tour by keyboard and finishing persists
- escape skips the tour and it never auto-starts again
- the tour restarts from the help menu
- members who completed the tour never see it auto-start

`test/system/workspace_icons_test.rb` — **WS8br2**:
- upload post in both themes then delete falls back to the shortcode

`test/controllers/accounts/users_controller_test.rb` — **WS8br2; WS9 sudo/security metadata seams**:
- destroy

Deferred inventory: 44 named Rails controller/system cases remain from the original 194; 150 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.

## Precisely what remains

Close the 44 named criteria and flagged owner seams above: profile chrome/live windows (12), agent/user pages (10), room/member-panel people flows (7), starred people (4), icon room/name/autocomplete flows (4), first-run room/composer tour flows (4), configured Google sign-in (1), icon upload/post/delete system integration (1), and agent-owner deactivation (1). Keep the two shared attachment P2s pending the separately owned fix; only integrate it when the lead requests the main merge. Full live-owner profile composition, actual push/call/status delivery and the broader parser boundaries remain partial. No production readiness or complete original Ruby-system run is claimed.

# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Verified implementation: `16f8404548e59c4dc5318d1763cb46c82ef6f75c`. The final documentation-only commit carries this report. All implementation slices are pushed. No PR, deployment, release build, stash or new main merge.

This continuation closes **13** criteria, **44 → 31** deferred. Across the original 194-case inventory, **163** now have equivalent coverage and **31** remain. These are criterion mappings, not claims that the Ruby controller/system files ran. Every remaining name and its owner appears below, largest files first.

## Pushed slices this continuation

| Commit | Coherent slice |
|---|---|
| `1013eac1` | Populate the real profile/application layout from persisted Rails settings and caches. Twelve profile criteria closed; 34 Rails configurations cover sound windows, Drive scopes, theme and zone. Real brand names and recent searches replace injected profile-golden inputs. |
| `4ee879b8` | Render the effective Calendar/manual OOO return date in the live profile. Twelve persisted/boundary/zone states compare complete Rails status-panel bytes. The manual-only profile-display seam is closed for the committed producer cache format; no extra original criterion is counted. |
| `3cbde07e` | Render configured Google sign-in beside public links through WS9's existing page. Eight complete pinned bodies/configurations. Reuse WS1's allowed-domain parser. One public-page criterion closed; OAuth start/callback remains WS14g. |
| `7a184599` | Full-workspace verification caught the stale 469-route assertion. The independently regenerated approved #163 oracle has 470. Check the added status-edit route's method, path, endpoint, default and action explicitly; leave all route goldens unchanged. |
| `16f84045` | Extend the tracked private media runtime to extract matching FFmpeg/ffprobe and all selected sonames. Host FFprobe conflicted with the pinned libvips overlay. All unchanged storage vectors pass with image **and video** bytes compared. No attachment code or host libraries changed. |

Prior delivered work is retained: users/cards/preferences; account mutations/views; WS9 audit deduplication and nine exactly-once checks; icon/logo multipart uploads; audit HTML/CSV; welcome/first run; public/PWA/QR/signup; complete profile with direct WS9 security/session panels; #163 popup/assets; directory/sidebar/picker behavior; real service-worker event assertions, CSV/sudo, timezone detection, durable ban lifecycle and WebP/BMP avatar checks. The last delivery was `8755c100` (implementation `938fb64a`); this report replaces its verification with a new clean-clone run.

WS9's `account_security.rs` and `authentication.rs` remain the audit producers from merged main. No duplicate producer was restored. Nine individual mutations still assert exactly one row with Rails' actor, subject, action and metadata. New production work is read-only display projection and the pinned sign-in partial: no schema, persistence callback, job, token-decryption or provider client is added.

## Changes by file and design

Paths are relative to `rust/`.

| Files | Change and evidence |
|---|---|
| `crates/campfire/src/controllers/presenters/{layout_preferences,view_context}.rs` | Actual `Layout::load` reads persisted DND/presence, quiet-hour bounds, manual OOO and cached meeting/OOO intervals. Preserve future intervals, order, duplicates and reversed pairs: Rails lets the browser decide whether a window is currently active. Meeting windows require both status and quiet opt-ins; OOO windows require the Calendar opt-in and no keep-notifications flag. Manual OOO produces `(0,end)` only while active. Drive metadata checks the exact ASCII-separated `drive.file` scope, including disconnected accounts, without requiring credentials or decrypting a token. Cache parsing retains fractional timestamps for effective OOO boundaries. |
| `crates/campfire/src/rich_text.rs`, `controllers/presenters/view_context.rs`, `controllers/users/profile_page_tests.rs` | Layout chrome loads vendored brand canonical names/aliases in Rails YAML order, workspace icon names sorted by name and the viewer's ten ordered recent searches. The existing complete-profile golden now uses the real preferences/chrome adapters from the seed; expected HTML does not supply those facts. Only the declared renderer-boundary CSRF/nonce loan remains. |
| `controllers/users/layout_preferences_tests.rs`, `reference-tools/users/layout_preferences.rb`, `layout-preferences-source-hashes.json`, `vectors/users_layout_preferences.json` | Twelve HTTP test groups cover 34 Rails settings/cache/configurations, including missing caches, disabled opt-ins, future intervals, malformed pairs, exact boundaries, equal quiet-hour bounds and exact/retired/case-sensitive/ASCII/Unicode Drive scopes. Compare the complete sound, Drive and time-zone meta tags verbatim through the signed-in HTTP profile path. No token or nonce masking of HTTP bodies. |
| `controllers/presenters/profile_sections.rs`, `controllers/users/profile_effective_ooo_tests.rs`, `reference-tools/users/profile_effective_ooo.rb`, `vectors/users_profile_effective_ooo.json` | Profile display takes the latest currently covering Calendar OOO end and active manual end, then formats the later date in the member's zone. Inclusive start, exclusive end and fractional seconds match Rails. Twelve tests load actual SQLite facts and the actual presenter, compare complete approved status fragments, then verify actual profile GET visibility. The Calendar producer, opt-in saves, refresh, claims and broadcasts remain owner work. |
| `crates/views/src/sessions.rs`, `views/templates/sessions/{new.html,_google_sign_in.html}`, `crates/campfire/src/{config.rs,controllers/sessions.rs}` | Exact pinned Google SVG/button/domain notice in the existing WS9 sign-in page. Configuration is a flagged read-only WS14g display input, reusing WS1's parser. The button posts to `/session/google` with the real form token. This does not implement OAuth or infer authentication from display configuration. |
| `controllers/public_pages/sign_in_google_tests.rs`, `controllers/public_pages.rs`, `app/full_page_tests.rs`, `reference-tools/users/sign_in_google.rb`, `sign-in-google-source-hashes.json`, `vectors/users_sign_in_google.json` | Eight exact complete content bodies/configurations and live HTTP public-link/button checks. Empty configuration preserves the 15 existing WS9 complete auth-page goldens. Credential gating, public-link target/rel, route, domain formatting and CSRF field are covered. |
| `reference-tools/users/media_runtime.sh` | Reproducibly extract pinned FFmpeg/ffprobe and the libvips/tool dependency closure; expose a private soname directory plus tool PATH. Covers the real full-workspace image/video pipeline without host library changes or skipped byte checks. |
| `crates/routes/src/tests.rs` | Correct the stale pin-count assertion against the independently regenerated approved #163 Rails route oracle and explicitly require all status-edit route fields. Test-only; no golden or production route change. |
| `reference-tools/users/{discrimination.py,file_counts.py,deferred_inventory.py,run_oracles.sh,verify_goldens.py}` | Ten new compiled regression probes, four-thread bound and no explicit extra build jobs; 121 executed owned Rust groups; complete original 194-case mapping; all 27 raw owned oracles plus shared source/core/routes/sidebar/WS9 comparisons. No masks, parity allowlists, ignores or expectations were loosened. |

The read-only adapter is a temporary owner integration boundary, not a replacement notification/Calendar domain. It performs no refresh, decryption, write, claim or broadcast. The live complete profile is now rendered using real database-derived sound/Drive/icon/search facts. Full-page byte identity is still proved at the complete renderer boundary with declared token/nonce values; individual actual HTTP sound/Drive/zone tags and OOO visibility use real request tokens. Still-default owner Picker/huddle configuration prevents a claim of byte identity for every possible live owner configuration.

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

`ws8br2-reference:d7c7de92-status-2e20b24c` derives from the owned pin image, copies exactly those sources and recompiles its assets. Fresh-clone verification checks 2076 Rails source/fixture/gem/runtime-bin inputs: precisely ten approved changes, all others pinned. Non-runtime `bin/release` is absent from the runtime image. Render-only comparisons lend `GLOBAL`, `method:path` and `NONCE` at both renderer boundaries. Real HTTP/browser checks retain actual authentication and CSRF. No authored Rails app changes or broader post-pin overlay. The unrelated board-nudge Rails drift is not imported into this oracle. The upstream controller re-diff was rerun: only `users/statuses_controller.rb` changed across the owned controller paths from the pin to `2e20b24c`. It is handled with the approved overlay.

The last merged main remains `eaba80d5` through `70ce22c7`. Per the lead's latest instruction, main is not merged again. Board-only drift `541c0f69` (#164) and `8952bed4` (#165) belongs to WS12 and does not share this branch's changed partials; no additional overlay was added. Board-nudge drift remains outside this worker's scope. Browser screenshot/pixel diffing is excluded by the updated common rule and is not a remaining item.

## Flagged owner seams and limits

| Owner | Exact seam and closure needed |
|---|---|
| WS9 | Security/profile sessions, sudo and audits use direct merged code. The configured sign-in partial now renders through its page; its existing unconfigured complete auth-page goldens remain exact. No fallback security or duplicate audit path added. |
| WS15g | `ProfileShow::github_connection` flags `campfire_views::github::connections::profile(&Connection)` from `rust/ws15g-github` / PR #167. Map `linked/usable/login/reason/app_token/app_configured` from the owner's domain. Replace reason-based projection with token usability/decryption and disconnect side effects at integration. |
| WS17 push | `_notifications.html` flags push settings, effective notification policy and its update controller. Delivery, subscription/test actions and actual call/push transport remain owner work. |
| WS17 status / WS14g Calendar | `controllers/users/statuses.rs` still returns 501 with `x-campfire-unported: users/statuses#WS17-calendar` for Calendar/OOO keys. Replace `StatusForm` with owner `UserStatusSettings/save_status`, then call public `after_save` / `render_invalid` for #163 frame/page behavior (frame 303, page 302, invalid 422). Opt-in refresh, clearing caches, claims and broadcasts are not implemented here. |
| WS17 / WS14g effective facts | Manual-only **profile return-date display** is closed for committed ISO cache inputs, including exact/fractional boundaries and the latest covering end. Replace the raw cache adapter with the typed effective-status reader. Broader `Time.zone.parse` grammar for non-producer cache values, other card/member-panel effective presence, boundary jobs and `Partial::UserStatus` after-commit broadcast/delivery remain unproved. Local browser `user-status:changed` does not prove cable delivery. |
| WS14g | Profile Calendar display, exact Drive metadata and configured sign-in display are integrated. Replace Google/Calendar cache/configuration projection with owner APIs. `/session/google` start/callback, grants, unusable-token handling, Drive Picker and live refresh remain owner work. The display configuration does not authenticate anyone. |
| WS11 | Inbox preferences/agent approval/work display inputs need domain API integration. Agent profile/capabilities/activity/privacy, revocation and agent-owner deactivation remain deferred. No agent producer duplicated. |
| WS13 | Call mode/key/error inputs render; bind actual call/huddle behavior and global configuration. Directory huddle eligibility is checked; ringing is not. |
| WS15e | Fizzy reason-based connection projection still needs usable-token/decryption/mutation and Slack import-link integration through the merged APIs. No provider client duplicated. |
| WS6 / WS8b-r / WS14g / WS17 | **Closed:** brand/workspace icon names, recent searches, Drive, meeting/OOO sound windows, DND and quiet hours now load in real `Layout::load`. **Still flagged:** `Chrome::google_picker`, huddle configuration and the searches-controller query input. WS17 owns sound-policy writes and delivery. |
| WS8b-m / WS8b-r / WS13 / WS17 | Room `show.shell.message_list` is not populated by this branch and `/rooms/:id/members.json` remains 501. Message-author/member-panel keyboard/focus/group/huddle/timeline scenarios remain deferred. Standalone directory/sidebar coverage does not close these room flows. |
| WS12 | Star mutations and room/directory/card integration remain deferred. Board-only Rails drift is not this worker's scope. |

Icon posting/removal and first-run tour flows require the room/composer integration. Broader Ruby `Date.parse` partial/relative grammar and Expat versus Nokogiri XML syntax/encoding outside committed cases remain unproved. These are partial integration boundaries, not additional cases counted as covered. No open policy decision requires a user answer; owners must supply the flagged APIs and the lead will authorize the main merge.

## Astra findings pending PR #171

The lead's historical independent review of `a37883d6` found no authorization bypass, 19/19 byte-identical oracle files, Rails readback of Rust rows and both logo variants, profile sections present and 5/5 mutations detected. It is not a new independent review of this continuation.

Two P2s remain in shared `controllers/presenters/attachments.rs` on main:

- `ActiveStorage::AnalyzeJob` uses the in-memory queue, so analysis can be lost on exit. The shared fix must enqueue durably and atomically with the attachment write.
- Existing signed-blob-ID assignment fails: icon 422 and logo 500 versus Rails 302. Ordinary multipart upload coverage does not prove that assignment path.

Both are pending PR #171 (`rust/durable-attachment-analysis`, including Rails-sanitized filename identification/copy behavior). They were deliberately left untouched. Main has not been merged again. When the lead requests the merge after #171 lands, merge main with a merge commit, run locked metadata and add icon/logo signed-ID and analysis durability/rollback regressions. These findings are outside the original 194-case accounting.

## Fresh-clone verification and commands

A new clone `.scratch/layout-clone` came from GitHub at `8755c100` with an empty target and no copied seed, archive, media library or scratch fixture. It fast-forwarded the pushed slices through `16f8404548e59c4dc5318d1763cb46c82ef6f75c`. Preflight source copies were checked byte-for-byte and removed before the final pull; final tracked sources are clean and remote-backed. Both seeds and the pinned source archive were regenerated for the final rendering inputs; the owned media extraction was rerun at the final implementation. The 27 oracles were regenerated at `7a184599`; the only later source change, `16f84045`, is the native media extraction helper and changes no oracle/render input. The only untracked source-tree entry is generated `.scratch/` tooling output. `CI=1` makes missing seeds fatal. Tests use committed inputs or generated private temporary state, not untracked local fixtures.

Playwright uses the committed lockfile/Dockerfile and owned source-verified image; Chromium 153.0.8010.12. Test threads are four (never above eight), `CARGO_BUILD_JOBS=2`, with no `-j`, throttle bypass or release build. No screenshot or pixel-diff work.

Setup/verification was rerun with these commands. Native checks and browsers run from `.scratch/layout-clone`; logs are the owning worktree's `.scratch/profile-*-final.log`. Docker/host tools run without the native `LD_LIBRARY_PATH`.

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/layout-clone
git -C .scratch/layout-clone pull --ff-only --no-rebase
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed check-image
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_NAMESPACE=ws8br2-layout-clone-seed PARITY_OWNER=ws8br2 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
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

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
python3 rust/reference-tools/users/discrimination.py sign-in-display-route sign-in-display-token sign-in-display-credentials ooo-start-boundary ooo-end-boundary ooo-overlap-end layout-future-window layout-meeting-gate layout-ooo-keep layout-drive-scope
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_routes -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_mail --test smtp -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_storage --test vectors -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -p campfire
python3 rust/reference-tools/users/file_counts.py ../profile-rust-tests-final.log
python3 rust/reference-tools/users/deferred_inventory.py
```

Golden verification runs without host media overlays. The browser driver removes those overlays from Rails and host forwarding subprocesses while keeping them for Rust:

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

The owned upstream controller re-diff used `git diff --name-only d7c7de9264c63015be398001d7a1094e7695a6db 2e20b24c3f2be9db8a646a1352c159b4afacad0e --` followed by `app/controllers/accounts_controller.rb`, `app/controllers/accounts`, `app/controllers/users_controller.rb`, `app/controllers/users`, `app/controllers/first_runs_controller.rb`, `app/controllers/welcome_controller.rb`, `app/controllers/public_pages_controller.rb`, `app/controllers/pwa_controller.rb`, `app/controllers/qr_code_controller.rb` and `app/controllers/workspace_icons_controller.rb`. It asserted the sole changed path was the approved statuses controller.

Locked metadata succeeded; Cargo.lock stayed unchanged. Final tracked-tree checks passed. No new test ignore, comparison exclusion, golden mask or approval exception. The target is deleted only after all verification and after checking that no process executes from its exact owned path. Logs and generated oracle inputs remain for review.

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

### Upstream controller re-diff

```text
WS8br2 upstream controller re-diff: only users/statuses_controller.rb changed from d7c7de92 to approved 2e20b24c; all other owned controller paths unchanged
```

### Regenerated Rails sources, seeds and complete byte comparisons

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
WS8br2 oracle verification: all 27 fresh files match byte for byte; no masks or normalization
WS8br2 shared core verification: all 29 generated files match byte for byte
WS8br2 shared routes verification: complete generated JSON matches byte for byte
WS8br2 shared recognition verification: complete generated JSON matches byte for byte
WS8br2 shared sidebar verification: complete generated JSON matches byte for byte
WS8br2 shared WS9 verification: all 15 complete auth pages match byte for byte
WS8br2 golden verification: sources, both fresh seeds, 27 owned oracles and shared core/routes/sidebar/auth pages passed; no masks or normalization
```

### Fresh seed validators — default, then first_run

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

### Tests shown failing before the fixes (this continuation)

```text
layout-red.log: test result: FAILED. 4 passed; 8 failed; 0 ignored; 0 measured; 549 filtered out; finished in 2.31s
ooo-red.log: test result: FAILED. 4 passed; 8 failed; 0 ignored; 0 measured; 561 filtered out; finished in 1.13s
sign-in-red.log: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.85s
```

These failures were exercised before the implementation: missing persisted layout metadata, manual-only effective OOO display and the missing configured sign-in partial. No golden was changed to make Rust pass. The first layout assertion omitted Rails escaping of a legacy zone name; it now compares the exact oracle meta tag. The first sign-in implementation added two blank lines; the Rust template was fixed, leaving Rails expectations unchanged.

### Ten compiled regression probes, then restored full suite

```text
sign-in-display-route: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 1.55s
sign-in-display-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 1.33s
sign-in-display-credentials: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.93s
ooo-start-boundary: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.38s
ooo-end-boundary: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.29s
ooo-overlap-end: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.43s
layout-future-window: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 1.10s
layout-meeting-gate: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 0.81s
layout-ooo-keep: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 1.01s
layout-drive-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 573 filtered out; finished in 1.16s
WS8br2 discrimination: 10 compiled regressions detected; sources restored
```

All ten mutations failed their named test after successful compilation. They cover route, form token, credential gate, OOO start/end/maximum end, future meeting windows, meeting opt-in gate, keep-notifications and Drive scope. The mutation runner restored sources in `finally`; final diff checks were clean before the full suite. Prior probes remain available but are not claimed rerun in this continuation.

### First broader workspace run — stale pinned route-count assertion

```text
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The first full-workspace run caught a previously retained `TABLE.len() == 469` assertion. The fresh approved #163 Rails route dump has 470 rows and is byte-identical to the committed routes JSON, including GET `/users/:user_id/status/edit(.:format)`, `users/statuses#edit`, default `user_id=me` and a defined action. The test was corrected to that independently regenerated count and now checks every one of those new-route fields explicitly. No Rails expectation/vector bytes or production route behavior changed. This test-only correction was pushed as its own slice before the restored complete run.

### Corrected approved-route check

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Unisolated SMTP port collision, then isolated unchanged tests

```text
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
```

The next full-workspace run received unrelated HTTP `POST /vscode/notify` bytes at the SMTP test peer instead of `EHLO localhost.localdomain`. The existing mail helper defaults to shared 40000–40049. Final verification sets `MAIL_TEST_PORT_RANGE=52640-52669` and `CABLE_TEST_PORT_RANGE=52670-52699`, away from the later 52610–52612 browser servers. The same five SMTP tests pass without source/assertion changes or retries hidden from the report. The complete workspace is then rerun with that isolation.

### Media setup conflict, then unchanged exact storage vectors

```text
ffprobe: symbol lookup error: /usr/lib/libass.so.9: undefined symbol: FcConfigSetDefaultSubstitute
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.70s
ffprobe: error while loading shared libraries: libpulsecommon-17.0.so: cannot open shared object file: No such file or directory
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.13s
byte-identical: ["moon.jpg", "moon-thumb", "earth.png", "earth-thumb", "black_hole.jpg", "black_hole-thumb", "alpha-centuri.mov", "alpha-centuri.mov preview_image", "alpha-centuri-preview-webp", "alpha-centuri-poster", "alpha-centuri-poster-from-key", "pixel.bmp", "earth.png", "earth-avatar", "moon.jpg", "moon-avatar", "black_hole.jpg", "black_hole-logo-large", "black_hole-logo-small"]
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.18s
```

### Final private pinned media extraction

```text
WS8br2 pinned media runtime: image ws8br2-reference:d7c7de92; libvips, FFmpeg tools and libraries extracted; no host libraries changed
```

The broader run exposed an incomplete private runtime: host `ffprobe` could not load the pinned fontconfig ABI; simply copying the pinned tools also revealed a Debian absolute PulseAudio RUNPATH. The tracked setup now extracts matching FFmpeg/ffprobe and their dependency closure, keeps selected sonames in one private loader directory and prints both PATH/LD_LIBRARY_PATH. All eight unchanged storage vectors pass and the exact video bytes are compared, rather than skipped behind the inherited version gate. No storage/attachment implementation or expected vectors changed.

### Complete workspace native suite (each raw summary, in log order)

```text
     Running unittests src/main.rs (rust/target/debug/deps/campfire-d4df0af23b1f90e2)
test result: ok. 571 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 138.65s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_assets-de6b6b844a46fb2e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/reference.rs (rust/target/debug/deps/reference-52334e28be9d328d)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_cable-099573072d53381c)
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/disconnect.rs (rust/target/debug/deps/disconnect-42cf31142ee556a4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.37s
     Running tests/golden.rs (rust/target/debug/deps/golden-a057e4bb65195f7c)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
     Running tests/protocol.rs (rust/target/debug/deps/protocol-1c38858ec89913df)
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_db-215201eeb95ea1a9)
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 80.13s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_jobs-2b4b600e1c1b7ef2)
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.57s
     Running tests/crash.rs (rust/target/debug/deps/crash-a0e513b65522039f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_kit-c778c94af2123e34)
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
     Running tests/front.rs (rust/target/debug/deps/front-58bf655879b34757)
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
     Running tests/http.rs (rust/target/debug/deps/http-e9043d1dc9499a1b)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/params_vectors.rs (rust/target/debug/deps/params_vectors-85d6c6bc32e6f7f4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/rails_vectors.rs (rust/target/debug/deps/rails_vectors-c66f9244e0fc195e)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_mail-7ce4648430c3074d)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
     Running tests/config.rs (rust/target/debug/deps/config-76fc90a75b18de48)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/goldens.rs (rust/target/debug/deps/goldens-0e5b54d85cab421b)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
     Running tests/inbound.rs (rust/target/debug/deps/inbound-9b7df7acddd50156)
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.23s
     Running tests/security.rs (rust/target/debug/deps/security-d6257643e5e860ea)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/smtp.rs (rust/target/debug/deps/smtp-3b7765b569fd2d4b)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_richtext-4a8389a15be07fd2)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
     Running tests/corpus.rs (rust/target/debug/deps/corpus-8a400ffa0cacd0c6)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.27s
     Running tests/fork_regressions.rs (rust/target/debug/deps/fork_regressions-47666ab224448bf8)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/hardening.rs (rust/target/debug/deps/hardening-ad131644ca4fd322)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
     Running tests/markdown_corpus.rs (rust/target/debug/deps/markdown_corpus-cf0d596a4179c82f)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
     Running tests/markdown_security.rs (rust/target/debug/deps/markdown_security-bdf3e1cdacba4fed)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/reference_tests.rs (rust/target/debug/deps/reference_tests-6fcdb456853a77f0)
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/sgid_json_corpus.rs (rust/target/debug/deps/sgid_json_corpus-a2a8310065446f52)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.44s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_routes-6b53a5291b6f9a17)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_storage-81b9778f444bb28c)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/vectors.rs (rust/target/debug/deps/vectors-7de607f29cb543fe)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.06s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_views-7023f6bdfcce7286)
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
     Running tests/core.rs (rust/target/debug/deps/core-fcd3834ff7bf58bc)
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/room_header.rs (rust/target/debug/deps/room_header-961f6b3f12885301)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/room_shell.rs (rust/target/debug/deps/room_shell-eb53fb695fb4cf11)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/sidebar.rs (rust/target/debug/deps/sidebar-bb6eb9fdcde048ad)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (rust/target/debug/deps/rails_compat-028047b1fc65456b)
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
   Doc-tests campfire_assets
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_cable
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_db
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_jobs
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_kit
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_mail
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_richtext
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_routes
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_storage
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_views
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests rails_compat
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The Rust harness reports **1,662 passes**, **zero failures** and **11 pre-existing ignores** (nine runtime, two doctests). One reported pass is the inherited ACME gate below, which returned without testing because no Pebble CA was configured; **1,661 test bodies executed**. The raw summaries retain every count. No seeded case silently skipped: the intentional missing-seed guard test exercises the local-skip branch.

### Inherited ACME conditional skip

```text
skipped: PEBBLE_MINICA isn't set
test acme_tls_alpn_certificate_cached_and_reused ... ok
```

### Existing ignored tests

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::presenters::accounts::tests::manages_bots ... ignored, WS11: resetting Bender's bot key leaves the original seeded key visible in the account bot list
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

### Full-workspace storage byte comparisons

```text
byte-identical: ["moon.jpg", "moon-thumb", "earth.png", "earth-thumb", "black_hole.jpg", "black_hole-thumb", "alpha-centuri.mov", "alpha-centuri.mov preview_image", "alpha-centuri-preview-webp", "alpha-centuri-poster", "alpha-centuri-poster-from-key", "pixel.bmp", "earth.png", "earth-avatar", "moon.jpg", "moon-avatar", "black_hole.jpg", "black_hole-logo-large", "black_hole-logo-small"]
```

### Workspace clippy, -D warnings

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 42s
```

### Normal debug binary

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 11s
```

### Initial status-browser failure retained

```text
sidebar-popup-save-closes-and-persists: passed
sidebar-popup-clear-closes-and-persists: passed
sidebar-popup-cancel-does-not-save: passed
sidebar-popup-invalid-save-stays-open: passed
sidebar-popup-phone-width: passed
WS8br2 browser status: 5 passed; 0 failed; Chromium 153.0.8010.12; real signed session and CSRF; member-panel integration deferred
sidebar-popup-save-closes-and-persists: passed
page.waitForFunction: Timeout 12000ms exceeded.
```

The first status browser attempt passed all five Rails cases, then Rust passed save/persistence but timed out after 12 seconds waiting for Stimulus in the second scenario. Server asset requests were 200. Diagnostic console/request logging subsequently passed all five scenarios on each app; the plain unchanged harness was restored and rerun. No timeout, retry logic, assertion or application code was changed. The timeout's cause was not established and its raw log is retained as an unresolved intermittent verification failure, not dismissed as inherited. Diagnostic logs also show the already flagged 501 presence/activity endpoints and room-list integration; local status events still do not prove broadcast delivery.

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

Browser behavior checks execute **25 scenarios per app, 50 total** using real signed sessions/CSRF. Each PWA run includes the 19 original event assertions and rejects three altered worker responses; each CSV run rejects one changed response byte. The missing-token timezone case supplies the declared negative DOM input, without changing app code. No pixel comparison, actual call ringing, room member-panel or push transport is claimed.

### Actual owned Rust per-file pass counts

```text
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
controllers/accounts/mutation_tests.rs: 11 passed; 0 failed; 0 ignored
WS8br2 file accounting: 121 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 9 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred
```

### Cleanup

```text
WS8br2 fresh-clone target cleanup: removed 14044118687 bytes from .scratch/layout-clone/rust/target; no executing target binaries; no fresh-clone target directories remain
```

## Rails criterion mapping and every remaining name

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 57 | 0 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 12 | 7 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 2 | 0 |
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

`test/system/people_group_dms_test.rb` — **WS8br2 people/cards; WS8br DM actions**:
- clicking a message author opens their profile card and Message lands in the DM
- the profile card opens by keyboard, traps focus, and returns it on Esc
- the member panel multi-select starts a huddle with exactly that set
- group members rename, add, and leave with system notes in the timeline
- starting a group huddle rings every other member
- Esc closes only the profile card inside the mobile member panel
- Tab cycles within the profile card opened from the member panel

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

`test/controllers/accounts/users_controller_test.rb` — **WS8br2; WS11 agent-owner deactivation, direct WS9 sudo/audits**:
- destroy

Deferred inventory: 31 named Rails controller/system cases remain from the original 194; 163 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.

## Precisely what remains

**31 original criteria:** agent/user profile pages (10), room/member-panel people flows (7), starred people (4), icon room/name/autocomplete flows (4), first-run room/composer tour flows (4), icon upload/post/delete system integration (1), agent-owner deactivation (1). The profile controller and public sign-in inventory are fully mapped to equivalent coverage; broader owner integration and parser boundaries remain partial as listed above.

The flagged owner seams, the two shared attachment P2s and the unreproduced initial Stimulus startup timeout are additional integration/review work outside that numerical inventory. Leave PR #171's attachment work to its owner and wait for the lead's requested main merge. No production readiness or complete original Ruby system-test run is claimed.

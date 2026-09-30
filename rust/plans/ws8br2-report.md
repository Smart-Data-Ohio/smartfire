# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Verified implementation: `a45b4a27f027f2229febce6a8b2753a11aa379fd`. The final documentation-only commit carries this report. All implementation slices are pushed; no PR, deployment, release build or cutover.

Rails pin: `d7c7de9264c63015be398001d7a1094e7695a6db`. The shared brief now explicitly requires the post-pin `2e20b24c` status templates: the profile oracle uses exact, hash-checked copies of its `_status` and `users/statuses/_fields` templates as read-only renderer inputs. Every other profile template and the application layout/assets remain at the pin. No Rails application files were edited. This proves the complete page for those stated inputs; it does not claim that the rest of the post-pin status-popup feature or live owner projections are complete.

## Pushed slices this continuation

- `ced275d6`: icons, private serving and media validation; logo upload/replacement/removal and exact PNG/cache/ETag responses; audit-log HTML/navigation/filtering/pagination/CSV. New media files and oracles are committed. Kept WS9's audit producers direct and unchanged.
- `950a6d6a`: first-run persistence, optional credentials, repeat/race protection, setup wording and exact first-run/welcome bodies. Rails saves the account before the room/user transaction: missing name returns 500 and leaves one account; empty name and absent/empty password are accepted. Seven real Rails HTTP/state cases plus five concurrent Rust requests verify these distinctions. Existing PWA/QR byte checks remain in the full suite and regenerated oracles.
- `e8a45a77`, `3f27759f`: complete seeded profile renderer, direct WS9 security sections, flagged projections for other owners, error previews, Edge-only installation instructions and expired/live DND timers. Corrected the shared room-name presenter to call the already-ported direct-room domain method: ordered first-name previews with comma separators, custom names and the viewer fallback.
- `a45b4a27`: complete signup body and join-code access/state checks, duplicate-email redirect, ignored untrusted role, all open-room grants, new session and WS9 enrollment redirect. Fixed two missing leading newlines.

Earlier received slices remain, including WS9/main `4278cb1e` merged by `279b9652`, deletion of the overlapping `73ebe8e1` audit producers, nine per-mutation exactly-one checks, core profile preferences, people/cards and account views. `authentication.rs` and `account_security.rs` are unchanged from reviewed main. The new logo checks additionally verify upload, replacement, removal and no-op audit behavior.

## Additional changes by file

Paths below are relative to `rust/`; the earlier file inventory follows.

| Files | Behavior and validation |
|---|---|
| `crates/db/src/models/audit_log/browsing.rs`; `controllers/accounts/audit_logs.rs`; views `src/accounts/audit_logs.rs`, `templates/accounts/audit_logs/show.html` | Bound filters, 50-row pages, 5000-row CSV cap/truncated filename, Rails quoting/formula neutralization/JSON escaping, private headers, WS9 sudo for export. Fourteen complete body/nav/CSV vectors and 15 date inputs, plus real HTTP access/cap checks. |
| `crates/db/src/models/workspace_icon.rs`; `crates/storage/src/workspace_icon.rs`; `controllers/accounts/icons.rs`, `controllers/workspace_icons.rs`; views `src/accounts/icons.rs`, icon templates | Ruby name/title checks, uniqueness race rollback, staged 256 KiB SVG/PNG validation, XML unsafe-element/attribute/URL checks, dimensions, attachment/audit atomicity and purge. Exact SVG/PNG serving bytes, CSP/nosniff/private cache/checksum 304, sign-out/enrollment gates. 39 validations, 19 committed media inputs, three complete body/nav states. |
| `controllers/accounts/logos.rs`, `logos/tests.rs`; `vectors/users_logos.json`, `vectors/users_logos/*`; `reference-tools/users/media_runtime.sh` | Nine exact stock/JPEG/BMP PNG bodies and headers; upload/replacement/purge with one WS9 audit per actual change. Host vips 8.18 differs in PNG chunk ordering: canonical checks use the pinned image's vips 8.16 libraries extracted into owned scratch, without changing the host or weakening bytes. Rust Dockerfile already builds the matching media version and includes expat. |
| `crates/kit/src/ctx.rs` | Flash ETag expansion now matches `ActiveSupport::Cache.expand_cache_key(FlashHash#to_a)` instead of Rust debug formatting. Expose existing freshness check for explicit icon ETags. Logo-with-notice differential caught the bug. Full kit unit and integration suite ran. Cross-stream touch: WS4/WS19. |
| `controllers/rich_text.rs`, `controllers/presenters/attachments.rs`, controller dispatch/exports | Reuse the pinned brand catalogue/aliases for reserved names; `WorkspaceIcon` attachment table/record handling and existing route dispatch. No schema or dependency/lockfile change. |
| `crates/db/src/models/first_run.rs`; `controllers/first_runs.rs`, `first_runs/tests.rs`; views `first_runs.rs`, template | Separate administrator/room creation from the account save in the HTTP service. Preserve existing domain entry point. Optional email/password, exact setup title/body, repeat/race and seven Rails state cases. |
| `controllers/welcome.rs`, `welcome/tests.rs`; views `welcome.rs`, template | First/last visible room redirects, inaccessible last-room fallback, exact empty body/sidebar/frame and workspace-navigation toggle. |
| `controllers/presenters/profile_sections.rs`; `controllers/users/profiles.rs`; views `users/profile_sections.rs`, `users.rs`, profile templates | Typed read-only owner inputs, non-secret error preview, inbox/call/status/notification/provider sections; direct WS9 2FA/device/session rendering. One complete seeded application page matches without output masks; real HTTP tests cover section integration, Edge-only UA, DND expiry and rejected settings rollback. |
| `controllers/presenters/accounts.rs` | Use `Room::direct_display_name` instead of the obsolete sentence-based helper. The complete profile differential exposed `Jason, Kevin, and JZ` versus Rails `Jason, JZ, Kevin`. Shared presenter touch for WS8b-r; existing domain naming tests and the complete app suite pass. |
| `controllers/users/joining_tests.rs`; `templates/users/new.html` | Complete pinned signup body and six HTTP cases, role filtering, duplicate redirect, room/session state. |
| `reference-tools/users/{audit_logs,icons,logos,onboarding,profile_page,joining}.rb`, source ledgers, `post-pin/*`; new `vectors/users_*` and media inputs | Pinned real Rails oracle execution and exact source hashes; approved post-pin templates independently checked against Git. Every fixture needed by new tests is committed; no tests read pre-existing scratch/target fixture files. |
| `reference-tools/users/{run_oracles.sh,discrimination.py,file_counts.py,deferred_inventory.py}` | Nineteen fresh unnormalized oracle comparisons, compiled regression detection/source restoration, actual per-file outcomes and complete deferred-name inventory. |


Paths relative to `rust/`; imported WS9 implementation is identified separately from owned changes.

| Files | Final behavior |
|---|---|
| `crates/campfire/src/public_policy.rs`, `config.rs`, `main.rs`, `controllers/public_pages.rs`; `crates/views/src/public_pages.rs`, `templates/public_pages/{about,privacy,terms}.html` | Installation policy, escaping/URI encoding, signed-out public routes and complete pinned bodies. Public requests expose no workspace/current-user state or session cookie. |
| `crates/db/src/models/user.rs`; `crates/views/src/helpers/users.rs`; `controllers/users/avatars.rs` | Ruby initials and 15 signed HTTP SVG/cache/ETag/304 vectors. Uploaded avatar variants remain partial. |
| `controllers/{qr_code,pwa}.rs` | QR capacity failure is 500; body/cache vectors. Default and first-run PWA manifest, service-worker source and offline body checks. Browser service-worker execution remains deferred. |
| `controllers/users/{time_zones,tours,preferences_tests}.rs`; `models/user.rs` | Current-user auth/CSRF scope, 17 browser-detection cases, explicit-choice protection, repeat completion touch and room tour flags. |
| `crates/db/src/models/user/presentation.rs`; `controllers/presenters/people.rs`; `controllers/users/{cards,people_tests}.rs`, `controllers/users.rs`, `controllers.rs` | Read-only people/card facts; active viewer exclusion/order, stars, human lease presence, idle/invisible/DND/custom-status expiry and agent heartbeat/suspension labels; thin authenticated HTTP rendering. |
| `crates/views/src/users/people.rs`, `templates/users/{index,cards/show,statuses/_badge}.html`, `templates/users/show.html`, `templates/users/_ban_button.html` | Exact directory/cards/status markup, signed stream keys, mentions/star buttons, own-profile/inactive/agent actions and accessible Message labels. |
| `crates/db/src/models/user/profile_settings.rs`; `controllers/users/{profiles,profile_settings_tests}.rs`; `crates/views/src/users/appearance.rs`, `users/profile_time_zones.json`, `templates/users/profiles/_appearance.html`, `templates/users/profiles/show.html`, `crates/views/src/users.rs` | Owned non-security profile settings and error preview/appearance controls. WS9 core updates run first, then preference validation/update in the same writer: any failure rolls back security audits, devices, core fields and attachments. Zones and the explicit marker use WS9's UserChanges directly. |
| `crates/db/src/slash_commands.rs`, `slash_commands/user_settings.rs`, `slash_commands/time_parser.rs`, `rails_time_zones.json`, `slash_commands/rails_{zone_identifiers,named_zones}.json` | Reuse the all-row validator. Runtime zone parsing is now WS9's exact pinned catalogue from main; the owned generated catalogues remain differential inputs. |
| `controllers/accounts.rs`, `accounts/{users,custom_styles,join_codes,logos}.rs`, `controllers/users/bans.rs` | Call WS9's producers once. Thin role/ban validation/status handling matches Rails. Account attachment staging remains in the same writer. |
| **Deleted** `crates/db/src/models/account_mutations.rs`, removed its `models.rs` export | Remove all overlapping audit producers from `73ebe8e1`. |
| **Unchanged from main** `crates/campfire/src/{authentication,account_security}.rs`, `concerns/{two_factor,sudo}.rs`, `controllers/{sudos,two_factor}.rs`, `controllers/users/sessions.rs`, `crates/db/src/models/two_factor.rs` | Reviewed WS9 profile security, account audits, sudo, 2FA, sessions and devices. No duplicated security implementation. |
| `controllers/accounts/{mutation_tests,view_tests}.rs` | 23 complete HTTP/state/audit vectors, nine individual exactly-one cases, authorization/sudo rollback checks, and account view goldens. The separate Rails agent-owner deactivation case is explicitly deferred. |
| `controllers/presenters.rs`; `crates/views/src/users/summary.rs`, `src/accounts.rs`, `templates/accounts/{edit,_invite}.html`, `templates/accounts/users/_user.html`, `templates/accounts/custom_styles/edit.html` | Read-only Google sign-in presentation metadata, account body/navigation/footer blocks and exact role/security/invite/style markup. |
| `controllers/presenters/test_support.rs`, `controllers/users/people_tests.rs` | Small shared test seams for seed/environment/frozen-clock and configurable golden view context. No test reads pre-existing scratch/target fixtures. |
| `reference-tools/users/{public,avatars,pwa,preferences,zones,people,profiles,appearance,accounts,account_views}.rb`; corresponding `*-source-hashes.json` ledgers | Real pinned Rails models, views or HTTP requests, with source drift checks. Render-only fixtures use WS6's fixed token/nonce inputs; real HTTP comparisons use real session and CSRF handling. |
| `vectors/users_{public,avatars,pwa_default,pwa_first_run,preferences,people,profile_settings,appearance,account_mutations,account_views}.json`; `reference-tools/users/{run_oracles.sh,discrimination.py,file_counts.py,deferred_inventory.py}` | Committed vectors, fresh raw byte comparison, compiled regression checks with source restoration, actual Rust per-file counts and exact remaining Rails criterion inventory. |
| `plans/ws8br2-report.md` | Tracked copy of this report. |


## Design limits and integration seams

Domain writes remain outside templates. Upload analysis runs before the writer transaction; record/attachment/audit changes stay atomic and purge follows commit. Icons resolve through the same brand catalogue as rich text. Existing WS9 producers write each audit once. No new ignores, parity masks or allowlists were added.

Each profile seam is explicit in `controllers/presenters/profile_sections.rs` and the view inputs in `users/profile_sections.rs`:

| Owner | Input/seam | Lead closure needed |
|---|---|---|
| WS11 | `inbox` / `inbox_errors`, including `agent_approvals` and `agent_work`; older people/agent projections | Replace read-only stored preference projection with owner facts where needed; complete agent profile/capabilities/activity and lifecycle. Ten bot/agent controller criteria and agent-owner deactivation remain deferred. |
| WS13 | `voice_mode`, `push_to_talk_key`, `call_errors` | Join to owned call/huddle domain and configuration. Existing preference writes/validation are tested; call behavior and global huddle chrome are not claimed. |
| WS14g | `google` | WS9 sign-in configuration and identity rows are used directly. Calendar currently projects the unconfigured state; connected/rejected/partial grant, Drive, meeting/OOO cache/configuration branches still need owner data and templates. Do not merely set `calendar_configured=true`: those branches are unfinished. |
| WS15g | `github`, `github_verified`, `github_app_configured` | Missing/rejected/connected/PAT/App markup is available. Replace the read-only reason-based projection with owned decrypted-token usability, unreadable-token disconnect side effects and real App configuration. No token/client code was duplicated. |
| WS15e / WS15 | `fizzy`; Slack import link | Replace reason-based Fizzy connection projection with owned usability/decryption and mutations; wire Slack import endpoint. Seed's connected Fizzy presentation matches Rails. Other provider states are render inputs, not verified live integrations. |
| WS17 | `status`, `notifications` | Stored status/quiet-hours/keyword/DND-exception inputs and manual DND expiry are rendered. Complete effective manual/calendar OOO date, meeting/calendar toggles/cache errors, notification policies and status/notification mutations. |
| WS6 / WS8b-r / WS14g / WS17 | live `Layout::load` chrome/preferences | Complete request-time brand catalogue, recent searches, Drive and notification-window population. Full-page test supplies real seed facts at the renderer boundary, as WS9's full-page tests do; the live HTTP page is not asserted byte-identical with all these owner defaults. |
| WS8b-r2 / WS17 / WS6 | post-pin `2e20b24c` status popup | Profile status/fields are integrated. Popup edit/update/card/sidebar/layout action/JS/routes and new popup tests remain outside this slice and outside the original 194-count inventory. Layout in the full-page oracle remains pinned. |

Additional partial boundaries: audit date parsing implements the explicitly exercised full-date grammar, not Ruby `Date.parse`'s whole partial/relative grammar. SVG validation uses Expat instead of Nokogiri/libxml2; all 19 committed media cases match, but broader XML syntax/encoding parity has not been proved. Uploaded avatar variants, browser tour/timezone/icon/audit/service-worker execution, agent revocation and live Google composition remain deferred as named below. This is not a claim that the Rails browser/system files ran.

## Upstream re-diff

Re-fetched origin/main and re-diffed the owned account/icon/audit/logo/onboarding/PWA/QR Rails paths at `2e20b24c3f2be9db8a646a1352c159b4afacad0e`. No changes from the pin in those paths. Profile/status drift is handled and bounded above. Both copied post-pin templates separately match their Git objects byte for byte; no modified reference application.

## Fresh-clone verification

Cloned the pushed branch from GitHub into `.scratch/delivery-clone` at `a45b4a27f027f2229febce6a8b2753a11aa379fd`. No target, seed, media runtime, source archive or scratch fixture was copied from the working checkout. Prepared the archive, built both seeds and extracted media independently in the clone. Missing seeds are fatal under `CI=1`. Own target/TMPDIR; ports 52600–52649; no release profile. Metadata completed with the locked lockfile. The fresh clone later pulled documentation-only commits; `git diff a45b4a27 HEAD --name-only` contains only `rust/plans/ws8br2-report.md`. A final image check found a stale `GIT_REVISION=bf3095479a415b9c9131442fae369ac5ee2ebf0a` marker. Before correcting it, checked 2,074 Rails source, fixture, Gemfile/lockfile and runtime-bin files in the owned image against the fresh pinned archive; every file matched byte for byte. Only the non-runtime `bin/release` CLI is absent from the runtime image. Preserved the old tag as `ws8br2-reference:d7c7de92-oldmarker`, then derived a configuration-only image with the correct `GIT_REVISION`. No source or library bytes changed. The image check now passes; both seeds, both validators, all 19 oracles and the complete Rust suite were rerun after this correction.

Commands below ran in the clone, except clone itself from the worktree. Logs live in the parent `.scratch/`. All cited validation commands were rerun this continuation.

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/delivery-clone
bash rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws8br2-reference:d7c7de92 bash rust/parity/bin/ci-seed check-image
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-delivery PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
bash rust/reference-tools/users/media_runtime.sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-delivery PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-delivery PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" bash rust/reference-tools/users/run_oracles.sh
```

The image-source check ran inside the owned image with the fresh pinned archive mounted read-only:

```sh
docker run --rm --network none -i --label parity.owner=ws8br2 --label parity.namespace=ws8br2-source --entrypoint ruby -v "$PWD/rust/parity/.ci/reference:/pin:ro" ws8br2-reference:d7c7de92 - <<'RUBY'
require 'digest'
paths = Dir.chdir('/pin') { Dir.glob('{app,config,db,lib,public,vendor,test,bin}/**/*', File::FNM_DOTMATCH).select { |path| File.file?(path) } }
paths += %w[Gemfile Gemfile.lock .ruby-version]
paths -= ['bin/release']
paths.each do |path|
  actual = File.join('/rails', path)
  raise "missing image file: #{path}" unless File.file?(actual)
  raise "source mismatch: #{path}" unless Digest::SHA256.file(actual).hexdigest == Digest::SHA256.file(File.join('/pin', path)).hexdigest
end
puts "WS8br2 image source verification: #{paths.length} Rails source/fixture/gem files match d7c7de9264c63015be398001d7a1094e7695a6db byte for byte; non-runtime bin/release omitted from image"
RUBY
docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' ws8br2-reference:d7c7de92 | rg '^GIT_REVISION='
```

Raw image-source, archive/seed/media and seed validator summaries (default, then first_run):

```text
WS8br2 image source verification: 2074 Rails source/fixture/gem files match d7c7de9264c63015be398001d7a1094e7695a6db byte for byte; non-runtime bin/release omitted from image
GIT_REVISION=d7c7de9264c63015be398001d7a1094e7695a6db
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-9259468407ef675b49c8961d58752d30a197f666305cd34b0f0295c7ae2e3fd5
seed_key=rust-parity-seed-v1-3852c7ae11a6e7c2b05c7f0d4313709db82b16e79ba53ebb2cc33e948178c7c4
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
WS8br2 pinned media runtime: image ws8br2-reference:d7c7de92; libraries extracted; no host libraries changed
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

Raw oracle summaries:

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
Rails full profile oracle: 1 complete application page and body; real seed memberships and WS9 security; reference d7c7de92, status templates 2e20b24c
Rails joining oracle: 1 complete signup body; 6 HTTP cases with user, room and session state; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails zone oracle: 485 case-sensitive TZInfo identifiers; reference d7c7de92
Rails named-zone oracle: 152 names, 2 unavailable; reference d7c7de92
WS8br2 oracle verification: all 19 fresh files match byte for byte; no masks or normalization
```

The Rust tests and compiled-regression checks use these seed, media, scratch and test-port settings; metadata and clippy require no runtime media:

```sh
export CI=1
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media/lib/x86_64-linux-gnu:$PWD/.scratch/rails-media/usr/lib/x86_64-linux-gnu"
export CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference"
export TMPDIR="$PWD/.scratch/tmp"
export CABLE_TEST_PORT_RANGE=52600-52649 MAIL_TEST_PORT_RANGE=52600-52649
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db -p campfire_kit -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views --test core -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

Raw test summaries, in order: app, database, kit unit, kit front/http/params/Rails integrations, database/kit doctests; views core; clippy:

```text
test result: ok. 498 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 93.46s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 51.59s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.45s
```

All active tests passed. Existing ignores: app's reference-recording test, WS11 `manages_bots`, and push-latency measurement; four DB external Rails comparison/export/rollback tests; two kit example doctests. No new ignored tests. These external opt-in comparisons and browser execution are not represented as passing.

## Compiled failing checks and restoration

The following actual source regressions were compiled in the fresh clone: allow SVG script; fail CSV formula neutralization; remove nosniff; invert first-run guard; alter inbox heading; invert join-code guard. Each selected test must fail at runtime, then source is restored in `finally`. These are discrimination experiments, not failures of the final branch, and are not a claim about TDD chronology.

```sh
python3 rust/reference-tools/users/discrimination.py icons-svg audit-csv-formula icons-header first-run-repeat profile-inbox join-code
mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire controllers::users::profile_page_tests -- --nocapture
git diff --exit-code
```

Raw results and restored control:

```text
join-code: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.51s
first-run-repeat: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.41s
profile-inbox: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.12s
icons-svg: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.12s
audit-csv-formula: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.14s
icons-header: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 500 filtered out; finished in 0.47s
WS8br2 discrimination: 6 compiled regressions detected; sources restored
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 496 filtered out; finished in 0.60s
```

The restored clone's tracked diff is empty. Initial complete-page checks also failed on signup/setup whitespace, omitted sections, direct-room names and ETag/media differences before correction; no expected output was loosened or masked.

## Per-file pass counts and deferred inventory

```sh
python3 rust/reference-tools/users/file_counts.py ../delivery-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
```

Raw executed-file counts:

```text
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
WS8br2 file accounting: 59 executed Rust groups; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred
```

First-run adds one complete body, seven HTTP/state cases and the five-request race; welcome adds body/sidebar/frame and redirects; profile adds one complete application page, owner/error integration, Edge and DND expiry; joining adds one complete body and six HTTP/state cases. PWA/QR and older checks ran in the complete app suite and freshly regenerated oracles.

The inventory is a named criterion mapping, not a claim of executed Ruby/system tests. Compared with the last 148-deferred report, 56 criteria now have additional equivalent checks, and two previously exercised criteria (basic user show and account-role divider) were corrected in the mapping. **90 remain**, largest files first. Their names/owners are exhaustive for the original 194; the additional post-pin popup work is listed separately above.

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 31 | 26 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 0 | 19 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 0 | 2 |
| `test/controllers/users_controller_test.rb` | 226 | 20 | 10 | 10 |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 217 | 15 | 15 | 0 |
| `test/system/starred_people_test.rb` | 182 | 4 | 0 | 4 |
| `test/system/icons_test.rb` | 169 | 4 | 0 | 4 |
| `test/controllers/accounts/icons_controller_test.rb` | 103 | 6 | 6 | 0 |
| `test/system/service_worker_test.rb` | 102 | 2 | 0 | 2 |
| `test/controllers/users/bans_controller_test.rb` | 99 | 8 | 5 | 3 |
| `test/controllers/users/profiles_two_factor_test.rb` | 96 | 7 | 0 | 7 |
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
- profile offers a connect button without an account
- profile links to connect for meeting status without an account
- profile offers the meeting toggle for a connected account
- profile shows the meeting fetch notice when a refresh failed
- profile asks to reconnect for meeting status left on after disconnect
- the layout sends meeting windows for the live sound gate
- the layout sends future meeting windows before the meeting starts
- the layout sends no meeting windows without cached intervals
- the layout sends no meeting windows when meeting status itself is off
- the layout sends OOO windows for the live sound gate
- the layout sends future calendar OOO windows before the OOO starts
- the layout sends no OOO windows when keeping notifications while out
- the layout leaves sounds alone for meetings when quiet-during-meetings is off
- profile shows the connected account with a disconnect button
- profile offers a reconnect when Google rejected the connection
- profile offers Drive previews for a connected account without the Drive scope
- profile shows Drive previews as enabled when the account has the Drive scope
- profile offers Drive previews again for the retired metadata grant
- profile asks to reconnect when the grant lacks the calendar scope
- profile shows Disconnect for a partial grant with Drive still active
- reconnect preserves a granted Drive scope
- reconnect without Drive requests the calendar scope only
- layout carries the Drive previews meta tag only with the Drive scope
- the layout carries the theme, time zone, and sound state
- the layout mutes sounds for the DND presence
- the layout sends the quiet-hours window and zone for the sound gate

`test/system/people_group_dms_test.rb` — **WS8br2 people/cards; WS8br DM actions**:
- clicking a message author opens their profile card and Message lands in the DM
- the profile card opens by keyboard, traps focus, and returns it on Esc
- Esc with a closed profile card stays unhandled for later listeners
- the sidebar avatar trigger opens the profile card by keyboard
- multi-selecting three people in the directory lands in their group DM
- the member panel multi-select starts a huddle with exactly that set
- shift-click extends the checkbox range
- long-press selects a row on touch
- agents are selectable for messages but excluded from huddles
- start huddle keeps agents in the DM but out of the call
- the new-DM picker filters as you type with no suggestion bubble or submit button
- picker selections survive filtering and Message starts the DM
- Enter in the picker filter selects the single visible match
- clicking a picker row toggles it while the name still opens the profile card
- the new-DM picker does not overflow at phone width
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

`test/controllers/users/profiles_two_factor_test.rb` — **WS9; WS8br2 profile panel integration**:
- profile shows the 2FA section with devices and revoke buttons
- profile asks for re-authentication on every sensitive 2FA action
- profile offers Google confirmation to linked members
- profile hides Google confirmation without a linked account
- profile points unenrolled users at setup
- changing the password revokes all remembered devices
- updating the name keeps remembered devices

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

Deferred inventory: 90 named Rails controller/system cases remain from the original 194; 104 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.

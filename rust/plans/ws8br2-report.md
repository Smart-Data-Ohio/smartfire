# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Reference: `d7c7de9264c63015be398001d7a1094e7695a6db`.
Started at WS8br `c4849d54` containing main `21a7332f`; merged reviewed WS9/main `4278cb1e` with merge commit `279b9652` and ran locked metadata before pushing.

Verified implementation SHA: `1c36f329ba65cee9ab09cb17cec58428baa6493e`. A final documentation-only commit carries this report. All implementation slices and the merge are pushed. No PR, release build, deployment or cutover was performed.

## Completed slices and merge resolution

- Previously received: `3da190f1` public/about/privacy/terms, Unicode/ASCII-boundary avatar initials, QR capacity/HTTP bytes, and PWA HTTP vectors; `9301593c` current-user browser timezone detection and repeat tour completion timestamps; `b13466da` the prior partial report.
- `8779ff6b`: people directory and cards, typed read-only presence/agent/star facts, signed streams, current-user exclusion, starred-first ordering, inactive actions and accessible message labels. Thirteen card bodies and two directory bodies match pinned Rails bytes.
- `b0f27646`: integrate WS9's then-current security/session implementation at `d336ca78` while retaining WS8br room/sidebar seams. This is superseded by the main merge below.
- `ffea7b1b`, `8c765d62`: manual profile appearance/settings, four byte-exact appearance states and 135 zone choices; 31 real PATCH/state cases for core name/bio, timezone/theme/text size, call keys, all five inbox switches, GitHub login normalization/verified-account protection, malformed/nil input, current-user scope and rollback. Explicit Not set remains clear after later browser detection.
- `73ebe8e1`: initial account/ban audited mutation differential, 23 real HTTP/state/audit vectors. Its duplicate producer module is **removed** after WS9 merged.
- `74fa2194`: honest partial checkpoint preserving account view probes/tests before merging main; its collection annotation and nav/footer capture were then completed.
- `279b9652`: merge `origin/main` at `4278cb1e`, remove `models/account_mutations.rs`, and select WS9's account and authentication producers. WS9 now handles core profile/raw password/email checks and explicit timezone writes; this branch's settings writer handles the other owned preferences in the same transaction, after WS9's core writer. The review's producer files are unchanged from main.
- `204c91fb`: nine individual HTTP tests asserting **exactly one** Rails-matching audit row for name, room-creation restriction, custom CSS, join-code reset, logo removal, role change, deactivation, ban and unban. Each checks actor/subject IDs, action, labels, complete details, IP and user agent. Keep WS9's producer unchanged, retain all-row preference validation before role/ban changes, redirect failed non-bang role saves without an audit, and return Rails' 422 on rejected bans. No-op/replay/forbidden/stale-sudo cases write zero rows.
- `1c36f329`: account settings body/nav/footer for administrator and member, 13 member row states, invitations and CSS editor match Rails bytes. Restore missing workspace-icon/audit/Slack-import admin links, Integration health, Smartfire wording, exact 2FA reset control whitespace, and read-only Google linked/self-changed/allowed email controls.

## Changes by file

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

No Rails edits, schema changes, parity masks or allowlist changes. No new ignored test was added here; WS9 imports one explicit rollback opt-in test.

## Validation/callback limits and cross-stream ownership

Domain mutations know no HTML. The people model is read-only; the presenter turns facts into view inputs. Core/auth/audit writes use WS9 directly. Settings updates reuse the existing persisted-row validator rather than inventing a second ruleset. Role and ban operations validate preferences before calling the unchanged WS9 producer. The first-run controller remains ours; its session/enrollment producer comes from WS9. Agent suspension/revocation remains WS11-owned.

The profile writer normalizes blank timezone/key/login, retains raw-key explicit timezone behavior, merges only the five allowed inbox keys, protects verified GitHub login, validates effective persisted preferences, and runs in the same transaction as core/security/attachment changes. Invalid preference requests roll back email/password markers, audits and remembered-device revocation. Account changes record actual name/restriction/logo differences; CSS audits count UTF-8 bytes and use the 12-character digest prefix, not CSS content. Join reset records no credential. Logo removal records Rails' true/false pair even if no attachment was present. Role failed saves and ban no-op/replays emit no row. Deactivation captures the label before the email rewrite.

WS8br room/sidebar/layout code is retained; only shared dispatch, test support and existing user-summary seams were combined during the WS9 merges. WS9's profile security/sudo/2FA/session code comes from merged main. Account-row Google data is a read-only WS14 seam; its mutation/OAuth endpoints were not reimplemented. Stars are read-only WS12 inputs. Full WS17 meeting/OOO/DND/status composition and WS11 agent lifecycle/presentation are still needed. The agent-owner deactivation Rails oracle captures `agent.suspend` followed by `user.deactivate`; the latter alone is not parity. That one case remains explicitly outside the 23 tested mutation vectors, with no new Rust ignore.

## Fresh-clone verification

All results below come from a newly cloned pushed branch at `1c36f329ba65cee9ab09cb17cec58428baa6493e` in `.scratch/final-clone`. No target directory, seed, or scratch fixture was copied from the working checkout. The pinned reference archive and both seeds were built from the tracked recipes; tests used `CI=1` so missing seeds fail. Each clone has its own Cargo target and TMPDIR. Port range is 52600–52649.

Commands were executed from the worktree for clone, then from `.scratch/final-clone` for the rest. Logs are in the parent `.scratch/`.

```sh
git clone --single-branch --branch rust/ws8br2-users-accounts https://github.com/Smart-Data-Ohio/smartfire.git .scratch/final-clone
bash rust/parity/bin/ci-seed prepare
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-final PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
```

Raw prepare/seed lines (metadata exited 0, producing JSON rather than a summary):

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-9259468407ef675b49c8961d58752d30a197f666305cd34b0f0295c7ae2e3fd5
seed_key=rust-parity-seed-v1-3852c7ae11a6e7c2b05c7f0d4313709db82b16e79ba53ebb2cc33e948178c7c4
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Seed validators (same reference/namespace/owner/image/runtime environment as seed build):

```sh
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-final PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" PARITY_NAMESPACE=ws8br2-final PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run
```

Raw JSON summaries, default then first_run:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

Tests and clippy:

```sh
CI=1 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" TMPDIR="$PWD/.scratch/tmp" CABLE_TEST_PORT_RANGE=52600-52649 MAIL_TEST_PORT_RANGE=52600-52649 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db -- --test-threads=4 --nocapture
CI=1 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" TMPDIR="$PWD/.scratch/tmp" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views --test core -- --nocapture
TMPDIR="$PWD/.scratch/tmp" CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

Raw summaries, app/database/doctests, views core, then clippy:

```text
test result: ok. 470 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 104.41s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 64.23s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 06s
```

Three app ignores did not run: cable live-reference recorder, WS11's known `manages_bots`, and push-latency measurement. Four DB opt-ins did not run: Rails fixtures comparison, scenario comparison, export rollback, and WS9 security rollback. No seed-dependent tests silently skipped. The full views legacy suite, remaining workspace crates' runtime tests, Ruby test files, browser system suites and screenshot/theme/viewport matrix were **not** run; the all-target clippy check compiles them but does not execute them.

Fresh source/oracle, producer, and per-file checks:

```sh
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" WS8BR2_ORACLE_DIR="$PWD/.scratch/verified-oracles" bash rust/reference-tools/users/run_oracles.sh
CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" python3 rust/reference-tools/users/discrimination.py people-auth people-self people-star people-call people-status profile-settings profile-fields account-admin account-sudo account-role ban-invalid audit-duplicate account-view-auth account-view-google
git diff --exit-code 4278cb1e -- rust/crates/campfire/src/authentication.rs rust/crates/campfire/src/account_security.rs rust/crates/campfire/src/concerns/two_factor.rs rust/crates/campfire/src/concerns/sudo.rs rust/crates/campfire/src/controllers/sudos.rs rust/crates/campfire/src/controllers/two_factor.rs rust/crates/campfire/src/controllers/users/sessions.rs rust/crates/db/src/models/two_factor.rs
python3 rust/reference-tools/users/file_counts.py ../final-tests.log
python3 rust/reference-tools/users/deferred_inventory.py
```

The producer diff exited 0 with empty output. Raw oracle and discrimination summaries:

```text
Rails public oracle: 15 page bodies, 31 policy inputs, 4 QR cases; reference d7c7de92
Rails avatar oracle: 15 initials SVG bodies; reference d7c7de92
Rails preference oracle: 17 time-zone cases, 1 tour touch; reference d7c7de92
Rails people oracle: 13 cards, 2 directories; reference d7c7de92
Rails profile settings oracle: 31 PATCH cases; reference d7c7de92
Rails appearance oracle: 4 bodies, 135 zone choices; reference d7c7de92
Rails account mutation oracle: 23 HTTP cases with audit snapshots, 1 deferred agent-owner case; reference d7c7de92
Rails account views oracle: 13 rows, 2 settings bodies/navs/footers, 2 invites, 2 CSS bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails zone oracle: 485 case-sensitive TZInfo identifiers; reference d7c7de92
Rails named-zone oracle: 152 names, 2 unavailable; reference d7c7de92
WS8br2 oracle verification: all 12 fresh files match byte for byte; no masks or normalization
account-view-auth: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.12s
account-view-google: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.12s
audit-duplicate: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.88s
account-admin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.47s
account-sudo: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.88s
account-role: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.94s
ban-invalid: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 1.03s
profile-settings: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.47s
profile-fields: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.09s
people-auth: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.43s
people-self: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.99s
people-star: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 1.28s
people-call: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.74s
people-status: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 472 filtered out; finished in 0.69s
WS8br2 discrimination: 14 compiled regressions detected; sources restored
```

Actual Rust pass counts by owned test file (consolidated groups, **not** Ruby per-file passes):

```text
controllers/accounts/view_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/users/people_tests.rs: 12 passed; 0 failed; 0 ignored
controllers/users/profile_settings_tests.rs: 4 passed; 0 failed; 0 ignored
controllers/accounts/mutation_tests.rs: 11 passed; 0 failed; 0 ignored
WS8br2 file accounting: 31 executed Rust groups; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred
```

Security/discrimination evidence: the first people HTTP checks failed with missing route/auth behavior; accessible label and manual profile checks also failed before their ports. The account audit/state test initially failed on missing rows. After merging WS9, the re-diff caught a persisted-invalid-theme role save and a private-IP ban response mismatch; the role save now redirects without an audit and the rejected ban returns 422. All four account view groups failed on their complete bodies before the template fixes. The committed discrimination script rebuilds every deliberate mutation, requires the named test to fail with a test-result summary (compile errors do not count), then restores exact source bytes in finally. Its new duplicate-producer mutation proves the exactly-one assertion detects a second row, and the member/Google-control mutations prove the row markup checks discriminate.

## Remaining work, in requested order

1. **Users/profiles/cards remain partial.** Complete Users#show agent identity/rooms/grants/24-hour activity/visibility/suspension through WS11/WS11-ui seams and full DND allow/status meeting/OOO composition through WS17. Finish the profile's GitHub/inbox/call controls and error paragraphs, status/notification and Google/Fizzy/Slack integration panels; compare the entire profile/layout and error pages. Security producers/panels are from main, but all seven named profile-2FA integration criteria remain explicitly deferred until mapped and exercised. JSON failure format differs (Rails missing JSON template 500 versus Rust 406); malformed scalar/boolean coercions beyond the recorded cases and uploaded avatar paths remain unproved.
2. **Accounts remain partial.** Re-diff/port icons create/delete/index, workspace-icon serving authentication/caching/ETags/SVG validation, audit-log HTML/filters/CSV/cache/truncation/formula behavior with WS9 export-sudo enforcement, uploaded logo/variant/type/concurrency paths and complete parameter coercions. Finish pagination/Turbo-stream byte checks. Integrate WS11 owned-agent suspension and its audit before owner deactivation. Current 23 mutation vectors and account/settings/member/invite/CSS markup are complete for the tested states; they do not establish these remaining paths.
3. **Welcome/first run not advanced in this session.** Re-diff/port their complete responses, persistence/callback/session and initial enrollment paths, and first-run tour keyboard, skip/Escape, restart and reload/browser persistence acceptance through WS9/room/composer seams.
4. **Deferred cases still partial.** 148 named cases remain from the received 194. The largest file, profiles (573 lines), now has 20 covered criteria and 37 remaining; people group DMs (459 lines) still has 19, audit logs (217 lines) still has 15. The inventory below lists every exact remaining name and owner, sorted by source-file size. None is implicitly covered by a broad happy-path count or by the fact that WS9 merged.

Open coordination dependency: no reviewed WS11 seam SHA was supplied in this session; no WS11 branch was merged. The explicit agent-owner Rails vector remains the handoff for suspension/audit integration. No permission request is pending for the owned reversible work.

## Exact deferred inventory and criterion accounting

This is a mapping to equivalent Rust assertions. The Ruby files/system tests themselves were not run. Six previously covered avatar/PWA cases were excluded from the received 194 baseline; 46 additional criteria now have mapped coverage. Rust pass counts are separately shown above.

| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |
|---|---:|---:|---:|---:|
| `test/controllers/users/profiles_controller_test.rb` | 573 | 57 | 20 | 37 |
| `test/system/people_group_dms_test.rb` | 459 | 19 | 0 | 19 |
| `test/controllers/public_pages_controller_test.rb` | 226 | 2 | 0 | 2 |
| `test/controllers/users_controller_test.rb` | 226 | 20 | 4 | 16 |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 217 | 15 | 0 | 15 |
| `test/system/starred_people_test.rb` | 182 | 4 | 0 | 4 |
| `test/system/icons_test.rb` | 169 | 4 | 0 | 4 |
| `test/controllers/accounts/icons_controller_test.rb` | 103 | 6 | 0 | 6 |
| `test/system/service_worker_test.rb` | 102 | 2 | 0 | 2 |
| `test/controllers/users/bans_controller_test.rb` | 99 | 8 | 5 | 3 |
| `test/controllers/users/profiles_two_factor_test.rb` | 96 | 7 | 0 | 7 |
| `test/system/first_run_tour_test.rb` | 89 | 4 | 0 | 4 |
| `test/controllers/users/cards_controller_test.rb` | 87 | 7 | 7 | 0 |
| `test/controllers/workspace_icons_controller_test.rb` | 78 | 7 | 0 | 7 |
| `test/controllers/pwa_controller_test.rb` | 66 | 1 | 0 | 1 |
| `test/system/timezone_detection_test.rb` | 64 | 2 | 0 | 2 |
| `test/controllers/accounts_controller_test.rb` | 61 | 4 | 3 | 1 |
| `test/system/workspace_icons_test.rb` | 60 | 1 | 0 | 1 |
| `test/controllers/first_runs_controller_test.rb` | 57 | 4 | 0 | 4 |
| `test/controllers/accounts/logos_controller_test.rb` | 56 | 6 | 0 | 6 |
| `test/system/audit_log_test.rb` | 49 | 2 | 0 | 2 |
| `test/controllers/accounts/users_controller_test.rb` | 36 | 3 | 2 | 1 |
| `test/controllers/users/avatars_controller_test.rb` | 34 | 2 | 0 | 2 |
| `test/controllers/accounts/custom_styles_controller_test.rb` | 30 | 3 | 3 | 0 |
| `test/controllers/accounts/join_codes_controller_test.rb` | 21 | 2 | 2 | 0 |
| `test/controllers/welcome_controller_test.rb` | 21 | 2 | 0 | 2 |

`test/controllers/users/profiles_controller_test.rb` — **WS8br2 integration; WS9 security panels, WS17 status/notifications, WS13 calls, WS14/WS15 Google, WS15g GitHub seams**:
- show gives the Edge install instructions to a browser identifying only as Edge
- profile shows Google Calendar as not configured without credentials
- profile offers a connect button without an account
- profile links to connect for meeting status without an account
- profile offers the meeting toggle for a connected account
- profile shows the meeting fetch notice when a refresh failed
- profile asks to reconnect for meeting status left on after disconnect
- profile lists the quiet-during-meetings switch
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
- profile shows no Drive row when Google is not configured
- profile asks to reconnect when the grant lacks the calendar scope
- profile shows Disconnect for a partial grant with Drive still active
- reconnect preserves a granted Drive scope
- reconnect without Drive requests the calendar scope only
- layout carries the Drive previews meta tag only with the Drive scope
- a github login cannot be claimed by a second user
- profile lists the notification switches with explanations
- profile rejects non-boolean notification input
- DND switch reflects the effective state after a timed expiry
- DND switch stays on while a timer runs
- profile lists the call settings with their defaults
- profile rejects an unknown microphone mode
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
- show
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
- new
- new does not allow a signed in user
- new requires a join code
- create
- creating a new user with an existing email address will redirect to login screen

`test/controllers/accounts/audit_logs_controller_test.rb` — **WS8br2; WS9 CSV sudo seam**:
- admins can browse the log
- members are forbidden
- visitors are sent to sign in
- visitors cannot export CSV
- members cannot export CSV
- filtering by actor matches names and emails in labels
- filtering by action and target type
- unknown filter values are ignored
- filtering by date range
- paging walks older entries
- CSV export carries headers and the filtered rows
- CSV export neutralizes formula injection
- past the export cap the page warns and the CSV filename says truncated
- within the export cap there is no truncation notice
- CSV export neutralizes formula injection in request columns

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

`test/controllers/accounts/icons_controller_test.rb` — **WS8br2**:
- index lists icons with previews shortcodes titles and uploaders
- create uploads an icon
- create renders validation errors inline
- create reports a name that raced past validation as taken
- destroy removes the icon and its blob
- members get forbidden on list create and delete

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

`test/controllers/workspace_icons_controller_test.rb` — **WS8br2**:
- serves an SVG with the documented headers
- serves a PNG without the SVG-only headers
- supports conditional GETs with the blob checksum
- returns not found for unknown names
- returns not found for signed-out users
- unenrolled sessions are sent to setup instead of served the icon
- stale enrolled sessions are signed out instead of served the icon

`test/controllers/pwa_controller_test.rb` — **WS8br2 service-worker event harness**:
- service worker fetch and notification logic

`test/system/timezone_detection_test.rb` — **WS8br2**:
- the browser does not report its zone without a CSRF token
- the browser reports its detected zone once

`test/controllers/accounts_controller_test.rb` — **WS8br2**:
- edit groups administrators separately from members with a divider

`test/system/workspace_icons_test.rb` — **WS8br2**:
- upload post in both themes then delete falls back to the shortcode

`test/controllers/first_runs_controller_test.rb` — **WS8br2; WS9 session seam**:
- new is permitted when no other users exit
- new is not permitted when account exist
- create
- create is not vulnerable to race conditions

`test/controllers/accounts/logos_controller_test.rb` — **WS8br2**:
- show stock
- show stock small size
- show custom
- show custom small size
- show stock when custom logo cannot be resized
- destroy

`test/system/audit_log_test.rb` — **WS8br2; WS9 export sudo seam**:
- admin browses filters and exports the audit log
- audit log stays usable at phone width

`test/controllers/accounts/users_controller_test.rb` — **WS8br2; WS9 sudo/security metadata seams**:
- destroy

`test/controllers/users/avatars_controller_test.rb` — **WS8br2**:
- show image
- show initials when image cannot be resized

`test/controllers/welcome_controller_test.rb` — **WS8br2**:
- redirects to the first created visible room the user has access to
- redirects to the last room visited, if we have one

Deferred inventory: 148 named Rails controller/system cases remain from the original 194; 46 criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.

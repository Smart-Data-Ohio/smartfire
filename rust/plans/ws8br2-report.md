# WS8br2 users, accounts and public pages — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8br2-users-accounts`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br2`.
Rails reference: `d7c7de9264c63015be398001d7a1094e7695a6db`.
Starting branch: `c4849d54`, containing main `21a7332f`.

Pushed implementation commits:

- `3da190f1b72cc9774cab88f58c941441242b21e4` — Match public pages, avatar initials and QR responses to Rails.
- `9301593c908a330878b8756bfc55df36ad33cfbf` — Port browser time-zone detection and tour completion.

This report follows in a documentation-only commit. No PR, main merge, deployment, release build, or cutover was performed. This is two coherent slices, not completion of the assigned users/accounts/public-page workstream.

## Delivered behavior

Public `/about`, `/privacy`, and `/terms` now render their complete Rails bodies through the existing public layout. They bypass the workspace authentication/browser callback chain, set no session cookie, and expose no current-user/workspace metadata, scripts, CSRF tokens, or revision headers. HTML, wildcard Accept, HEAD, unsupported formats, no account, configured installation, malicious values, email URI escaping, and Unicode policy inputs are covered. Five installation states yield fifteen byte-exact Rails HTML bodies; thirty-one model vectors cover operator/contact/date normalization and validation.

Default avatar initials now match Ruby `name.scan(/\b\w/)`: matching characters are ASCII word characters, while word boundaries account for Unicode letters, marks, numbers, connector punctuation and join controls. Fifteen signed HTTP SVG bodies, ETags, cache behavior and conditional 304s match Rails, including combining marks and non-ASCII names. The shared view helper returns the same initials as the domain method. Uploaded/non-resizable images remain deferred.

Oversized QR input now raises the Rails-equivalent internal error (500), replacing the inherited 422. Four pinned QR cases exercise successful body/cache bytes and capacity failure. Existing PWA serving code already matched; new HTTP vectors verify manifest, service-worker source and offline shell for both default and first-run seeds. Served-source equality does not establish service-worker execution in a browser.

`PATCH /users/:user_id/time_zone` now uses the authenticated current user, ignoring the path/body user id as Rails does. Detection only fills an unset, non-explicit zone. Explicit Not set and saved choices win. Invalid/unknown/wrong-shaped input, exact zone casing, failed persisted-setting validation, rollback/reload JSON and timestamps match seventeen Rails cases. Missing session, CSRF, bot-key and invalid bearer checks are exercised. A normal member can update their own preferences even when the URL names another member, while that other row remains unchanged.

`PATCH /users/:user_id/tour` touches the current user's completion and updated timestamps, returns an empty 204 and refreshes both timestamps on repeat completion, like Rails `touch`. It bypasses model validation, including an invalid persisted theme. An HTTP room request verifies auto-start changes from true to false and that the help-menu control is present; keyboard navigation, Escape/skip, restart and browser persistence acceptance remain deferred.

## Changes by file

All implementation paths below are relative to `rust/`.

| Files | Change |
| --- | --- |
| `crates/campfire/src/public_policy.rs` | Pure installation-policy model: Ruby strip/blank behavior, email validation and Unicode date allowlist. A freshly expanded Rails vector exposed Number versus Decimal_Number and the second slice fixes it. |
| `crates/campfire/src/config.rs`, `main.rs` | Load the policy from the three LEGAL environment values and register its module. |
| `crates/campfire/src/controllers.rs` | Wire three public routes and the two user preference routes. |
| `crates/campfire/src/controllers/public_pages.rs` | Thin public controller, format handling and seeded HTTP byte/security tests. |
| `crates/views/src/public_pages.rs` | Plain render inputs and public-page composition through the existing layout. |
| `crates/views/templates/public_pages/about.html`, `privacy.html`, `terms.html` | Port our pinned ERB body markup and whitespace to Askama. |
| `crates/campfire/src/controllers/presenters/test_support.rs` | Small shared test seam: select seed/clock and pass installation environment values. |
| `crates/campfire/src/controllers/pwa.rs` | Default and first-run endpoint HTTP differential test; no production serving change. |
| `crates/campfire/src/controllers/qr_code.rs` | Capacity error status correction and HTTP byte/cache/capacity regression test. |
| `crates/campfire/src/controllers/users/avatars.rs` | Initials SVG/cache/ETag/304 differential test, including view-helper parity. |
| `crates/db/src/models/user.rs` | Ruby-equivalent initials and typed current-user timezone/tour domain operations. |
| `crates/views/src/helpers/users.rs` | Match domain initials in the render helper. |
| `crates/campfire/src/controllers/users.rs` | Register time_zones, tours and their test module. |
| `crates/campfire/src/controllers/users/time_zones.rs`, `tours.rs` | Authenticate/verify request, call the domain, render JSON or 204. |
| `crates/campfire/src/controllers/users/preferences_tests.rs` | Four consolidated HTTP/domain tests for all seventeen detection vectors, auth/CSRF/current-user scope, repeat tour touch and room-layout flags. |
| `crates/db/src/slash_commands.rs`, `slash_commands/user_settings.rs` | Make the existing settings update validator visible within the database crate; no duplicate validator. |
| `crates/db/src/slash_commands/time_parser.rs` | Case-sensitive Rails TZInfo identifier gate before Jiff lookup, plus complete identifier/named-zone availability differential. |
| `crates/db/src/slash_commands/rails_zone_identifiers.json` | 485 sorted identifiers generated by the pinned Rails runtime, used for the case-sensitive gate. |
| `crates/db/src/slash_commands/rails_named_zones.json` | 152 Rails named-zone mappings with actual runtime availability, used by the differential test. Two are unavailable in that runtime: Kyiv and Rangoon. |
| `reference-tools/users/public.rb`, `avatars.rb`, `pwa.rb`, `preferences.rb`, `zones.rb` | Run real Rails policy/model/controller requests and extract unmasked vectors. |
| `reference-tools/users/source-hashes.json` | Pin the owned Rails model/controller/helper/template sources; the probes reject drift. |
| `reference-tools/users/run_oracles.sh` | Regenerate all seven files and compare raw bytes with the committed vectors/catalogues. |
| `reference-tools/users/discrimination.py` | Twelve compiled mutation checks; compile errors and false passes are rejected, sources restored in finally. |
| `reference-tools/users/deferred_inventory.py` | Enumerate every deferred named controller/system case from the fixed source pin, with ownership. |
| `vectors/users_public.json`, `users_avatars.json`, `users_pwa_default.json`, `users_pwa_first_run.json`, `users_preferences.json` | Pinned HTTP/policy/state vectors. |
| `plans/ws8br2-report.md` | Tracked copy of this report. |

No schema, Cargo dependencies/lockfile, Rails source, parity mask or allowlist changes.

## Domain design and cross-workstream seams

The public policy model knows no HTML. The timezone/tour operations live in User's database domain, not controllers or Askama. Controllers only authenticate, call those operations and render results. Rendering uses plain values and the existing public layout.

| Operation | Rails validation/callback parity in this slice |
| --- | --- |
| Browser zone detection | Reuse the existing settings validator for persisted presence/theme/text-size/status lengths, quiet-hour bounds/completeness, zone validity, inbox/voice settings and GitHub uniqueness. Only zone assignment is changed; login/icon/status assignment-conditional callbacks do not fire. Invalid rows roll back, then the controller reloads the saved zone for 422 JSON. |
| Tour completion | One transaction updates tour_completed_at and updated_at; no validation, audit, notification, or queue effect is introduced. Repeat requests touch again. |
| Public policy | Environment-derived values; no account, user, session or database state is read. |
| Initials | Pure string operation in the domain and equivalent view helper, with common pinned vectors. |

Shared seams are the route registry/config, TestApp's seed/env constructor, User's pure helpers/domain operations, view initials, and the database settings validator/zone parser. The case-sensitive parser also serves slash commands; the complete database suite passes after this change. WS17 still owns status/DND/notification behavior. WS9 owns security panels/sudo/session/device behavior; those panels were not fabricated or ported here. WS11 bot/agent facts, WS12 stars, WS13 calls and WS14/WS15/WS15g integration facts remain owned by those streams. WS8br's room/layout implementation was not edited; the tour test consumes its existing HTTP shell. No broadcasts changed.

## Rails test accounting

The following pinned Rails cases have equivalent coverage in this slice. They are consolidated Rust tests rather than one Rust function per Rails test. The actual Ruby controller/model test suite was not run wholesale. Probes execute the real Rails app; the full Rust suites and the fresh probes are listed below.

`test/controllers/public_pages_controller_test.rb`:

- about renders signed-out with stable title and navigation
- privacy renders signed-out with Google disclosures
- privacy discloses picker-only Drive previews
- terms renders signed-out with software-license framing
- public pages never redirect to sign-in and set no session cookie
- public pages ignore the modern-browser gate, even for crawlers
- public pages render without OAuth configured
- public pages disclose no private state, credentials, or scripts
- public pages allow zoom and honor color schemes
- non-HTML formats expose nothing
- wildcard Accept header receives the HTML page
- HEAD requests succeed
- unconfigured installation uses generic wording without env names
- configured installation names the operator and contact
- operator name is escaped and malicious contact email is dropped
- public pages open in a new tab so following them never disturbs the current tab

`test/models/public_policy_test.rb`:

- operator name is blank-friendly with no company default
- contact email accepts valid addresses
- contact email rejects blank and malformed values
- contact email rejects header-injection and markup-breaking values
- effective date is stable by default and constrained when configured

`test/controllers/users/time_zones_controller_test.rb`:

- detects the browser zone when none is saved
- a hand-picked zone wins over later detections
- detection never overwrites an explicit choice, not even Not set
- an unknown zone is ignored

`test/controllers/users/tours_controller_test.rb`:

- update stamps the tour as completed
- update is idempotent
- update requires sign-in
- room pages carry the tour shell for members who never completed it
- room pages skip auto-start once the tour completed

`test/controllers/users/avatars_controller_test.rb`:

- show initials
- show image with invalid token responds 404

`test/controllers/pwa_controller_test.rb`:

- service worker serves as JavaScript with the fetch and notification handlers
- service worker caches static assets only
- notification clicks focus an existing window before opening a new one
- offline shell renders signed-out with reconnect behavior

Additional cases go beyond those named Rails tests: hostile/operator/mail URI bytes, every public policy boundary represented by thirty-one inputs, fifteen Unicode initials, QR capacity errors, seventeen zone inputs and persisted settings, current-user path spoofing, auth/CSRF/bot/bearer denials, all 485 identifiers and 152 named-zone availability results. Invalid avatar-signature behavior uses the existing Rust test, run by the full app suite; it was not newly implemented here. Four PWA controller source assertions are subsumed by exact served-source/body equality; the fifth event-execution test and both browser system tests remain deferred.

## First-failure evidence

Before fixes, new checks failed on public 501 routes, QR 422 instead of 500, combining-mark initials, missing preference routes and wrong-case timezone acceptance. Expanded fresh policy vectors then exposed the overly broad Unicode Number allowlist. These are historical pre-fix observations, not the final test state. Their raw summary lines are retained under this worktree's ignored `.scratch/`:

Public missing routes / access checks (`.scratch/public-before.log`):

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 339 filtered out; finished in 0.33s
```

QR capacity status (`.scratch/qr-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.37s
```

Unicode initials boundary (`.scratch/avatars-before.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 344 filtered out; finished in 0.40s
```

Missing timezone/tour routes, including authorization checks (`.scratch/preferences-before.log`):

```text
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.65s
```

Wrong-case timezone accepted by Jiff (`.scratch/preferences-after.log`):

```text
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.52s
```

Expanded Rails policy vector rejects superscript Number (`.scratch/app-slice2-check.log`):

```text
test result: FAILED. 346 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 27.73s
```

Security-relevant failing-first checks include public access/absence of private state, hostile-value escaping and session-cookie isolation, preference CSRF and current-user scoping. The compiled mutation runner below re-shows failures for all twelve properties against the final test set. An early current-user mutation falsely passed when `/users/me` shadowed the body id; the test was corrected to use a foreign numeric path, checked against real Rails, and then killed the mutation. Compile failures never count as discrimination.

## Reference and final verification

Scratch, seed databases and Cargo artifacts stayed in this worktree; Docker resources used the ws8br2 namespace and network tests used ports 52600–52699. The owned reference image is `ws8br2-reference:d7c7de92`, checked against the source pin and Gemfile.lock before seed creation. The probes additionally verify their source hash ledger. Actual full default and first_run seeds were rebuilt, not placeholder directories. CI=1 prevents missing-seed skips in the app/database suites.

Every command in the following blocks was rerun in this session. Working directory for each is the worktree root named above; TMPDIR is the worktree's `.scratch/tmp`. Raw summaries follow each command; no output is inferred from an exit code.

Seed creation:

```bash
PARITY_NAMESPACE=ws8br2 PARITY_OWNER=ws8br2 PARITY_IMAGE=ws8br2-reference:d7c7de92 PARITY_RUNTIME=docker rust/parity/bin/seed build default first_run > .scratch/seeds-final.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Seven fresh Rails oracles, raw comparisons:

```bash
bash rust/reference-tools/users/run_oracles.sh > .scratch/oracles-final.log 2>&1
```

```text
Rails public oracle: 15 page bodies, 31 policy inputs, 4 QR cases; reference d7c7de92
Rails avatar oracle: 15 initials SVG bodies; reference d7c7de92
Rails preference oracle: 17 time-zone cases, 1 tour touch; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails PWA oracle: 3 endpoint bodies; reference d7c7de92
Rails zone oracle: 485 case-sensitive TZInfo identifiers; reference d7c7de92
Rails named-zone oracle: 152 names, 2 unavailable; reference d7c7de92
WS8br2 oracle verification: all 7 fresh files match byte for byte; no masks or normalization
```

Compiled regression discrimination (failures are required):

```bash
python3 rust/reference-tools/users/discrimination.py > .scratch/discrimination-final.log 2>&1
```

```text
public-date: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.05s
public-cookie: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.40s
public-escaping: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.45s
public-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.45s
avatar-boundary: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.75s
qr-capacity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.46s
pwa-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.40s
preference-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.39s
preference-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.45s
preference-explicit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.45s
preference-zone-case: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.47s
tour-stamp: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.47s
WS8br2 discrimination: 12 compiled regressions detected; sources restored
```

Full seeded app suite, after restoring mutation sources:

```bash
CI=1 TMPDIR="$PWD/.scratch/tmp" CABLE_TEST_PORT_RANGE=52600-52649 MAIL_TEST_PORT_RANGE=52600-52649 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```

```text
test result: ok. 347 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 29.67s
```

Full database library suite:

```bash
CI=1 TMPDIR="$PWD/.scratch/tmp" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db --lib -- --test-threads=4 --nocapture > .scratch/db-final.log 2>&1
```

```text
test result: ok. 400 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 47.49s
```

Views core suite:

```bash
TMPDIR="$PWD/.scratch/tmp" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views --test core -- --test-threads=4 --nocapture > .scratch/views-final.log 2>&1
```

```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

Workspace clippy (all targets; prescribed vendor exclusion):

```bash
TMPDIR="$PWD/.scratch/tmp" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.65s
```

Fresh origin/main comparison, isolated scratch target and ports:

```bash
CAMPFIRE_REFERENCE="$PWD" CI=1 TMPDIR="$PWD/.scratch/tmp" CABLE_TEST_PORT_RANGE=52650-52699 MAIL_TEST_PORT_RANGE=52650-52699 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/main-baseline/rust/Cargo.toml -p campfire -- --test-threads=4 --nocapture > .scratch/main-baseline-app.log 2>&1
```

```text
test result: ok. 306 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 28.85s
```

Named deferred inventory:

```bash
python3 rust/reference-tools/users/deferred_inventory.py > .scratch/deferred-inventory.md
```

```text
Deferred inventory: 194 named Rails controller/system cases. Existing upstream Rust tests do not constitute a re-diff or browser acceptance of these cases.
```

The three app ignores are inherited and explicit: `channels::tests::golden::record_reference` needs the reference recorder; `controllers::presenters::accounts::tests::manages_bots` is the known WS11 bot-key failure; `jobs::tests::push_latency` is a measurement. The three database ignores are `tests::differential_test::scenario_matches_ruby`, `tests::fixtures_test::export_database_for_rails`, and `tests::fixtures_test::fixtures_match_ruby_row_for_row`, which require dedicated comparison/export databases. They did not run. Negative helper tests deliberately exercise a missing seed and may print a local skip message; the seeded HTTP cases ran with real seeds and CI=1.

Workspace clippy ran all targets with warnings denied, using the prescribed html5ever vendor exclusion. The full workspace test suite, Rails rollback exporter and browser/system screenshot matrix were not run; app, full database library and views core tests are the explicit scope of the successful test commands above.

One earlier app run failed `jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner` because its final queue assertion observed a concurrently running `Retention::PruneJob` after the quote job had completed. The test/source is unchanged from origin/main. A fresh isolated origin/main build at `21a7332f2d3c324f0862cdf448baf17a84395aa0` passed, as did an isolated current-branch rerun and later full branch runs. This is an observed transient, not a main-reproduced inherited failure. It was not modified or suppressed. The fresh main comparison command and raw result are included above.

## Exact next slices and limitations

Restart with the profile update/security re-diff: the inherited profile writer still lacks the modified Rails email-change/current-password behavior, self-change timestamp, security audit/device-revocation effects and complete error-form parity. Do not treat the now-working browser-detection route as completion of manual profile timezone/theme/text-size saving. The 422 validation/render path and current-user protection for profile updates need their own failing-first security checks.

The remaining coherent slices are:

1. User directory and cards (currently unported routes), profile/show/new/create presentation and accessible names; integrate WS11 agent facts, WS12 stars and WS17 presence. Manual profile fields, Google/GitHub/call/status panels and security panels need their owning seams and complete body differentials.
2. Bans, account/user role and deactivation actions, join-code and custom-style mutations: re-diff admin/sudo denials, validations, audit deltas and all side effects. Deactivation includes session/device/setup-secret cleanup, memberships/stage/stream/grant changes, integration disconnects and owned-agent suspension. Writes and durable enqueues must remain atomic. These broad inherited paths were not certified by this slice's green suite.
3. Account settings and users views, Google/2FA metadata, account audits, logo upload/destroy/purge/resize and cache behavior; byte-exact templates remain to be generated and tested.
4. Account workspace-icon administration and authenticated workspace-icon serving: domain validation, SVG handling, upload name race, purge/audit, CSP/content-type/ETag/cache and WS9 enrollment/security gates. No icon parity implementation was delivered here.
5. Audit-log page/filter/pagination and CSV: 5,000-row cap/truncation warning/filename, formula neutralization in every supported column, no-store and WS9 sudo seam. No audit-log acceptance was delivered here.
6. Welcome and first-run setup re-diff, creation-race behavior and templates, followed by the complete browser tour/timezone/people/icons/audit/PWA system flows and exact pixel matrix. No screenshot masks or browser acceptance shortcuts were introduced.

No new product/policy decision is requested. Existing runtime behavior, including the two unavailable Rails named zones, is the reference. Partial work is due to scope, not a permission request or an external blocker. The exact deferred named tests and owners follow; WS8br2 retains integration responsibility rather than handing its entire profile/directory suite to adjacent owners.

## Deferred Rails tests, by owner

`test/controllers/users_controller_test.rb` — **WS8br2; agent/bot presentation facts from WS11**:

- show
- profile message buttons carry the accessible name
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
- index lists active members with presence and selection
- index lists starred people first with a star marker
- index requires sign-in

`test/controllers/users/profiles_controller_test.rb` — **WS8br2 integration; WS9 security panels, WS17 status/notifications, WS13 calls, WS14/WS15 Google, WS15g GitHub seams**:

- show
- show gives the Edge install instructions to a browser identifying only as Edge
- update
- updates are limited to the current user
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
- linking a github login strips and downcases it
- a github login cannot be claimed by a second user
- profile lists the notification switches with explanations
- profile saves the notification switches
- profile rejects non-boolean notification input
- DND switch reflects the effective state after a timed expiry
- DND switch stays on while a timer runs
- profile lists the call settings with their defaults
- profile saves the call settings
- profile rejects an unknown microphone mode
- clearing a github login unlinks it
- changing email requires the current password
- changing email with a wrong current password is refused
- a new password cannot stand in for the current one
- changing email with the current password records a self-change
- other profile edits and case-only email edits need no password and record nothing
- profile asks for the current password only when the account has one
- update saves the theme and time zone
- update saves the text size
- an IANA time zone round-trips through the form
- a legacy Rails time zone name still shows selected
- choosing a time zone or Not set records an explicit choice
- the layout marks an explicit Not set so the browser skips detection
- update rejects an unknown theme or time zone
- the layout carries the theme, time zone, and sound state
- the layout mutes sounds for the DND presence
- the layout sends the quiet-hours window and zone for the sound gate

`test/controllers/users/profiles_two_factor_test.rb` — **WS9; WS8br2 profile panel integration**:

- profile shows the 2FA section with devices and revoke buttons
- profile asks for re-authentication on every sensitive 2FA action
- profile offers Google confirmation to linked members
- profile hides Google confirmation without a linked account
- profile points unenrolled users at setup
- changing the password revokes all remembered devices
- updating the name keeps remembered devices

`test/controllers/users/cards_controller_test.rb` — **WS8br2; WS11 agents, WS12 stars, WS17 presence/status seams**:

- card shows identity, presence, role, and actions for a peer
- offline peers read offline
- card shows the presence dot and custom status badge
- your own card offers editing your profile instead
- agents can be messaged but not called
- inactive users show status without message actions
- card requires sign-in

`test/controllers/users/bans_controller_test.rb` — **WS8br2; WS9 sudo, WS11/WS13 revocation seams**:

- create bans user and creates ban records from sessions
- create destroys user sessions
- create succeeds when the user has a pending two-factor setup secret
- create enqueues RemoveBannedContentJob
- RemoveBannedContentJob deletes messages
- non-admins cannot ban users
- destroy removes ban records and sets user to active
- non-admins cannot unban users

`test/controllers/accounts_controller_test.rb` — **WS8br2**:

- edit
- edit groups administrators separately from members with a divider
- update
- non-admins cannot update

`test/controllers/accounts/users_controller_test.rb` — **WS8br2; WS9 sudo/security metadata seams**:

- update
- destroy
- non-admins cannot perform actions

`test/controllers/accounts/icons_controller_test.rb` — **WS8br2**:

- index lists icons with previews shortcodes titles and uploaders
- create uploads an icon
- create renders validation errors inline
- create reports a name that raced past validation as taken
- destroy removes the icon and its blob
- members get forbidden on list create and delete

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

`test/controllers/accounts/custom_styles_controller_test.rb` — **WS8br2; WS9 sudo seam**:

- edit
- update
- non-admins cannot update

`test/controllers/accounts/logos_controller_test.rb` — **WS8br2**:

- show stock
- show stock small size
- show custom
- show custom small size
- show stock when custom logo cannot be resized
- destroy

`test/controllers/accounts/join_codes_controller_test.rb` — **WS8br2; WS9 sudo seam**:

- create new join code
- only administrators can create new join codes

`test/controllers/workspace_icons_controller_test.rb` — **WS8br2**:

- serves an SVG with the documented headers
- serves a PNG without the SVG-only headers
- supports conditional GETs with the blob checksum
- returns not found for unknown names
- returns not found for signed-out users
- unenrolled sessions are sent to setup instead of served the icon
- stale enrolled sessions are signed out instead of served the icon

`test/controllers/first_runs_controller_test.rb` — **WS8br2; WS9 session seam**:

- new is permitted when no other users exit
- new is not permitted when account exist
- create
- create is not vulnerable to race conditions

`test/controllers/welcome_controller_test.rb` — **WS8br2**:

- redirects to the first created visible room the user has access to
- redirects to the last room visited, if we have one

`test/controllers/users/avatars_controller_test.rb` — **WS8br2**:

- show image
- show initials when image cannot be resized

`test/controllers/public_pages_controller_test.rb` — **WS9 sign-in page integration**:

- sign-in page links the public pages without OAuth
- sign-in page keeps public links beside Google sign-in when configured

`test/controllers/pwa_controller_test.rb` — **WS8br2 service-worker event harness**:

- service worker fetch and notification logic

`test/system/first_run_tour_test.rb` — **WS8br2 with WS8b-m composer/room integration**:

- a new member is walked through the tour by keyboard and finishing persists
- escape skips the tour and it never auto-starts again
- the tour restarts from the help menu
- members who completed the tour never see it auto-start

`test/system/timezone_detection_test.rb` — **WS8br2**:

- the browser does not report its zone without a CSRF token
- the browser reports its detected zone once

`test/system/workspace_icons_test.rb` — **WS8br2**:

- upload post in both themes then delete falls back to the shortcode

`test/system/audit_log_test.rb` — **WS8br2; WS9 export sudo seam**:

- admin browses filters and exports the audit log
- audit log stays usable at phone width

`test/system/service_worker_test.rb` — **WS8br2**:

- the worker caches static assets and never authenticated responses
- the offline shell renders with working retry behavior

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

Deferred inventory: 194 named Rails controller/system cases. Existing upstream Rust tests do not constitute a re-diff or browser acceptance of these cases.

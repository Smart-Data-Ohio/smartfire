# WS8bm: messaging HTTP — partial delivery

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Reference: our Rails at `d7c7de92`; starting Rust base: `bb6c5d78`.
Implementation HEAD: `462a7595cd1fa43405dbb03087478ca04bbf89de`. The report is committed afterward as a documentation slice.

**Status: PARTIAL.** Two coherent implementation slices are pushed: root-message authorization and Markdown preview (`0fa03758`), then root-message create assignments/retries/error responses and Markdown message fragments (implementation HEAD above). This is not completion of the WS8bm brief and is not ready for cutover. The full seeded app binary has the common brief's known WS11 bot-key failure. No other failure remains in the final run. Clippy is clean.

## What changed, by file

- `rust/crates/campfire/src/controllers.rs`: registers `messages#preview` in the existing Rails-derived router.
- `rust/crates/campfire/src/controllers/messages.rs`: root endpoints require a membership in an alive room and use `Timeline::Room`, which excludes thread messages. Edits require the author, including when the other user is an administrator. System notes reject both editing and deletion. Ordinary deletion allows the author or an administrator. The old shared bot authorization helper remains separate.
- The same controller adds preview using WS8a's Markdown-body generation and WS5's sanitized presentation. Preview has the Rails 50,000-character boundary and over-limit JSON error, and performs no writes. Existing authentication and CSRF before-actions apply.
- Root create checks the raw client id's presence, then looks for the duplicate before validating/staging a new payload. A retry returns the original and repeats no save, queue enqueue, immediate broadcast or webhook release. It passes Markdown source, resolved root reply id, reply notification flag and normalized Drive ids into existing `NewMessage`. Non-null Markdown source takes precedence over legacy body. Active Record string-column boolean casts are `t`/`f`; the raw false id remains blank for the retry check. Drive arrays retain first occurrence order after Ruby-style ASCII strip, reject invalid shapes/ids, and use the Rails validation error shape. Record-invalid replies are JSON 422 or empty 422 by format; inaccessible create/reply targets retain the room-not-found response.
- `rust/crates/campfire/src/rich_text.rs`: exposes an application adapter to WS5's existing Markdown presentation, using the existing icon catalog and DB-backed mention resolver.
- `rust/crates/campfire/src/controllers/presenters.rs`: selects Markdown presentation for Markdown and forwarded-Markdown messages, after the existing attachment and sound branches. Legacy text keeps its original path. No room shell or layout change.
- `rust/crates/richtext/src/markdown.rs`: coalesces adjacent pending text before shortcode expansion around unresolved mentions. Nokogiri presents `:smile: @[Nobody]` with its separating space; the old Rust node-by-node expansion lost it. This is a narrow WS5 presentation seam.
- `rust/crates/campfire/src/controllers/messages/http_tests.rs`: 15 seeded request/fragment tests; the seed is mandatory, with an explicit failure when absent. They cover authorization, preview differential/CSRF/no-write behavior, real-record fragment parity/cache hits, Markdown create/retry semantics, scalar casts, Drive create validation/normalization, replies and rollback when the transactional job insertion is rejected.
- `rust/reference-tools/messaging/preview.rb` and `rust/vectors/messaging/preview.json`: eight real Rails HTTP previews, six real invalid-create HTTP responses, and eight actual Active Record scalar-column assignments. The HTTP create test exercises the six nonblank/non-null scalar probes. Nil/empty-source probes are recorded but not claimed as positive create coverage.
- `rust/reference-tools/messaging/fragments.rb` and `rust/vectors/messaging/fragments.json`: five real Rails messages created over the default seed, with whole message partials rendered as David and JZ through one Rails fragment cache. Cases: Markdown heading/bold, table/task/mention/catalog icon, code/unresolved mention, immutable system note, forwarded Markdown table. Rust creates the corresponding real domain records and compares the whole detached cached fragment bytes twice against those viewer outputs. This is detached render/cache-hit coverage, not two authenticated Rust browser sessions.
- `rust/reference-tools/messaging/discriminate.py`: 16 sequential compiled regressions. It requires the intended named test to fail, records each raw summary, and restores each source in `finally`; a compiler/setup failure cannot count as a detected regression.
- `rust/reference-tools/messaging/reference-check.py`: verifies 11 controller/concern/helper/model/template/catalog files in the borrowed reference image against Git's accepted pin, including exact file-set equality. Its two injected source-byte/file-set differences must be rejected.

## Design and boundaries

Controllers authorize and assign request attributes, then call the existing WS8a model API. Markdown domain generation remains in the application's `RichText` implementation; rendering remains in the presenter/WS5 adapter. The create path retains existing Active Storage staging/processing and the existing WS7 immediate broadcaster; attachment processing was not newly ported or exhaustively verified here. The SQLite trigger test confirms the request rolls back the message when an existing durable callback cannot enqueue its job. No production data is used.

The fragment vector contains normal Rails cache output. The generator does not override token helpers. It asserts no authenticity-token input or non-empty CSP nonce. Rust also asserts its detached fragment has neither a token nor a nonce attribute. The existing WS7 guard was not changed or weakened. No allowlist, mask, fixture authentication key, domain schema or Rails source was changed.

Strong-parameter and Ruby coercion parity is only proven for the reported probes. The malformed scalar/array message guard preserves Ruby's failed `dig` shape, but there is no exhaustive odd-shape differential for every create attribute. Valid agent-credential rejection remains an integration gap for WS11; applying the existing default before-actions is not claimed as proof of all agent auth cases.

## Verification commands and raw output

All commands below were executed in this worktree during the final verification pass, with scratch and target paths local to this worker. Cargo uses Rust 1.98.1, `--locked`, `-j 4`, and no release build. Docker names use `ws8bm-`; tests bind in the assigned 52000–52099 range. The borrowed image is `triage-reference-d7c7de92`; its relevant Rails source bytes were independently checked.

### Reference source identity

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-check-final.log 2>&1
```
```text
WS8bm reference source check: 11 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
```

### Seed rebuild (not a no-seed pass)

```sh
PARITY_NAMESPACE=ws8bm PARITY_OWNER=ws8bm PARITY_CPUS=2 PARITY_IMAGE=triage-reference-d7c7de92 TMPDIR="$PWD/.scratch" bash rust/parity/bin/seed build default first_run > .scratch/seeds-final.log 2>&1
```
```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

### Regenerate real Rails preview/create/cast oracle and compare committed bytes

```sh
mkdir -p .scratch/final-goldens && cp rust/parity/.seed/default/db/production.sqlite3 .scratch/preview-reference/db/production.sqlite3 && docker run --rm --cpus 2 --name ws8bm-preview-oracle --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/.scratch/preview-reference/db:/rails/storage/db" -v "$PWD/.scratch/preview-reference/storage:/rails/storage/files" -v "$PWD/.scratch/final-goldens:/out" -v "$PWD/rust:/work:ro" triage-reference-d7c7de92 bash -c 'bin/rails runner /work/reference-tools/messaging/preview.rb /out/preview.json' > .scratch/preview-oracle-final.log 2>&1 && cmp .scratch/final-goldens/preview.json rust/vectors/messaging/preview.json
```
```text
WS8bm preview oracle: 8 real Rails HTTP responses; 0 messages written
WS8bm invalid-create oracle: 6 real Rails HTTP responses; 0 messages written
WS8bm scalar-cast oracle: 8 actual Rails model assignments
```
The command and `cmp` returned 0; `cmp` emitted no output. The reference DB/storage directories are worker scratch copies, prepared from WS19's seed.

### Regenerate real Rails message fragments and compare committed bytes

```sh
cp rust/parity/.seed/default/db/production.sqlite3 .scratch/fragments-reference/db/production.sqlite3 && docker run --rm --cpus 2 --name ws8bm-fragments-oracle --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/.scratch/fragments-reference/db:/rails/storage/db" -v "$PWD/.scratch/fragments-reference/storage:/rails/storage/files" -v "$PWD/.scratch/final-goldens:/out" -v "$PWD/rust:/work:ro" triage-reference-d7c7de92 bash -c 'bin/rails runner /work/reference-tools/messaging/fragments.rb /out/fragments.json' > .scratch/fragments-oracle-final.log 2>&1 && cmp .scratch/final-goldens/fragments.json rust/vectors/messaging/fragments.json
```
```text
WS8bm fragment oracle: 5 real Rails messages; 2 viewers through one fragment cache; 0 session-bound values
```
The command and `cmp` returned 0; `cmp` emitted no output. No output normalization, whitespace masking or golden rewriting from Rust was used.

### Four Rails controller suites as reference validation

```sh
docker run --rm --cpus 2 --name ws8bm-rails-controllers --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -e RAILS_LOG_LEVEL=warn -v "$PWD/test:/rails/test:ro" triage-reference-d7c7de92 -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails test test/controllers/messages_controller_test.rb test/controllers/messages_drive_attachments_test.rb test/controllers/messages/cached_fragment_csrf_test.rb test/controllers/messages/legacy_presentation_cache_test.rb' > .scratch/rails-controllers-final.log 2>&1
```
```text
81 runs, 419 assertions, 0 failures, 0 errors, 0 skips
```
The production runtime image omits controller test sources, so the repository test tree was mounted read-only. These 81 Rails tests (56 + 19 + 4 + 2) are reference validation, not a claim that 81 Rust equivalents were ported.

### New seeded Rust tests

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::messages::http_tests -- --test-threads=4 --nocapture > .scratch/http-final.log 2>&1
```
```text
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 305 filtered out; finished in 0.84s
```
15 executed, zero ignored, zero missing-seed skips; 305 filtered by this selection.

### Full seeded app binary

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```
```text
test result: FAILED. 317 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 26.30s
```
318 executed (317 pass, one fail), two explicit inherited ignores. The ignores are `channels::tests::golden::record_reference` (reference-vector recorder, requires running reference) and `jobs::tests::push_latency` (optional measurement). No missing-seed skip occurred. The sole final failure is `controllers::presenters::accounts::tests::manages_bots`, the common brief's known WS11 failure: the old Bender key is still in the account-bots view after a reset. No test was excluded to hide that failure. This is the full app binary, not a complete workspace test run.

### Entire affected richtext package (81 tests plus empty doctest target)

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_richtext -- --test-threads=4 --nocapture > .scratch/richtext-final.log 2>&1
```
```text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.85s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.09s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
81 executed, zero failed, zero ignored; no missing-seed skips.

### Existing core view differential suite

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views --test core -- --test-threads=4 --nocapture > .scratch/views-core-final.log 2>&1
```
```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```
28 executed, zero failed/ignored. This is the core view test target only.

### Compiled negative controls

```sh
python3 rust/reference-tools/messaging/discriminate.py > .scratch/discrimination-final.log 2>&1
```
```text
author-edit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.42s
immutable-notes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.41s
member-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.42s
member-delete: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.41s
deleted-room: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.46s
root-thread-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.57s
preview-sanitization: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.41s
preview-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.36s
create-markdown: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.38s
create-dedup: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.40s
scalar-columns: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.41s
raw-retry-blank: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.67s
create-drive: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.45s
create-job-atomicity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.39s
markdown-fragments: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.09s
mention-shortcode-whitespace: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.40s
WS8bm discrimination: 16 compiled regressions detected; sources restored
```
Each probe rebuilt and ran exactly one intended HTTP/fragment test, which failed at runtime. The script restored every mutated source. Per-probe detail is under `.scratch/messaging-discrimination/`. No compile failure is counted.

### Locked metadata and all-target workspace clippy

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > .scratch/metadata-final.json
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_BUILD_JOBS=4 CABLE_TEST_PORT_RANGE=52000-52049 MAIL_TEST_PORT_RANGE=52000-52049 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```
Metadata returned 0 and wrote valid workspace metadata to the scratch JSON file. Clippy returned 0; raw summary:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.77s
```
`git diff --check` also returned 0 without output. No lockfile change.

## Extra timing failure and fresh-main comparison

An earlier full app run also failed `integrations::opengraph::document::tests::parses_pathological_pages_quickly`: 1.120343041s exceeded its 1s timing assertion. I did not label it inherited without checking. A fresh Git archive of `origin/main` at `21a7332f2d3c324f0862cdf448baf17a84395aa0`, built into a separate scratch target, passed the exact timing test. The parser file was byte-identical to this branch. The final branch app run passed it. This supports an intermittent load/timing explanation, not proof that main failed the same test.

Commands actually used for that comparison:
```sh
mkdir -p .scratch/main-baseline && git archive origin/main rust | tar -xf - -C .scratch/main-baseline
CAMPFIRE_REFERENCE="$PWD" TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/.scratch/main-baseline/target" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/main-baseline/rust/Cargo.toml -p campfire --bin campfire integrations::opengraph::document::tests::parses_pathological_pages_quickly -- --exact --nocapture > .scratch/main-timing-baseline.log 2>&1
git show origin/main:rust/crates/campfire/src/integrations/opengraph/document.rs > .scratch/main-opengraph-document.rs && cmp .scratch/main-opengraph-document.rs rust/crates/campfire/src/integrations/opengraph/document.rs
```
Archive creation and `cmp` returned 0 without output; isolated baseline raw test summary:
```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 308 filtered out; finished in 0.98s
```
This diagnostic snapshot was not merged into the worker branch. No lead instruction to merge main was received.

## Failing-first evidence

The initial compiled authorization/preview selection failed author/admin editing, note immutability, deleted-room access, root-versus-thread access and the missing preview route/contracts before their fixes. Removed/non-member access already passed inherited membership checks. Exact historical summary from `.scratch/auth-preview-before.log`:
```text
test result: FAILED. 2 passed; 6 failed; 0 ignored; 0 measured; 305 filtered out; finished in 0.58s
```
The real-record Markdown fragment differential failed before selecting Markdown presentation. Initial create tests failed lost Markdown/client id, lost replies/Drive ids and invalid-create responses; the atomic enqueue rollback test already passed the inherited WS8a transaction. Scalar probes then exposed the Rust `true`/`false` string casts. Historical summaries:
```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 313 filtered out; finished in 0.09s
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 314 filtered out; finished in 0.43s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.77s
```
These are retained pre-fix traces, not claimed as final passing commands. The final 16-probe run above repeats the discrimination with compiled regressions, including removal of CSRF, membership enforcement, root scoping, sanitization and transactional enqueue. It checks the actual intended runtime assertion, so a bad fixture or a compiler error cannot satisfy it.

## Rails test coverage and deferrals

Every owned controller test file below is accounted for. "Partial" means selected behavior is covered by the 15 new Rust tests and/or real Rails differentials; it does not mean the whole Rails file was ported. Deferred ownership stays with WS8bm unless a named adjacent workstream owns the feature.

| Rails controller test file under `test/controllers/` | Status and owner |
| --- | --- |
| `messages_controller_test.rb` | Partial, WS8bm: root Markdown create/retry, preview/limit/no-write/CSRF, author-only edits, non-author deletion denial, immutable notes and Markdown/system-note fragments. New additional guards cover removed/non-members, deleted room and root access to thread messages. Remaining successful show/index JSON, paging/ETag, edit/update, actions, tombstone/thread-refresh broadcasts, agent delivery and broader response tests deferred. |
| `messages_drive_attachments_test.rb` | Partial, WS8bm: create normalization/order/dedup and six invalid-create differential cases, including scalar Drive input, invalid id, Unicode-space-wrapped id and nested object. Update replacement/removal, viewer credentials/consent, JSON and edit-form cases deferred. The WS8a max-count/body-with-attachment model behavior was not newly claimed as request coverage. |
| `messages/cached_fragment_csrf_test.rb` | Partial, WS8bm/WS6: new five-message Rails two-viewer cache oracle and Rust detached cache-hit checks contain no session-bound values. Full authenticated HTTP warming/update/broadcast scenarios deferred. |
| `messages/legacy_presentation_cache_test.rb` | Partial, WS8bm/WS6: existing core views pass; this slice leaves the legacy path unchanged. Full HTTP legacy edit conversion/cache invalidation scenarios deferred. |
| `messages/boosts_controller_test.rb` | Deferred, WS8bm; bot subclass integration is WS11. |
| `messages/pins_controller_test.rb` | Deferred, WS8bm. |
| `message_forwards_controller_test.rb` | Deferred, WS8bm: reachable destinations, locked/stale threads, board refusal, direct display names, nested source routes and bounded queries. |
| `message_forward_sources_controller_test.rb` | Deferred, WS8bm: no-store canonical source URL and absence of inaccessible source identity. |
| `channel_threads_controller_test.rb` | Deferred, WS8bm. |
| `channel_thread_messages_controller_test.rb` | Deferred, WS8bm. |
| `channel_thread_messages_drive_attachments_test.rb` | Deferred, WS8bm. |
| `rooms/polls_controller_test.rb` | Deferred, WS8bm. |
| `rooms/pins_controller_test.rb` | Deferred, WS8bm. |
| `rooms/message_links_controller_test.rb` | Deferred, WS8bm. |
| `rooms/slash_commands_controller_test.rb` | Deferred, WS8bm; agent execution WS11 and huddle launch WS13 are integration seams. |
| `rooms/files_controller_test.rb` | Deferred, WS8bm; storage processor implementation stays WS storage. |
| `autocompletable/icons_controller_test.rb` | Deferred, WS8bm. Catalog use inside Markdown is covered; picker HTTP is not. |
| `autocompletable/slash_commands_controller_test.rb` | Deferred, WS8bm. |
| `autocompletable/users_controller_test.rb` | Deferred, WS8bm. |
| `saved_items_controller_test.rb` | Deferred, WS8bm; reminder notification handler WS17 and inbox policy WS12. |
| `scheduled_messages_controller_test.rb` | Deferred, WS8bm. |
| `searches_controller_test.rb` | Deferred, WS8bm. |
| `channel_threads_board_test.rb` | Deferred integration seam with WS12; board/work-thread feature is WS12, thread pane/message HTTP is WS8bm. |

No new browser/system test was ported or executed. Each owned system file is deferred to WS8bm: `boosting_messages_test.rb`, `code_highlighting_test.rb`, `pins_saved_test.rb`, `polls_test.rb`, `scheduled_messages_test.rb`, `search_forward_edit_test.rb`, `sending_messages_test.rb`, `slash_commands_test.rb`, `threads_test.rb`, `workspace_markdown_test.rb`. Additional matching system files are also deferred to WS8bm: `composer_test.rb`, `composer_attach_menu_test.rb`, `message_interactions_test.rb`, `message_actions_mobile_test.rb`, `message_toolbar_test.rb`, `message_list_a11y_test.rb`, `drive_attachments_test.rb`, `icons_test.rb`, `workspace_icons_test.rb`, `global_search_test.rb`, `search_files_test.rb`, `search_recents_clear_test.rb` and `unread_divider_test.rb`. Huddle/agent portions need WS13/WS11 coordination; Drive picker/consent portions need WS14. Message portions of `keyboard_shortcuts_test.rb`, `content_security_policy_test.rb`, `motion_test.rb`, `mobile_layout_test.rb` and `timezone_detection_test.rb` remain WS8bm integration coverage with WS8b-r/WS4. No claim is made for these mixed system files.

`messages/by_bots_controller_test.rb`, `messages/boosts/by_bots_controller_test.rb` and `agents/*` are WS11, not newly ported here. `work_threads*`, board controllers and `threads/work/*` remain WS12. The common room shell/sidebar/room CRUD and user/account pages are WS8b-r. The failing bots presenter remains WS11.

## Cross-workstream touches

- WS5: one adjacent-text/mention shortcode presentation fix, proven by real Rails preview/fragment bytes. No parser redesign or mask change.
- WS6: Markdown render-path selection in the application presenter and use of existing message templates/cache. No shell/layout edits. Full message view-model expansion remains WS8bm work.
- WS8a: existing `Message::create`, validations, reply and Drive associations and transactional callback jobs are used. No permanent WS8a DB/model changes; the enqueue mutation restores its source. Future explicit clearing of Markdown on legacy update may need a small `MessageChanges` seam.
- WS7: inherited immediate create broadcasting now gets the Markdown presenter; the broadcast guard and publisher are unchanged. WS8a's described deferred broadcasts are not newly registered here.
- WS11: the shared `set_message` helper now excludes thread messages; check human/bot subclass root scope while integrating modern bot authorization. Root create retains the existing webhook handoff. Agent authorization/delivery and the known bot-key test are not owned or solved by this slice.
- WS8b-r: no room shell/sidebar/template edit. The existing message partial now benefits from Markdown presentation.
- WS14/WS15e/WS15g/WS13/WS17: no integrations or external clients changed; their message-related UI and notification seams are still pending.

## Precise remaining work and restart point

The next coherent slice should complete root `messages#edit/update/show/actions/index` before expanding to threads. Root create's positive JSON/payload and all odd parameter/upload paths also still need complete Rails response parity. Specifically:

1. Modern edit/composer templates, legacy-to-Markdown inline attachment preservation, `edited_at` rules, explicit Markdown clearing on legacy saves, reply and Drive update assignment, successful JSON payloads/actions/source privacy, related/pin/presentation ETags, JSON no-store and removal of the inherited Last-Modified validator.
2. Root/thread paging tuple boundaries, before/after/around coercions, time-zone behavior, any required total-count headers, last-read/unread state; complete thread authorization/lifecycle/deleted-room/locked states, thread pane and indicators, and thread message HTTP.
3. Pins, boosts, room poll/message-link/file endpoints and partials, saved-item/reminder and scheduled-message rows/send-now HTTP. Poll open/voted/closed and pin badges/lists need real Rails fixtures.
4. Forward and forward-source picker/privacy endpoints, wiring `ForwarderCopier`, forwarded attachments and source visibility. Direct upload/variant/preview integration needs request/system parity beyond retaining the inherited storage path.
5. Search preloads, operators, cursor, DST/date coercions, results/operator chips; slash HTTP/pickers, icon/user autocomplete and `/play` presentation. Coordinate agent calls with WS11 and huddle launch with WS13.
6. Reply/forward context, reference/quote/card placeholders, attachment/Drive credential states, polls/pins/saves/threads and scheduled presentation models/cache keys. Existing small Markdown goldens do not cover these state combinations.
7. Register and render WS8a's append/replace/remove/tombstone/thread/poll/pin/quote/scheduled descriptions via WS7, matching partial bytes and ensuring guard acceptance. Inherited immediate create is the only broadcast path touched indirectly here.
8. Port the deferred controller/system files above, generate broader real Rails seeded rows, run browser pixel parity, and test valid agent credentials on human endpoints when the WS11 seam is ready. No production cutover, deployment or PR was attempted.

Open integration questions are the smallest explicit-clear API for Markdown updates, viewer-specific message payload/render preloads, modern MessageKey facts, deferred broadcast consumers and agent-auth ownership. None changes the accepted Rails behavior or the fixed architecture decisions.

## Delivery

The branch contains only Rust source/vector/tool/report changes plus untracked `.scratch/` verification artifacts. All implementation commits end with `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>`. No stash, rebase, PR, Rails edit, parity mask or destructive cleanup occurred. The requested external report and tracked `rust/plans/ws8bm-report.md` contain the same report text.

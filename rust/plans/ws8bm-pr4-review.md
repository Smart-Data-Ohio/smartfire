# WS8bm -4 after #230 review and main #231

Current controller coverage is **156/156**. Current system inventory is **133 passed / 2 deferred / 0 owner-blocked**, superseding the historical 134/1 below. Both branches merge main `566c1bd77` (including approved client fix #231), and round 4 carries all round-3 review corrections. The workspace assertion targets the actual committed mobile message with visible body text within two seconds. Motion starts in HQ and its negative executes the original native off-canvas assertion. [The correction report](ws8bm-review230.md) records the original-line audit and failing-first escapes.

The only current deferrals are:

| Declaration | Exact reason retained | Pin |
| --- | --- | --- |
| Mobile drawer keeps room-list scroll position | The earlier Rust negative failed while establishing scrollTop=400, before the intended closed-offset assertion. A new paired intended failure does not explain that setup race; it stays deferred. | motion_test.rb:177-238 |
| Mobile drawer reopens on current room already in view | The earlier Rails negative failed initial current-link focus, before reopen. The later unchanged-path success is not a race fix or closure proof. | motion_test.rb:260-292 |

The Markdown reply/file-upload declaration is now passed: main #231 prevents late upload progress/failure callbacks from overwriting delivered messages. This was a shared client race, not a Rust server defect. The exact approved client module is compiled into the pinned reference host, while Rust serves main's revendored assets. No app/ file is edited by this review correction. Original deadlines, predicate scopes and byte checks remain intact.

The following earlier report and broad-run failures are retained as historical evidence; its upload deferral and 134/1 inventory are superseded. The merged-source checkpoint and raw receipts follow at the end.

# Historical WS8bm -4: Drive transport and actual test layouts

Stacked branch `rust/ws8bm-messages-http-4`, from `origin/rust/ws8bm-messages-http-3` at `69d9e3b9d7c8dca348df7b872a5b43427e73f4ac`. The -3 branch is unchanged. Rails remains pinned to `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift. **156/156 controller declarations; 134 passed / 1 deferred / 0 owner-blocked system declarations.** Four Drive declarations and the test-environment motion declaration close. The upload declaration remains deferred after reproducing its original 10-second preview failure on Rust in the literal pinned Capybara body, after repairing the Rails test host.

The list/composer entry points and caller inputs remain [ws8bm-integration.md](ws8bm-integration.md). No Rails JavaScript, response golden, mask, ignore, deadline, retry limit or machine throttle changes. Duplicate-delivery coverage still injects browser Turbo markup and does not verify server-originated redelivery. No pixel work is included.

## Transport and fixture boundary

Rails materializes `GoogleCalendarTestHelper` directly from the pin and installs its original `stub_google_drive_list` / `stub_google_drive_file` WebMock calls in the live host. The connected JZ account uses the original scopes, access/refresh tokens and expiry. The two-file edit retains the original helper's second file payload, including its ID; expectations are not corrected to a different payload. The thread case creates and joins the same Designers / JZ / Drive thread fixture. Kevin remains disconnected.

Rust's generated `cfg(test)` browser host injects the already-existing `google::client::Client` through `Api::new` / `install_api`. That client forwards real outbound list/file requests to a held local HTTP fake, which validates the original host and WebMock query conditions and returns the same pinned payloads. Picker routes, selection, upload metadata, composer writes, authorization, database transactions and rendered messages remain real. Headers are forwarded unchanged; like the original stubs, the fake does not add an Authorization-header match. A real-HTTP regression verifies that matching request without that header still receives the original response. No production configuration, endpoint or transport path was added for this fake. Its listener closes through an `ExitStack` even when browser diagnostics or app teardown fail; the real-HTTP helper test injects a diagnostic exception and verifies the listener is closed.

All six new cases now boot **actual `RAILS_ENV=test` hosts**, with cloned identical test databases. Jobs match pinned Rails `ActiveJob::TestAdapter` with `perform_enqueued_jobs=nil`; Rust uses `TestApp::without_job_runner()`. The upload host also enables the original test's forgery protection. Rails' test layout opts out of automatic service-worker registration, avoiding the production worker claiming the page during the picker flow. Rust now matches that helper: registration is off in tests unless the original `enable_service_worker` cookie is present/nonblank. Production layout expectations are untouched.

The production reference image also omits `test/support/test_session_controller.rb`, required by the pinned `config/routes.rb:371-375` in the test environment. Rails partially drew application routes before that missing-file error prevented engine routes from finishing. Upload views rescued `undefined method rails_blob_path` and rendered an empty attachment. The browser image now restores that controller **byte for byte from the pin**, and both live/native hosts forward the caller's existing `CI` flag (`config/environments/test.rb:16`). The controller still checks the real password and refuses non-test environments. A source-authenticity test and an actual test boot probe verify the restoration. This was a harness defect, distinct from the late progress callback described below; replayed results after its fix are identified separately.

## Original scopes and mutation targets

All visible lookups, actions, counts and text use the pinned Selenium 4.35.0 atom and Capybara visible-text semantics. The default is two seconds, with only the original explicit deadlines. Hidden-all inputs/context queries remain DOM-based. Geometry is read only where the original uses `evaluate_script`.

| Declaration/check | Pinned original | Restored or preserved assertion/action | Intended served fault |
| --- | --- | --- | --- |
| Legacy picker | composer_attach_menu_test.rb:34-44 | Actual visible +/From Google Drive clicks; visible Q3 Planning picker item, 2 s; no message/blob writes | Item opacity zero; the item assertion itself |
| Textless send and standalone edit removal | drive_attachments_test.rb:10-76,143-150,165-173 | Visible picker option/attach siblings and negative nested button; hover/attach two chips; minimum 24 px remove control; actual remove/send; visible name/link and original immediate Message.last identity/file IDs; actual edit, literal remove ARIA label, saved body 10 s before checking absent links/rows | Delivered name opacity zero; initial Q3 Planning name assertion |
| Root composer edit/remove one | drive_attachments_test.rb:79-113 | Actual two selections/send; two visible links and names; immediate Message.last DOM key, right-click/menu/edit; context 10 s; actual remove/send; one visible exact link, hidden context DOM query, exact retained file ID | Delivered name opacity zero; initial Q3 Planning name assertion |
| Thread Drive attachment | drive_attachments_test.rb:116-140,151-173 | Actual joined deep link; conversation/button/settled transform/dialog/item/link each original 10 s; hover/attach/field/send; immediate thread's last message and saved file IDs | Thread attachment link opacity zero; delivered link assertion |
| Test motion default | motion_test.rb:19-23; application.html.erb:2 | Actual server-emitted off attribute and all three 0ms tokens; no attribute injection; full cable setup matches original | Real server off attribute removed by served connect; exact attribute assertion and before/after witness |
| Upload preview | workspace_markdown_test.rb:165 | Previously selected a containing row and required its preview to remain during the later filename check. Restore the asserted preview element's global selector and visible text directly, 10 s | Constructor discards an actual nonnull reply ID; causal witness now mandatory, so natural upload failures cannot borrow credit |
| Upload filename | workspace_markdown_test.rb:166; system_test_helper.rb:122-124 | Previously conjoined preview with filename and required extra recipient/download assertions. Restore the independent global visible message-body filename, 2 s, on the original author session | Existing filename-opacity fault; earlier preview failures remain invalid |
| Upload positive sequence | workspace_markdown_test.rb:143-173 | Literal original Capybara 3.40.0 / Selenium 4.35.0 body, all actions/assertions/deadlines intact. Only the file path is adapted to a private path shared by Ruby and host ChromeDriver. Original filename and 47-byte contents retained | Served negatives keep the matching translated original element scopes |

The Drive readbacks require exactly one new global message, all original IDs preserved, JZ/Designers and exact thread identity, exact saved source and exact final attachment set. The original intermediate `Message.last` / thread-last lookups occur immediately, without SQL polling. Legacy picker and motion readbacks require unchanged messages, threads and identities. Browser optimism alone is insufficient.

## Failing-first motion difference

The actual Rails test host emits `data-test-motion="off"`; the original Rust test host omitted it. The unchanged paired check fails only Rust before the layout fix (`motion-before.log`):

```text
WS8bm positive application FAILED: Rust: motion is off by default in the test environment:
AssertionError [ERR_ASSERTION]: motion: server test attribute
undefined !== 'off'
WS8bm behaviour check: 0 named cases passed on Rails and Rust; 1 failed; no pixel checks
```

Afterward both apps pass, including the three actual computed tokens. The served removal fault rejects at that same assertion on both apps. A views regression asserts the exact pinned test-environment HTML opening line; production golden tests are not modified. The causal unit test refuses credit when the server attribute never existed.

## Exact remaining deferral: pinned upload progress overwrites a delivered row

The unchanged filename fault at the -3 boundary can fail early at the preview and is invalid, not rejected at its intended assertion. The baseline run (`attachment-before.log`) saved the correct parent, reply, attachment and unchecked notification flag while the delivered DOM lost its body/preview. Both real POSTs return 200. Diagnostic DOM/progress traces subsequently show the committed row replaced by `markdown-workspace-attachment.txt - 100%`. Early actual-test-host attempts also encountered the separate missing Rails engine-route dependency above; those attempts cannot establish an application difference.

The shared pinned path is `composer_controller.js:331-333` -> `messages_controller.js:178-180` -> `models/client_message.js:38-43`: a late upload progress callback replaces `.message__body-content` for the client ID even after that ID belongs to a delivered message. This can erase both the reply preview and attachment presentation. Saved reply/attachment fields and successful server rendering match; the observed failure is a delivered row overwritten by client progress. The clean repaired-host samples establish the preview failure on Rust; they do not claim a natural Rails failure after the harness repair. The same unsafe client update function exists at the pin on both hosts. The files match the Rails pin and are not changed here.

The literal original Capybara/Selenium body reproduces the original Rust preview failure too. After restoring Rails engine routes, Rails passes all thirteen original assertions in the three samples; Rust passes those assertions once and fails twice. The passing original pair then exposed the independent test-disk readback path error, now fixed. A final paired original control also passes exact rows and real stored bytes; these successes do not erase the earlier UI failures. Setup uses the original test viewport/theme, all cable subscriptions and actual test layout; read-only Rails models perform the original `Message.find_by!` and attachment queries against each live host's database. Remote ChromeDriver and Ruby share the upload path, so its detector declines Grid transfer and performs the same native file-input action as the original local driver. Original ActiveSupport assertions are loaded. Neither a response drain, a progress wait nor a longer deadline is added. Native SQL/DOM diagnostics are protected so they cannot skip session/container/driver cleanup.

This is a possible shared Rails client bug requiring lead disposition or a deterministic original sequence. It remains **one own deferral**, not an owner block or a headless API impossibility. Passing individual attempts and successful diagnostic probes do not promote it. A trace-enabled run is diagnostic only and receives no declaration credit.

## Verification

Code verification runs from no-hardlinks fresh clone `.scratch/ws8bm-pr4/verification`. The full positive set started at `500f26bf9`; protected diagnostic readbacks and the exact fake-header predicate arrived at `192080cee`. The restored Rails test boot is `a5e499f71`; the exact motion setup viewport is `710d15bb3`. Final affected replays and Rust checks use the latter source. The subsequent readback-only fix is `3bd42229d`; final upload replays and helpers use that source. No Rust product input changes after the workspace/lint/build run. The initial broad receipts are preserved rather than relabelled as a run after those harness repairs. Rust product inputs are identical from `744d47a51` throughout. Seeds and browser dependencies are recreated from tracked inputs. Cargo reuses only compilation cache in the owned scratch target; no test reads a pre-existing scratch fixture or target file. Private canonical media is extracted from the pinned image: libvips 8.16.1 / FFmpeg 7.1.5. Tests use at most eight threads, compiler jobs remain two, and discrimination retries are explicitly one (one attempt; no automatic retry).

The first fresh-clone Node helper invocation preceded `npm ci` and failed with `Cannot find module 'playwright'`; its raw log is retained as `node-before-npm.log`. The final helper invocation follows tracked dependency installation. This was setup sequencing, not an application or parity result.

The first broad-run setup also exported the private media library path to the whole harness, which made the host `curl` resolve an incompatible `libcurl` (`undefined symbol: curl_multi_notify_enable`) and fail readiness before case assertions. Those interrupted logs remain `all-controls-media-env-aborted.log` and `all-mutants-media-env-aborted.log`. The correction removes that global library override: only Cargo's canonical test runner and its media subprocesses receive the private libraries. The completed runs below use the corrected environment.

Final raw receipts and broad-run failures follow below.

### Fresh-clone Rust verification

After `bash rust/parity/bin/seed build default first_run agents_ui`, each command below ran in the no-hardlinks verification clone at `710d15bb3`, with `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, `CI=1`, the private canonical test runner and its owned `CARGO_TARGET_DIR`. `html5ever` is the existing shared CI vendor exclusion. No exclusion or ignore was added.

```sh
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

All 59 raw Cargo target summaries (including empty/doc targets):

```text
test result: ok. 2718 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 475.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.17s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.00s
test result: ok. 1359 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 79.87s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.39s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.65s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
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
```

Summing those summaries gives **4,807 passed / 0 failed / 15 existing ignores**. The seeded Campfire target is 2,718 passed / 0 failed / 6 existing ignores. Strict clippy and the release-input build raw completion lines, respectively:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 07s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 10s
WS8bm workspace exit: 0
WS8bm clippy exit: 0
WS8bm release inputs exit: 0
```

The actual runner libraries were checked again with `ctypes.CDLL("libvips.so.42").vips_version_string()` and the extracted `ffmpeg -version`; only those processes receive the private library path:

```text
WS8bm canonical libvips: 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

### Final helper checks and input audit

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*test.py'
node --test --test-concurrency=2 rust/reference-tools/messaging/*.test.mjs
mise exec rust@1.98.1 -- cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 >/dev/null
```

Raw helper summaries at `62e4b4d60` (the final change closes the expected HTTPError response in the new fake-transport unit test):

```text
Ran 32 tests in 2.355s
OK
ℹ tests 59
ℹ suites 0
ℹ pass 59
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
```

`tomllib.loads` parses the whole workspace manifest and therefore rejects any duplicate keys. The added-line audit of `git diff --unified=0 69d9e3b9..HEAD -- rust/crates` finds no new include macro at all; neither new product code nor its regression introduces a dependency outside `crates/`:

```text
WS8bm workspace dependency keys: unique (78 keys; TOML duplicate-key parse passed)
WS8bm added external source includes: 0 (no new include_str! or include_bytes! in crates)
```

The before/after Rails boot probe (`RAILS_ENV=test`, also tested with `CI=1`) uses `bundle exec rails runner` and evaluates `Rails.application.routes.url_helpers.respond_to?(:rails_blob_path)`. Before the restored controller it fails while drawing routes; after restoration it passes:

```text
/rails/config/routes.rb:372:in 'Kernel#require_relative': cannot load such file -- /rails/test/support/test_session_controller (LoadError)
WS8bm pinned test engine routes: true
```

### Completed broad browser receipts and honest failures

From the verification clone, with `WS8BM_DISCRIMINATION_RETRIES=1` (one attempt, no retries), `WS8BM_BROWSER_PORT_BASE=22020`, `CI=1` and the same compilation cache:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
```

Raw summaries:

```text
WS8bm behaviour check: 142 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm invalid discrimination attempts: 3; bounded fresh-fixture retries only
WS8bm discrimination check: 198 served mutants rejected on Rails and Rust across 134 named checks; 3 invalid or escaped
```

There were **zero escapes**, three invalid attempts and zero automatic retries. The broad commands exit 1 and remain reported as such. The sole positive failure is the upload declaration. The three negative invalids are:

| Case / variant | Actual failure before its intended target | Credit |
| --- | --- | --- |
| Mobile drawer animation / default | Rails `behavior-motion.mjs:74` attached-listener count was not 2, before the off-canvas assertion | Invalid; inherited prerequisite timing, not fixed or retried here |
| Profile button names / default | Rails composer Stimulus startup timed out at 15 s before visiting the mutated profile assertion | Invalid; inherited startup timing, not fixed or retried here |
| Upload / transparent-attachment-filename | Rails had no attachment filename node for the mutation to encounter, under the incomplete test routes above | Invalid; never counted as rejection; repaired-host replay follows |

All observed assertion failures preserve their precise target/phase checks and causal mutation witnesses. The completed broad tests preceded the test-route repair; final affected controls and mutations below replay that repair, rather than claiming the earlier broad log was green.

### Final affected replays

These execute on the repaired actual test hosts from the fresh clone (`710d15bb3` for the first repaired-host set; `3bd42229d` after the readback fix). All writing positives verify real saved rows; the final upload positive also verifies both actual 47-byte disk files.

```sh
python3 rust/reference-tools/messaging/behavior-check.py drive_attachments --keep-going
python3 rust/reference-tools/messaging/behavior-check.py drive_attachments --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer_attach_menu --case 'From Google Drive starts the legacy picker flow' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer_attach_menu --case 'From Google Drive starts the legacy picker flow' --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'motion is off by default in the test environment' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'motion is off by default in the test environment' --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'Markdown replies and file attachments remain usable' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'Markdown replies and file attachments remain usable' --negative --keep-going
```

The first six commands pass with **five paired declarations and five intended paired rejections**. The upload positive ran in three preplanned fresh-fixture samples; all outcomes are retained, not retried/selected for a pass. The later changed-code invocation verifies the corrected disk readback. The preceding four-fault set was valid; the final set has an invalid early preview failure, so no consistent upload closure is claimed.

Raw summaries, with their receipt labels:

```text
Drive attachment controls:
WS8bm behaviour check: 3 named cases passed on Rails and Rust; 0 failed; no pixel checks
Drive attachment mutants:
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 3 served mutants rejected on Rails and Rust across 3 named checks; 0 invalid or escaped
Legacy picker control:
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
Legacy picker mutant:
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
Test motion control:
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
Test motion mutant:
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
Repaired-host upload mutants before the readback-only fix:
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 4 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
Final upload control with actual disk bytes:
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
Final upload mutants; invalid retained:
WS8bm invalid discrimination attempts: 1; bounded fresh-fixture retries only
WS8bm discrimination check: 3 served mutants rejected on Rails and Rust across 1 named checks; 1 invalid or escaped
```

The literal pinned upload samples after the route repair:

| Sample | Rails original assertions | Rust original assertions | Subsequent byte readback |
| --- | --- | --- | --- |
| 1 | 13 / 0 failures | 9 / 1 failure at workspace_markdown_test.rb:165 | UI failure; original row and blob diagnostics retained |
| 2 | 13 / 0 failures | 13 / 0 failures | Harness looked in the production storage mount for the test-service file; FileNotFoundError, unconditional teardown still ran |
| 3 | 13 / 0 failures | 9 / 1 failure at workspace_markdown_test.rb:165 | UI failure; original row and blob diagnostics retained |
| After disk-readback fix | 13 / 0 failures | 13 / 0 failures | Exact saved parent/attachment/notify=false and actual 47-byte files pass on both hosts |

Failing-first sample 2 shows the independent readback defect at `710d15bb3`. The fix adds `blobs.service_name` to the unchanged attachment lookup and reads Rails' actual `test` disk through `docker exec ... cat /rails/tmp/storage/<key-prefix>/<key>`, matching pinned `config/storage.yml:1-3`. Rust's actual local disk is read directly. Neither the byte-size check nor exact byte comparison is removed. Missing files still propagate failure; the existing outer finally always shuts down both servers. The added helper tests cover both disk paths and refusal to fabricate a missing file. The passing live final control verifies the corrected path, without turning the earlier UI failures into successes.

At `3bd42229d` the final filename variant is **invalid on Rust**: it fails at the earlier `behavior.mjs:483` preview assertion (10,000 ms), while its intended filename assertion is at :491 (2,000 ms). Even though hidden filename nodes were observed, the phase mismatch denies credit. Its companion Rails rejection remains recorded. No escape occurred, and no retry followed.

The diagnostic delivered Rust row contains `markdown-workspace-attachment.txt - 100%` and no reply preview despite its saved parent ID, false notify flag and real attachment. The upload client files were compared byte-for-byte with `git show d7c7de92:<path>`:

```text
WS8bm pinned upload client: app/javascript/controllers/composer_controller.js SHA256 64753cf574935829f58ac23e62dd92c7dc7a9b1865202591bf38c853693e7aba identical
WS8bm pinned upload client: app/javascript/controllers/messages_controller.js SHA256 afe56b096047e8e367df23aeb6ef607819a41929e16330416ca2083fe428187b identical
WS8bm pinned upload client: app/javascript/models/client_message.js SHA256 73452e0f3f6686249128f344842b7cf4838c0c19ae3fdca5b3ae2d356cfdc7fb identical
```

One own declaration therefore remains deferred: the delivered-row progress overwrite needs a shared-client disposition or a deterministic **original** sequence. A passing later sample and successful mutation runs do not erase the reproducible original deadline failures. No delay is added to Rust responses, no client source is changed, and no deadline is widened.

### Inventory and cleanup

`python3 rust/reference-tools/messaging/deferred-system-inventory.py` verifies every original name and source hash:

```text
WS8bm system inventory: 135 named declarations; 134 mapped behaviour passes; 1 deferred; 0 WS12 blocked; no pixel checks
```

Controller inventory remains 156/156. Only the one exact upload follow-up above remains; no owner-blocked declaration is reclassified or hidden.

After all test and browser processes exited, the owned compilation target was cleaned:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path .scratch/ws8bm-pr4/verification/rust/Cargo.toml --target-dir .scratch/ws8bm-pr4/fresh/rust/target
```

```text
     Removed 24651 files, 31.4GiB total
```

The first resource audit found only a 44 KiB generated-output target in the verification clone, with no processes, listeners or containers. It contained exactly the five files below. Cargo refused to clean that directory without a `CACHEDIR.TAG`; the refusal is preserved in `cleanup-generated.log`. After verifying the exact file set and file types, `shutil.rmtree` removed only that regenerable output directory. Final raw cleanup/resource receipts:

```text
WS8bm generated target cleanup: removed 5 generated files; 34076 bytes; .rustc_info.json, campfire_session_keys_rust_output.json, kit_security_rust_output.json, rails_compat_rust_output.json, rails_compat_smartfire_rust_output.json
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "scratch_targets": []}
```

No test process, owned listener, container or scratch target remains. The model server was not touched. Logs stay in the owned `.scratch/ws8bm-pr4/` directory; generated fixtures/targets are not required to rerun any check. The branch stops at this partial pushed checkpoint.

## Merged-source #230 checkpoint

Both branches contain main `566c1bd77`; round 3's pushed review/verification head is `1f3678ed5`, merged into round 4 with merge commits. The runtime replay uses the no-hardlinks fresh clone at `a8f5409b6`. Later commits carry only docs/deferral wording: crates/ and reference-tool sources are unchanged. `run4.sh` runs these eight affected declarations on both real hosts:

- Original native HQ drawer animation, and actual test-environment motion default.
- Workspace theme/navigation and actual saved mobile message.
- Original native Markdown reply/file attachment body (13 assertions per host; original preview wait 10 s).
- Legacy Drive picker, textless two-file send/edit removal, root two-file edit/remove-one, and thread Drive attachment.

Commands are `python3 rust/reference-tools/messaging/behavior-check.py motion workspace_markdown composer_attach_menu drive_attachments --keep-going`, restricted by exact case names to the eight listed declarations using the runner's `--exclude-case` selector, followed by the identical selection with `--negative`. All applicable variants are scheduled (14); none is excluded. `WS8BM_DISCRIMINATION_RETRIES=1` means one attempt, with no automatic retry. Metadata/source/persistence witnesses and hardened intended-phase attribution remain mandatory. Native motion negatives run the unmodified pinned Capybara/Selenium body; filename negatives use the independent original two-second element scope, after the original ten-second reply-preview assertion. Both hosts retain real browser writes and exact stored bytes/row identities.

Raw receipts (`round4-positive.log`, `round4-negative.log`, `round4-summary.log`):

```text
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 13 assertions, 0 failures, 0 errors, 0 skips
1 runs, 13 assertions, 0 failures, 0 errors, 0 skips
WS8bm behaviour check: 8 named cases passed on Rails and Rust; 0 failed; no pixel checks
1 runs, 3 assertions, 1 failures, 0 errors, 0 skips
1 runs, 3 assertions, 1 failures, 0 errors, 0 skips
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 14 served mutants rejected on Rails and Rust across 8 named checks; 0 invalid or escaped
round-4 positive exit: 0
round-4 negative exit: 0
```

Fresh-clone helper commands:

```sh
node --test --test-concurrency=1 rust/reference-tools/messaging/*.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

Raw final helper summaries (`node4-sequential.log`, `python4-final.log`):

```text
ℹ tests 65
ℹ suites 0
ℹ pass 65
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 7332.426224
Ran 32 tests in 3.517s

OK
```

These final affected replays have zero failures, invalid proofs or escapes. They do not reclassify the earlier broad-run failures recorded above; the old upload failure is addressed by main #231 and re-proved here. The reviewed scroll/reopen failures remain two explicit deferrals, despite one newer intended-negative pair each. No diagnostic-only readiness wait, unchanged-path success or unrelated timeout supplies closure credit. The two new workspace probes demonstrably escape the exact `69d9e3b9d` assertion before correction, then fail its restored predicate/deadline. Main's client module is vendored byte-identically from `0373dfbd9` and compiled by Rails; native mutation transport forwards real HTTP writes and Cable bytes and always tears down.

### Fresh-clone merged round-4 Rust suite

`validate4.sh` runs from the same no-hardlinks fresh source clone at `a8f5409b6`, with `CAMPFIRE_REFERENCE` pointing to that clone, eight test threads, compiler jobs two and the existing machine-wide rustc throttle. It builds all three tracked seeds first. A single owned target is compilation cache only; no check reads a pre-existing target or untracked fixture. The canonical child runner supplies libvips 8.16.1 and ffmpeg 7.1.5 without a global library-path override.

```sh
bash rust/parity/bin/seed build default first_run agents_ui
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

All 59 raw workspace summaries (`workspace4.log`); totals **4,923 passed / 0 failed / 22 existing ignores**. The CI html5ever exclusion is unchanged; no new ignore or timing/concurrency change is included.

```text
test result: ok. 2813 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 550.34s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.09s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.00s
test result: ok. 1379 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 79.38s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.62s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.19s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.96s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
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
```

Strict lint and release-input command, run after the fresh-clone workspace suite:

```sh
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

Raw command summaries (`validation-summary4.log`, `clippy4.log`, `release4.log`):

```text
fresh seed exit: 0
fresh workspace exit: 0
strict clippy exit: 0
release inputs exit: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.00s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 11s
```

`cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 >/dev/null` passes; Python `tomllib` parses all 78 workspace dependency keys without duplicates on both merged branches. The newer crate diff adds no include_str!/include_bytes! expression; the release-input build confirms only Cargo.toml, Cargo.lock and crates/ are needed. Both `origin/main` and the pushed round-3 head are ancestors of round 4.

### Cleanup and stopping point

Measured owned output: 33G compiler target and 40K generated JSON target. `cargo clean --manifest-path rust/Cargo.toml --target-dir .scratch/ws8bm-review230/target` removes the compiler cache. Cargo correctly refuses the second directory because it is not a tagged Cargo cache; it holds exactly four generated diagnostic JSON files. The cleanup verifies the bounded path, exact four filenames, regular-file/non-symlink status and valid JSON, unlinks those files and removes the empty directory. No pre-existing root rust/target or another worker's output is removed.

Raw cleanup/resource lines:

```text
     Removed 25050 files, 33.6GiB total
Removed four generated JSON diagnostics and their empty target directory
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "scratch_targets": []}
```

`python3 .scratch/ws8bm-review230/resource-check.py` enumerates owned active processes, all relevant fixed listeners, owned Docker containers/mounts and scratch target directories; its final assertion passes. Native temporary profiles, proof mounts and container instances are removed by their unconditional teardown. Logs and canonical media remain for review and later runs. The Python model server is untouched. Work stops after the requested push; the two explicit drawer deferrals remain. No current affected check or Rust verification failed. Earlier invalid setup/network attempts and historical broad failures above remain disclosed and receive no credit.

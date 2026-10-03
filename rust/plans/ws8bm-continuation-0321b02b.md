# WS8bm continuation: PR #189 review fixes merged

Branch: `rust/ws8bm-messages-http-2`. Review branch pushed at `21e4a77d76e121c851b0fd434d64fcbd74220ccd`. These review changes are tools/docs only; both branches' product behaviour and tests are retained.

Current accepted inventory remains **156/156 controllers; 118 passed / 17 deferred / 0 owner-blocked systems**. The controller attribution is PR-ready; the system port remains partial. This merge does not claim blanket system sign-off, original Ruby-system execution or pixel checks.

[The review report and assertion audit table](ws8bm-review-189-labels.md) contain the failing-first label proof, strict intended-assertion contract, held URL-job boundary, all raw run summaries and retained failures. [The accepted continuation checkpoint](ws8bm-continuation-cd64b2e45.md) records the preceding sixteen new system credits and ten controller attributions; its commands are historical evidence, not new runs.

The merge preserves continuation's `includeHidden` role lookup followed by Selenium visibility, test-only motion input, controller/model proofs, three paused provider/upload cases and inventory. It also retains the review's exact `perform_enqueued_jobs=nil` guard, current-source host builder, visible label text, model-ID forward scope and failure-attribution/retry rules. No assertion deadline, concurrency, compiler throttle, atom, golden or normalization mask changed.

The additional twenty-six continuation mutants receive explicit intended-assertion targets too; no continuation case falls back to the old any-error rejection rule. Injected panel-opacity and nested coarse-media probes record their mutated DOM state. Merge validation and the exact remaining inventory follow below.


## Merge verification

Merge commit: `ac045dd75809f437701f9014f74efdde4d6e9a8e`. The continuation-only mutation witnesses/targets follow-up is `2f98d1348748f7bb801988032f79281324a5d187`. The main-overflow target regression follows at `b19dbb8bae4870a790a02f5b4d69366512be96a9`. The generated-host fixture-input fix follows at `882f6affbe81590b8b29dca58587b6051be1f3a8`. These are the verification inputs; this report is a later docs-only checkpoint. The fresh source clone `.scratch/ws8bm-label-fresh` was switched to that merged input, then to the main-overflow target fix for the final helper/planner, full continuation discrimination and profile control reruns. Finally the clone takes the generated-host input fix for the final helper/planner, URL repetitions and PR Discuss control. No positive assertion changed between those inputs. It retains its independently built target from the review run; no previous fixture, target or scratch data is an oracle. The driver regenerates seeds and per-case fixtures and verifies the pinned Selenium atom on every invocation. Environment: `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, the configured global rustc throttle, and `CAMPFIRE_REFERENCE` / `CARGO_TARGET_DIR` pointing to this clone. All browser invocations were sequential.

The review branch's full 153-mutant discrimination run and thirty-check run are recorded in [the review receipt](ws8bm-review-189-labels.md). The combined source additionally reruns all 23 continuation named checks / 26 variants, all five changed credited labels and their controls, five consecutive held-job URL controls, the model-scoped forward control and all three hidden-allowed probes. These are separate, explicitly named cohorts, not a claimed sealed full-179 merged run. No new declaration credit is added by this review merge.

The continuation positive batch retained one Rails startup failure: CSS assets reported `net::ERR_NETWORK_CHANGED` and the 15 s Stimulus startup gate timed out before the joined-thread assertions. Rust was independently attempted and passed. Its exact fresh-fixture isolated rerun is recorded below; the original batch is not rewritten as green. The first continuation discrimination run retained three invalid attempts of the profile-width mutant: it had registered documentOverflow while the served fieldset min-width overflowed the scrollable main content. The actual state witness records 1000 px fieldsets; the pinned original has separate document and main-content assertions. Registration now names only the latter (mobile_layout_test.rb:56). A unit refuses credit for the earlier document assertion. The full continuation cohort is rerun after that correction; the old 25/26 summary stays below.

The first merged URL repetition stopped before browser startup: generating the test host omitted the tracked root public/500.html and provider callback Ruby include. Both compile errors are retained below. The tools-only copier now preserves these exact relative inputs from tracked source; two cold-copy regressions check byte refreshes, old generated-file removal and exclusion of Cargo targets. The URL repetitions and PR Discuss control are rerun with that input correction.

The preceding review's attachment preview deferral and retained broad-run failures remain disclosed in its linked report.

Commands below were run from the fresh clone. Raw summary lines are copied from their logs in `.scratch/ws8bm-review-labels/`. No Rust workspace, strict-clippy or release-input run is quoted as new verification for this tools/docs-only merge.

`merged-helpers.log`:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs rust/reference-tools/messaging/behavior-discrimination.test.mjs
```

```text
ℹ tests 15
ℹ pass 15
ℹ fail 0
ℹ skipped 0
```

`merged-planner.log`:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

```text
Ran 8 tests in 0.001s
OK
```

`merged-system-inventory.log`:

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

```text
WS8bm system inventory: 135 named declarations; 118 mapped behaviour passes; 17 deferred; 0 WS12 blocked; no pixel checks
```

`merged-controller-inventory.log`:

```sh
python3 rust/reference-tools/messaging/controller-case-inventory.py
```

```text
WS8bm controller inventory: 156 named declarations; 156 scoped attributions; 0 owner-blocked
```

`merged-continuation-positive.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --slice continuation --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 40s
WS8bm behaviour check: 22 named cases passed on Rails and Rust; 1 failed; no pixel checks
```

`merged-continuation-negative.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --slice continuation --negative --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
WS8bm invalid discrimination attempts: 3; bounded fresh-fixture retries only
WS8bm discrimination check: 25 served mutants rejected on Rails and Rust across 22 named checks; 1 invalid or escaped
```

`merged-label-positive.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'profile message and ban buttons have accessible names' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`merged-attach-boost-positive.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py composer_attach_menu boosting_messages --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
WS8bm behaviour check: 11 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`merged-label-negative.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y composer_attach_menu boosting_messages --mutant-set labels --negative --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 7 served mutants rejected on Rails and Rust across 3 named checks; 0 invalid or escaped
```

`merged-url-five.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --repeat 5 --keep-going
```

```text
error: could not compile `campfire` (bin "campfire" test) due to 2 previous errors
error: couldn't read `crates/campfire/src/app/../../../../../public/500.html`: No such file or directory (os error 2)
error: couldn't read `crates/campfire/src/controllers/message_features/../../../../../reference-tools/messaging/older_provider_callbacks.rb`: No such file or directory (os error 2)
```

`merged-forward-control.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'forwarded Markdown keeps tables and code blocks' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.17s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`merged-hidden.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`merged-final-helpers.log`:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs rust/reference-tools/messaging/behavior-discrimination.test.mjs
```

```text
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
```

`merged-final-planner.log`:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

```text
Ran 8 tests in 0.001s
OK
```

`merged-final-continuation-negative.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --slice continuation --negative --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 26 served mutants rejected on Rails and Rust across 23 named checks; 0 invalid or escaped
```

`merged-final-profile-positive.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py mobile_layout --case 'the profile page fits phone widths without scrolling sideways' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`merged-read-isolated.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'marks a joined thread read only while the conversation is visible' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`merged-final-ban-negative.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'profile message and ban buttons have accessible names' --mutant transparent-ban-text --negative --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`merged-host-final-helpers.log`:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs rust/reference-tools/messaging/behavior-discrimination.test.mjs
```

```text
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
```

`merged-host-final-planner.log`:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

```text
Ran 10 tests in 0.003s
OK
```

`merged-host-url-five.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --repeat 5 --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 16s
WS8bm behaviour repetition: 5 paired attempts; 1 named declaration; 0 failed
```

`merged-host-pr-control.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'discusses a pull request from its card' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.74s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 49.11s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```


## Exact remaining system work

All controller declarations are attributed. No system item remains owner-blocked. These seventeen declarations remain deferred; this review does not implement or promote them. The original twenty-seven-item disposition table is preserved in [the historical continuation report](ws8bm-continuation-cd64b2e45.md).

| Declaration | Pinned source | Concrete remaining work |
| --- | --- | --- |
| Markdown replies and file attachments remain usable | workspace_markdown_test.rb:143 | Rust missed the visible attachment reply preview at the original 10 s delivery deadline in the fresh fe4cbd716 checkpoint after Rails passed. Both actual message POSTs returned 200, but the delivered child had no reply-preview/body target. The original three-root-message fixture, test-only motion input and nonperforming job adapter produced an earlier paired pass at 8d4302d96, so they are necessary fixture corrections but do not repair the remaining delivery/render failure. Trace the real response and broadcast, distinguish pending replacement from server partials, and obtain stable paired control/row proof before credit. Not blocked on another owner. |
| workspace follows the system theme and mobile navigation remains reachable | workspace_markdown_test.rb:242 | Own mapped sequence remains unimplemented: compact composer geometry/empty field, JZ online presence (5 s), theme background change, profile-bar containment, drawer focus trapping/current room, Escape focus return, and coarse-pointer Enter/newline/send with saved source (workspace_markdown_test.rb:242-290). These are behavior assertions, not pixel work. |
| From Google Drive starts the legacy picker flow | composer_attach_menu_test.rb:34 | Needs the pinned server-side WebMock Drive list response and actual Google connection fixture, then visible Q3 Planning through the real picker route (composer_attach_menu_test.rb:37-47). Current harness does not yet provide that server-side transport; no owner block. |
| From Google Drive starts the enhanced share flow when sharing is configured | composer_attach_menu_test.rb:46 | Needs the original GOOGLE_PICKER_API_KEY/project-number environment and DriveShareMocks SDK/dialog scenario (composer_attach_menu_test.rb:50-69). The account/Drive controllers are merged, but this own harness boundary is not ported. |
| a release click landing on the just-opened menu does not activate it | message_interactions_test.rb:55 | Pinned message_interactions_test.rb:68-91 assumes the fixed 700 ms press point hits the newly opened menu. Metadata can change its height before this instantaneous hit test; it is not made deterministic by the original. Keep the original hit requirement and hold; record current paired attempts rather than waiting for metadata or changing the press point. |
| attach Drive files from the picker, send textless, and remove through edit | drive_attachments_test.rb:10 | Needs server-side Drive list/file WebMock fixtures, the two actual picker selections, chip geometry/removal, textless persisted file-ID set, and edit-frame replacement before final empty attachment rows (drive_attachments_test.rb:10-76). Current harness lacks those external transports; no owner block. |
| edit a room message in the composer and remove one of two attachments | drive_attachments_test.rb:79 | Needs two Drive metadata/list responses followed by real root composer edit, removal, one remaining visible link and exact persisted FILE_ID, plus hidden context DOM assertion (drive_attachments_test.rb:79-113). Own server-side mock transport remains unported. |
| attach a Drive file from the thread composer | drive_attachments_test.rb:116 | Needs pinned Drive list/file transport, real joined-thread fixture and the original 10 s settled drawer transform/picker/link assertions, followed by thread-message attachment IDs (drive_attachments_test.rb:116-140,151-164). Own harness work; merged WS12/#201/automations are not prerequisites. |
| motion is off by default in the test environment | motion_test.rb:19 | Asserts server-emitted data-test-motion=off and all three 0ms tokens (motion_test.rb:19-23). Both live harnesses boot production, where the Rails layout intentionally omits that attribute. Supplying the test-only input for other cases cannot earn credit for this server-emission declaration; a real test-environment host contract is still needed. |
| mobile drawer animates in, lands in place, and returns focus with motion on | motion_test.rb:26 | Own transition sequence remains unported: original 30 s token override, attached transitionrun listeners, initial/mid-travel transform, live transform animation, Web Animations finish and restored focus with original 5 s polling (motion_test.rb:26-70). No screenshots are needed; do not merely assert landed geometry. |
| member selection mode moves no rows and resizes nothing | motion_test.rb:73 | Own HQ multi-select sequence remains unported: original optional Show members 5 s, minimum-two 10 s, avatar-left/content-height equality across control-click/two selections/clear, and visible Message (2) (motion_test.rb:73-100). No owner block. |
| people directory bar shifts no rows when toggling | motion_test.rb:103 | Own directory selection sequence remains unported: minimum-two rows, exact row-top array before/after visible multi-select bar and after clearing all checked boxes (motion_test.rb:103-116). No owner block. |
| people directory bar stays stuck while scrolling | motion_test.rb:119 | Needs the original 12 Sticky User rows and 1400x400 viewport, actual scrolling >100 px, original 5 s wait for scrollTop=100, and sticky bar-bottom <= main-bottom+1 (motion_test.rb:119-146). Those fixture and sequence assertions remain own unported work. |
| room menu measures at full scale when clamping to the viewport edge | motion_test.rb:149 | Own motion-on contextmenu sequence remains unported: original synthetic far-right pointer, visible menu, 5 s transform-none polling, full-scale right <= innerWidth-8+1 and hidden-all Escape assertion (motion_test.rb:149-174). No owner block. |
| mobile drawer keeps the room list scroll position across close and reopen | motion_test.rb:177 | Needs 15 real Scroll rooms, focus/blur and actual >=400 px overflow, exact scrollTop=400, current room outside view, exact closed/open offset retention and visible focus after reopening (motion_test.rb:177-238). Own fixture and sequence remain unported; preserve the original 5 s polling. |
| mobile drawer reveals a current room far down the list on first open | motion_test.rb:241 | Needs 15 Scroll rooms plus Zz far room, hidden-all current-link lookup, zero initial offset/out-of-view precondition and first-open current-link focus/5 s reveal (motion_test.rb:241-257). Own fixtures and sequence remain unported. |
| mobile drawer reopens on the current room when it is already in view | motion_test.rb:260 | Needs 15 Scroll rooms, original focused current link, 5 s settled-scroll/current-in-view predicate, exact closed/reopened scroll offset and focused current link (motion_test.rb:260-292). Own fixture and sequence remain unported. |

## Scope and cleanup

Compared with the accepted `cd64b2e45` continuation, every changed tracked path is under `rust/plans/` or `rust/reference-tools/messaging/`. Both sides' pre-existing Rust tests/product behaviour are retained. No product crate, asset, golden, mask, vendored atom, deadline or workspace dependency changed in this review merge. The public list/composer seam is unchanged and remains documented in [ws8bm-integration.md](ws8bm-integration.md).

The only own scratch Cargo target was `.scratch/ws8bm-label-fresh/rust/target`. It was removed after all own browser/compiler processes finished; a final search found no `.scratch` directory named `target`. Logs and generated source evidence remain. No other worker's listener, target or service was stopped or deleted.


Cleanup command, run from the worktree after all own verification processes exited:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path .scratch/ws8bm-label-fresh/rust/Cargo.toml --target-dir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-label-fresh/rust/target
```

```text
     Removed 14600 files, 30.3GiB total
```

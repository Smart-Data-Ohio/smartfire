# WS8bm #230 review corrections

Round 3 merges `origin/main` `566c1bd77f543e9c273a1beb7c698fb3a1103ae3` with merge commit `00723123038fdf39a42557a7cb0041bb42914501`. Locked metadata passes and TOML parsing finds no duplicate workspace dependency keys. This correction changes tools/docs only beyond main; it does not edit app/, product responses, goldens, masks, assertion deadlines or retry limits. Current controller inventory stays **156/156**; system inventory is **128 passed / 7 deferred / 0 owner-blocked**, replacing the invalid 129/6 claim.

| Finding | Corrected contract | Pin |
| --- | --- | --- |
| Workspace mobile send | Look up the actual committed HQ/JZ `Mobile draft\n` row; require that row's `.message__body` itself and visible text, within one two-second budget. An optimistic/unrelated body cannot substitute. The later response drain is not a selector deadline. | workspace_markdown_test.rb:289; system_test_helper.rb:114-115 |
| Native motion setup/proof | Open HQ, execute the unmodified native Capybara/Selenium animation body/helpers in positives **and negatives**, and serve zero-duration CSS through a real HTTP proxy. Credit only the original off-canvas assertion at :43 plus the observed HQ/open/0 s/identity-transform state. | motion_test.rb:12,26-70,43 |
| Scroll preservation | Returned to deferred: prior Rust failure at initial scrollTop=400 precedes the closed-offset assertion. A new pair rejects at the intended phase, but the earlier setup race is unexplained. | motion_test.rb:177-238 |
| Reopen focus | Returned to deferred: prior Rails initial-open focus failure precedes the post-reopen assertion. A later unchanged-path success is not used as closure credit. | motion_test.rb:260-292 |
| Attachment preview race | Main #231 fixes the shared client overwrite: delivered messages ignore late progress/failure callbacks. The original 10 s preview predicate now passes with actual writes/bytes on both hosts; all four faults reject at their intended assertions, including the filename variant. | workspace_markdown_test.rb:143-173,165-168; client_message.js:42 before #231 |

The old attachment failure was a shared **client** race, not a Rust server defect: an already-delivered author row was overwritten by upload progress; persisted rows, HTTP/Cable payloads, the recipient and fresh GET were correct. The approved #231 module is copied byte-for-byte into the pinned reference image and compiled by Rails; a source-authenticity regression checks `0373dfbd9`. There is no browser-side client replacement. Both images use the current Rails schema like `parity/bin/ci-seed`, because main now requires migration `20261003180000`; the rest of the Rails behavior source stays at d7c7de92.

Native fault transport forwards HTTP writes and Cable frames unchanged, including Chromium's CONNECT tunnels. Negative native runs bypass service-worker caches using Selenium/CDP, matching the existing translated served-negative boundary; positive controls retain normal service-worker behavior. No wait or diagnostic state supplies parity credit. Resource shutdown is nested in unconditional finally blocks, including diagnostic failures. Native browser network failures are invalid, never credited as rejected.

Failing-first receipts: exact 69d9e3b9d workspace module in an isolated tools copy accepts both the served 3 s delay and unrelated-body probes on **both** apps. The fixed checks reject them at their specific persisted-row assertion. The old HQ setup source regression fails; the review's old animation negative is explicitly invalid, not a rejection. New native controls pass 8 assertions per app and the real served fault fails the original native assertion on both. The full independent before receipts remain read-only at `/home/riels/.cache/rust-port/ws11uirr/review230/`.

Run scripts/logs retained in `.scratch/ws8bm-review230/`: `run-affected.sh`, `run-proofs.sh`, `run-proofs-final.sh`, `run-native-bypass.sh`, `run-scope-before.sh`. Each invocation sets `WS8BM_DISCRIMINATION_RETRIES=1`, `RUST_TEST_THREADS=8`, `CARGO_BUILD_JOBS=2`; no automatic retry or increased wait is added. Commands are `python3 rust/reference-tools/messaging/behavior-check.py <group> --case <exact name> --keep-going` and the same with `--negative`, plus the baseline tools-copy command with `--mutant delayed-workspace-mobile-draft` / `--mutant unrelated-workspace-mobile-body`. Native motion uses group `motion`; workspace and attachment use `workspace_markdown`.

Raw affected summaries:

```text
before-delayed-workspace-mobile-draft.log
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
before-scope-final.log
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
workspace-positive-corrected.log
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
workspace-negative.log
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 4 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
native-motion-positive.log
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
native-motion-negative-bypass.log
1 runs, 3 assertions, 1 failures, 0 errors, 0 skips
1 runs, 3 assertions, 1 failures, 0 errors, 0 skips
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
upload-positive.log
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
upload-negative.log
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 4 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

Retained failures, with no credit: first bootstrap had the old seed schema (fixed by the CI schema overlay); the initial new SQL predicate omitted the pinned trailing newline (corrected, new positive passes); initial proxy lacked CONNECT and failed Cable setup (fixed); the first native attribution mistakenly named line42 rather than the assertion at43 (corrected). Two isolated old-tools copies initially lacked the vendored Selenium directory (corrected before the actual escape proofs). Later Rust native and Rails old-scope attempts suffered `net::ERR_NETWORK_CHANGED` during startup; they remain invalid and their raw logs are retained. The final native-negative run, corrected workspace control, both exact old-tool escapes and all affected mutation sets above have no failures/invalid results.

The two rejected closures are deliberately still deferred despite one newer valid pair each; no unchanged-path rerun is presented as a race fix. Round 3's seven remaining declarations are those two, legacy Drive picker, two-files/textless Drive send/removal, root two-files Drive removal, thread Drive attachment, and actual test-environment default motion. The stacked round-4 branch already covers the five Drive/layout items; its counts are recomputed separately after carrying this correction.

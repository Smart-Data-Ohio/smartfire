# WS8bm #230 review corrections

Round 3 merges `origin/main` `566c1bd77f543e9c273a1beb7c698fb3a1103ae3` with merge commit `00723123038fdf39a42557a7cb0041bb42914501`. Locked metadata passes and TOML parsing finds no duplicate workspace dependency keys. This correction changes tools/docs only beyond main; it does not edit app/, product responses, goldens, masks, assertion deadlines or retry limits. Current controller inventory stays **156/156**; system inventory is **127 passed / 8 deferred / 0 owner-blocked**, replacing the invalid 129/6 claim.

| Finding | Corrected contract | Pin |
| --- | --- | --- |
| Workspace mobile send | Look up the actual committed HQ/JZ `Mobile draft\n` row; require that row's `.message__body` itself and visible text, within one two-second budget. An optimistic/unrelated body cannot substitute. The later response drain is not a selector deadline. | workspace_markdown_test.rb:289; system_test_helper.rb:114-115 |
| Native motion setup/proof | Open HQ, execute the unmodified native Capybara/Selenium animation body/helpers in positives **and negatives**, and serve zero-duration CSS through a real HTTP proxy. Credit only the original off-canvas assertion at :43 plus the observed HQ/open/0 s/identity-transform state. | motion_test.rb:12,26-70,43 |
| Release-click | Deferred: the original hit assertion can return DIV instead of menu, and unchanged passing repetitions/mutant receipts do not causally resolve it. | message_interactions_test.rb:55-86,80 |
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

The three disputed release/drawer closures are deferred despite one newer valid pair each; no unchanged-path rerun is presented as a race fix. Round 3's eight remaining declarations are release-click, those two, legacy Drive picker, two-files/textless Drive send/removal, root two-files Drive removal, thread Drive attachment, and actual test-environment default motion. The stacked round-4 branch already covers the five Drive/layout items; its counts are recomputed separately after carrying this correction.

## Fresh-clone merged-source verification

Ran at `d2d05351d` in the no-hardlinks clone `.scratch/ws8bm-review230/fresh`. The clone began with an empty owned Cargo target; dependencies, assets and three seeds come from tracked inputs. Canonical libvips 8.16.1 / ffmpeg 7.1.5 are used through Cargo's child-process runner, without a global library override. `validate.sh` sets compiler jobs two, test threads eight and CI, builds `default first_run agents_ui`, and runs exactly:

```sh
bash rust/parity/bin/seed build default first_run agents_ui
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

All 59 raw workspace summaries (4,922 passed / 0 failed / 22 existing ignores; the shared CI html5ever exclusion is unchanged):

```text
test result: ok. 2813 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 572.47s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.04s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1379 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 96.61s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.77s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.55s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.37s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
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

Raw lint/build and tool summaries:

```text
fresh seed exit: 0
fresh workspace exit: 0
strict clippy exit: 0
release inputs exit: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 20s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 14s
ℹ tests 60
ℹ pass 60
ℹ fail 0
Ran 29 tests in 6.247s

OK
```

Node: `node --test rust/reference-tools/messaging/*.test.mjs`. Python: `python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'`. Their receipts are `node-final.log` and `python-final.log`; workspace/lint/release receipts are `workspace.log`, `clippy.log`, `release.log`. Main's new existing ignores remain unchanged. The release-input build completes with only Cargo.toml, Cargo.lock and crates/; no non-test include of external vectors is added.

Follow-up ledger/schema correction and current verifier receipts: [ws8bm-review230-ledger.md](ws8bm-review230-ledger.md). No browser rerun or new release-click credit is included.

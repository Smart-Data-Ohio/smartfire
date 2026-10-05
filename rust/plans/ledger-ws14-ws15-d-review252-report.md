# PR #252 review follow-up

Review base: `9172466d49e27467dcf01ac3176c4c2fa89fa066`. Rails remains pinned to `78b9b1546bdab4c6c1c9b8ddb94512f661289112`.

**127 closed; 0 remaining.** The D map contains **547 direct assertions and 142 required helper assertions**, with zero unmapped assertions. Independent source inventory reproduced the review's **35 omissions across 27 declarations**. All 35 now cite discriminating assertions through the real path; no declaration was relabeled or reopened.

## Changes and scope sweep

- **WS15g-048:** the card count selects actual `.github-pr-thread-header` DOM descendants. The old string suffix included sibling elements after a prematurely closed header.
- **Supplemental WS15g-046 and WS15g-051:** the same header helper affected loaded card/title and private lazy-frame/card/files checks. All now use parsed DOM ancestry and exact frame attributes. These are supplemental checks outside the 127 D IDs.
- **Supplemental WS15g-044 and WS15g-045:** Discuss form/button and link checks now use full parsed DOM scopes. This is additional cleanup; the sweep did not find a further confirmed D containment defect there.
- The sweep reviewed all 127 declarations/547 direct citations, including event message/card descendants, malicious-path/image containment, Turbo stream ancestry, document-global selectors that match Rails, and 53 original browser wrappers. No additional containment mismatch was found. The 13 original browser files remain byte-identical to the pin.
- **26 cookie helper entries:** native setup executes pinned Rails' GET/password test-session bridge through real credential authentication, verified-session creation, and cookie producers, then asserts the issued cookie's nonempty value. The browser bridge shares this path.
- **8 sudo helper entries:** use real POST `/sudo`, assert the returned redirect, and carry returned confirmation state into subsequent requests. The 14/16-minute branches retain their real clock/state setup.
- **1 approval helper entry:** validates the actual model action before persistence/dispatch through `Action::errors()`.
- The checker derives helper callsites from pinned Ruby syntax independently of the maps, follows inherited callbacks and nested/recursive helpers, and enforces exact completeness and pinned bytes. Direct local/inherited lifecycle callback assertions also have regression coverage. Historical B/C snapshots remain explicitly historical; current maps cannot downgrade their pin/policy to evade completeness.

This follow-up corrects test setup, assertions, and checker coverage. No permanent producer behavior, Rails source, asset, or CodeQL change was required.

## Mutation verification

`ledger-ws14-ws15-d-review252-mutations.json` and `rust/reference-tools/cutover/d-review252-mutations.json` record **9/9 exact assertion kills**, including **8 distinct declarations from the 27 helper gaps** and the reported card-placement mutation. Compilation/setup failures do not count.

| Declaration | Real producer fault | Cited failure |
|---|---|---|
| WS14e-102 | Suppress authentication cookie | `presenters/test_support.rs:82` |
| WS14g-016 | Suppress authentication cookie | `presenters/test_support.rs:82` |
| WS14g-021 | Suppress authentication cookie | `presenters/test_support.rs:82` |
| WS15g-011 | Suppress authentication cookie | `presenters/test_support.rs:82` |
| WS14g-023 | Sudo confirmation returns 200 | `app/cutover_d_tests.rs:143` |
| WS14g-024 | Sudo confirmation returns 200 | `app/cutover_d_tests.rs:143` |
| WS15g-023 | Sudo confirmation returns 200 | `github/cutover_d_tests.rs:72` |
| WS15g-058 | Reject valid approval action | `github/cutover_d_tests.rs:1124` |
| WS15g-048 | Card moves to balanced sibling outside header | `github/cutover_d_tests.rs:723`, 0 versus 1 |

Every producer was restored byte-for-byte, with saved SHA256s independently matching both restored source and the review base. The original 34 attempts/32 confirmed closures remain preserved separately, including the two initial survivors and their successful retries. Positive baseline before mutations: 104 passed. Positive restored baseline below: 104 passed.

## Targeted verification

From `rust/`; compiler jobs 2, nextest concurrency 4. No local full workspace suite.

```bash
CI=true CARGO_BUILD_JOBS=2 WS14_BROWSER_RUBY_IMAGE=ws8bm-browser-reference-78b9b1546 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire -p campfire_db -j 4 -E 'test(cutover_d_) | test(controller_time_and_parameter_casts_match_pinned_rails) | test(unicode_parity_manual_github_login_uses_downcase_without_folding) | test(controllers::ws14_original_browser_tests::) | test(controllers::ws15_original_github_browser_tests::)' --run-ignored all --no-fail-fast --success-output immediate --failure-output immediate
```

**104 passed, 0 failed, 4500 skipped (125.341s)**: 51 selected native tests and 53 original browser wrappers. SHA256: `2322c5f717e0dc27f0a64a5a896c767ba6bf7719fc49a294c738372a83443638`.

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo clippy --locked -p campfire -p campfire_db -p campfire_views -p rails_compat --all-targets -- -D warnings
```

**Passed, zero warnings** for all targets of the four PR-touched crates. SHA256: `cddbe165918925805914ae59e63b4da65a0907697cab9c7a43153d536056e0cb`.

From repository root:

```bash
python3 -m unittest discover -s rust/reference-tools/cutover -p test_helper_inventory.py -v
python3 rust/ci/ignored_tests.py
python3 rust/reference-tools/users/check_ws14_original_browser.py --receipt-log rust/target/ledger-d-tools/review252-final.log --id-prefix WS
python3 rust/reference-tools/cutover/assertion-maps.py --render --pass-log rust/target/ledger-d-tools/review252-final.log rust/plans/ledger-ws14-ws15-d-assertions.json
python3 rust/reference-tools/cutover/assertion-maps.py --render rust/plans/ledger-ws14-ws15-d-root-assertions.json rust/plans/ledger-ws14-ws15-d-events-assertions.json rust/plans/ledger-ws14-ws15-d-github-assertions.json rust/plans/ledger-ws14-ws15-d-browser-assertions.json
python3 rust/reference-tools/cutover/check-ws14-ws15.py --slice-log rust/target/ledger-d-tools/review252-final.log
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list rust/target/ledger-d-tools/review252-list.json
python3 rust/reference-tools/cutover/mutation-check.py rust/reference-tools/cutover/d-review252-mutations.json rust/target/ledger-d-tools/review252-mutations --recheck
```

Helper regressions: **10/10**. Ignored-test guard: **74 correctness wrappers, 7 utilities, 0 unowned**. Original-browser checker: **53 declarations / 326 direct / 92 helper calls**. D physical maps: **127 declarations / 547 direct / 142 helpers / 0 unmapped**; aggregate and partition match exactly. Parent ledger: **445 records / 0 open**, retaining two explicitly superseded historical B omissions. Compiled listing: **98 mapped test identities for 127 declarations**. Mutation recheck: **9/9**, all production hashes restored.

The compiled listing is generated from `rust/` with:

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest list --locked -p campfire -p campfire_db --run-ignored all --message-format json > target/ledger-d-tools/review252-list.json
```

Independent Astra review approved all 35 new entries, DOM scopes, source-derived inventory, callback regressions, 9 mutation receipts, exact assertion coordinates, and producer restoration. Its 35 individual helper-removal checks and 10 historical-pin downgrade attempts all rejected.

After the single follow-up push, dispatch the correctness matrix from `main` for the pushed head with `gh workflow run rust.yml --ref main -f ref=<sha>` and require every job green. The dispatched run URL/head are reported with the final PR handoff; they are not a substitute for the local receipts above.

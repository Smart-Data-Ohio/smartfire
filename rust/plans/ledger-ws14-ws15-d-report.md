# WS14 / WS15 continuation D

All **127 remaining exact Rails declarations are closed; 0 remain open**. The structured source of truth is `ledger-ws14-ws15.json`, with the unchanged Rails reference `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Base: `c59d0ee3154d144dee90c190c45eea82949e23b7`.

`ledger-ws14-ws15-d-assertions.json` and its rendered Markdown map **547 declaration assertion calls and 142 nested helper assertion entries**, with **0 unmapped D assertions**. The review252 follow-up adds 35 previously omitted helpers across 27 declarations and enforces independent completeness; see `ledger-ws14-ws15-d-review252-report.md` for current verification. Every entry cites a physical assertion through the real model API, signed HTTP, actual registered jobs/Action Cable, or native rendered browser markup. The four smaller D maps retain the reviewed work packages. The earlier parent/B/C reopens and receipts remain historical; WS14e-101's incomplete B map is explicitly superseded by the complete original-browser receipt.

The 53 browser wrappers execute the original declaration bodies and helper assertions from **13 byte-identical pinned Rails test files**, using Capybara/Selenium against the running Rust HTTP/Turbo/Cable path and original Rails fixtures. Only the external Google/GitHub exchanges are stubbed. Assertions retain the original geometry, exact scopes, bodies, headers, verbs, partial input and error variants. GitHub TLS validates a runtime fixture certificate named `api.github.com`; the producer constructs the actual method, target, headers and JSON forwarded to the original WebMock expectations. All wrappers are registered in `rust/ci/ignored-tests.json` and the messaging correctness job.

## Producer fixes

- Event attendance creation rejects duplicate member/event responses. Direct response updates enqueue the exact event/user arguments; timestamp-only saves do not enqueue.
- Duplicate Google-account creation validates the existing user association before persistence and preserves the existing connection.
- A linked GitHub identity uses the connected account's normalized login for the Rails dirty-field validation. Profile HTTP parameters follow Rails blank/connected semantics, including tab and nonbreaking-space logins.
- GitHub webhook delivery uses the injected transport used by registered jobs; transport failures emit a credential-free warning. Room rendering caches the account along with preloaded message facts, eliminating per-message account reads (two versus six PR messages both issue 78 queries).
- Event cards retain wide calendar years from Rails/Ruby through rendering, including the original Chrome-entered year 60310. Finite TZInfo boundary periods match Rails beyond 2126 and across wide-year summer/DST cases; ordinary event card/page/form goldens remain identical. UTC civil arithmetic also handles Jiff's minimum and maximum instants without overflow.

The TOTP state methods and escaping JSON-parser seam expose the same production path exercised by Rails without replacing the HTTP/model readers.

## Mutation evidence

`ledger-ws14-ws15-d-mutations.json` records **34 executions across 32 distinct closures; all 32 have a confirmed kill at a mapped assertion**, and every producer is restored. The two initial survivors remain recorded:

- WS15g-034: removing only the quoted-source cache projection left legitimate parent timestamp invalidation intact. Breaking real `Message::destroy` instead fails the mapped quoted-message cache inequality.
- WS15g-037: the initial test left the clock advanced between Rails `travel` blocks, masking a missing pin dependency. Restoring base time exactly makes pin and unpin occur at the same base+1 instant, and removing the pin stamp fails the mapped unpin inequality.

The newer-card unpin case also compares a key captured immediately after the PR update, independently discriminating unpin beneath the newer card. Native raw panic locations preserve execution-time coordinates; each kill is linked to the same final physical assertion. Browser mutations alter the actual served JS/CSS only within the test proxy, and the original geometry/focus assertions fail. The normal final browser run follows restoration.

## Targeted verification

Commands below run from `rust/`. No full workspace suite was run locally. Compiler jobs stayed at two and nextest concurrency at four; the shared compiler throttle was unchanged.

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire -p campfire_db -j 4 -E 'test(cutover_d_) | test(controller_time_and_parameter_casts_match_pinned_rails) | test(unicode_parity_manual_github_login_uses_downcase_without_folding)' --no-fail-fast --success-output final --failure-output immediate
```

**51 passed, 0 failed, 4480 skipped** (21.843s). Raw log SHA-256: `5915214ad98049a90354b5827be810bcc11014e01014e26349945b84280ea42c`.

```bash
CI=true CARGO_BUILD_JOBS=2 WS14_BROWSER_RUBY_IMAGE=ws8bm-browser-reference-78b9b1546 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire -j 4 -E 'test(controllers::ws14_original_browser_tests::) | test(controllers::ws15_original_github_browser_tests::)' --run-ignored only --no-fail-fast --success-output immediate --failure-output immediate
```

**53 passed, 0 failed, 3021 skipped** (127.829s). Raw log SHA-256: `ae37c8a899afece2ef3b5f0a0d8d1c68f727560fa9e169b62f78d777a019e427`.

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire_views -j 4 -E 'test(event_cards_are_byte_identical_to_rails_fragments) | test(event_card_collections_match_rails_empty_and_populated) | test(wide_calendar_time_) | binary(event_pages)' --no-fail-fast
```

**5 passed, 0 failed, 158 skipped** (0.104s). Raw log SHA-256: `c0e911da4246597c2c41a8f91ddac2a5fbb7ddc442ca7b84a72e3911d1f8360b`.

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo clippy --locked -p campfire -p campfire_db -p campfire_views -p rails_compat --all-targets -- -D warnings
```

**Passed, zero warnings**, all targets of the four touched crates. Raw log SHA-256: `a316fd299bacddeaad3d86209673172977a76c6590ddb5c8fc528dc7be40ae2a`.

The native run includes 45 mapped receipt tests plus six supplemental regressions/setup-caster checks. The browser run supplies the other 53 distinct mapped tests: **98 compiled receipt identities close 127 declarations**. View verification adds five targeted golden/boundary tests. The Rails CI seed was prepared, image-checked, built and validated: **73 seed checks passed**. From the repository root:

```bash
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed prepare
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed check-image
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed build
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed validate
```

Additional commands run from the repository root:

```bash
python3 -m unittest discover -s rust/ci -p 'test_ignored_tests.py'
node --test rust/reference-tools/messaging/behavior-native-proxy.test.mjs
python3 rust/ci/ignored_tests.py
python3 rust/reference-tools/users/check_ws14_original_browser.py --receipt-log rust/target/ledger-d-tools/browser-restored-final.log --id-prefix WS
python3 rust/reference-tools/cutover/assertion-maps.py --render rust/plans/ledger-ws14-ws15-d-assertions.json
python3 rust/reference-tools/cutover/assertion-maps.py --pass-log rust/target/ledger-d-tools/maps-final-pass.log rust/plans/ledger-ws14-ws15-d-assertions.json
python3 rust/reference-tools/cutover/check-ws14-ws15.py --slice-log rust/target/ledger-d-tools/native-final.log --slice-log rust/target/ledger-d-tools/browser-restored-final.log
python3 rust/reference-tools/check-cutover-ledgers.py --nextest-list rust/target/ledger-d-tools/nextest-list.json
```

The Python registry regressions pass **6/6**; proxy regressions pass **4/4**. The initial registry source guard reports **67 CI correctness wrappers, 7 utilities, 0 unowned**; after synchronization with main it reports **74 correctness wrappers, 7 utilities, 0 unowned**, retaining main's seven additional WS11 browser wrappers. The original-browser checker verifies **53 declarations, 326 direct calls and 92 helper calls**, in addition to pinned input bytes. The ledger and per-assertion checkers pass: **445 records; 86 retained baseline receipts, 358 implementation receipts, 1 existing test-only outside-gate record, 0 open**. Historical partial WS8/WS11 cutover inventories remain outside this WS14/WS15 slice.

The compiled receipt listing is produced from `rust/` using:

```bash
CI=true CARGO_BUILD_JOBS=2 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest list --locked -p campfire -p campfire_db --run-ignored all --message-format json > target/ledger-d-tools/nextest-list.json
```

Native mutations use `python3 rust/reference-tools/cutover/mutation-check.py MANIFEST OUTPUT -- rust/target/ledger-d-tools/cargo-env`; exact manifests, commands, package filters, raw log hashes, panics and restoration hashes are recorded in the mutation receipt. Browser mutations use `python3 rust/reference-tools/users/ws14_original_browser_mutations.py`. No compilation, infrastructure or unrelated assertion failure is credited as a kill.

The final self-sweep and independent source review checked fixture values and starting mutations, room/user/event scoping, GET/HEAD/PUT/self-GET variants, missing/error/rejected input branches, exact job arguments, body/database changes, original geometry and nested helper execution. Rails sources, production JS/CSS assets and CodeQL configuration were not changed. The slim PR gate is Rust port; full correctness jobs remain registered for main/nightly.

Nothing remains open in this requested 127-declaration slice.

## Synchronization with concurrent main

Main advanced to `be5cace8e` after the initial fetch and before PR creation. GitHub therefore reported a conflict and did not start any PR checks. The sole conflict was the documentation line counting ignored wrappers. The local resolution combines the 53 new D wrappers with main's 21: **74 correctness wrappers**, retaining main's browser registration and source changes. Application code merged without conflicts or manual producer changes. This synchronization was completed and verified locally before requesting an exception to the user's one-push limit.

Final merged-source native/browser command from `rust/`:

```bash
CI=true CARGO_BUILD_JOBS=2 WS14_BROWSER_RUBY_IMAGE=ws8bm-browser-reference-78b9b1546 PATH="$PWD/target/ledger-d-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire -p campfire_db -j 4 -E 'test(cutover_d_) | test(controller_time_and_parameter_casts_match_pinned_rails) | test(unicode_parity_manual_github_login_uses_downcase_without_folding) | test(controllers::ws14_original_browser_tests::) | test(controllers::ws15_original_github_browser_tests::)' --run-ignored all --no-fail-fast --success-output immediate --failure-output immediate
```

**104 passed, 0 failed, 4500 skipped**, including all 51 selected native tests and all 53 original browser wrappers. SHA-256: `907f25f0f67dfb8df535ba1a7ee06fdbc239e345f9ff090666f6ef7ccb784d99`. The same targeted view command above was rerun: **5 passed, 0 failed, 158 skipped**; SHA-256: `9b675f5c40224037de92c3c8a6a728e081b34331395045b5ed938f8f5fd4f41a`. The same strict four-crate clippy command passed again with zero warnings; SHA-256: `26d15a94b9306380f2f480374db444aab97a2044942d53b9f46430d4793604eb`. Original-browser, per-assertion, ledger and ignored-registry checkers passed again on merged source.

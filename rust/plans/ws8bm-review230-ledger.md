# WS8bm #230 follow-up: release-click deferral and inventory schema

This is a tools/docs ledger correction, not a new browser or Rust-suite certification. No browser suite, product code, fixture, golden, mask, retry, assertion deadline or app/ file is changed. Controllers remain 156/156. Corrected system counts are **http-3: 127 passed / 8 deferred / 0 owner-blocked** and **http-4: 132 passed / 3 deferred / 0 owner-blocked**. All earlier 128/7 and 133/2 current claims are superseded.

| File | Change |
| --- | --- |
| plans/ws8bm-system-cases.json | Release-click is deferred again: message_interactions_test.rb:55-86 (hit assertion :80) intermittently returns DIV rather than menu; unchanged passing repetitions and the disputed mutant receipt have no causal resolution. Remove active/historical closure evidence and the pass-claim diagnostic. All deferred records contain a concrete remaining_reason and no closure evidence. Scroll/reopen remain deferred at motion_test.rb:177-238 and :260-292. |
| reference-tools/messaging/deferred-system-inventory.py | Missing evidence is permitted for unresolved cases, which require a nonblank reason. Evidence still requires passed status, and a passed case still requires evidence. Original pin hashes, complete name lists, statuses, counts and 135-declaration gate are unchanged. |
| reference-tools/messaging/deferred_system_inventory_test.py | Execute the real verifier against in-memory schema fixtures: absent/null evidence deferrals, forbidden pass evidence, missing/blank/invalid reasons, unsupported passes and owner-blocked counts. The actual ledger regression checks that all three disputed closures stay reason-only deferrals. Git source retrieval alone is stubbed for synthetic fixtures; the real CLI separately verifies every pinned source/name/hash. |

The prior closure receipts remain in historical reports; they are not active ledger evidence or new closure credit. Four Drive flows, test-environment motion, upload, workspace and HQ native motion remain unchanged and approved by the review. No headless reruns are used to restore disputed credit.

Failing first on http-3 `1f3678ed5`: the actual verifier exits 1 with KeyError: 'evidence'. The new seven-test module before the fix reports:

```text
Ran 7 tests in 0.009s

FAILED (failures=10, errors=2)
```

The no-evidence deferral and blocked-case controls raise KeyError, reason-less records are wrongly accepted, and the actual ledger regression rejects release-click's passed status plus scroll/reopen's historical closure fields. Failure receipts: .scratch/ws8bm-ledger-followup/inventory3-before.log and tests-before.log. The independent follow-up evidence stays read-only at /home/riels/.cache/rust-port/ws11uirr/review230-followup/REVIEW.md.

Commands rerun on the corrected branch:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p 'deferred_system_inventory_test.py'
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

Raw http-3 regression/helper lines:

```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.007s

OK

....................................
----------------------------------------------------------------------
Ran 36 tests in 1.873s

OK
```

Raw http-3 verifier output (actual pinned names and SHA256 checks, no browser execution):

```text
test/system/boosting_messages_test.rb: 4 named declarations; 4 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/code_highlighting_test.rb: 6 named declarations; 6 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/sending_messages_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/threads_test.rb: 15 named declarations; 15 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/workspace_markdown_test.rb: 8 named declarations; 8 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/composer_test.rb: 11 named declarations; 11 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/composer_attach_menu_test.rb: 9 named declarations; 8 mapped behaviour passes; 1 deferred; 0 WS12 blocked
test/system/message_interactions_test.rb: 10 named declarations; 9 mapped behaviour passes; 1 deferred; 0 WS12 blocked
test/system/message_actions_mobile_test.rb: 2 named declarations; 2 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/message_toolbar_test.rb: 13 named declarations; 13 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/message_list_a11y_test.rb: 29 named declarations; 29 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/drive_attachments_test.rb: 3 named declarations; 0 mapped behaviour passes; 3 deferred; 0 WS12 blocked
test/system/unread_divider_test.rb: 5 named declarations; 5 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/search_forward_edit_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/motion_test.rb: 9 named declarations; 6 mapped behaviour passes; 3 deferred; 0 WS12 blocked
test/system/mobile_layout_test.rb: 5 named declarations; 5 mapped behaviour passes; 0 deferred; 0 WS12 blocked
WS8bm system inventory: 135 named declarations; 127 mapped behaviour passes; 8 deferred; 0 WS12 blocked; no pixel checks
```

## Carried into http-4

Http-3's pushed ledger correction `aef598492` is merged into http-4 with a merge commit. JSON conflict resolution preserves all five approved Drive/layout closures from http-4, applies the three disputed reason-only deferrals from http-3, and keeps upload passed. Locked Cargo metadata passes after the merge; no Rust or browser suite is rerun. The same three Python commands above are rerun on this branch.

Raw http-4 regression/helper lines:

```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.007s

OK

.......................................
----------------------------------------------------------------------
Ran 39 tests in 2.397s

OK
```

Raw http-4 verifier output:

```text
test/system/boosting_messages_test.rb: 4 named declarations; 4 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/code_highlighting_test.rb: 6 named declarations; 6 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/sending_messages_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/threads_test.rb: 15 named declarations; 15 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/workspace_markdown_test.rb: 8 named declarations; 8 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/composer_test.rb: 11 named declarations; 11 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/composer_attach_menu_test.rb: 9 named declarations; 9 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/message_interactions_test.rb: 10 named declarations; 9 mapped behaviour passes; 1 deferred; 0 WS12 blocked
test/system/message_actions_mobile_test.rb: 2 named declarations; 2 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/message_toolbar_test.rb: 13 named declarations; 13 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/message_list_a11y_test.rb: 29 named declarations; 29 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/drive_attachments_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/unread_divider_test.rb: 5 named declarations; 5 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/search_forward_edit_test.rb: 3 named declarations; 3 mapped behaviour passes; 0 deferred; 0 WS12 blocked
test/system/motion_test.rb: 9 named declarations; 7 mapped behaviour passes; 2 deferred; 0 WS12 blocked
test/system/mobile_layout_test.rb: 5 named declarations; 5 mapped behaviour passes; 0 deferred; 0 WS12 blocked
WS8bm system inventory: 135 named declarations; 132 mapped behaviour passes; 3 deferred; 0 WS12 blocked; no pixel checks
```

Current http-4 deferrals are exactly release-click (message_interactions_test.rb:55-86,80), scroll preservation (motion_test.rb:177-238), and reopen focus (motion_test.rb:260-292). Http-3 also retains its five stacked Drive/layout deferrals, giving eight. No deferral carries active or historical closure evidence. Metadata correction does not re-certify previous full-suite/browser results. The final cleanup audit creates no Cargo target and starts no browser/app container; no owned resources remain.

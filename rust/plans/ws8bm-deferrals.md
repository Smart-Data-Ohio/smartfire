# WS8bm cutover browser deferrals

Branch `rust/ws8bm-deferrals`, from fresh main `78b9b1546`. The system inventory is **135 passed / 0 deferred / 0 blocked**. These are three new causal closures; the other 132 declarations retain their reviewed attribution. No Rust product code, Rails assets, vectors, masks, assertions or original deadlines changed.

## Causal diagnoses and closure bar

| Declaration and pin | Reproduced cause | Fix and discrimination |
| --- | --- | --- |
| Release click, `message_interactions_test.rb:55-86` | The translation used a **content viewport** of 390×844 and CDP touch. Holding the real actions response for 250 ms reproduced `DIV != menu` on **both** hosts: the cold menu starts near y351 while the unchanged message center is y307.34. | Execute the literal pinned body and helpers with Selenium's **window** resize and W3C touch actions, on both positive and negative hosts. The resulting content viewport is 390×701; the cold menu starts at y210.31 and covers the same y307.34 point, even with that response hold. The 700 ms hold, exact center hit, `menu` equality, menu-visible checks and hidden-all composer predicate are unchanged. Removing suppression delivers a trusted compatibility click at the press point, activates the menu and fails the original menu assertion at line 60. Attribution requires the broken guard, menu-visible click, original press point and native failure line; startup, line-80 geometry and unrelated failures earn no credit. |
| Scroll retention, `motion_test.rb:177-238` | Layout Cable connectivity does not establish lazy sidebar readiness. Opening during frame load lets `syncCurrentRoom` focus a link before the queued enter-focus callback. The harness blurs that first focus and sets smooth `scrollTop=400`; the second focus interrupts it. Holding the actual frame response 250 ms and delaying the existing enter-focus callback 400 ms reproduces the original five-second `drawer scroll lands` failure on both hosts. The callback arrives at offset **327 on Rails / 345 on Rust**, and neither reaches 400. | Wait for real `#user_sidebar[complete]`, its scroller and its synchronized current-room identity **before opening**, within the unchanged two-second default setup budget. With identical scheduling stress, first focus now follows frame completion, the scroll reaches 400, close retains 400 and reopen focuses a visible control at 400. The default `display:none` fault still fails specifically at the original **closed** offset assertion, with the destroyed boxes observed. |
| Reopen focus, `motion_test.rb:260-292` | Without that barrier, a 1,250 ms frame-response hold opens the drawer without its current link. The queued first-open callback runs too early; late frame synchronization invokes the reopen-only fault during initial setup. Both initial-focus failures are **invalid**, not intended negative rejections. | The same real frame/current-link readiness signal precedes opening. Original initial focus, five-second settled-scroll/current-in-view polling, Escape, reopen, two-second current focus and exact unchanged offset remain. Under frame/focus scheduling stress, the default fault now rejects on **both** hosts specifically at `reopenedCurrentFocus`; it cannot borrow initial-focus failure. |

The 15 real Scroll rooms, HQ, three original Designers messages and all IDs remain unchanged. Motion uses the original single signed-in browser; the previous unused second viewer is removed. Every complete positive pair compares persisted message/thread/user/room identities for motion, and exact message/history/reaction/thread rows for release. No message write is expected from these read-only original cases.

`behavior-browser-setup.mjs` records frame, focus and scroll order. `WS8BM_SETUP_DELAY` holds real response bytes; `WS8BM_OPEN_FOCUS_DELAY` delays the existing opening callback once without replacing its logic. These optional causal scheduling probes are **not acceptance evidence by themselves**, do not change a deadline, and are off during stock repetitions. Diagnostics cannot satisfy assertions. Native/outer teardown remains unconditional, including when readback diagnostics fail.

The inventory verifier now requires a reasoned closure note, independent fixtures, zero automatic retries, at least ten Rust successes, a Rails positive and paired **intended** negative rejections for these three formerly disputed entries. Its regressions reject missing notes/proofs, nine Rust passes, failed or retried runs, and invalid companion negatives. Ordinary reason-only deferrals remain valid and cannot claim closure evidence.

## Commands and receipts

Commands below ran from the worktree. All browser invocations use:

```sh
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1
export CARGO_TARGET_DIR="$PWD/rust/target"
export WS8BM_BROWSER_PORT_BASE=22020 WS8BM_DISCRIMINATION_RETRIES=1
```

The runner builds its seeds, binary and browser inputs. No previous target or untracked fixture is required. Raw logs live in `.scratch/ws8bm-deferrals/`.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --repeat 10 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --repeat 10 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view' --repeat 10 --keep-going
```

The successful independent-fixture batches are `release-ten.log`, `scroll-ten-final.log`, and `reopen-ten.log`, respectively. Each has the raw line:

```text
WS8bm behaviour repetition: 10 paired attempts; 1 named declaration; 0 failed
```

Thus **each fixed original case has at least 10/10 complete Rust passes**, with ten complete Rails pairs and real saved-row checks. Reopen's first batch spans removal of the unused viewer; its sidebar barrier and every original case predicate were unchanged throughout.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --negative --keep-going
WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400 python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --negative --keep-going
WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400 python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view' --negative --keep-going
```

`release-negative-final.log`, `scroll-negative-final.log`, `reopen-negative-final.log` each report:

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

The environment sets the attempt limit to one; **no automatic retry ran**. There are three valid paired intended rejections, not three generic assertion failures.

The causal before controls temporarily removed only the new `readySidebar` call, restoring it in `finally`. With `WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400`, the original scroll predicate fails on both hosts (`scroll-focus-before.log`); the same command with the barrier passes both (`scroll-focus-after.log`). With a 1,250 ms real response hold, the pre-barrier reopen negative fails both initial-focus assertions (`reopen-causal-before.log`) and gets zero rejection credit. The translated release control with a 250 ms real metadata hold fails both center-hit equalities (`release-delay-before.log`); the native unchanged-body control with that same hold passes (`release-native-delay.log`). No server response contents were rewritten in these latency controls.

Retained experimental failures are not overwritten: the initial scroll repetition reports **8 complete pairs / 2 failures**, and an additional reopen repetition reports **9 complete pairs / 1 failure**. All three are Rails `ERR_NETWORK_CHANGED` import/startup failures at the harness's pre-case composer startup check, before the HQ case; Rust completes its case. They are not asserted to be product differences, are not retry credit, and do not replace the complete passing receipts above. The initial native negative also remains recorded as invalid: the original matcher expected a later synthetic click and refused the real compatibility-click failure at line 60. The corrected native attribution requires that real mutated state and exact original failure line.

## Local helper verification

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
python3 -m unittest discover -s rust/reference-tools/messaging -p '*test.py'
node --test --test-concurrency=4 rust/reference-tools/messaging/*.test.mjs
```

```text
WS8bm system inventory: 135 named declarations; 135 mapped behaviour passes; 0 deferred; 0 WS12 blocked; no pixel checks
Ran 41 tests in 2.624s
OK
ℹ tests 69
ℹ pass 69
ℹ fail 0
ℹ skipped 0
```

Fresh-clone verification and final resource audit are added below after execution.

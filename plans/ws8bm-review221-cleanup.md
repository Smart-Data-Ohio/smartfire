# WS8bm #221: diagnostic failure cannot skip server teardown

Review baseline `2e7116509b22ab948ab8159e4d9e41c6e7549e23`; fixed tools source `b2d7369d8`; branch `rust/ws8bm-messages-http-2`. Rails remains pinned to `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift. Changes are messaging reference tools and reports only. Product code, Rails JavaScript, vectors, masks, deadlines, ignores and concurrency are unchanged. Inventory remains 156/156 controller declarations and 118 passed / 17 deferred / 0 owner-blocked system declarations; this review credits no declaration.

## Fix and diagnostic audit

The producer replay's readback is inside a `try` whose `finally` runs the shared server cleanup. An exception escaping any diagnostic cannot bypass shutdown. The shared cleanup itself puts Rails/container teardown in `finally` after attempting Rust shutdown; the audit found that the former inline Rust terminate/wait block could also skip Rails shutdown on an error. The original 15-second graceful-shutdown deadline is unchanged. Rust shutdown falls back to kill/wait on timeout or an OS error.

`behavior_work_diagnostics.py` reads each app independently and closes both read-only SQLite connections explicitly. An absent original message is represented as `history_client_id: null` and passed to the unchanged `assert_work_rows` assertion. The actual `work-history` lookup then fails at the original identity assertion and is recorded as `FAIL`, not a TypeError. Unexpected query/setup errors are recorded as `INVALID`; the peer readback still runs, and neither an invalid diagnostic nor missing paired evidence earns rejection credit.

The three reviewer producer variants—wrong event type, wrong actor and displaced history identity—are now registered alongside the original five. They mutate actual producers in both apps and require their intended assertion plus the persisted mutated state. Existing event-count, permission and agent-event attribution remains intact.

| Messaging readback/diagnostic path | Audit result |
| --- | --- |
| `work-producer-discrimination.py` generated `READBACK` | **Fixed:** diagnostics are nested inside unconditional server teardown; missing history is nullable evidence; operational errors are invalid and do not skip the peer. |
| `behavior-check.py` persisted SQL readbacks | Already inside the protected server `try/finally`. **Additional cleanup fix:** shared teardown always attempts Rails/container shutdown after the Rust terminate/wait/kill path. No persisted-row assertion is weakened. |
| `behavior.mjs` thread-create, continuation, attachment-preview, PR discussion and probe-observer diagnostics | Inside the application attempt's `try/catch`, with context cleanup in `finally` and browser shutdown in the outer `finally`. Diagnostic errors cannot bypass browser teardown or the Python server `finally`; unchanged. |
| `behavior_action_rows.py`, `behavior_thread_rows.py`, `behavior_work_rows.py` | Called inside protected parent readback scopes; they start no servers and own no teardown. History absence already raises the intended identity assertion in `behavior_work_rows.py`; unchanged. Aggregate `COUNT(*)` queries return a row even when their count is zero. |
| Messaging compiled-mutation scripts (`discriminate`, `root`, `features`, `continuation`, `recipient`, `upload`, `lifecycle`, `check-owned-mutations`, `work-model`, `work-global-model`) | Their `finally` blocks restore source; no SQL/diagnostic template precedes server teardown in those blocks. Cargo owns each test process, and the global-model replay separately verifies its restored control; unchanged. |

The four new unit regressions execute the **actual generated driver finally AST** with real temporary SQLite databases. They cover deleted history, an unexpected schema/query error with a still-observed peer, an uncaught diagnostic exception, and a Rust terminate error. The first three tests present before the fix all fail against `2e711650`; the fixed suite includes the additional uncaught-exception case. None reads a prior scratch fixture or target.

## Failing first: real deleted-history producer

The own no-hardlinks verification clone is first checked out at `2e711650`. Its target was absent and is rebuilt by the ordinary producer bootstrap. The reviewer's Ruby producer body is copied read-only into an own temporary initializer: after each real Rails work update it rewrites and destroys the original history message. Rust uses the same registered client-ID corruption as the reviewer reproduction. Browser responses and assertions are unchanged. PID logging is diagnostic only and earns no declaration credit. The generated probe uses a private `ws8bm-cleanup221` namespace and own ports 52020–52022.

At the baseline, the original identity assertion fails, then `fetchone()[0]` raises TypeError inside the diagnostic `finally`. The process/container snapshot is taken **before** an independent supervisor cleans the deliberately leaked resources. After the fix, the same missing message yields `FAIL` at `work-history identity:`; the driver cleans every resource before the supervisor runs. No supervisor action supplies cleanup credit to the fixed driver.

```text
WS8bm history cleanup before: {"driver_exit": 1, "pids": [4124709, 4123773], "remaining_processes": [4124709, 4123773], "listeners": [52020, 52021, 52022], "containers": [{"id": "8a14a8748eb3", "name": "ws8bm-cleanup221-reference-52020"}], "identity_assertion_failed": true, "readback_type_error": true, "row_receipts": []}
WS8bm history cleanup supervisor: {"processes": [], "listeners": [], "containers": []}
WS8bm history cleanup proof: baseline leak reproduced; supervisor cleaned owned resources
WS8bm history cleanup after: {"driver_exit": 1, "pids": [95859, 95478], "remaining_processes": [], "listeners": [], "containers": [], "identity_assertion_failed": true, "readback_type_error": false, "row_receipts": [{"app": "Rails", "producer": "rewrite-history-client-id", "thread": ["planned", 712064548], "events": [[773523953, "work_update", null, "planned", null, null], [773523953, "work_assignment", "planned", "planned", null, 712064548]], "agent_events": 0, "history_client_id": null, "original_history_id": 935962058, "history_lookup": null, "global_event_delta": 2, "target_event_delta": 2, "row_assertion": "FAIL", "row_error": "work-history identity: expected message 935962058; lookup returned None"}, {"app": "Rust", "producer": "rewrite-history-client-id", "thread": ["planned", 712064548], "events": [[773523953, "work_update", null, "planned", null, null], [773523953, "work_assignment", "planned", "planned", null, 712064548]], "agent_events": 0, "history_client_id": "corrupted-work-history", "original_history_id": 935962058, "history_lookup": null, "global_event_delta": 2, "target_event_delta": 2, "row_assertion": "FAIL", "row_error": "work-history identity: expected message 935962058; lookup returned None"}]}
WS8bm history cleanup supervisor: {"processes": [], "listeners": [], "containers": []}
WS8bm history cleanup proof: intended rejection; driver cleaned all owned resources
```

## Verification commands and receipts

All verification uses the independent clone `.scratch/ws8bm-pr2-review/fresh` at the fixed tools source, with `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, its own absolute `CARGO_TARGET_DIR`, `CAMPFIRE_REFERENCE` pointing at the clone, and `TMPDIR` under its `.scratch`. The configured machine-wide rustc throttle remains enabled. The producer bootstrap constructs seeds, binaries, npm/browser inputs and the server image from tracked source. Baseline probe setup errors in temporary script path/namespace routing were corrected and receive no credit; the valid before snapshot below includes all leaked resources and its supervisor's cleanup.

```sh
python3 rust/reference-tools/messaging/work-producer-discrimination.py
python3 rust/reference-tools/messaging/behavior-check.py channel_threads_controller --keep-going
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
node --test rust/reference-tools/messaging/behavior-*.test.mjs
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

The own probe wrapper invokes the actual generated driver once at each source. It changes only the producer initializer, private resource namespace and PID logging, retaining the committed SQL assertion and diagnostic/teardown templates:

```sh
python3 .scratch/ws8bm-cleanup221/run-history-cleanup.py --root /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh --baseline
python3 .scratch/ws8bm-cleanup221/run-history-cleanup.py --root /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh
```

```text
WS8bm real producer discrimination: missing-history: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: allow-reassignment: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: missing-agent-events: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: extra-foreign-event: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: rewrite-history-client-id: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: wrong-event-type: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: wrong-event-actor: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: displaced-history-identity: REJECTED on Rails and Rust at intended assertion
WS8bm producer discrimination check: 8 paired proofs; 0 invalid or unexpected; no escapes accepted
WS8bm behaviour check: 8 named cases passed on Rails and Rust; 0 failed; no pixel checks
............................
----------------------------------------------------------------------
Ran 28 tests in 2.237s

OK
ℹ tests 36
ℹ suites 0
ℹ pass 36
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 4036.729008
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 15s
``` The canonical app/media suite from the previous checkpoint is not rerun: this change touches reference tools only, and the requested Rust check is strict clippy.

## Checkpoint and remaining work

Both previous P2 assertion fixes remain intact. The additional teardown defect is fixed. The exact seventeen deferred system declarations stay in [the primary report](ws8bm-report.md#exact-remaining-system-work); no owner-blocked declaration remains. The sole own scratch target was removed after verification. Final resource enumeration checks both the ordinary and private probe namespaces, the own fresh-clone server/forwarder commands, and ports 52020–52023. Both the live fixed-probe snapshot (before its supervisor) and the final inventory list no processes or containers. Raw logs and sources are preserved. Work stops after the push and a repeated final cleanup check.

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml --target-dir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh/rust/target
python3 .scratch/ws8bm-cleanup221/resource-check.py
```

```text
     Removed 12200 files, 10.3GiB total
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "scratch_targets": []}
```

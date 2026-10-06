# WS13 committed presence slice

Partial WS13 delivery. Grant issuance/reuse, revocation and successful leave describe presence after commit; gateway first-sighting jobs render the current committed roster. The application cable sink renders sidebar once, fans it out to memberships, and publishes the header once through WS7. A rolled-back revocation publishes nothing. Missing grants/rooms and unconfigured huddles are silent. Boot retries only BroadcastPresenceJob rows failed with the exact previous unknown-handler error, preserving actual rendering failures.

The single participants partial matches 50 pinned Rails renders (five room types, two placements, five roster sizes, escaped names, both avatar limits). The socket test uses the production seeded app/router and real WebSockets in WS13's port range. Other grant callbacks, join/push handlers, full invitation and stream/stage effects remain subsequent slices. No other huddle HTML parity is claimed.

Commands run from the WS13 worktree:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db -p campfire_views -- --test-threads=4 --nocapture > .scratch/presence-slice-tests.log 2>&1
```

Raw summary, app/DB/views/views integration/docs in order (explicit ignores retained):

```text
test result: ok. 330 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 30.05s
test result: ok. 418 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 48.69s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/presence-slice-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.43s
```

First-failure tests before implementation: the DB callback test failed `leave did not emit committed presence`; the worker test failed on the retained unknown-handler row; the 50-render comparison failed against the empty render. Initial compile/harness corrections were not counted as evidence. The socket test rejects these compiled regressions (sources restored before clippy):

```sh
python3 rust/reference-tools/huddle_discrimination.py presence-before-commit presence-worker-bypassed presence-sink-bypassed presence-recovery-disabled > .scratch/presence-slice-discrimination.log 2>&1
```

```text
presence-before-commit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.16s
presence-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 333 filtered out; finished in 3.14s
presence-sink-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 333 filtered out; finished in 3.11s
presence-recovery-disabled: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.11s
WS13 discrimination: 4 compiled regressions detected; sources restored
```

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_presence.rb > .scratch/huddle-presence.json 2> .scratch/huddle-presence.log
```

Exit 0, 50 cases. Only the oracle's own database setup and external fixtures are controlled; the Rails render and helpers are unchanged. No schema, Rails, sidecar, dependency, mask or allowlist changes.

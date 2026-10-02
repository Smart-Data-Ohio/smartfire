# WS11-api wave 4 — PARTIAL PR handoff after main merge, 2026-10-02

Merge commit: `94452491c6e5e8f9128649d22fbe7b9c4ee2e609` (parents dc4d22c1 and origin/main `3ab3a4db58e094cea0f41b90e63f54d59ce9309e`). Verified code head: `3707f6f1cf2339e0937d60aee5af0fb3461c0e98` on `rust/ws11api-rest-mcp`; the report-only commit follows. This supersedes the previous report's verification section. No WS12 branch merge, stash or rebase occurred.

## PR scope: complete versus flagged

The branch adds the agent REST surface and stateless Streamable HTTP MCP transport with four protocol versions, all 38 tool names, credential/grant/owner authorization, shared throttling and exact error/Retry-After responses for the selected matrices. Selected success coverage remains **31/35 REST actions and 33/38 MCP tools**. The committed corpus has **1,432 Rails request/response vectors in 17 artifacts**, including 192 readers, 50 legacy bot/fanout/replacement cases, polling, permission changes, attachments and concurrent bot budget/idempotency checks. Responses compare complete JSON bytes, statuses and the explicitly selected headers. These are selected matrices, not a claim of exhaustive validation or every possible header. WS14g's previously exposed polling HTTP shape remains present.

**Nine WS12 write paths remain flagged:** REST board create and work update/result/handoff; MCP create_board_post, update_board_post, update_work, set_result and handoff_work. Their adapters retain the explicit `agent_api_pending::execute` seam after the implemented authorization/validation paths. No successful domain result is fabricated. The published WS12 contract was read in the earlier continuation; its branch was not merged. Main at 3ab3a4db has the WS12a activity domain, but not the agent-work services required to replace these adapters. Main's WS15g repository-access seam is unchanged; this branch adds no live private GitHub access implementation.

**Not only owner-blocked items remain.** Exhaustive input/ID/array/hash/coercion, length and callback-precedence matrices remain for other endpoint/tool families, context limits and IDs, remaining work/filter/cursor cases, compact/partial dates and broader zone/DST grammar. Remaining bot/media coverage includes legacy boosts without Agent rows, multi-hop agent/legacy bounce and reply-source chains, root/thread replay and attachment callback interactions, unshared purge execution and additional MIME/representation/viewer/cache permutations. Permission work still includes concurrent revocation versus writes/finalization, remaining viewer and credential/account snapshots, and delivery/finalization failure interactions. External delivery/network behavior remains with its domain/integration owners.

## Merge changes, by file

- `crates/campfire/src/controllers.rs`: preserve both the agent API endpoint/test registration and main's reviewed Slack endpoint list. The content conflict joined the two lists.
- `crates/views/src/messages.rs`: retain main's reviewed ReactionsPartial and merge the competing documentation into one comment. The actual rendering implementation is main's.
- `crates/campfire/src/controllers/agent_fizzy_tests.rs`: replace the literal fixture bearer header with `format!` at test time. Rails' error message text is unchanged.
- `crates/campfire/src/controllers/messages/boosts.rs`: remove the duplicate reaction renderer left by the clean merge and expose main's reviewed renderer to human, bot and agent callers. Remove obsolete create/append helpers that became unused after both sides moved to grouped reactions.
- `crates/campfire/src/channels/broadcasts.rs` and `controllers/presenters/page.rs`: preserve the legacy append-frame primitives and their existing cable wire tests under cfg(test), following the existing test-only legacy remove primitive. Production no longer carries those unused members. No test or assertion was removed or ignored.
- Main's WS16, WS8b-m, #186 release-input guard and WS12a activity work are retained. Cargo.lock merged without a regeneration; locked metadata passes. WS11 models/domain APIs remain the shared implementation. No additional domain stand-in was added.

The integration commits are 98f38b7f (shared main renderer), 3a0a0dbe (obsolete helper removal) and 3707f6f1 (test-only legacy frame primitives), following merge 94452491.

## Failing-first and deferred Rails cases

The earlier branch contains committed negative-proof runners for credential, grant, owner, protocol/header/origin, throttle, attachment rollback, private payload and bot fanout/budget guards. Those historical mutation runs are not re-claimed as newly executed here; their raw evidence remains in the report at dc4d22c1. No new security assertion or policy guard was introduced in this merge task. The initial strict clippy run rejected three unused legacy append members; they were made test-only and strict clippy was rerun successfully.

The full Rust suite reruns all 77 API/MCP groups. This turn did not rerun the Rails controller-file runner or claim new one-for-one file pass counts. The 17 oracle captures below reran against pinned Rails. Selected contexts/work/posts tests cover readers and authority changes; remaining work writes/callbacks stay with WS12 and exhaustive context/permission cases remain API. Capability/revocation/deactivation cases still defer the races listed above. By-bots and bot-boost cases still defer the legacy/media cases listed above. Poll cases still defer the broader date/zone/input grammar. These deferrals are unchanged by the merge.

## Fresh clone, seeds and locked metadata

All commands below were rerun from the assigned worktree. The clone was created at the merge, then fetched and checked out at the final integration commit before the reported full rerun. Both seeds were built in that clone from d7c7de92. Cargo used two jobs, the existing machine-wide rustc throttle and at most eight test threads. One extra target directory was used, then deleted.

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/fresh10
git -C .scratch/fresh10 fetch origin
git -C .scratch/fresh10 checkout --detach origin/rust/ws11api-rest-mcp
PARITY_NAMESPACE=ws11api-fresh10 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh10/rust/parity/bin/seed build default first_run > .scratch/merge-main/logs/seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

```bash
CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/merge-main-tmp" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh10/rust/Cargo.toml --format-version 1 > .scratch/merge-main/logs/fresh-metadata-final.json
```


Locked metadata exited 0; its JSON output is retained in the cited log.

## Full workspace tests and compiled vectors

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh10/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/merge-main/logs/workspace-final.log 2>&1
python3 rust/reference-tools/agents/summarize-http-tests.py .scratch/merge-main/logs/workspace-final.log > .scratch/merge-main/logs/workspace-summary.log
```

```text
WS11-api cargo totals: 3958 passed; 2 failed; 12 ignored; 58 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice

test result: FAILED. 2058 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 643.57s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.84s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1194 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 112.61s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 17.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.89s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.43s
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

The native command exits 101 with exactly two failures: account-logo PNG bytes and the storage pinned-media version gate. Both pass with the same final test executables in the pinned runtime below. This is not a claim that the entire workspace was rerun inside Docker. There are 12 explicit ignored tests and zero actual seed skips; the one missing-seed notice belongs to the intentional missing-seed unit test.

The named API/MCP groups were counted from that complete log (all passed):

```bash
python3 - <<'PY'
from pathlib import Path
import re
s=Path('.scratch/merge-main/logs/workspace-final.log').read_text()
rows=re.findall(r'^test controllers::(?:agent_[^ ]+|bot_http_tests::[^ ]+) \.\.\. (ok|FAILED|ignored)',s,re.M)
assert rows and 'FAILED' not in rows and 'ignored' not in rows
print(f'WS11-api compiled HTTP/MCP groups: {rows.count("ok")} passed; 0 failed; 0 ignored')
Path('.scratch/merge-main/logs/api-groups.log').write_text(f'WS11-api compiled HTTP/MCP groups: {rows.count("ok")} passed; 0 failed; 0 ignored\n')
PY
```

```text
WS11-api compiled HTTP/MCP groups: 77 passed; 0 failed; 0 ignored
```

## Native media exception, verified in the pin

```bash
vips --version > .scratch/merge-main/logs/native-vips.log 2>&1
ffmpeg -version > .scratch/merge-main/logs/native-ffmpeg.log 2>&1
docker run --rm --name ws11api-merge10-vips-version --network none --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"' > .scratch/merge-main/logs/pinned-vips.log 2>&1
docker run --rm --name ws11api-merge10-ffmpeg-version --network none --entrypoint /usr/bin/ffmpeg ws11api-reference:d7c7de92 -version > .scratch/merge-main/logs/pinned-ffmpeg.log 2>&1
```

```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

```bash
docker run --rm --name ws11api-fresh10-pinned-logo --network none --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/merge-main-tmp" -v "$PWD/.scratch/fresh10:$PWD/.scratch/fresh10" -v "$PWD/.scratch/merge-main-tmp:$PWD/.scratch/merge-main-tmp" --entrypoint "$PWD/.scratch/fresh10/rust/target/debug/deps/campfire-0ffdcc3282cd8417" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/merge-main/logs/pinned-logo.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2061 filtered out; finished in 1.77s
```

```bash
docker run --rm --name ws11api-fresh10-pinned-storage --network none --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/merge-main-tmp" -v "$PWD/.scratch/fresh10:$PWD/.scratch/fresh10:ro" -v "$PWD/.scratch/merge-main-tmp:$PWD/.scratch/merge-main-tmp" --entrypoint "$PWD/.scratch/fresh10/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/merge-main/logs/pinned-storage.log 2>&1
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.87s
```

## Strict clippy and release inputs

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh10/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/merge-main/logs/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.85s
```

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main-tmp" CARGO_TARGET_DIR="$PWD/.scratch/fresh10/rust/target" mise exec rust@1.98.1 -- bash .scratch/fresh10/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/merge-main/logs/release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
```

The release-input guard built from only Cargo manifests/lock, crates and the explicit Docker asset context; no external vectors, parity directory or other Rails files were available. Its temporary input copy was removed by the script. Both commands exited 0.

## Fresh Rails vectors and verifier checks

```bash
PARITY_NAMESPACE=ws11api-merge10 RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/record-http-vectors.py .scratch/merge-main/oracles > .scratch/merge-main/logs/record-oracles.log 2>&1
python3 rust/reference-tools/agents/verify-http-vectors.py .scratch/merge-main/oracles > .scratch/merge-main/logs/vectors.log 2>&1
RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/check-http-reference.py > .scratch/merge-main/logs/reference.log 2>&1
RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/test-http-vector-verifier.py > .scratch/merge-main/logs/verifier.log 2>&1
```

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 269 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 71 request/response pairs; byte-identical committed vectors
WS11-api fresh legacy bot/fanout/replacement oracle: 50 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh readers oracle: 192 request/response pairs; byte-identical committed vectors
WS11-api fresh pins oracle: 18 request/response pairs; byte-identical committed vectors
WS11-api fresh polls oracle: 168 request/response pairs; byte-identical committed vectors
WS11-api fresh polling oracle: 12 request/response pairs; byte-identical committed vectors
WS11-api fresh reactions oracle: 44 request/response pairs; byte-identical committed vectors
WS11-api fresh bot reactions oracle: 13 request/response pairs; byte-identical committed vectors
WS11-api fresh work validation oracle: 99 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.011s

OK
```

All 17 captures exited 0 and match committed artifacts byte for byte. All 84 pinned source files match image and checkout. The verifier negative checks pass. No golden, mask, timing threshold, assertion or ignored-test list was loosened.

## Cleanup and stopping point

After every owned process exited, the single 21G Cargo target `.scratch/fresh10/rust/target` was deleted with a guarded Python removal that checked its exact owned path, Cargo marker and absence of users. The process and Cargo-target inventory was checked again. Logs/oracles/source clones are retained under `.scratch/merge-main` and `.scratch/fresh10`. No WS11-api Docker container remains. The Python model server was untouched.

```text
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```

The requested merge/verification handoff is complete. API parity remains partial with both owner-blocked and unblocked items explicitly listed above; no further feature work was undertaken in this stop-and-report task.

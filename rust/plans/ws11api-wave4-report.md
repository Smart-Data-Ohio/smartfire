# WS11-api wave 4 — PARTIAL PR handoff after Google main merge, 2026-10-02

Merge commit and verified code head: `dee593271824d68d413375d535a8af33d0e2f084` (parents `b1324ce621b421c14f8e98f7964e28884b20ece2` and origin/main `0700e59d43cb0a0122036a6c3a8ea06ad3d9fc1a`) on `rust/ws11api-rest-mcp`. The report-only commit follows. This supersedes the previous report's verification section. No WS12 branch merge, stash or rebase occurred.

## PR scope: complete versus flagged

The branch adds the agent REST surface and stateless Streamable HTTP MCP transport with four protocol versions, all 38 tool names, credential/grant/owner authorization, shared throttling and exact error/Retry-After responses for the selected matrices. Selected success coverage remains **31/35 REST actions and 33/38 MCP tools**. The committed corpus has **1,432 Rails request/response vectors in 17 artifacts**, including 192 readers, 50 legacy bot/fanout/replacement cases, polling, permission changes, attachments and concurrent bot budget/idempotency checks. Responses compare complete JSON bytes, statuses and the explicitly selected headers. These are selected matrices, not a claim of exhaustive validation or every possible header. WS14g's previously exposed polling HTTP shape remains present.

**Nine WS12 write paths remain flagged:** REST board create and work update/result/handoff; MCP create_board_post, update_board_post, update_work, set_result and handoff_work. Their adapters retain the explicit `agent_api_pending::execute` seam after the implemented authorization/validation paths. No successful domain result is fabricated. The published WS12 contract was read in the earlier continuation; its branch was not merged. Main at 0700e59d has the WS12a activity domain, but not the agent-work services required to replace these adapters. Main's WS15g repository-access seam is unchanged; this branch adds no live private GitHub access implementation.

**Not only owner-blocked items remain.** Exhaustive input/ID/array/hash/coercion, length and callback-precedence matrices remain for other endpoint/tool families, context limits and IDs, remaining work/filter/cursor cases, compact/partial dates and broader zone/DST grammar. Remaining bot/media coverage includes legacy boosts without Agent rows, multi-hop agent/legacy bounce and reply-source chains, root/thread replay and attachment callback interactions, unshared purge execution and additional MIME/representation/viewer/cache permutations. Permission work still includes concurrent revocation versus writes/finalization, remaining viewer and credential/account snapshots, and delivery/finalization failure interactions. External delivery/network behavior remains with its domain/integration owners.

## Merge changes, by file

- `crates/campfire/src/app.rs`: retain main's Google state, error reporter, Calendar notification routes and sign-in/Calendar boot wiring alongside this branch's Fizzy state, BootIntegrations test injection and MCP raw-body routes. Keep main's relocated CSP routes once; remove the duplicate registrations from the conflicting block. No endpoint or test was removed.
- `crates/campfire/src/integrations/web_push/tests.rs`: both explicit AppState fixtures retain Fizzy and clone main's Google/error fields.
- `crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs`: the explicit AppState fixture retains Fizzy and clones main's Google/error fields.
- The presenter test-support merge retains Fizzy fixture injection and main's Google changes. All explicit AppState constructors were checked. Main's reviewed #190 Google sign-in, Calendar, Drive and consumer/domain changes remain present.
- Cargo.lock merged without regeneration; locked metadata passes. Shared WS11 model/domain services, the WS12 pending adapters and main's WS15g repository-access seam are retained. No duplicate domain implementation was added. The fixture bearer header remains built with `format!` at test time.

## Failing-first and deferred Rails cases

The earlier branch contains committed negative-proof runners for credential, grant, owner, protocol/header/origin, throttle, attachment rollback, private payload and bot fanout/budget guards. Those historical mutation runs are not re-claimed as newly executed here; their raw evidence remains in the report at dc4d22c1. No new security assertion or policy guard was introduced in this merge task. Strict clippy passed without an integration fix in this turn.

The full Rust suite reruns all 77 API/MCP groups. This turn did not rerun the Rails controller-file runner or claim new one-for-one file pass counts. The 17 oracle captures below reran against pinned Rails. Selected contexts/work/posts tests cover readers and authority changes; remaining work writes/callbacks stay with WS12 and exhaustive context/permission cases remain API. Capability/revocation/deactivation cases still defer the races listed above. By-bots and bot-boost cases still defer the legacy/media cases listed above. Poll cases still defer the broader date/zone/input grammar. These deferrals are unchanged by the merge.

## Fresh clone, seeds and locked metadata

All commands below were rerun from the assigned worktree. A fresh clone was created at the merge commit; no code changes followed it. Both seeds were built in that clone from d7c7de92. Cargo used two jobs, the existing machine-wide rustc throttle and at most eight test threads. One extra target directory was used, then deleted.

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/fresh11
PARITY_NAMESPACE=ws11api-fresh11 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh11/rust/parity/bin/seed build default first_run > .scratch/merge-main2/logs/seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

```bash
CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/merge-main2-tmp" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh11/rust/Cargo.toml --format-version 1 > .scratch/merge-main2/logs/fresh-metadata.json
```

Locked metadata exited 0; its JSON output is retained in the cited log.

## Full workspace tests and compiled vectors

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main2-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh11/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/merge-main2/logs/workspace.log 2>&1
python3 rust/reference-tools/agents/summarize-http-tests.py .scratch/merge-main2/logs/workspace.log > .scratch/merge-main2/logs/workspace-summary.log
```

```text
WS11-api cargo totals: 4104 passed; 2 failed; 14 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice

test result: FAILED. 2168 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 464.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.25s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1219 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 117.64s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.87s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.96s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.96s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
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

The native command exits 101 with exactly two failures: account-logo PNG bytes and the storage pinned-media version gate. Both pass with the same final test executables in the pinned runtime below. This is not a claim that the entire workspace was rerun inside Docker. Explicit ignored tests and seed notices are counted in the raw aggregate above; no test or timing requirement was weakened.

```bash
python3 - <<'PY'
from pathlib import Path
import re
s=Path('.scratch/merge-main2/logs/workspace.log').read_text()
rows=re.findall(r'^test controllers::(?:agent_[^ ]+|bot_http_tests::[^ ]+) \.\.\. (ok|FAILED|ignored)',s,re.M)
assert rows and 'FAILED' not in rows and 'ignored' not in rows
line=f'WS11-api compiled HTTP/MCP groups: {rows.count("ok")} passed; 0 failed; 0 ignored'
print(line)
Path('.scratch/merge-main2/logs/api-groups.log').write_text(line+'\n')
PY
```

```text
WS11-api compiled HTTP/MCP groups: 77 passed; 0 failed; 0 ignored
```

Main marks two WS14g polling HTTP comparisons as pending this API. Both were run explicitly against the fresh-clone executable and passed. Their ignore annotations were left unchanged in this merge task; they are included in the workspace ignored count above and verified separately below.

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main2-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- "$PWD/.scratch/fresh11/rust/target/debug/deps/campfire-4cda423f68651c03" integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http --ignored --nocapture --test-threads=8 > .scratch/merge-main2/logs/ws14g-polling.log 2>&1
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2172 filtered out; finished in 1.06s
```

## Native media exception, verified in the pin

```bash
vips --version > .scratch/merge-main2/logs/native-vips.log 2>&1
ffmpeg -version > .scratch/merge-main2/logs/native-ffmpeg.log 2>&1
docker run --rm --name ws11api-merge11-vips-version --network none --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"' > .scratch/merge-main2/logs/pinned-vips.log 2>&1
docker run --rm --name ws11api-merge11-ffmpeg-version --network none --entrypoint /usr/bin/ffmpeg ws11api-reference:d7c7de92 -version > .scratch/merge-main2/logs/pinned-ffmpeg.log 2>&1
```

```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

```bash
docker run --rm --name ws11api-fresh11-pinned-logo --network none --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/merge-main2-tmp" -v "$PWD/.scratch/fresh11:$PWD/.scratch/fresh11" -v "$PWD/.scratch/merge-main2-tmp:$PWD/.scratch/merge-main2-tmp" --entrypoint "$PWD/.scratch/fresh11/rust/target/debug/deps/campfire-4cda423f68651c03" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/merge-main2/logs/pinned-logo.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2173 filtered out; finished in 1.10s
```

```bash
docker run --rm --name ws11api-fresh11-pinned-storage --network none --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/merge-main2-tmp" -v "$PWD/.scratch/fresh11:$PWD/.scratch/fresh11:ro" -v "$PWD/.scratch/merge-main2-tmp:$PWD/.scratch/merge-main2-tmp" --entrypoint "$PWD/.scratch/fresh11/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/merge-main2/logs/pinned-storage.log 2>&1
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.04s
```

## Strict clippy and release inputs

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main2-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh11/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/merge-main2/logs/clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 46s
```

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/merge-main2-tmp" CARGO_TARGET_DIR="$PWD/.scratch/fresh11/rust/target" mise exec rust@1.98.1 -- bash .scratch/fresh11/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/merge-main2/logs/release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 23s
```

The release-input guard built from only Cargo manifests/lock, crates and the explicit Docker asset context; no external vectors, parity directory or other Rails files were available. This is the script's dev-profile binary build. Its temporary input copy was removed by the script. Both commands exited 0.

## Fresh Rails vectors and verifier checks

```bash
PARITY_NAMESPACE=ws11api-merge11 RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/record-http-vectors.py .scratch/merge-main2/oracles > .scratch/merge-main2/logs/record-oracles.log 2>&1
python3 rust/reference-tools/agents/verify-http-vectors.py .scratch/merge-main2/oracles > .scratch/merge-main2/logs/vectors.log 2>&1
RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/check-http-reference.py > .scratch/merge-main2/logs/reference.log 2>&1
RUST_TEST_THREADS=8 python3 rust/reference-tools/agents/test-http-vector-verifier.py > .scratch/merge-main2/logs/verifier.log 2>&1
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
Ran 7 tests in 0.005s

OK
```

All 17 captures exited 0 and match committed artifacts byte for byte. All 84 pinned source files match image and checkout. The verifier negative checks pass. No golden, mask, timing threshold, assertion or ignored-test list was loosened.

## Cleanup and stopping point

After every owned process exited, the single 20G Cargo target `.scratch/fresh11/rust/target` was deleted with a guarded Python removal that checked its exact owned path, Cargo marker and absence of users. The process and Cargo-target inventory was checked again. Logs/oracles/source clones are retained under `.scratch/merge-main2` and `.scratch/fresh11`. No WS11-api Docker container remains. The Python model server was untouched.

```text
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```

The requested merge/verification handoff is complete. API parity remains partial with both owner-blocked and unblocked items explicitly listed above; no further feature work was undertaken in this stop-and-report task.

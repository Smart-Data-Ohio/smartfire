# WS16 Wave 4 — owned scope complete; Google claim interaction owner-blocked

All requested WS16-owned continuation work is complete. **Only the first-Google-sign-in placeholder-claim interaction remains blocked on WS14g.** The lead can open the WS16 PR with that dependency stated explicitly. This report supersedes the prior partial report; the declaration ledger has 244 covered, zero partial and zero deferred original Rails tests. Covered means executable behavior coverage, not that the original Ruby test classes ran against Rust.

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws16`; branch `rust/ws16-slack-import`. Main `2b05160758307effa035b7d2581618014c46b755` was merged first with merge commit `b4ed4e87`, including #172, #177, #179 and #180. No stash or rebase. Pushed slices: `dca7647a` closes complete run HTTP bodies and the personal Slack interaction; `906efac51ac6b50762b20bd84407bff5f599b5ed` closes the lifecycle/model/converter/client/payload cases. The final reply identifies the pushed report-only SHA.

Rails reference: `d7c7de9264c63015be398001d7a1094e7695a6db`. All 21 Slack source files match the actual reference image and remain unchanged on merged main, so the approved importer-undo drift contingency did not apply. Complete application pages use exactly the three approved `2e20b24c` inputs: application layout, `people.css`, and `profile_card_controller.js`. `reference-tools/slack/build_approved_reference.py` obtains those bytes with `git show`, overlays the pinned image and recompiles Rails assets. No Rust output supplied a golden; no response mask, comparison exclusion or fingerprint allowlist was added.

## What changed

| Files under `rust/` | Behavior and evidence |
|---|---|
| `crates/campfire/src/integrations/slack/store/lifecycle_tests.rs`, runner/store/undoer tests | The remaining 13 lifecycle and four workspace declarations: nonmember mention fallback; deleted mapped room; large-group automatic/explicit/personal/dry targets; personal target exclusion; room/membership rollback and successful resumption; exact microsecond bounds on every history page; scope/auth/transient failure and retained mappings; public-only discovery; deleted queued parent; truncated reactors; completion/undo queue handoff. Tests execute native transactional stores and durable job functions over local recorded TLS fixtures. |
| `crates/campfire/src/integrations/slack/store.rs`, `jobs.rs`, runner and undoer tests | The remaining eight model declarations: heartbeat and owned lease refresh, unleased runner preserving another lease, undo heartbeat/lease save, lease-only uniqueness conflict versus actual progress, atomic undo claim after a queued run arrives, failed/cancelled handoff, cancelled finishing remaining stopped and successful finishing handing off. Production jobs still supply a lease token; the direct runner also supports Rails' no-token invocation. |
| `crates/db/src/models/slack_import.rs`, `crates/db/src/tests/slack_import_test.rs` | Extract the existing atomic undo UPDATE/enqueue seam, without removing the public eligibility/LIFO precheck. A separate stale-precheck test introduces a queued run before that UPDATE and verifies no status change or events. |
| `crates/campfire/src/integrations/slack/client.rs`, `client/tests.rs`, new `payload.rs`; mapper/converter/writer modules | Close the original network-retry declaration and malformed payload edges. Preserve Ruby access/coercion exceptions, their classes/messages, mapped-user validation order, HTTP-null distinctions, retry counts and unrescued TLS exceptions. Reuse the existing Ruby JSON comment parser. Actual Rails goldens contain 92 rows: 12 client roots, 23 converter inputs, 23 user-mapper inputs, 12 conversation inputs, 11 transport faults and 11 real status/body/retry cases. Actual native dial failure retries four times and retains the final Rails error text. This is the tested fixture boundary; it does not claim exhaustive parity for arbitrary malformed values or every platform-specific socket diagnostic. |
| `crates/campfire/src/integrations/slack/writer/tests.rs`, `reference-tools/slack/rendering_vectors.rb`, `vectors/slack/rendering.json` | Close all four real-save converter declarations. Four actual Rails persisted ActionText bodies and mentionee lists match byte for byte through the real native imported-message save; broadcasts remain absent. Nonmember mapped mentions remain tokens and unknown labels fall back. |
| `crates/campfire/src/controllers/slack/{runs,run_tests,tests}.rs`, `crates/views/src/slack.rs`, `crates/views/templates/slack/imports/index.html`, `reference-tools/slack/runs_http.rb`, `vectors/slack/runs_http.json` | All 109 action observations now compare the entire response body, including application wrapper and error bodies, through real TCP HTTP, signed sessions and CSRF middleware. This exposed and fixed the scoped `/users/me/profile` link and Rails' empty personal `head :not_found` response. Status, redirects, flash/session values, every run field, audit rows and durable queue requests remain checked. The combined personal interaction executes opt-in, password sudo, local-fixture Slack OAuth/state, preview, native import, progress, rejected CSRF undo, accepted undo and final page/data state. |
| `reference-tools/slack/{payload_vectors,check_lifecycle_mutations,build_approved_reference}.*`, vectors and `plans/ws16-test-inventory.md` | Reproducible Rails-only producers, deliberately failing guards and the complete declaration mapping. |

The HTTP byte claim is **109 complete response bodies**, plus 83 detached run template bodies and 11 setup bodies. Rendering CSRF/CSP entropy is supplied before rendering; request CSRF verification and signing remain real. Redirect/status/session semantics are compared separately; randomized encrypted Set-Cookie strings are not asserted byte-identical wire serialization. No substring-only body comparison or post-render rewrite is used.

Prior foundations remain exercised: durable single-worker `slack_import` queue, 25-second steps, Tier 2/3 pacing, Retry-After re-enqueue, 30-second stalled sweep, claimable placeholders, quiet bulk messages/threads/replies, data-deleting per-conversation LIFO undo naming the later importer, catch-up/finishing, token encryption/scopes and setup/manifest/OAuth. The thirteen workspace/personal/retained differentials still compare every field in 89 tables after import, undo and reimport. Retained variants are saved reply, poll reply, pending quoted reply, sent scheduled reply, foreign thread, room event/schedule, session, Google account, Google identity, password and placeholder authorship. Real huddle destruction callbacks, six-table Rails rows, events and rollback remain covered. Schema bookkeeping and FTS shadow exclusions are unchanged; encrypted columns remain compared.

## Declaration coverage and exact remaining dependency

All 30 previously partial/deferred declarations are now covered: 13 lifecycle, four workspace, one client, four converter and eight model cases. The full per-declaration mapping is `plans/ws16-test-inventory.md`.

| Rails file | Covered declarations |
|---|---:|
| Admin runs | 33 |
| Admin setup | 10 |
| Slack disconnect | 6 |
| Personal imports | 20 |
| OAuth | 22 |
| Lifecycle jobs | 29 |
| Workspace jobs | 34 |
| Client | 18 |
| Converter | 30 |
| SlackImport model | 41 |
| System workflow | 1 |
| Total | 244 |

**Owner-blocked, WS14g:** run the actual first Google sign-in start/callback for an imported, allowlisted active placeholder, verify the claim preserves its historical author/mapping, then complete the combined personal claim/opt-in/import/undo interaction against Rails. Merged main has no `sessions/google#...` handler bindings in `controllers.rs`; `controllers/sessions.rs` explicitly assigns those configured-provider/start/callback handlers to WS14g. Its branch is not merged here. Slack eligibility and session/Google-identity/password retention already have real row differentials; injecting those DB rows does not verify the missing first-sign-in callback. No WS16-owned implementation or original declaration remains deferred. No product ruling is requested.

Cross-workstream seams: WS1 encryption/signing, WS3 jobs, WS4 session/CSRF, WS5 rich text, merged users/accounts and huddles are consumed. The small DB undo-seam extraction retains the original transaction and guards. Main's cache and PR refresh fixes are included. No Google authentication implementation, real Slack/Google/LiveKit call, pixel work or Python model-server operation occurred.

## Failing-first evidence

Complete HTTP comparison failed before the two response fixes: the personal back link differed, then a foreign personal run returned the public 404 body instead of Rails' empty body. Both corrections pass every full HTTP body case afterward. The new mutation script separately changes malformed-root errors into null, erases the transport exception class, and changes the atomic undo exclusion so a queued run is admitted. Each must reach its own failing runtime assertion; compilation errors are not accepted as evidence. Its `finally` restores the exact source bytes, and final source cleanliness is checked.

## Fresh verification and exact commands

Independent HTTPS GitHub clone: `.scratch/ws16-continuation-final`, with no object alternates, at the executable source SHA above. Fresh pinned Rails `default` and `first_run` seeds validate 29 and four checks respectively. The user-mapper oracle uses `default` to preserve the fixture's existing user's bio and claim flag; all other producers use `first_run`. An initial regeneration with the wrong mapper seed correctly failed the unchanged-vector guard; correcting the input reproduces all golden bytes without updating any expected output.

Canonical image: `sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2`. Actual observed versions:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
libvips 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

Pinned Cargo commands run with Docker network `none`, uid 1000, four CPUs, offline/locked Cargo, `CI=1`, two build jobs, at most eight test threads, and both listener ranges 53300–53399. The wrapper mounts the host's `/tmp/rust-port-rustc-slots` and configured slot-count file; the compiler loop holds the same `flock` descriptor through rustc. The throttle is unchanged. Only one extra scratch Cargo target exists during verification; the worktree's normal `rust/target` is preserved.

The exact `.scratch/pinned-continuation.sh NAME COMMAND...` wrapper and compiler loop:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-continuation-final:/src" \
  -v "$root/.scratch/rustc-wrapper.sh:/rustc-wrapper:ro" \
  -v /tmp/rust-port-rustc-slots:/rustc-slots \
  -v /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
  -v /home/riels/.cargo/registry:/src/rust/.cargo-home/registry:ro \
  -w /src/rust --env CARGO_HOME=/src/rust/.cargo-home \
  --env RUSTC_WRAPPER=/rustc-wrapper --env TMPDIR=/src/tmp-review \
  --env CI=1 --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_TEST_DEBUG=0 --env CARGO_PROFILE_DEV_DEBUG=0 \
  --env RUST_TEST_THREADS=8 --env CABLE_TEST_PORT_RANGE=53300-53399 \
  --env INTEGRATION_TEST_PORT_RANGE=53300-53399 \
  sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 "$@"
```

```sh
#!/bin/bash
rustc="$1"; shift
case " $* " in *" --crate-name "*) ;; *) exec "$rustc" "$@" ;; esac
slots=$(cat /slot-count)
while :; do
  for ((i=0;i<slots;i++)); do
    exec {fd}>>"/rustc-slots/$i"
    flock -n "$fd" && exec "$rustc" "$@"
    exec {fd}>&-
  done
  sleep 0.3
done
```

Commands were actually rerun in this continuation; summarized raw output follows. Assigned worktree is the default cwd. Source/image checks use the fresh clone. The oracle orchestrator runs the real reference runner for `setup_views`, `connections_http`, `oauth_vectors`, `markdown_vectors`, `mapper_vectors`, `run_views`, `runs_http`, `options_vectors`, `client_vectors`, `undo_huddle`, `payload_vectors`, `rendering_vectors` and all thirteen `sequence_vectors` arguments listed above, then requires unchanged tracked vector bytes. Its environment is `PARITY_NAMESPACE=ws16-continuation-final`, `PARITY_OWNER=ws16`, the pinned image (approved-layout image for `runs_http` only), the fresh pinned Rails archive, clock `2026-03-02T16:00:00Z --freeze`, and the seed distinction above.

```sh
# Fresh GitHub clone; fetch main for the source drift check.
git clone --single-branch --branch rust/ws16-slack-import https://github.com/Smart-Data-Ohio/smartfire.git .scratch/ws16-continuation-final
git -C .scratch/ws16-continuation-final fetch origin main:refs/remotes/origin/main
# Populate rust/parity/.ci/reference from the exact pin using git archive.
# Source/image checks and inventory run from the fresh clone:
python3 rust/reference-tools/slack/check_reference.py
python3 rust/reference-tools/slack/build_approved_reference.py
python3 rust/reference-tools/slack/write_test_inventory.py
# From the assigned worktree:
PARITY_NAMESPACE=ws16-continuation-final PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-continuation-final/rust/parity/.ci/reference" .scratch/ws16-continuation-final/rust/parity/bin/seed build default first_run
python3 .scratch/ws16-continuation-verify-seeds.py
python3 .scratch/ws16-continuation-oracles.py
.scratch/pinned-continuation.sh ws16-continuation-metadata cargo metadata --offline --locked --no-deps --format-version 1
.scratch/pinned-continuation.sh ws16-continuation-workspace cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 .scratch/summarize-suite.py .scratch/continuation-workspace.log
.scratch/pinned-continuation.sh ws16-continuation-mutations env WS16_CARGO=cargo python3 reference-tools/slack/check_lifecycle_mutations.py
.scratch/pinned-continuation.sh ws16-continuation-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-continuation.sh ws16-continuation-release bash ci/with-release-inputs.sh cargo build --offline --locked --bin campfire
```

```text
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main

Slack Rails HTTP oracle: pinned d7c7de92 plus exactly three approved 2e20b24c layout inputs
Slack Rails test inventory: 244 covered, 0 partial, 0 deferred; 244 total
Locked/offline metadata: valid; workspace members: 13

seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)

Seed assertions (default, then first_run):
  "passed": 29,
  "failed": 0
WS16 fresh seed validated: default
  "passed": 4,
  "failed": 0
WS16 fresh seed validated: first_run

Oracle summary lines:
Slack setup views: 11 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack OAuth vectors: 41 real Rails exchange/revoke/team cases, 4 authorization URLs, manifest and signed state generated
Slack markdown vectors: 481 cases generated from Rails
Slack mapper vectors: 9 fixture users; preview, import and repeat recorded
Slack run views: 83 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack run HTTP oracle: 109 real Rails action cases with signed sessions and verified CSRF generated
Slack options oracle: 33 Ruby coercion and ISO time-bound cases generated
Slack client vectors: 32 Rails error and success mappings generated
Slack undo huddle oracle: real Rails User.destroy callbacks; all rows and fields in six affected tables generated
Slack payload edges: 12 client, 23 converter, 23 mapper, 12 conversation, 11 transport, 11 HTTP cases generated
Slack real-save rendering: 4 persisted body and mention goldens generated
Slack Rails sequence (workspace): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (personal): import -> undo -> reimport; 89 tables per snapshot; 18 recorded API requests
Slack Rails sequence (saved_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (poll_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (pending_quoted_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (sent_reply): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (foreign_thread): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (room_event_schedule): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_session): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_google_account): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_google_identity): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (claimed_password): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (placeholder_authorship): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack fresh-clone oracle regeneration: 25 producers; all vector bytes unchanged

Deliberately broken sources (expected failures; restored before clippy/build):
test integrations::slack::payload::tests::slack_malformed_payloads_match_actual_rails_classes_messages_and_results ... FAILED
test integrations::slack::client::tests::slack_client_transport_classes_messages_and_retryability_match_rails ... FAILED
test result: FAILED. 74 passed; 4 failed; 0 ignored; 0 measured; 1680 filtered out; finished in 12.19s
test tests::slack_import_test::slack_import_atomic_undo_claim_refuses_queue_arriving_after_precheck ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 981 filtered out; finished in 0.06s
Slack lifecycle mutation guards: malformed root, erased transport class and stale undo precheck rejected; sources restored

Final clippy and crates-only production build:
    Finished `dev` profile [unoptimized] target(s) in 59.14s
    Finished `dev` profile [unoptimized] target(s) in 1m 07s
```

The exact seed validator and regeneration orchestrators used above:

```python
from pathlib import Path
import subprocess, os
rust=Path.cwd()/'.scratch/ws16-continuation-final/rust'
env=dict(os.environ,PARITY_NAMESPACE='ws16-continuation-final',PARITY_OWNER='ws16',
 PARITY_IMAGE='ws16-reference:d7c7de92',CAMPFIRE_REFERENCE=str(rust/'parity/.ci/reference'))
for seed in ['default','first_run']:
 subprocess.run([str(rust/'parity/bin/reference'),'runner','--seed',seed,'--time',
  '2026-03-02T16:00:00Z','--freeze',str(rust/'reference-tools/campfire/verify_parity_seed.rb'),seed],env=env,check=True)
 print('WS16 fresh seed validated: '+seed,flush=True)
```

```python
from pathlib import Path
import os
import subprocess

root = Path.cwd() / '.scratch/ws16-continuation-final'
rust = root / 'rust'
env = dict(os.environ, PARITY_NAMESPACE='ws16-continuation-final', PARITY_OWNER='ws16',
           PARITY_IMAGE='ws16-reference:d7c7de92',
           CAMPFIRE_REFERENCE=str(rust / 'parity/.ci/reference'))
producers = [(name, []) for name in ['setup_views', 'connections_http', 'oauth_vectors',
    'markdown_vectors', 'mapper_vectors', 'run_views', 'runs_http', 'options_vectors',
    'client_vectors', 'undo_huddle', 'payload_vectors', 'rendering_vectors']]
producers += [('sequence_vectors', ['workspace']), ('sequence_vectors', ['personal'])]
producers += [('sequence_vectors', ['workspace', name]) for name in ['saved_reply',
    'poll_reply', 'pending_quoted_reply', 'sent_reply', 'foreign_thread',
    'room_event_schedule', 'claimed_session', 'claimed_google_account',
    'claimed_google_identity', 'claimed_password', 'placeholder_authorship']]
for name, args in producers:
    current_env = dict(env)
    if name == 'runs_http':
        current_env['PARITY_IMAGE'] = 'ws16-reference:d7c7de92-layout-2e20b24c'
    subprocess.run([str(rust / 'parity/bin/reference'), 'runner', '--seed', 'default' if name == 'mapper_vectors' else 'first_run',
        '--time', '2026-03-02T16:00:00Z', '--freeze',
        str(rust / f'reference-tools/slack/{name}.rb'), *args], env=current_env, check=True)
subprocess.run(['git', '-C', str(root), 'diff', '--exit-code', '--', 'rust/vectors/slack'], check=True)
print('Slack fresh-clone oracle regeneration: 25 producers; all vector bytes unchanged')
```

All final seeded cases actually ran under `CI=1`. The vendored third-party `html5ever` test package is excluded by the established workspace command; application crates remain included. The explicit ignored tests are listed in the raw suite output below. The huddle gateway Node harness is an upstream ignore; huddle callback/row/event/rollback coverage is exercised here, without claiming that ignored gateway harness ran.

```text
Pinned workspace totals: 58 summary blocks; 3435 passed; 0 failed; 12 ignored
Raw libtest summaries:
test result: ok. 1755 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 666.51s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.71s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 978 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 77.67s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.45s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.44s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.52s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
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
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.41s
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
Explicit ignored tests:
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

## Cleanup and remote state

After all gates passed, native Cargo cleanup removed the sole extra target. Removal after Cargo cleanup was limited to an empty target directory if one remained. Fixture/source/oracle artifacts and the normal worktree `rust/target` cache are preserved. No WS16 verification process, container or listener remains. The report-only commit is pushed and its remote SHA is read back before the final reply; executable source remains exactly the verified code SHA.

```sh
.scratch/pinned-continuation.sh ws16-continuation-clean cargo clean --offline --locked
```
```text
Removed 21020 files, 11.9GiB total

Scratch Cargo targets remaining: 0
Original worktree rust/target preserved: yes
WS16 verification containers running: 0
Listeners in WS16 port range: 0
```

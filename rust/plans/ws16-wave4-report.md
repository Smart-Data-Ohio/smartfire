# WS16 OAuth/setup continuation — PARTIAL

Branch: `rust/ws16-slack-import`. The OAuth/setup slice is `52ee3ff39888c80fbebb19a2d5aba3fbf49615dd`. Subsequent fixture and verification fixes are listed below. The final report-only commit follows the verified source; the worker reply supplies its pushed SHA.

OAuth, encrypted grant persistence, disconnect, administrator credential setup/removal, and the manifest are implemented. The setup template body matches eleven complete Rails bodies byte for byte. The previously delivered durable runner, mappers, quiet writer, destructive LIFO undo and workspace/personal 89-table comparisons remain in this branch.

**Acceptance remains partial. WS16-owned work remains; this is not the owner-blocked-only handoff.** Run creation/control pages, personal opt-in pages and the remaining domain/error/huddle regressions are still outstanding. This continuation stops at the coherent OAuth/setup slice permitted by the original brief. The exact named declarations and owner are in `rust/plans/ws16-test-inventory.md`.

No Rails source/schema, comparison masks or allowlists were changed. No real Slack calls, pixel comparisons, stash, rebase, PR or deployment were performed. Work stayed in the assigned worktree, except the requested external report. Fixture authorization headers are assembled at runtime.

## Pushed slices and reference

- `e26bcc389ac3eb76162fef859df03ee46796440e`: merge commit consuming `origin/main` at `434d1c14a0d47a6e6b5172404bcda2607530b167`, including #174/#175/#178. Locked/offline metadata passed. Preserve both the Slack and streaming periodic entries, main's legacy attachment/stream-finalization behavior, main's thread-deletion snapshot/actor behavior, and its shared User removal API. Imported deletion keeps its narrow quiet path.
- `52ee3ff39888c80fbebb19a2d5aba3fbf49615dd`: OAuth/setup/manifest slice, runtime security/request tests, actual Rails transport/HTTP/view producers and goldens.
- `6435bf979069afc86df7e8e7fb48fe95b2fbb84a`: construct the older sequence probe's authorization header at runtime; rerun both actual Rails sequences and retain their fresh encrypted initial rows/snapshots.

- `ab454008b5659b0b70e08414a2899ad41c44a9e0`: stop the merged thread-page producer fixture workers before queued-job inspection; retain every original assertion. The precise zero-versus-one failure was reproduced on pristine main before fixing the fixture.

Pinned Rails is `d7c7de9264c63015be398001d7a1094e7695a6db`. A fresh source check compares all 21 Slack model/job/controller files from the reference image with that pin and confirms no Slack drift on `origin/main` at `434d1c14`. Its LIFO-per-conversation, kept-parent, pending-scheduled-reply, finish-every-room and later-importer rules are consequently the reference here. Approved unrelated Rails drift is preserved through main.

## Changes by file

Paths are relative to `rust/`; `integrations/` and `controllers/` abbreviate the corresponding directories under `crates/campfire/src/`.

| Files | Change |
|---|---|
| `crates/campfire/src/integrations/slack/oauth.rs`, `oauth/tests.rs` | Ordered eleven user scopes, exact authorize URL/manifest JSON, Rails-signed user-bound state, fixed host/timeouts, form exchange/revoke and team lookup. Ruby truthiness/presence/structural response behavior and class-only transport failures match Rails vectors. |
| `integrations/slack/connections.rs` | Grant validation, workspace mismatch/missing scopes, other-user and unique-index conflicts, encrypted relink, first-administrator workspace naming, active-own-run disconnect restriction, best-effort revocation and audit/association cleanup. Remote work occurs outside the SQLite writer. |
| `crates/db/src/models/slack.rs` | Validated encrypted configure/relink, blank-secret preservation, unchanged-plaintext ciphertext preservation, team naming and Rails callback-free credential removal. Reuse WS1 Active Record encryption. |
| `crates/campfire/src/controllers/slack.rs`, `controllers/slack/setup.rs`, `crates/campfire/src/controllers.rs` | Six routed actions: OAuth start/callback, connection destroy, administrator setup show/update/destroy. Rails authorization/sudo order, two-path return allowlist, one-use state, CSRF, redirects/flashes, validation response and filtered sensitive parameters. |
| `crates/views/src/slack.rs`, `crates/views/src/lib.rs`, `crates/views/templates/accounts/slack_imports/show.html` | Separate presentation data and exact Rails setup body, including manifest, credential errors/write-only secret, connected/rejected/active states and controls. Comparisons cover the complete template body, not the complete application layout/nav. |
| `controllers/slack/tests.rs`, `controllers/slack/test_support.rs` | Authenticated real router/session tests and local TLS transport: 37 actual Rails HTTP state/audit/request scenarios, eleven byte goldens, state replay/owner/sudo/CSRF checks and actual SQLite unique rejection. Producers are inspected with fixture workers stopped; the earlier real worker tests remain active. |
| `reference-tools/slack/{oauth_vectors,connections_http,setup_views}.rb`, `vectors/slack/{oauth,connections_http,setup_views}.json` | Actual pinned Rails methods/controllers/templates; only transport and shared nondeterministic inputs are supplied. Forty-one transport cases, four URLs and signed state; 37 HTTP cases; eleven full setup bodies. |
| `reference-tools/slack/check_oauth_mutations.py` | Inject four faults, require runtime assertion failures, restore all sources. |
| `reference-tools/slack/sequence_vectors.rb`, `vectors/slack/sequence*.json` | Runtime header construction and newly executed Rails import → undo → reimport oracles. Every value, including fresh encrypted credentials, is retained. |
| `plans/ws16-test-inventory.md`, `reference-tools/slack/write_test_inventory.py` | Map the 38 newly covered OAuth/connection/setup Rails declarations; total 133 covered, 10 partial, 101 deferred out of 244. |
| `crates/db/src/models/{user/destruction.rs,user.rs,channel_thread.rs,message.rs}`, periodic tests | Merge adapters reuse main's hard-removal API and preserve the quiet importer path and every exact scheduler entry. Obsolete `user/lifecycle.rs` removed. Huddle callback completion remains below. |

## Behavior and limits

State uses Rails' existing verifier and session ownership. Start requires sudo; callback consumes both stored state and return path before rejection/exchange. Tamper, wrong owner/verifier/purpose, explicitly expired signatures and replay are rejected. Rails issues ordinary state without a newly invented TTL. Only the two internal Rails return paths are accepted. CSRF protects credential writes/disconnect; OAuth callback follows Rails' exemption/state contract.

Connection and workspace secrets remain Active Record encrypted. Missing scopes, wrong workspace and another user's account are rejected with Rails' exact text. The real SQLite unique-error rescue is tested beyond the precheck. Reconnect clears the disconnect reason and does not rotate an unchanged encrypted plaintext. Disconnect tolerates unreadable grants/revoke failure and still removes the local link; only the user's own active run blocks it. Setup credential removal blocks on any active run and uses Rails' callback-free connection deletion, preserving historical runs and their original connection IDs.

Rails' save/after-save boundaries are retained: a malformed scalar team-info response returns the production 500 after the connection has committed; a reconnect over an unreadable old encrypted token returns 500 before overwriting it. Both were obtained from the actual Rails HTTP oracle. Audits include the actor/IP/user-agent but exclude codes, tokens and secrets.

The 41 OAuth transport vectors include seven Rails exception classes. Their Rust class conversion is unit-tested; the other 34 cases traverse real local TLS and compare method/path/form/header behavior. This does not claim every arbitrary network-error shape is finished. The eleven template comparisons fix only shared CSRF inputs, with no HTML normalization. Whole-page and interaction parity for the remaining run/personal pages is still pending.

The retained two DB comparisons execute the real Rust runner/store/undoer against the Rails initial rows/pages and compare every row/field after import, undo and reimport: 89 tables × three snapshots each. JSON columns compare as JSON values; primary-key sorting is presentation only. Framework schema bookkeeping and FTS shadow tables are excluded; `sqlite_sequence` and the real search table are included. No new comparison exclusions were introduced.

## Exact remaining scope — WS16 continuation

1. **Administrator run controllers:** all nine actions (`index`, `create`, `show`, `status`, `plan`, `start_import`, `catch_up`, `cancel`, `undo`) and their 33 named Rails declarations. This includes date parsing/full-day bounds, selected conversation filtering, alive Open/Closed target filtering, connected/global-active checks, completed-preview checks, catch-up eligibility, cancellation/undo rejection and exact messages.
2. **Personal import controllers:** all six actions (`index`, `create`, `show`, `status`, `cancel`, `undo`) including both create-mode branches, and their 20 named declarations. Workspace readiness, per-user ownership/404s, per-user single-flight with global queuing, completed-preview/selection checks, no personal dates/room targets, personal plan/progress and control eligibility remain.
3. **Run/personal rendering and behavior:** remaining administrator and personal templates/shared status/plan rendering, complete application responses, issue pagination, progress polling/finished-frame markers, queued-behind explanations, escaped samples/targets/checkboxes, opt-in and placeholder-claim interaction, cancel/undo controls, and the one Rails system behavior declaration. Pixel work is excluded by user decision.
4. **Domain/error completion:** the ledger's ten partial and 47 deferred non-controller declarations: options/date/Ruby coercion normalization; malformed converter/client/mapper payloads; exact non-OAuth transport/error/backoff and retry accounting; catch-up/new/late/bounded/multi-step history and coverage invalidation; deleted mappings/parents/rooms; large MPIM/dry previews; membership rollback; finishing all earlier-written rooms/forward-only pointers/heartbeats/kicks; retry/scope/exhaustion/failed-resume cases; truncated reactions and identity seek regressions.
5. **Retained undo and huddle:** session/Google/authorship claims, retained reimport mappings, rooms with events/schedules, saved/poll/foreign-thread/sent-vs-pending schedule variants, and remaining model validation/lifecycle declarations. `destroy_for_slack_undo` now reuses main's shared User removal operation, but its huddle phase still persists revocation without the full huddle stream/presence/remote cleanup callbacks. Integrate the relevant huddle domain seam and compare its effects. WS16 owns the continuation; this is not silently reassigned to WS13.

The ledger lists every remaining declaration individually. Covered is a mapping to executable behavior, not a claim that the original Rails test classes ran against Rust. Counts by file: admin runs 33 deferred; setup 10 covered; disconnect 6 covered; personal imports 20 deferred; OAuth 22 covered; run lifecycle 16 covered/3 partial/10 deferred; workspace import 13/2/19; client 14/4/0; converter 26/0/4; SlackImport model 26/1/14; system 0/0/1.

## Cross-workstream boundaries

WS1 encryption and verifier APIs are reused. WS3 owns the durable queue; the earlier serial Slack worker and atomic continuation writes are retained. Main's Room/Message/Thread/Event/Agent callbacks are preserved and consumed rather than copied. The importer adds only its imported quiet variants and User dependency adapter. The setup renderer uses the merged view/CSRF/page APIs. No product decision or approval question is open. The huddle adapter above is still incomplete owned integration work.

## Verification

Fresh GitHub clone: `.scratch/ws16-oct1`, no alternates. Fresh `default`/`first_run` seeds were built there from an archive of the pinned Rails commit and validated with the Rails seed validator. `CI=1` makes missing seeds fail. Tests use isolated Docker networking, offline Cargo, the pinned image below, host machine-wide rustc slots, two Cargo jobs and at most eight test threads. The full test command excludes vendored `html5ever` as documented in `rust/AGENTS.md`; final clippy includes the entire workspace and all targets.

Image: `sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2` (Rust 1.98.1, libvips 8.16.1, ffmpeg 7.1.5). The strict media prerequisite/pipeline is exercised by the workspace suite. The release-input guard runs an actual binary build with only crates/ and the explicit assets context; vectors/parity/reference-tools are unavailable to production.

Native wrapper `.scratch/pinned-oct1.sh` and compiler wrapper:

```sh
#!/bin/bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
name="$1"; shift
docker run --rm --name "$name" --network none --cpus 4 --user 1000:1000 \
  -v "$root/.scratch/ws16-oct1:/src" \
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

Fresh clone/seed/reference commands and raw outputs:

```sh
git clone --single-branch --branch rust/ws16-slack-import https://github.com/Smart-Data-Ohio/smartfire.git .scratch/ws16-oct1
git archive d7c7de9264c63015be398001d7a1094e7695a6db | tar -x -C .scratch/ws16-oct1/rust/parity/.ci/reference
PARITY_NAMESPACE=ws16-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-oct1/rust/parity/.ci/reference" .scratch/ws16-oct1/rust/parity/bin/seed build default first_run
PARITY_NAMESPACE=ws16-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-oct1/rust/parity/.ci/reference" .scratch/ws16-oct1/rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze .scratch/ws16-oct1/rust/reference-tools/campfire/verify_parity_seed.rb default
PARITY_NAMESPACE=ws16-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-oct1/rust/parity/.ci/reference" .scratch/ws16-oct1/rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze .scratch/ws16-oct1/rust/reference-tools/campfire/verify_parity_seed.rb first_run
python3 .scratch/ws16-oct1/rust/reference-tools/slack/check_reference.py
python3 rust/reference-tools/slack/write_test_inventory.py
PARITY_NAMESPACE=ws16-oauth-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/slack/oauth_vectors.rb
PARITY_NAMESPACE=ws16-connections-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-01-01T12:00:00Z --freeze rust/reference-tools/slack/connections_http.rb
PARITY_NAMESPACE=ws16-setup-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-01-01T12:00:00Z --freeze rust/reference-tools/slack/setup_views.rb
PARITY_NAMESPACE=ws16-sequence-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/slack/sequence_vectors.rb
PARITY_NAMESPACE=ws16-sequence-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/rust/parity/.ci/reference" rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/slack/sequence_vectors.rb personal
```

The archive/clone/metadata operations completed with exit 0; clone's initial HEAD was `52ee3ff3`, then fast-forwarded from the remote to verified source `ab454008`. Required destination directories were created first. Raw producer/seed summary lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Slack Rails reference: 21 source files match d7c7de9264c63015be398001d7a1094e7695a6db; no Slack drift on origin/main
Slack Rails test inventory: 133 covered, 10 partial, 101 deferred; 244 total
Slack OAuth vectors: 41 real Rails exchange/revoke/team cases, 4 authorization URLs, manifest and signed state generated
Slack connection HTTP oracle: 37 real Rails callback/disconnect/setup/remove cases generated
Slack setup views: 11 complete Rails template bodies generated with shared deterministic CSRF inputs
Slack Rails sequence (workspace): import -> undo -> reimport; 89 tables per snapshot; 20 recorded API requests
Slack Rails sequence (personal): import -> undo -> reimport; 89 tables per snapshot; 18 recorded API requests
```

Rails seed validator raw summary fields, default then first_run:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```


Security discrimination (assigned worktree, before final positive fresh-clone checks):

```sh
python3 rust/reference-tools/slack/check_oauth_mutations.py
```

```text
test integrations::slack::oauth::tests::slack_oauth_manifest_urls_and_state_match_real_rails ... FAILED
test integrations::slack::oauth::tests::slack_oauth_transport_errors_never_include_details ... FAILED
test integrations::slack::oauth::tests::slack_oauth_exchange_revoke_and_team_info_match_rails_through_real_tls ... FAILED
test controllers::slack::tests::slack_connections_http_persistence_audits_and_requests_match_rails ... FAILED
test controllers::slack::tests::slack_setup_views_are_byte_identical_to_rails_and_write_only ... FAILED
test result: FAILED. 52 passed; 5 failed; 0 ignored; 0 measured; 1422 filtered out; finished in 27.15s
Slack OAuth mutation guards: wrong owner, transport class, scope grant, and view bytes rejected; source restored
```

The two HTTP start/state tests also failed before route wiring with 501 versus expected 302; raw summary:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1474 filtered out; finished in 1.73s
```

Fresh-clone native commands/results, complete workspace summaries, clippy, release build, encryption readback and cleanup:

Verified source: `ab454008b5659b0b70e08414a2899ad41c44a9e0`, tracked source clean in the independent clone. Full workspace exit 0: **2,938 passed, 0 failed, 11 explicit ignores across 58 summary blocks**. All **81 owned tests** ran (57 app + 24 DB), without owned ignores or missing-seed skips. The pinned media pipeline passes. Clippy and the crates-only production binary build both exit 0. Rails decrypted and validated both newly exported Rust ciphertexts.

First full workspace command at `52ee3ff3` (exit 101):

```sh
.scratch/pinned-oct1.sh ws16-oct1-suite cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

```text
Pinned workspace totals: 58 summary blocks; 2937 passed; 1 failed; 11 ignored
test result: FAILED. 1476 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 655.86s
```

The sole failure was the merged thread-page producer expecting one queued GitHub refresh but reading zero while its live worker consumed it. A pristine archive of `origin/main` at `434d1c14` was built with the same fresh seeds, image, offline settings and machine slot loop. Baseline used the same sole scratch target (mounted at `/src/rust/target`), not a second compiler cache. Its DB/views/app were recompiled from main; the exact failure reproduced:

```sh
.scratch/pinned-main-oct1.sh ws16-oct1-main-pr cargo test --offline --locked -p campfire pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure -- --nocapture --test-threads=8
```

```text
    Finished `test` profile [unoptimized] target(s) in 1m 25s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1421 filtered out; finished in 0.94s
```

The baseline wrapper is the native wrapper above with the source mount changed to `.scratch/ws16-main-oct1:/src` and an additional `.scratch/ws16-oct1/rust/target:/src/rust/target` bind. The pristine baseline was created with `git archive origin/main | tar -x -C .scratch/ws16-main-oct1`; both fresh seeds were copied from the independent clone.

On switching back, one focused build reused main's metadata at the same logical paths and failed compilation with 42 missing Slack/imported API errors (no tests ran). This cache mistake was corrected by cleaning the three packages whose sources differ; no production source workaround was added. Then the fixed fixture passed, preserving every original assertion:

```sh
.scratch/pinned-oct1.sh ws16-oct1-pr-fixed cargo test --offline --locked -p campfire pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure -- --nocapture --test-threads=8
.scratch/pinned-oct1.sh ws16-oct1-clean cargo clean --offline --locked -p campfire_db -p campfire_views -p campfire
.scratch/pinned-oct1.sh ws16-oct1-pr-fixed-rebuilt cargo test --offline --locked -p campfire pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure -- --nocapture --test-threads=8
```

```text
error: could not compile `campfire` (bin "campfire" test) due to 42 previous errors
     Removed 3636 files, 6.6GiB total
    Finished `test` profile [unoptimized] target(s) in 1m 30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1478 filtered out; finished in 0.99s
```

Final full workspace, full clippy, restricted-input binary build, locked metadata and owned checks:

```sh
.scratch/pinned-oct1.sh ws16-oct1-suite-verified cargo test --offline --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 .scratch/summarize-suite.py .scratch/oct1-suite-verified.log
.scratch/pinned-oct1.sh ws16-oct1-clippy cargo clippy --offline --locked --workspace --all-targets -- -D warnings
.scratch/pinned-oct1.sh ws16-oct1-release bash ci/with-release-inputs.sh cargo build --offline --locked --bin campfire
.scratch/pinned-oct1.sh ws16-oct1-metadata-final cargo metadata --offline --locked --format-version 1
.scratch/pinned-oct1.sh ws16-oct1-db env SLACK_RUST_CRYPTO_OUTPUT=/src/.scratch/crypto-oct1/db/rust-slack-crypto.json cargo test --offline --locked -p campfire_db --lib slack_ -- --nocapture --test-threads=8
.scratch/pinned-oct1.sh ws16-oct1-slack cargo test --offline --locked -p campfire slack_ -- --nocapture --test-threads=8
```

The summarizer totals every libtest line, rejects failures/filtered full-run tests and detects missing-seed skips. It introduces no comparison mask. Every raw final workspace summary and explicit ignore, in execution order:

```text
Pinned workspace totals: 58 summary blocks; 2938 passed; 0 failed; 11 ignored
Raw libtest summaries:
test result: ok. 1477 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 657.44s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.04s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 763 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 72.13s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.68s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.15s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 29.98s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.19s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.51s
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

Raw clippy, restricted-input build, metadata validator and focused owned outputs:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 04s
    Finished `dev` profile [unoptimized] target(s) in 1m 39s
Locked/offline metadata: valid; workspace members: 13
    Finished `test` profile [unoptimized] target(s) in 42.38s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 743 filtered out; finished in 2.42s
    Finished `test` profile [unoptimized] target(s) in 27.37s
Slack DB differential (personal): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
Slack DB differential (workspace): import -> undo -> reimport; 89 tables x 3 snapshots; every row and field matched
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 1422 filtered out; finished in 65.26s
```

Reverse encrypted readback: create the export directory before the DB command, copy its newly exported JSON and the fresh clone's default seed to `.scratch/crypto-oct1/db/{rust-slack-crypto.json,production.sqlite3}`, then run:

```sh
PARITY_NAMESPACE=ws16-crypto-oct1 PARITY_OWNER=ws16 PARITY_IMAGE=ws16-reference:d7c7de92 CAMPFIRE_REFERENCE="$PWD/.scratch/ws16-oct1/rust/parity/.ci/reference" .scratch/ws16-oct1/rust/parity/bin/reference runner --storage "$PWD/.scratch/crypto-oct1" .scratch/ws16-oct1/rust/reference-tools/slack/verify_rust_crypto.rb
```

```text
Slack encryption readback: Rails decrypted and validated 2 Rust-written columns
```

Cleanup and final remote readback:

The scratch target and empty baseline mount point were removed. Native Cargo cleanup was used, followed by checked removal of empty target directories and a recursive scratch target/container inventory. The original worktree's normal `rust/target` cache was preserved. No verification process/listener remains.

```sh
.scratch/pinned-oct1.sh ws16-oct1-clean-final cargo clean --offline --locked
```

```text
     Removed 24126 files, 13.1GiB total
Scratch Cargo targets remaining: 0
WS16 verification containers running: 0
```

Both the fresh clone and assigned worktree had clean tracked source before the report-only update. Final push/ref synchronization is checked after committing this report; its SHA is supplied in the final worker reply. No source changes follow verified `ab454008`.

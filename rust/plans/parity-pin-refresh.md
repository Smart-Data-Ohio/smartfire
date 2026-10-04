# Parity pin refresh to 78b9b1546

The oracle now uses `78b9b1546bdab4c6c1c9b8ddb94512f661289112`, replacing
`d7c7de9264c63015be398001d7a1094e7695a6db`. The plain pinned image includes
#226, #228, #231 and #235. All changed captures came from the repository's Rails
producers, vector scripts, seed DSL or assets exporter; no golden payload was
edited by hand. The Rails application remains unchanged by this work.

## Provenance and overlays

`PARITY_REFERENCE_SHA` is the inspected image's Rails revision. Current-pin
producer entrypoints check it against `parity/reference.sha`; the general harness
also records the actual revision for intentional old/new controls.
`GIT_REVISION=parity` remains the harness's stable HTTP `X-Rev` input. Direct Docker
producers use the same fail-closed revision check.
The profile source ledger is produced from the actual image, including the
native `_two_factor.html.erb` and reauthentication partials.

Removed the attachment-processing image overlay and the approved board, Slack,
event, profile and UI layout substitutions. Verified test declarations needed by
the Google/huddle producers are read-only oracle inputs, not application source
overlays. Source guards compare those copies with the actual pin.

`ci-seed` retains its general checkout overlay of `db/schema.rb` and `db/migrate`.
All 130 schema/migration inputs match this pin, so it is currently a no-op. It is
still useful when a compatible Rails migration lands before a future pin bump;
the image/seed fingerprints invalidate on those checkout inputs. Incompatible
schema changes or migrations needing newer application code require a pin bump
and revalidation, as documented in `parity/seeds/README.md`.

## Changed-golden attribution

Most families had already adopted approved Rails files selectively. Their new
captures change only provenance, while their functional output stays identical.

| Family | Actual output change | Rails commit explaining it |
|---|---|---|
| Agent attachment diagnostics | Successful committed state and observed transaction count replace the original crash diagnosis | #226, `2d7c65b7f`, `6b38d1274`, `c75ee7f95` (merge `b06d19114`) |
| Messaging: thread review, rendered dependencies, paging, legacy/page validators | Missing-file post commits with 201; blob lease columns appear; ActionView dependency digests and their ETags change | #226, `2d7c65b7f`, `6b38d1274`, `c75ee7f95` |
| Room/form pages and agents/history/bots/access UI pages | 99 complete pages change only `people.css` and `profile_card_controller.js` URLs | #163, `8b13a68fd` (merge `2e20b24c3`) |
| WS17 settings views and status requests | 12 status partial strings use the shared fields' whitespace; five request cases include the additional badge broadcasts | #163, `8b13a68fd` |
| Profile source ledger | Native two-factor/reauthentication hashes, plus explicit shared-fields/layout/route coverage; rendered profile output already matched | #228, `ef92c656e`; #235, `bc42efa12`; shared fields from #163, `8b13a68fd` |
| Browser/test declaration ledgers | New source identities; application output otherwise identical | Board/layout fixes #164 `c368b7bc5`, #165 `45fb1e428`, and subsequent test deflakes #178 `20dc8ea3c`, #208 `b0f96a71b`, #213 `453a910da`, #217 `941a5ef4a` |
| Markdown | All 4,200 output cases unchanged; bundle source identity changes | #159, `e5b2a29c0` (rubyzip 3.7.0) |
| Users/rooms, boards/work, Slack, Google, remaining agents, huddle/WS13b, WS15e embeds/integrations, authentication, remaining WS17 | Pin/approved-overlay provenance becomes the actual plain pin; functional captures unchanged | Earlier selective captures already included their relevant Rails fixes; no additional behavior change |
| Assets manifest/static responses/vendor ledger | Re-exported together; manifest entry ordering and provenance change, with identical parsed mappings and asset bytes | No application-byte change: #228/#231/#235 assets were already vendored on main |

Two metadata corrections are not Rails behavior changes: the old Markdown helper
hash was stale even against the old pin, and the assets manifest's filesystem
entry order is not an application contract. Fresh producers now record the actual
helper hash and the exact hash of the simultaneously exported manifest.

The regenerated manifest SHA-256 and its static-response record both equal
`e2b8050840b6987af285adbd6ecb654c005d989227f26198e8f7427c13cb14c1`.

Rust now includes the generated index template digest rather than maintaining a
separate literal. The fresh status-request oracle also exposed an unported #163
callback: successful changes to Rails' four `STATUS_ATTRIBUTES` now announce the
badge through the existing after-commit event path. The initial workspace control
failed nine tests (four validators and five status-broadcast cases) before these
two corrections. A later view-only control found one legacy settings renderer
still duplicating the pre-#163 fields; it now reuses the existing shared fields
template, and all 17 settings tests match the new native partials.

## Reproducible fixture inputs

Random output and cache timing are not attributed to a Rails commit. Scoped
producer inputs replay the original encryption IVs and webhook secret before
Rails encrypts/serializes/persists. They assert draw counts; no ciphertext output
is substituted. The generic Recorder replays only three persisted membership
timestamp pairs and five encrypted seed inputs, verifies identical decrypted
plaintext, then calls the real Recorder. The stage lifecycle probe replays its
original UUID and message clock inputs before the real note is created. These
seven families retain their original complete non-pin output.

Query probes declare query-cache and icon-cache conditions before measurement.
The recorder's measured write is uncached, reproducing its original 30/76/136
queries on both pins. The settings probes declare their original warm/cold icon
cache fixture states. Raw SQL remains measured; no count is injected, subtracted
or normalized. Captures fail if the one-second icon-cache TTL expires unexpectedly.

`older_embed_wire_orders.json` retains its truthful historical pin: it is an
archived Redis arrival-order sample replayed against fresh frame payloads, not a
new-pin publication-order golden. Likewise the unkeyed web-push encryption sample
is retained; regenerating its random receiver/sender inputs is a separate opt-in
operation. Historical reports and discriminator evidence retain their provenance.

## Reproduction and verification

Commands below run from `rust/`, with `PARITY_IMAGE` naming the rebuilt plain image.

```sh
parity/bin/ci-seed prepare
parity/bin/ci-seed image
parity/bin/ci-seed build
parity/bin/ci-seed validate
REFERENCE_IMAGE="$PARITY_IMAGE" REFERENCE_SHA="$(cat parity/reference.sha)" crates/assets/script/revendor

WS8BR2_REGENERATE=1 reference-tools/users/run_oracles.sh
python3 reference-tools/users/verify_room_merge.py --write
python3 reference-tools/messaging/check-goldens.py --write
WS8BM2_REGENERATE=1 python3 reference-tools/messaging/verify_oracles.py
python3 reference-tools/messaging/verify_oracles.py

python3 reference-tools/agents/record-http-vectors.py /tmp/agent-http --write
python3 reference-tools/agents/regenerate-domain.py /tmp/agent-domain --write
python3 reference-tools/agents/regenerate-aggregates.py /tmp/agent-aggregates --write
python3 reference-tools/google/regenerate.py /tmp/google --write
python3 reference-tools/google/regenerate-calendar.py --log-dir /tmp/calendar
python3 reference-tools/ws13_verify_corpora.py --output /tmp/ws13 --write
python3 reference-tools/ws13b_regenerate.py /tmp/ws13b --write
python3 reference-tools/ws17_regenerate_all.py /tmp/ws17

reference-tools/auth/generate.sh
reference-tools/markdown/verify-reference.sh --write
reference-tools/markdown/run.sh

python3 parity/test_ci_seed.py
cargo test --locked --workspace --exclude html5ever -- --test-threads=8
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

The boards/work, events, Slack, GitHub, embeds and UI families were additionally
captured with their individual `reference-tools/` producers. Attachment processing
used its plain-image wrapper for all three existing corpora, each byte-identical
to its committed vector. Independent checks cover 36 user oracles, 19 room corpora,
44 messaging oracles/46 files, 60 WS8bm2 replays and 721 Rails/Stimulus huddle cases.

The `default`, `first_run` and `agents_ui` seeds passed 29, 4 and 40 Rails validation
checks respectively, with zero failures. Media byte tests passed inside both the rebuilt pinned Rails image and the
CI toolchain image (11/11 storage vector tests). Workspace tests execute in the CI
image with `CI=1` and all three seeds present; Cargo builds retain the host rustc
throttle, one build at a time and eight test threads at most.

The complete workspace command exited successfully: 4,948 passed, zero failed
and 22 ignored across 59 test summaries (including doctests). The Docker runner
uses a mounted temporary directory for host-compiled doctest binaries.

Raw workspace summaries:

```text
test result: ok. 2837 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 486.54s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.11s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1379 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 83.34s
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.61s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.55s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.39s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.66s
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

Other raw gate summaries:

```text
WS8bm2 oracle replay: 60/60 independently replayed fixtures byte-identical
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.89s
Ran 11 tests in 1.517s
OK
Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.92s
```

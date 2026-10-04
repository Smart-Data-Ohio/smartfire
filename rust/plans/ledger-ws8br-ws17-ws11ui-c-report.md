# Original assertion reconciliation — C checkpoint

Stacked on PR #244's assertion-audit head `f05c3f2a5146a586a80149840f79f2ea52c6d87c`.
Its 120-declaration audit preserves 112 closures and eight reopened records, rather
than claiming missing assertions passed. This checkpoint closes all eight reopened declarations, including the Node subprocess exit/output assertion through the served service-worker action.

The first C batch closes 83 broad original declarations and all eight sidebar
original declarations: 91 declarations, 269 direct assertion sites and 15 private
PNG-helper assertion expansions. The JSON manifest and human table name every
original assertion and its actual discriminating assertion file:line, including
multi-step requests, exact reloaded audit fields/count deltas, media headers/bytes,
permission statuses, redirect destinations, CSV fields, and complete scoped DOM.
Historical file-level Rails runs remain historical; new enabled Rust test identities
are checked against actual current nextest output and the current test list.

Two missing behaviours are fixed: configured sidebar DM rows now use Rails'
membership/participant collection cache, preserving a renamed peer until the
participant key changes; icon creation/deletion audits now store `:shortcode:`
labels as Rails does. The original sidebar cache sequence and Acme upload/list/
delete test failed before these fixes. No assets or Rails production code changed.

Actual producer controls rejected 40 distinct tests in the first batch, 29 in the
second and five in the third. The header test overlaps these groups; totals are
reported by batch, not as unique counts. Compilation/transport/setup failures are
invalid controls. A partial-only Google fragment mutation survived because another
real status partial correctly rendered the same email; that control was rejected
as insufficient, then replaced with a mutation of the common email reader. All five
profile/header defects now fail the intended original assertions. Source restoration
is guaranteed in each control's `finally` block.

The mixed quiet-room request stays flat: Rust 23 → 23 SELECTs, Rails 22 → 22;
HuddleGrant source reads stay at one. These are the original total-read-growth
and single-grant-query clauses; the one fixed additional Rust read is explicit.

Remaining after the 120-declaration checkpoint: 104 broad browser declarations, no sidebar receipts, six overlapping criteria,
the muted-room and Calendar sequences, and aggregate mappings. All are individually
listed in the remaining manifest. The 81 API comparisons remain with WS11-API;
the three geometry-only exclusions retain their existing dispositions. No missing
acceptance evidence is relabelled an absent domain API.

Completed commands and fresh-clone verification follow at the final checkpoint.

First batch restored baseline and discrimination receipts:

```text
     Summary [  43.923s] 108 tests run: 108 passed, 4958 skipped
Controller per-assertion receipts: 91 audited declarations (91 closed, 0 reopened); 269 assertion sites; 80 enabled native test identities passed; 0 explicit reopened gaps; 0 unaccounted assertions
C receipt discrimination: 19 producer mutations; 40 distinct tests rejected; 0 invalid controls
C audit/media discrimination: 17 producer mutations; 29 distinct tests rejected; 0 invalid controls
C profile discrimination: 5 producer mutations; 5 tests rejected; 0 invalid controls
```

Commands: the three committed `check-controller-c-*-controls.py` scripts; restored
`cargo nextest run --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever -j 4 --no-fail-fast -E 'test(original_) or test(controllers::users::ban_lifecycle_tests) or test(controllers::accounts::icons::tests) or test(controllers::accounts::logos::tests) or test(ws15e_rails_composer) or test(controllers::public_pages::tests::public_page_bodies) or test(controllers::users::profile_settings_tests)'`;
`cargo nextest list --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --message-format json`;
`python3 rust/reference-tools/check-controller-assertion-receipts.py --manifest rust/plans/ledger-ws8br-ws17-ws11ui-c-receipts.json --nextest-list <list.json> --native-log <restored.log>`.

Second batch adds the two original per-test CSV export-cap overrides and 27
browser declarations: 18 people/group/picker/mobile interactions, four tours,
four starred-people interactions, and the original Node event-harness receipt.
The total is **120 declarations** (112 broad and eight sidebar), **420 direct
Rails assertion sites**, **54 private-helper expansions**, and four setup
assertions. The controller manifest accounts for 93 declarations/279 sites and
82 distinct enabled native identities. The browser manifest accounts for 27
original declarations/141 sites through seven explicitly enabled ignored tests
in `rust/parity/system/ws12` on the `rust/ci-full-gate` workflow. They are not
claimed as ordinary toolchain-CI tests. Both targets execute all 27 cases;
stateful DM recipients/canonical destinations, group departure and tour stamps
are checked on each actual server database after the browser action.

The new browser driver uses the existing network-none container and unchanged
HTTP/WebSocket byte-forwarder. This causally avoids Chromium's host-namespace
`ERR_NETWORK_CHANGED` when other workers start Docker containers. An unrelated
seed rich-text preview pointing to an external Twitter image is made inert on
both targets; these original cases do not exercise that preview. The original
0.7-second touch hold and finite member-panel animation condition are preserved.
No retries or widened deadlines were added. Visible label versus accessible-name
selector mistakes were corrected in the harness, with the original assertions
unchanged. The star controls mutate the actual polled members payload, preserving
all other fields and the real HTTP producer, so periodic refresh cannot hide the
defect. Invalid/surviving controls (including an initially selected hidden picker
controller, a nonexistent refresh method and an unconfigured mutant queue) are
excluded and were replaced by controls that reach the intended assertions.

The aggregate mappings retain their history and explicitly credit the eight
sidebar controller originals while keeping the phone/header/member/pins browser
aggregates partial. None are closed by a file-level controller receipt. The six
overlapping criteria still needing acceptance evidence are group-huddle ringing,
colon brand-icon autocomplete in both themes, room-icon sidebar/header rendering,
icon-room search-arrow suppression, LobeHub icons in both themes, and workspace
icon upload/post/delete fallback in both themes. Real owner APIs are present;
these are missing named acceptance receipts, not absent-owner API claims.
The 104 exact broad declarations are listed by name in the remaining JSON.
Muted-room noise -> mention delivery and Calendar injected-clock busy -> clear
remain separate missing sequences. The 81 WS11-API comparisons stay with their
owner; the three geometry-only exclusions retain their prior dispositions.

Fresh-clone test, clippy, release-input and complete browser gate summaries follow
below after verification. The only seeds built for that clone are `default`,
`first_run` and `agents_ui`, matching `.github/workflows/rust.yml`.

Additional restored-baseline and actual producer-control summaries:

```text
     Summary [   1.358s] 2 tests run: 2 passed, 2950 skipped
Original audit cap discrimination: 3 producer mutations; 2 distinct tests rejected; 0 invalid controls
C lifecycle discrimination: 6 producer mutations; 6 tests rejected; 0 invalid controls
Original browser people: 10 producer defects rejected; 0 invalid controls
Original browser pickers: 5 producer defects rejected; 0 invalid controls
Original browser members: 2 producer defects rejected; 0 invalid controls
Original browser group: 1 producer defects rejected; 0 invalid controls
Original browser tours: 4 producer defects rejected; 0 invalid controls
Original browser stars: 4 producer defects rejected; 0 invalid controls
Original browser worker: 1 producer defects rejected; 0 invalid controls
```

The four native control groups reject all 80 first-batch native identities; the
cap controls add both new identities. Exact assertions are preserved after
restoring producer bytes. CSV limit parameterization is a pure extraction; its
per-request override is compiled only in tests and follows the normal router,
CSRF, permissions, sudo, selection and rendering path.

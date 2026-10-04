# Original assertion reconciliation — C checkpoint

Stacked on PR #244's assertion-audit head `f05c3f2a5146a586a80149840f79f2ea52c6d87c`.
Its 120-declaration audit preserves 112 closures and eight reopened records, rather
than claiming missing assertions passed. This checkpoint closes seven of those
reopened declarations; the Node subprocess exit/output declaration remains open.

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

Remaining at this batch: 133 broad declarations (130 browser declarations and
three controller receipts: the two original reduced CSV-cap overrides and the Node
subprocess exit/output receipt), no sidebar receipts, 14 overlapping criteria,
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
